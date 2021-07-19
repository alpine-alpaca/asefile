use crate::cel::CelId;
use crate::external_file::{ExternalFile, ExternalFilesById};
use crate::layer::{LayerData, LayersData};
use crate::reader::AseReader;
use crate::slice::Slice;
use crate::tileset::{Tileset, TilesetsById};
use crate::user_data::UserData;
use crate::{error::AsepriteParseError, AsepriteFile, PixelFormat};
use log::debug;
use std::io::Read;

use crate::Result;
use crate::{cel, color_profile, layer, palette, slice, tags, user_data, Tag};

// LayerParseInfo holds Layer data during file parsing.
enum LayerParseInfo {
    // When this is the InProgress variant, parsed layers are pushed onto the vec.
    InProgress(Vec<LayerData>),
    // Once all layers are parsed, the vec data is moved into the LayersData for sorting and processing.
    Finished(LayersData),
}
impl LayerParseInfo {
    fn new() -> Self {
        Self::InProgress(Vec::new())
    }
    fn finalize(self) -> Result<Self> {
        if let Self::InProgress(layers) = self {
            layer::collect_layers(layers).map(Self::Finished)
        } else {
            Err(AsepriteParseError::InternalError(
                "Attempted to collect already Finished layer data.".into(),
            ))
        }
    }
    fn inner(&self) -> Option<&LayersData> {
        if let Self::Finished(layers_data) = self {
            Some(layers_data)
        } else {
            None
        }
    }
    fn into_inner(self) -> Option<LayersData> {
        if let Self::Finished(layers_data) = self {
            Some(layers_data)
        } else {
            None
        }
    }
    fn layer_mut(&mut self, index: u32) -> Option<&mut LayerData> {
        let index = index as usize;
        match self {
            LayerParseInfo::InProgress(vec) => vec.get_mut(index),
            LayerParseInfo::Finished(data) => data.layers.get_mut(index),
        }
    }
}

struct ParseInfo {
    palette: Option<palette::ColorPalette>,
    color_profile: Option<color_profile::ColorProfile>,
    layers: LayerParseInfo,
    framedata: cel::CelsData, // Vec<Vec<cel::RawCel>>,
    frame_times: Vec<u16>,
    tags: Option<Vec<Tag>>,
    external_files: ExternalFilesById,
    tilesets: TilesetsById,
    sprite_user_data: Option<UserData>,
    user_data_context: Option<UserDataContext>,
    slices: Vec<Slice>,
}

