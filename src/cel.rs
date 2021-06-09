use crate::layer::LayerFlags;
use crate::{
    layer::LayersData, AsepriteFile, AsepriteParseError, ColorPalette, PixelFormat, Result,
};
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use image::RgbaImage;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::{fmt, ops::DerefMut};

/// A reference to a single Cel. This contains the image data at a specific
/// layer and frame. In the timeline view these are the dots.
#[derive(Debug)]
pub struct Cel<'a> {
    pub(crate) file: &'a AsepriteFile,
    pub(crate) layer: u32,
    pub(crate) frame: u32,
}

impl<'a> Cel<'a> {
    /// This cel as an image. Result has the same dimensions as the [AsepriteFile].
    /// If the cel is empty, all image pixels will be transparent.
    pub fn image(&self) -> RgbaImage {
        self.file
            .layer_image(self.frame as u16, self.layer as usize)
    }

    /// Returns `true` if the cel contains no data.
    pub fn is_empty(&self) -> bool {
        self.file
            .framedata
            .cel(self.frame as u16, self.layer as u16)
            .is_some()
    }
}

/// Organizes all Cels into a 2d array.
pub(crate) struct CelsData {
    // Mapping: frame_id -> layer_id -> Option<RawCel>
    data: Vec<Vec<Option<RawCel>>>,
    num_frames: u32,
}

struct CelId {
    frame: u16,
    layer: u16,
}

impl fmt::Debug for CelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "F{}_L{}", self.frame, self.layer)
    }
}

impl fmt::Debug for CelsData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_map();
        for frame in 0..self.data.len() {
            for (layer, cel) in self.data[frame].iter().enumerate() {
                if let Some(ref cel) = cel {
                    d.entry(
                        &CelId {
                            frame: frame as u16,
                            layer: layer as u16,
                        },
                        cel,
                    );
                }
            }
        }
        d.finish()
    }
}

impl CelsData {
    pub fn new(num_frames: u32) -> Self {
        let mut data = Vec::with_capacity(num_frames as usize);
        // Initialize with one layer (outer Vec) and zero RawCel (inner Vec).
        data.resize_with(num_frames as usize, || vec![None]);
        CelsData { data, num_frames }
    }

    fn check_valid_frame_id(&self, frame_id: u16) -> Result<()> {
        if !((frame_id as usize) < self.data.len()) {
            return Err(AsepriteParseError::InvalidInput(format!(
                "Invalid frame reference in Cel: {}",
                frame_id
            )));
        }
        Ok(())
    }

    pub fn add_cel(&mut self, frame_id: u16, cel: RawCel) -> Result<()> {
        self.check_valid_frame_id(frame_id)?;

        let layer_id = cel.layer_index;
        let min_layers = layer_id as u32 + 1;
        let layers = &mut self.data[frame_id as usize];
        if layers.len() < min_layers as usize {
            layers.resize_with(min_layers as usize, || None);
        }
        if let Some(_) = layers[layer_id as usize] {
            return Err(AsepriteParseError::InvalidInput(format!(
                "Multiple Cels for frame {}, layer {}",
                frame_id, layer_id
            )));
        }
        layers[layer_id as usize] = Some(cel);

        Ok(())
    }

    pub fn frame_cels(&self, frame_id: u16) -> impl Iterator<Item = (u32, &RawCel)> {
        self.data[frame_id as usize]
            .iter()
            .enumerate()
            .filter_map(|(layer_id, cel)| match cel.as_ref() {
                Some(c) => Some((layer_id as u32, c)),
                None => None,
            })
    }

    // Frame ID must be valid. If Layer ID is out of bounds always returns an
    // empty Vec.
    pub fn cel(&self, frame_id: u16, layer_id: u16) -> Option<&RawCel> {
        let layers = &self.data[frame_id as usize];
        if (layer_id as usize) >= layers.len() {
            None
        } else {
            layers[layer_id as usize].as_ref()
        }
    }

