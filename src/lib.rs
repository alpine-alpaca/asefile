#![warn(clippy::all)]
use byteorder::{LittleEndian, ReadBytesExt};
//use log::debug;
use std::io::{self, Read};
use std::string::FromUtf8Error;

pub mod blend;
pub mod cel;
pub mod color_profile;
pub mod file;
pub mod layer;
pub mod palette;
pub mod rgba16;
pub mod tags;
#[cfg(test)]
mod tests;

pub use color_profile::ColorProfile;
pub use file::{AsepriteFile, PixelFormat};
pub use layer::Layers;
pub use palette::ColorPalette;
pub use tags::{AnimationDirection, Tag};

// TODO: impl Error
#[derive(Debug)]
pub enum AsepriteParseError {
    InvalidInput(String),
    UnsupportedFeature(String),
    IoError(io::Error),
}

impl From<io::Error> for AsepriteParseError {
    fn from(err: io::Error) -> Self {
        AsepriteParseError::IoError(err)
    }
}

impl From<FromUtf8Error> for AsepriteParseError {
    fn from(err: FromUtf8Error) -> Self {
        AsepriteParseError::InvalidInput(format!("Could not decode utf8: {}", err))
    }
}

type Result<T> = std::result::Result<T, AsepriteParseError>;

struct ParseInfo {
    palette: Option<palette::ColorPalette>,
    color_profile: Option<color_profile::ColorProfile>,
    layers: Option<layer::Layers>,
    framedata: Vec<Vec<cel::Cel>>,
    frame_times: Vec<u16>,
    tags: Option<Vec<Tag>>,
}

impl ParseInfo {
    fn add_cel(&mut self, frame_id: u16, cel: cel::Cel) {
        let idx = frame_id as usize;
        self.framedata[idx].push(cel);
    }
}

// file format docs: https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md
pub fn read_aseprite<R: Read>(mut input: R) -> Result<AsepriteFile> {
    let _size = input.read_u32::<LittleEndian>()?;
    let magic_number = input.read_u16::<LittleEndian>()?;
    if magic_number != 0xA5E0 {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Invalid magic number for header: {:x} != {:x}",
            magic_number, 0xA5E0
        )));
    }

    let num_frames = input.read_u16::<LittleEndian>()?;
    let width = input.read_u16::<LittleEndian>()?;
    let height = input.read_u16::<LittleEndian>()?;
    let color_depth = input.read_u16::<LittleEndian>()?;
    let _flags = input.read_u32::<LittleEndian>()?;
    let default_frame_time = input.read_u16::<LittleEndian>()?;
    let _placeholder1 = input.read_u32::<LittleEndian>()?;
    let _placeholder2 = input.read_u32::<LittleEndian>()?;
    let transparent_color_index = input.read_u32::<LittleEndian>()? & 0xff;
    let _num_colors = input.read_u16::<LittleEndian>()?;
    let pixel_width = input.read_u8()?;
    let pixel_height = input.read_u8()?;
    let _grid_x = input.read_i16::<LittleEndian>()?;
    let _grid_y = input.read_i16::<LittleEndian>()?;
    let _grid_width = input.read_u16::<LittleEndian>()?;
    let _grid_height = input.read_u16::<LittleEndian>()?;
    let mut rest = [0_u8; 84];
    input.read_exact(&mut rest)?;

    if !(pixel_width == 1 && pixel_height == 1) {
        return Err(AsepriteParseError::UnsupportedFeature(
            "Only pixel width:height ratio of 1:1 supported".to_owned(),
        ));
    }

    let mut framedata = Vec::with_capacity(num_frames as usize);
    framedata.resize_with(num_frames as usize, Vec::new);
    let mut parse_info = ParseInfo {
        palette: None,
        color_profile: None,
        layers: None,
        framedata,
        frame_times: vec![default_frame_time; num_frames as usize],
        tags: None,
    };

    let pixel_format = parse_pixel_format(color_depth)?;

    for frame_id in 0..num_frames {
        // println!("--- Frame {} -------", frame_id);
        parse_frame(&mut input, frame_id, pixel_format, &mut parse_info)?;
    }

    let layers = parse_info
        .layers
        .ok_or_else(|| AsepriteParseError::InvalidInput("No layers found".to_owned()))?;

    // println!("==== Layers ====\n{:#?}", layers);
    // println!("{:#?}", parse_info.framedata);

    // println!("bytes: {}, size: {}x{}", size, width, height);
    // println!("color_depth: {}, num_colors: {}", color_depth, num_colors);

    Ok(AsepriteFile {
        width,
        height,
        num_frames,
        pixel_format,
        color_profile: parse_info.color_profile,
        frame_times: parse_info.frame_times,
        framedata: parse_info.framedata,
        layers,
        palette: parse_info.palette,
        transparent_color_index: transparent_color_index as u8,
        tags: parse_info.tags.unwrap_or_default(),
    })
}