impl ParseInfo {
    fn new(num_frames: u16, default_frame_time: u16) -> Self {
        Self {
            palette: None,
            color_profile: None,
            layers: LayerParseInfo::new(),
            framedata: cel::CelsData::new(num_frames as u32),
            frame_times: vec![default_frame_time; num_frames as usize],
            tags: None,
            external_files: ExternalFilesById::new(),
            tilesets: TilesetsById::new(),
            sprite_user_data: None,
            user_data_context: None,
            slices: Vec::new(),
        }
    }
    fn add_cel(&mut self, frame_id: u16, cel: cel::RawCel) -> Result<()> {
        let cel_id = CelId {
            frame: frame_id,
            layer: cel.data.layer_index,
        };
        self.framedata.add_cel(frame_id, cel)?;
        self.user_data_context = Some(UserDataContext::CelId(cel_id));
        Ok(())
    }
    fn add_layer(&mut self, layer_data: LayerData) {
        if let LayerParseInfo::InProgress(layers) = &mut self.layers {
            let idx = layers.len();
            layers.push(layer_data);
            self.user_data_context = Some(UserDataContext::LayerIndex(idx as u32));
        }
    }
    fn add_tags(&mut self, tags: Vec<Tag>) {
        self.tags = Some(tags);
        self.user_data_context = Some(UserDataContext::TagIndex(0));
    }
    fn add_external_files(&mut self, files: Vec<ExternalFile>) {
        for external_file in files {
            self.external_files.add(external_file);
        }
    }
    fn set_tag_user_data(&mut self, user_data: UserData, tag_index: u16) -> Result<()> {
        let tags = self.tags.as_mut().ok_or_else(|| {
            AsepriteParseError::InternalError(
                "No tags data found when resolving Tags chunk context".into(),
            )
        })?;
        let tag = tags.get_mut(tag_index as usize).ok_or_else(|| {
            AsepriteParseError::InternalError(format!(
                "Invalid tag index stored in chunk context: {}",
                tag_index
            ))
        })?;
        tag.set_user_data(user_data);
        self.user_data_context = Some(UserDataContext::TagIndex(tag_index + 1));
        Ok(())
    }
    fn add_user_data(&mut self, user_data: UserData) -> Result<()> {
        let user_data_context = self.user_data_context.ok_or_else(|| {
            AsepriteParseError::InvalidInput(
                "Found dangling user data chunk. Expected a previous chunk to attach user data"
                    .into(),
            )
        })?;
        match user_data_context {
            UserDataContext::CelId(cel_id) => {
                let cel = self.framedata.cel_mut(&cel_id).ok_or_else(|| {
                    AsepriteParseError::InternalError(format!(
                        "Invalid cel id stored in chunk context: {}",
                        cel_id
                    ))
                })?;
                cel.user_data = Some(user_data);
            }
            UserDataContext::LayerIndex(layer_index) => {
                let layer = self.layers.layer_mut(layer_index).ok_or_else(|| {
                    AsepriteParseError::InternalError(format!(
                        "Invalid layer id stored in chunk context: {}",
                        layer_index
                    ))
                })?;
                layer.user_data = Some(user_data);
            }
            UserDataContext::OldPalette => {
                self.sprite_user_data = Some(user_data);
            }
            UserDataContext::TagIndex(tag_index) => {
                self.set_tag_user_data(user_data, tag_index)?;
            }
            UserDataContext::SliceIndex(slice_idx) => {
                let slice = self.slices.get_mut(slice_idx as usize).ok_or_else(|| {
                    AsepriteParseError::InternalError(format!(
                        "Invalid slice index stored in chunk context: {}",
                        slice_idx
                    ))
                })?;
                slice.user_data = Some(user_data);
            }
        }
        Ok(())
    }
    fn add_slice(&mut self, slice: Slice) {
        let context_idx = self.slices.len();
        self.slices.push(slice);
        self.user_data_context = Some(UserDataContext::SliceIndex(context_idx as u32));
    }
    fn finalize_layers(&mut self) -> Result<()> {
        // Move the layers vec out to collect
        let layers = std::mem::replace(&mut self.layers, LayerParseInfo::new());
        self.layers = layers.finalize()?;
        Ok(())
    }
    // Validate moves the ParseInfo data into an intermediate ValidatedParseInfo struct,
    // which is then used to create the AsepriteFile.
    fn validate(self, pixel_format: &PixelFormat) -> Result<ValidatedParseInfo> {
        let layers = self
            .layers
            .into_inner()
            .ok_or_else(|| AsepriteParseError::InvalidInput("No layers found".to_owned()))?;
        let tilesets = self.tilesets;
        let palette = self.palette;
        tilesets.validate(pixel_format, &palette)?;
        layers.validate(&tilesets)?;

        let framedata = self.framedata;
        framedata.validate(&layers)?;

        Ok(ValidatedParseInfo {
            layers,
            tilesets,
            framedata,
            external_files: self.external_files,
            palette,
            tags: self.tags.unwrap_or_default(),
            frame_times: self.frame_times,
            sprite_user_data: self.sprite_user_data,
            slices: self.slices,
        })
    }
}

struct ValidatedParseInfo {
    layers: layer::LayersData,
    tilesets: TilesetsById,
    framedata: cel::CelsData,
    external_files: ExternalFilesById,
    palette: Option<palette::ColorPalette>,
    tags: Vec<Tag>,
    frame_times: Vec<u16>,
    sprite_user_data: Option<UserData>,
    slices: Vec<Slice>,
}

// file format docs: https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md
// v1.3 spec diff doc: https://gist.github.com/dacap/35f3b54fbcd021d099e0166a4f295bab
pub fn read_aseprite<R: Read>(input: R) -> Result<AsepriteFile> {
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
    reader.skip_reserved(84)?;

    if !(pixel_width == 1 && pixel_height == 1) {
        return Err(AsepriteParseError::UnsupportedFeature(
            "Only pixel width:height ratio of 1:1 supported".to_owned(),
        ));
    }

    let mut parse_info = ParseInfo::new(num_frames, default_frame_time);

    let pixel_format = parse_pixel_format(color_depth, transparent_color_index)?;

    for frame_id in 0..num_frames {
        // println!("--- Frame {} -------", frame_id);
        parse_frame(&mut reader, frame_id, pixel_format, &mut parse_info)?;
    }

    let layers = parse_info
        .layers
        .inner()
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

    let ValidatedParseInfo {
        layers,
        tilesets,
        framedata,
        external_files,
        palette,
        tags,
        frame_times,
        sprite_user_data,
        slices,
    } = parse_info.validate(&pixel_format)?;

    Ok(AsepriteFile {
        width,
        height,
        num_frames,
        pixel_format,
        palette,
        layers,
        frame_times,
        tags,
        framedata,
        external_files,
        tilesets,
        sprite_user_data,
        slices,
    })
}

