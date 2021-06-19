use crate::external_file::{ExternalFile, ExternalFilesById};
use crate::reader::AseReader;
use crate::tileset::{Tileset, TilesetsById};
use crate::{error::AsepriteParseError, AsepriteFile, PixelFormat};
use log::debug;
use std::io::{Read, Seek};

use crate::Result;
use crate::{cel, color_profile, layer, palette, slice, tags, user_data, Tag};

struct ParseInfo {
    palette: Option<palette::ColorPalette>,
    color_profile: Option<color_profile::ColorProfile>,
    layers: Option<layer::LayersData>,
    framedata: cel::CelsData, // Vec<Vec<cel::RawCel>>,
    frame_times: Vec<u16>,
    tags: Option<Vec<Tag>>,
    external_files: ExternalFilesById,
    tileset: TilesetsById,
}

impl ParseInfo {
    fn add_cel(&mut self, frame_id: u16, cel: cel::RawCel) -> Result<()> {
        self.framedata.add_cel(frame_id, cel)
        // let idx = frame_id as usize;
        // self.framedata[idx].push(cel);
    }
    fn add_external_files(&mut self, files: Vec<ExternalFile>) {
        for external_file in files {
            self.external_files.add(external_file);
        }
    }
}

// file format docs: https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md
// v1.3 spec diff doc: https://gist.github.com/dacap/35f3b54fbcd021d099e0166a4f295bab
pub fn read_aseprite<R: Read + Seek>(input: R) -> Result<AsepriteFile> {
    let mut reader = AseReader::with(input);
    let _size = reader.dword()?;
    let magic_number = reader.word()?;
    if magic_number != 0xA5E0 {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Invalid magic number for header: {:x} != {:x}",
            magic_number, 0xA5E0
        )));
    }

    let num_frames = reader.word()?;
    let width = reader.word()?;
    let height = reader.word()?;
    let color_depth = reader.word()?;
    let _flags = reader.dword()?;
    let default_frame_time = reader.word()?;
    let _placeholder1 = reader.dword()?;
    let _placeholder2 = reader.dword()?;
    let transparent_color_index = reader.byte()?;
    let _ignore1 = reader.byte()?;
    let _ignore2 = reader.word()?;
    let _num_colors = reader.word()?;
    let pixel_width = reader.byte()?;
    let pixel_height = reader.byte()?;
    let _grid_x = reader.short()?;
    let _grid_y = reader.short()?;
    let _grid_width = reader.word()?;
    let _grid_height = reader.word()?;
    let mut rest = [0_u8; 84];
    reader.read_exact(&mut rest)?;

    if !(pixel_width == 1 && pixel_height == 1) {
        return Err(AsepriteParseError::UnsupportedFeature(
            "Only pixel width:height ratio of 1:1 supported".to_owned(),
        ));
    }

    let framedata = cel::CelsData::new(num_frames as u32);
    // let mut framedata = Vec::with_capacity(num_frames as usize);
    // framedata.resize_with(num_frames as usize, Vec::new);
    let mut parse_info = ParseInfo {
        palette: None,
        color_profile: None,
        layers: None,
        framedata,
        frame_times: vec![default_frame_time; num_frames as usize],
        tags: None,
        external_files: ExternalFilesById::new(),
        tileset: TilesetsById::new(),
    };

    let pixel_format = parse_pixel_format(color_depth, transparent_color_index)?;

    for frame_id in 0..num_frames {
        // println!("--- Frame {} -------", frame_id);
        parse_frame(&mut reader, frame_id, pixel_format, &mut parse_info)?;
    }

    let layers = parse_info
        .layers
        .ok_or_else(|| AsepriteParseError::InvalidInput("No layers found".to_owned()))?;

    // println!("==== Layers ====\n{:#?}", layers);
    // println!("{:#?}", parse_info.framedata);

    // println!("bytes: {}, size: {}x{}", size, width, height);
    // println!("color_depth: {}, num_colors: {}", color_depth, num_colors);

    //println!("framedata: {:#?}", parse_info.framedata);
    match pixel_format {
        PixelFormat::Rgba => {}
        PixelFormat::Grayscale => {}
        PixelFormat::Indexed {
            transparent_color_index,
        } => {
            if let Some(ref palette) = parse_info.palette {
                parse_info
                    .framedata
                    .resolve_palette(palette, transparent_color_index, &layers)?;
            } else {
                return Err(AsepriteParseError::InvalidInput(
                    "Input file uses indexed color mode but does not contain a palette".into(),
                ));
            }
        }
    }

    parse_info.framedata.validate()?;

    Ok(AsepriteFile {
        width,
        height,
        num_frames,
        pixel_format,
        // color_profile: parse_info.color_profile,
        frame_times: parse_info.frame_times,
        framedata: parse_info.framedata,
        layers,
        palette: parse_info.palette,
        tags: parse_info.tags.unwrap_or_default(),
        external_files: parse_info.external_files,
        tilesets: parse_info.tileset,
    })
}