fn parse_frame<R: Read>(
    input: &mut R,
    frame_id: u16,
    pixel_format: PixelFormat,
    parse_info: &mut ParseInfo,
) -> Result<()> {
    let bytes = input.read_u32::<LittleEndian>()?;
    let magic_number = input.read_u16::<LittleEndian>()?;
    if magic_number != 0xF1FA {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Invalid magic number for frame: {:x} != {:x}",
            magic_number, 0xF1FA
        )));
    }
    let old_num_chunks = input.read_u16::<LittleEndian>()?;
    let frame_duration_ms = input.read_u16::<LittleEndian>()?;
    let _placeholder = input.read_u16::<LittleEndian>()?;
    let new_num_chunks = input.read_u32::<LittleEndian>()?;

    parse_info.frame_times[frame_id as usize] = frame_duration_ms;

    let num_chunks = if new_num_chunks == 0 {
        old_num_chunks as u32
    } else {
        new_num_chunks
    };

    //println!("Num chunks: {}, bytes: {}", num_chunks, bytes);

    let mut found_layers: Vec<layer::Layer> = Vec::new();

    let mut bytes_available = bytes as i64 - 16;
    for _chunk in 0..num_chunks {
        // chunk size includes header
        let chunk_size = input.read_u32::<LittleEndian>()?;
        let chunk_type_code = input.read_u16::<LittleEndian>()?;
        let chunk_type = parse_chunk_type(chunk_type_code)?;
        check_chunk_bytes(chunk_size, bytes_available)?;
        let chunk_data_bytes = chunk_size as usize - CHUNK_HEADER_SIZE;
        let mut chunk_data = vec![0_u8; chunk_data_bytes];
        input.read_exact(&mut chunk_data)?;
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
                parse_info.add_cel(frame_id, cel);
            }
            ChunkType::Tags => {
                let tags = tags::parse_palette_chunk(&chunk_data)?;
                if frame_id == 0 {
                    parse_info.tags = Some(tags);
                } else {
                    println!("Ignoring tags outside of frame 0");
                }
            }
            _ => {
                println!("Ignoring chunk: {:?}", chunk_type);
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
}

fn parse_chunk_type(chunk_type: u16) -> Result<ChunkType> {
    match chunk_type {
        0x0004 => Ok(ChunkType::OldPalette04),
        0x0011 => Ok(ChunkType::OldPalette11),
        0x2004 => Ok(ChunkType::Layer),
        0x2005 => Ok(ChunkType::Cel),
        0x2006 => Ok(ChunkType::CelExtra),
        0x2007 => Ok(ChunkType::ColorProfile),
        0x2016 => Ok(ChunkType::Mask),
        0x2017 => Ok(ChunkType::Path),
        0x2018 => Ok(ChunkType::Tags),
        0x2019 => Ok(ChunkType::Palette),
        0x2020 => Ok(ChunkType::UserData),
        0x2022 => Ok(ChunkType::Slice),
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

pub(crate) fn read_string<R: Read>(input: &mut R) -> Result<String> {
    let str_len = input.read_u16::<LittleEndian>()?;
    let mut str_bytes = vec![0_u8; str_len as usize];
    input.read_exact(&mut str_bytes)?;
    let s = String::from_utf8(str_bytes)?;
    Ok(s)
}

fn parse_pixel_format(color_depth: u16) -> Result<PixelFormat> {
    match color_depth {
        8 => Ok(PixelFormat::Indexed),
        16 => Ok(PixelFormat::Grayscale),
        32 => Ok(PixelFormat::Rgba),
        _ => Err(AsepriteParseError::InvalidInput(format!(
            "Unknown pixel format. Color depth: {}",
            color_depth
        ))),
    }
}