fn parse_frame<R: Read>(
    reader: &mut AseReader<R>,
    frame_id: u16,
    pixel_format: PixelFormat,
    parse_info: &mut ParseInfo,
) -> Result<()> {
    let num_bytes = reader.dword()?;
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

    let bytes_available = num_bytes as i64 - FRAME_HEADER_SIZE;

    let chunks = Chunk::read_all(num_chunks, bytes_available, reader)?;

    for chunk in chunks {
        let Chunk { chunk_type, data } = chunk;
        match chunk_type {
            ChunkType::ColorProfile => {
                let profile = color_profile::parse_chunk(&data)?;
                parse_info.color_profile = Some(profile);
            }
            ChunkType::Palette => {
                let palette = palette::parse_chunk(&data)?;
                parse_info.palette = Some(palette);
            }
            ChunkType::Layer => {
                let layer_data = layer::parse_chunk(&data)?;
                parse_info.add_layer(layer_data);
            }
            ChunkType::Cel => {
                let cel = cel::parse_chunk(&data, pixel_format)?;
                parse_info.add_cel(frame_id, cel)?;
            }
            ChunkType::ExternalFiles => {
                let files = ExternalFile::parse_chunk(&data)?;
                parse_info.add_external_files(files);
            }
            ChunkType::Tags => {
                let tags = tags::parse_chunk(&data)?;
                if frame_id == 0 {
                    parse_info.add_tags(tags);
                } else {
                    debug!("Ignoring tags outside of frame 0");
                }
            }
            ChunkType::Slice => {
                let slice = slice::parse_chunk(&data)?;
                parse_info.add_slice(slice);
                //println!("Slice: {:#?}", slice);
            }
            ChunkType::UserData => {
                let user_data = user_data::parse_userdata_chunk(&data)?;
                parse_info.add_user_data(user_data)?;
                //println!("Userdata: {:#?}", ud);
            }
            ChunkType::OldPalette04 | ChunkType::OldPalette11 => {
                // An old palette chunk precedes the sprite UserData chunk.
                // Update the chunk context to reflect the OldPalette chunk.
                parse_info.user_data_context = Some(UserDataContext::OldPalette);

                // parse_info.sprite_user_data = &data.user_data;
            }
            ChunkType::Tileset => {
                let tileset = Tileset::parse_chunk(&data, pixel_format)?;
                parse_info.tilesets.add(tileset);
            }
            ChunkType::CelExtra | ChunkType::Mask | ChunkType::Path => {
                debug!("Ignoring unsupported chunk type: {:?}", chunk_type);
            }
        }
    }

    if frame_id == 0 {
        parse_info.finalize_layers()?;
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum UserDataContext {
    CelId(CelId),
    LayerIndex(u32),
    OldPalette,
    TagIndex(u16),
    SliceIndex(u32),
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
const FRAME_HEADER_SIZE: i64 = 16;

struct Chunk {
    data: Vec<u8>,
    chunk_type: ChunkType,
}

impl Chunk {
    fn read<R: Read>(bytes_available: &mut i64, reader: &mut AseReader<R>) -> Result<Self> {
        let chunk_size = reader.dword()?;
        let chunk_type_code = reader.word()?;
        let chunk_type = parse_chunk_type(chunk_type_code)?;

        check_chunk_bytes(chunk_size, *bytes_available)?;

        let chunk_data_bytes = chunk_size as usize - CHUNK_HEADER_SIZE;
        let mut data = vec![0_u8; chunk_data_bytes];
        reader.read_exact(&mut data)?;
        *bytes_available -= chunk_size as i64;
        Ok(Chunk { chunk_type, data })
    }
    fn read_all<R: Read>(
        count: u32,
        mut bytes_available: i64,
        reader: &mut AseReader<R>,
    ) -> Result<Vec<Self>> {
        let mut chunks: Vec<Chunk> = Vec::new();
        for _idx in 0..count {
            let chunk = Self::read(&mut bytes_available, reader)?;
            chunks.push(chunk);
        }
        Ok(chunks)
    }
}

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
