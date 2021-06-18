use crate::pixel::{self, Pixels};
use crate::reader::AseReader;
use crate::tilemap::Tilemap;
use crate::{
    layer::LayersData, AsepriteFile, AsepriteParseError, ColorPalette, PixelFormat, Result,
};

use image::RgbaImage;
use std::io::Read;
use std::io::Seek;
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
                CelData::Raw(image_content) => {
                    match image_content.pixels {
                        Pixels::Rgba(_) => {
                            // TODO: Verify data length
                        }
                        Pixels::Grayscale(_) => todo!(),
                        Pixels::Indexed(_) => {
                            return Err(AsepriteParseError::InvalidInput("Internal error: unresolved Indexed data".into()));
                        },
                    }

                },
                CelData::Linked(other_frame) => {
                    match self.cel(*other_frame, layer as u16) {
                        Some(other_cel) => {
                            if let CelData::Linked(_) = &other_cel.data {
                                return Err(AsepriteParseError::InvalidInput(
                                    format!("Invalid Cel reference. Cel (f:{},l:{}) links to cel (f:{},l:{}) but that cel links to another cel.",
                                frame, layer, *other_frame, layer)
                                ))
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
                    if let CelData::Raw(data) = &cel.data {
                        if let Pixels::Indexed(pixels) = &data.pixels {
                            let layer = &layer_info[cel.layer_index as u32];
                            let layer_is_background = layer.is_background();
                            let rgba_pixels = pixel::resolve_indexed(
                                pixels,
                                palette,
                                transparent_color_index,
                                layer_is_background,
                            )?;
                            cel.data = CelData::Raw(ImageContent {
                                size: data.size,
                                pixels: Pixels::Rgba(rgba_pixels),
                            })
                        }
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
#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageSize {
    pub width: u16,
    pub height: u16,
}
impl ImageSize {
    pub(crate) fn parse<R: Read + Seek>(reader: &mut AseReader<R>) -> Result<Self> {
        let width = reader.word()?;
        let height = reader.word()?;
        Ok(Self { width, height })
    }
    pub(crate) fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

pub(crate) struct ImageContent {
    pub size: ImageSize,
    pub pixels: Pixels,
}

#[derive(Debug)]
pub(crate) enum CelData {
    Raw(ImageContent),
    Linked(u16),
    Tilemap(Tilemap),
}
impl CelData {
    fn parse<R: Read + Seek>(
        mut reader: AseReader<R>,
        pixel_format: PixelFormat,
        cel_type: u16,
    ) -> Result<Self> {
        match cel_type {
            0 => parse_raw_cel(reader, pixel_format).map(CelData::Raw),
            1 => reader.word().map(CelData::Linked),
            2 => parse_compressed_cel(reader, pixel_format).map(CelData::Raw),
            3 => Tilemap::parse_chunk(reader).map(CelData::Tilemap),
            _ => Err(AsepriteParseError::InvalidInput(format!(
                "Invalid/Unsupported Cel type: {}",
                cel_type
            ))),
        }
    }
}

impl fmt::Debug for ImageContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} bytes>", self.pixels.byte_count())
    }
}

fn parse_raw_cel<R: Read + Seek>(
    mut reader: AseReader<R>,
    pixel_format: PixelFormat,
) -> Result<ImageContent> {
    let size = ImageSize::parse(&mut reader)?;
    Pixels::from_raw(reader, pixel_format, size.pixel_count())
        .map(|pixels| ImageContent { size, pixels })
}

fn parse_compressed_cel<R: Read + Seek>(
    mut reader: AseReader<R>,
    pixel_format: PixelFormat,
) -> Result<ImageContent> {
    let size = ImageSize::parse(&mut reader)?;
    Pixels::from_compressed(reader, pixel_format, size.pixel_count())
        .map(|pixels| ImageContent { size, pixels })
}

pub(crate) fn parse_cel_chunk(data: &[u8], pixel_format: PixelFormat) -> Result<RawCel> {
    let mut reader = AseReader::new(data);

    let layer_index = reader.word()?;
    let x = reader.short()?;
    let y = reader.short()?;
    let opacity = reader.byte()?;
    let cel_type = reader.word()?;
    // Reserved bytes
    reader.skip_bytes(7)?;

    CelData::parse(reader, pixel_format, cel_type).map(|data| RawCel {
        layer_index,
        x,
        y,
        opacity,
        data,
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