fn parse_frame<R: Read + Seek>(
    reader: &mut AseReader<R>,
    frame_id: u16,
    pixel_format: PixelFormat,
    parse_info: &mut ParseInfo,
) -> Result<()> {
    let bytes = reader.dword()?;
    let magic_number = reader.word()?;
    if magic_number != 0xF1FA {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Invalid magic number for frame: {:x} != {:x}",
            magic_number, 0xF1FA
        )));
    }
    let old_num_chunks = reader.word()?;
    let frame_duration_ms = reader.word()?;
    let _placeholder = reader.word()?;
    let new_num_chunks = reader.dword()?;

    parse_info.frame_times[frame_id as usize] = frame_duration_ms;

    let num_chunks = if new_num_chunks == 0 {
        old_num_chunks as u32
    } else {
        new_num_chunks
    };

    let mut found_layers: Vec<layer::LayerData> = Vec::new();

    let mut bytes_available = bytes as i64 - 16;
    for _chunk in 0..num_chunks {
        // chunk size includes header
        let chunk_size = reader.dword()?;
        let chunk_type_code = reader.word()?;
        let chunk_type = parse_chunk_type(chunk_type_code)?;
        check_chunk_bytes(chunk_size, bytes_available)?;
        let chunk_data_bytes = chunk_size as usize - CHUNK_HEADER_SIZE;
        let mut chunk_data = vec![0_u8; chunk_data_bytes];
        reader.read_exact(&mut chunk_data)?;
        bytes_available -= chunk_size as i64;
        // println!(
        //     "chunk: {} size: {}, type: {:?}, bytes read: {}",
        //     chunk,
        //     chunk_size,
        //     chunk_type,
        //     chunk_data.len()
        // );
        match chunk_type {
            ChunkType::ColorProfile => {
                let profile = color_profile::parse_color_profile(&chunk_data)?;
                parse_info.color_profile = Some(profile);
            }
            ChunkType::Palette => {
                let palette = palette::parse_palette_chunk(&chunk_data)?;
                parse_info.palette = Some(palette);
            }
            ChunkType::Layer => {
                let layer = layer::parse_layer_chunk(&chunk_data)?;
                found_layers.push(layer);
            }
            ChunkType::Cel => {
                let cel = cel::parse_cel_chunk(&chunk_data, pixel_format)?;
                parse_info.add_cel(frame_id, cel)?;
            }
            ChunkType::ExternalFiles => {
                let files = ExternalFile::parse_chunk(&chunk_data)?;
                parse_info.add_external_files(files);
            }
            ChunkType::Tags => {
                let tags = tags::parse_tags_chunk(&chunk_data)?;
                if frame_id == 0 {
                    parse_info.tags = Some(tags);
                } else {
                    debug!("Ignoring tags outside of frame 0");
                }
            }
            ChunkType::Slice => {
                let _slice = slice::parse_slice_chunk(&chunk_data)?;
                //println!("Slice: {:#?}", slice);
            }
            ChunkType::UserData => {
                let _ud = user_data::parse_userdata_chunk(&chunk_data)?;
                //println!("Userdata: {:#?}", ud);
            }
            ChunkType::OldPalette04 | ChunkType::OldPalette11 => {
                // ignore old palette chunks
            }
            ChunkType::Tileset => {
                let tileset = Tileset::parse_chunk(&chunk_data)?;
                parse_info.tileset.add(tileset);
            }
            ChunkType::CelExtra | ChunkType::Mask | ChunkType::Path => {
                debug!("Ignoring unsupported chunk type: {:?}", chunk_type);
            }
        }
    }

    if frame_id == 0 {
        let layers = layer::collect_layers(found_layers)?;
        parse_info.layers = Some(layers);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum ChunkType {
    OldPalette04, // deprecated
    OldPalette11, // deprecated
    Palette,
    Layer,
    Cel,
    CelExtra,
    ColorProfile,
    Mask, // deprecated
    Path,
    Tags,
    UserData,
    Slice,
    ExternalFiles,
    Tileset,
}

fn parse_chunk_type(chunk_type: u16) -> Result<ChunkType> {
    match chunk_type {
        0x0004 => Ok(ChunkType::OldPalette04),
        0x0011 => Ok(ChunkType::OldPalette11),
        0x2004 => Ok(ChunkType::Layer),
        0x2005 => Ok(ChunkType::Cel),
        0x2006 => Ok(ChunkType::CelExtra),
        0x2007 => Ok(ChunkType::ColorProfile),
        0x2008 => Ok(ChunkType::ExternalFiles),
        0x2016 => Ok(ChunkType::Mask),
        0x2017 => Ok(ChunkType::Path),
        0x2018 => Ok(ChunkType::Tags),
        0x2019 => Ok(ChunkType::Palette),
        0x2020 => Ok(ChunkType::UserData),
        0x2022 => Ok(ChunkType::Slice),
        0x2023 => Ok(ChunkType::Tileset),
        _ => Err(AsepriteParseError::UnsupportedFeature(format!(
            "Invalid or unsupported chunk type: 0x{:x}",
            chunk_type
        ))),
    }
}

const CHUNK_HEADER_SIZE: usize = 6;

fn check_chunk_bytes(chunk_size: u32, bytes_available: i64) -> Result<()> {
    if (chunk_size as usize) < CHUNK_HEADER_SIZE {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Chunk size is too small {}, minimum_size: {}",
            chunk_size, CHUNK_HEADER_SIZE
        )));
    }
    if chunk_size as i64 > bytes_available {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Trying to read chunk of size {}, but there are only {} bytes available in the frame",
            chunk_size, bytes_available
        )));
    }
    Ok(())
}

fn parse_pixel_format(color_depth: u16, transparent_color_index: u8) -> Result<PixelFormat> {
    match color_depth {
        8 => Ok(PixelFormat::Indexed {
            transparent_color_index,
        }),
        16 => Ok(PixelFormat::Grayscale),
        32 => Ok(PixelFormat::Rgba),
        _ => Err(AsepriteParseError::InvalidInput(format!(
            "Unknown pixel format. Color depth: {}",
            color_depth
        ))),
    }
}
