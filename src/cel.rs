use crate::layer::LayerFlags;
use crate::{
    layer::LayersData, AsepriteFile, AsepriteParseError, ColorPalette, PixelFormat, Result,
};
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use image::RgbaImage;
use std::io::{Cursor, Read};
use std::{fmt, ops::DerefMut};

/// A reference to a single Cel. This contains the image data at a specific
/// layer and frame. In the timeline view these are the dots.
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
                                CelData::RawRgba {..} | CelData::RawIndexed {..} => {},
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
                    if let CelData::RawIndexed {
                        ref width,
                        ref height,
                        ref mut data,
                    } = cel.data
                    {
                        let mut output: Vec<u8> = Vec::with_capacity(4 * data.0.len());
                        let layer_is_background = layer_info[cel.layer_index as u32]
                            .flags
                            .contains(LayerFlags::BACKGROUND);
                        for index in &data.0 {
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
                        cel.data = CelData::RawRgba {
                            width: *width,
                            height: *height,
                            data: CelBytes(output),
                        };
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

pub(crate) struct CelBytes(pub Vec<u8>);

#[derive(Debug)]
pub(crate) enum CelData {
    RawRgba {
        width: u16,
        height: u16,
        data: CelBytes,
    },
    RawIndexed {
        width: u16,
        height: u16,
        data: CelBytes,
    },
    Linked(u16),
    // ZlibData { width: u16, height: u16, data: CelBytes },
}

impl fmt::Debug for CelBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} bytes>", self.0.len())
    }
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
        0 => {
            // Raw cel
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
            match pixel_format {
                PixelFormat::Rgba => CelData::RawRgba {
                    width,
                    height,
                    data: CelBytes(output),
                },
                PixelFormat::Grayscale => CelData::RawRgba {
                    width,
                    height,
                    data: CelBytes(output),
                },
                PixelFormat::Indexed { .. } => CelData::RawIndexed {
                    width,
                    height,
                    data: CelBytes(output),
                },
            }
        }
        1 => {
            // Linked cel
            let linked = input.read_u16::<LittleEndian>()?;
            CelData::Linked(linked)
        }
        2 => {
            // Compressed cel
            let width = input.read_u16::<LittleEndian>()?;
            let height = input.read_u16::<LittleEndian>()?;
            let expected_output_size =
                width as usize * height as usize * pixel_format.bytes_per_pixel();
            let decoded_data = unzip(input, expected_output_size)?;
            match pixel_format {
                PixelFormat::Rgba => CelData::RawRgba {
                    width,
                    height,
                    data: CelBytes(decoded_data),
                },
                PixelFormat::Grayscale => CelData::RawRgba {
                    width,
                    height,
                    data: CelBytes(decoded_data),
                },
                PixelFormat::Indexed { .. } => CelData::RawIndexed {
                    width,
                    height,
                    data: CelBytes(decoded_data),
                },
            }
        }
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