    fn validate_cel(&self, frame: u32, layer: usize) -> Result<()> {
        let layers = &self.data[frame as usize];
        if let Some(ref cel) = layers[layer] {
            match &cel.data {
                CelData::RawRgba { .. } => {
                    // TODO: Verify data length
                },
                CelData::RawIndexed { .. } => {
                    return Err(AsepriteParseError::InvalidInput("Internal error: unresolved Indexed data".into()));
                }
                CelData::Linked(other_frame) => {
                    match self.cel(*other_frame, layer as u16) {
                        Some(other_cel) => {
                            match &other_cel.data {
                                CelData::RawRgba {..} | CelData::RawIndexed {..} => {}, | CelData::Tilemap { .. } => {}, | CelData::TilemapIndexed {..} => {},
                                CelData::Linked(_) => {
                                    return Err(AsepriteParseError::InvalidInput(
                                        format!("Invalid Cel reference. Cel (f:{},l:{}) links to cel (f:{},l:{}) but that cel links to another cel.",
                                    frame, layer, *other_frame, layer)
                                    ))
                                }
                            }
                        }
                        None => {
                            return Err(AsepriteParseError::InvalidInput(
                                format!("Invalid Cel reference. Cel (f:{},l:{}) links to cel (f:{},l:{}) but that cel contains no data.",
                            frame, layer, *other_frame, layer)
                            ))
                        }
                    }
                }
                CelData::Tilemap { .. } => {
                    // TODO: Verify
                }
                CelData::TilemapIndexed { .. } => {
                    // TODO: Verify
                }
            }
        }
        Ok(())
    }

    // Turn indexed-color cels into rgba cels.
    pub(crate) fn resolve_palette(
        &mut self,
        palette: &ColorPalette,
        transparent_color_index: u8,
        layer_info: &LayersData,
    ) -> Result<()> {
        let max_col = palette.num_colors();
        dbg!(
            max_col,
            transparent_color_index,
            palette.color(0),
            palette.color(1)
        );
        for frame in 0..self.num_frames {
            let layers = &mut self.data[frame as usize];
            for mut cel in layers {
                if let Some(cel) = cel.deref_mut() {
                    if let CelData::RawIndexed(cel_bytes) = &cel.data {
                        let width = cel_bytes.width;
                        let height = cel_bytes.height;
                        let data = &cel_bytes.bytes;
                        let mut output: Vec<u8> = Vec::with_capacity(4 * data.len());
                        let layer_is_background = layer_info[cel.layer_index as u32]
                            .flags
                            .contains(LayerFlags::BACKGROUND);
                        for index in data {
                            if *index as u32 >= max_col {
                                return Err(AsepriteParseError::InvalidInput(format!(
                                    "Index out of range: {} (max: {})",
                                    *index, max_col
                                )));
                            }
                            let col = palette.color(*index as u32).unwrap();
                            let alpha = if *index == transparent_color_index && !layer_is_background
                            {
                                0
                            } else {
                                col.alpha()
                            };
                            output.push(col.red());
                            output.push(col.green());
                            output.push(col.blue());
                            output.push(alpha);
                        }
                        cel.data = CelData::RawRgba(CelBytes {
                            width,
                            height,
                            bytes: output,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        for frame in 0..self.num_frames {
            let layers = &self.data[frame as usize];
            for layer in 0..layers.len() {
                self.validate_cel(frame, layer)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct RawCel {
    pub layer_index: u16,
    pub x: i16,
    pub y: i16,
    pub opacity: u8,
    pub data: CelData,
}

pub(crate) struct CelBytes {
    pub width: u16,
    pub height: u16,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct TilemapData {
    pub cel_bytes: CelBytes,
    pub bits_per_tile: u16,
    pub tile_id_bitmask: u32,
    pub x_flip_bitmask: u32,
    pub y_flip_bitmask: u32,
    pub rotate_90cw_bitmask: u32,
}

#[derive(Debug)]
pub(crate) enum CelData {
    RawRgba(CelBytes),
    RawIndexed(CelBytes),
    Linked(u16),
    Tilemap(TilemapData),
    TilemapIndexed(TilemapData),
}

impl fmt::Debug for CelBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} bytes>", self.bytes.len())
    }
}

fn parse_raw_cel(mut input: Cursor<&[u8]>, pixel_format: PixelFormat) -> Result<CelData> {
    let width = input.read_u16::<LittleEndian>()?;
    let height = input.read_u16::<LittleEndian>()?;
    let data_size = width as usize * height as usize * pixel_format.bytes_per_pixel();
    let mut output = Vec::with_capacity(data_size);
    input.take(data_size as u64).read_to_end(&mut output)?;
    if output.len() != data_size {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Invalid cel data size. Expected: {}, Actual: {}",
            data_size,
            output.len()
        )));
    }
    let cel_bytes = CelBytes {
        width,
        height,
        bytes: output,
    };
    Ok(match pixel_format {
        PixelFormat::Rgba => CelData::RawRgba(cel_bytes),
        PixelFormat::Grayscale => CelData::RawRgba(cel_bytes),
        PixelFormat::Indexed { .. } => CelData::RawIndexed(cel_bytes),
    })
}

fn parse_compressed_cel(mut input: Cursor<&[u8]>, pixel_format: PixelFormat) -> Result<CelData> {
    let width = input.read_u16::<LittleEndian>()?;
    let height = input.read_u16::<LittleEndian>()?;
    let expected_output_size = width as usize * height as usize * pixel_format.bytes_per_pixel();
    let decoded_data = unzip(input, expected_output_size)?;
    let cel_bytes = CelBytes {
        width,
        height,
        bytes: decoded_data,
    };
    Ok(match pixel_format {
        PixelFormat::Rgba => CelData::RawRgba(cel_bytes),
        PixelFormat::Grayscale => CelData::RawRgba(cel_bytes),
        PixelFormat::Indexed { .. } => CelData::RawIndexed(cel_bytes),
    })
}

fn parse_compressed_tilemap(
    mut input: Cursor<&[u8]>,
    pixel_format: PixelFormat,
) -> Result<CelData> {
    // Compressed tilemap
    let width = input.read_u16::<LittleEndian>()?;
    let height = input.read_u16::<LittleEndian>()?;
    let bits_per_tile = input.read_u16::<LittleEndian>()?;
    let tile_id_bitmask = input.read_u32::<LittleEndian>()?;
    let x_flip_bitmask = input.read_u32::<LittleEndian>()?;
    let y_flip_bitmask = input.read_u32::<LittleEndian>()?;
    let rotate_90cw_bitmask = input.read_u32::<LittleEndian>()?;
    // Skip 10 reserved bytes
    input.seek(SeekFrom::Current(10))?;
    // Tiles are 8-bit, 16-bit, or 32-bit
    let bytes_per_tile = bits_per_tile as usize / 8;
    let expected_output_size = width as usize * height as usize * bytes_per_tile;
    let decoded_data = unzip(input, expected_output_size)?;
    let cel_bytes = CelBytes {
        width,
        height,
        bytes: decoded_data,
    };
    let tilemap_data = TilemapData {
        cel_bytes,
        bits_per_tile,
        tile_id_bitmask,
        x_flip_bitmask,
        y_flip_bitmask,
        rotate_90cw_bitmask,
    };
    Ok(match pixel_format {
        PixelFormat::Rgba => CelData::Tilemap(tilemap_data),
        PixelFormat::Grayscale => CelData::Tilemap(tilemap_data),
        PixelFormat::Indexed { .. } => CelData::TilemapIndexed(tilemap_data),
    })
}

pub(crate) fn parse_cel_chunk(data: &[u8], pixel_format: PixelFormat) -> Result<RawCel> {
    let mut input = Cursor::new(data);

    let layer_index = input.read_u16::<LittleEndian>()?;
    let x = input.read_i16::<LittleEndian>()?;
    let y = input.read_i16::<LittleEndian>()?;
    let opacity = input.read_u8()?;
    let cel_type = input.read_u16::<LittleEndian>()?;
    let mut reserved = [0_u8; 7];
    input.read_exact(&mut reserved)?;

    let cel_data = match cel_type {
        0 => parse_raw_cel(input, pixel_format)?,
        1 => {
            // Linked cel
            let linked = input.read_u16::<LittleEndian>()?;
            CelData::Linked(linked)
        }
        2 => parse_compressed_cel(input, pixel_format)?,
        3 => parse_compressed_tilemap(input, pixel_format)?,
        _ => {
            return Err(AsepriteParseError::InvalidInput(format!(
                "Invalid/Unsupported Cel type: {}",
                cel_type
            )))
        }
    };

    Ok(RawCel {
        layer_index,
        x,
        y,
        opacity,
        data: cel_data,
    })
}

// For debugging
#[allow(dead_code)]
fn dump_bytes(data: &[u8]) {
    let mut column = 0;
    for d in data {
        print!("{:02x} ", d);
        column += 1;
        if column >= 16 {
            column = 0;
            println!();
        }
    }
}

pub(crate) fn unzip(input: Cursor<&[u8]>, expected_output_size: usize) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(input);
    let mut buffer = Vec::with_capacity(expected_output_size);
    decoder.read_to_end(&mut buffer)?;
    Ok(buffer)
}
