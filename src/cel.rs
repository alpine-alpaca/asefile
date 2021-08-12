use crate::layer::{LayerData, LayerType};
use crate::pixel::Pixels;
use crate::reader::AseReader;
use crate::tilemap::Tilemap;
use crate::user_data::UserData;
use crate::{
    layer::LayersData, AsepriteFile, AsepriteParseError, ColorPalette, PixelFormat, Result,
};

use image::RgbaImage;
use std::fmt;
use std::io::Read;

/// A reference to a single Cel. A cel contains the image data at a specific
/// layer and frame. In the timeline view these are the dots.
///
/// You can get a `cel` by going either via frame then layer or vice versa.
///
/// [Official docs for cels](https://www.aseprite.org/docs/cel/).
#[derive(Debug)]
pub struct Cel<'a> {
    pub(crate) file: &'a AsepriteFile,
    pub(crate) cel_id: CelId,
}

impl<'a> Cel<'a> {
    /// This cel as an image. Result has the same dimensions as the [AsepriteFile].
    /// If the cel is empty, all image pixels will be transparent.
    pub fn image(&self) -> RgbaImage {
        self.file.layer_image(self.cel_id)
    }

    /// Returns `true` if the cel contains no data.
    pub fn is_empty(&self) -> bool {
        self.file.framedata.cel(self.cel_id).is_some()
    }

    /// Returns the cel's user data, if any is present.
    pub fn user_data(&self) -> Option<&UserData> {
        self.file
            .framedata
            .cel(self.cel_id)
            .and_then(|c| c.user_data.as_ref())
    }
}

/// Organizes all Cels into a 2d array.
pub(crate) struct CelsData {
    // Mapping: frame_id -> layer_id -> Option<RawCel>
    data: Vec<Vec<Option<RawCel>>>,
    num_frames: u32,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct CelId {
    pub frame: u16,
    pub layer: u16,
}

impl fmt::Display for CelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CelId(F{},L{})", self.frame, self.layer)
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
    pub(crate) fn new(num_frames: u32) -> Self {
        let mut data = Vec::with_capacity(num_frames as usize);
        // Initialize with one layer (outer Vec) and zero RawCel (inner Vec).
        data.resize_with(num_frames as usize, || vec![None]);
        CelsData { data, num_frames }
    }

    fn check_valid_frame_id(&self, frame_id: u16) -> Result<()> {
        if (frame_id as usize) >= self.data.len() {
            return Err(AsepriteParseError::InvalidInput(format!(
                "Invalid frame reference in Cel: {}",
                frame_id
            )));
        }
        Ok(())
    }

    pub(crate) fn add_cel(&mut self, frame_id: u16, cel: RawCel) -> Result<()> {
        self.check_valid_frame_id(frame_id)?;

        let layer_id = cel.data.layer_index;
        let min_layers = layer_id as u32 + 1;
        let layers = &mut self.data[frame_id as usize];
        if layers.len() < min_layers as usize {
            layers.resize_with(min_layers as usize, || None);
        }
        if layers[layer_id as usize].is_some() {
            return Err(AsepriteParseError::InvalidInput(format!(
                "Multiple Cels for frame {}, layer {}",
                frame_id, layer_id
            )));
        }
        layers[layer_id as usize] = Some(cel);

        Ok(())
    }

    pub(crate) fn frame_cels(&self, frame_id: u16) -> impl Iterator<Item = (u32, &RawCel)> {
        self.data[frame_id as usize]
            .iter()
            .enumerate()
            .filter_map(|(layer_id, cel)| cel.as_ref().map(|c| (layer_id as u32, c)))
    }

    // Frame ID must be valid. If Layer ID is out of bounds always returns an
    // empty Vec.
    pub(crate) fn cel(&self, cel_id: CelId) -> Option<&RawCel> {
        let CelId { frame, layer } = cel_id;
        let layers = &self.data[frame as usize];
        if (layer as usize) >= layers.len() {
            None
        } else {
            layers[layer as usize].as_ref()
        }
    }

    pub(crate) fn cel_mut(&mut self, cel_id: &CelId) -> Option<&mut RawCel> {
        let frame = cel_id.frame;
        let layer = cel_id.layer;
        let layers = &mut self.data[frame as usize];
        if (layer as usize) >= layers.len() {
            None
        } else {
            layers[layer as usize].as_mut()
        }
    }

    fn validate_cel(
        &self,
        frame: u32,
        layer_index: usize,
        layer: &LayerData,
        palette: Option<&ColorPalette>,
    ) -> Result<()> {
        let by_layer = &self.data[frame as usize];
        if let Some(ref cel) = by_layer[layer_index] {
            match &cel.content {
                CelContent::Raw(image_content) => match &image_content.pixels {
                    Pixels::Rgba(_) => {}
                    Pixels::Grayscale(_) => {}
                    Pixels::Indexed(indexed_pixels) => {
                        let palette = palette.ok_or_else(|| {
                            AsepriteParseError::InvalidInput(
                                "No palette present for indexed pixel data".into(),
                            )
                        })?;
                        palette.validate_indexed_pixels(indexed_pixels)?;
                    }
                },
                CelContent::Linked(other_frame) => {
                    match self.cel(CelId{ frame: *other_frame, layer: layer_index as u16 }) {
                        Some(other_cel) => {
                            if let CelContent::Linked(_) = &other_cel.content {
                                return Err(AsepriteParseError::InvalidInput(
                                    format!("Invalid Cel reference. Cel (f:{},l:{}) links to cel (f:{},l:{}) but that cel links to another cel.",
                                frame, layer_index, *other_frame, layer_index)
                                ))
                            }
                        }
                        None => {
                            return Err(AsepriteParseError::InvalidInput(
                                format!("Invalid Cel reference. Cel (f:{},l:{}) links to cel (f:{},l:{}) but that cel contains no data.",
                            frame, layer_index, *other_frame, layer_index)
                            ))
                        }
                    }
                }
                CelContent::Tilemap(_) => {
                    // Verify that a Tilemap cel belongs to a Tilemap layer.
                    if let LayerType::Tilemap(_) = layer.layer_type {
                        // Tilemap Layer, ok
                    } else {
                        return Err(AsepriteParseError::InvalidInput(format!(
                            "Invalid cel. Tilemap Cel (f:{},l:{}) outside of tilemap layer.",
                            frame, layer_index
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate(
        &self,
        layers_data: &LayersData,
        palette: Option<&ColorPalette>,
    ) -> Result<()> {
        for frame in 0..self.num_frames {
            let by_layer = &self.data[frame as usize];
            for layer_index in 0..by_layer.len() {
                let layer = &layers_data[layer_index as u32];
                self.validate_cel(frame, layer_index, layer, palette)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageSize {
    pub width: u16,
    pub height: u16,
}

impl ImageSize {
    pub(crate) fn parse<R: Read>(reader: &mut AseReader<R>) -> Result<Self> {
        let width = reader.word()?;
        let height = reader.word()?;
        Ok(Self { width, height })
    }

    pub(crate) fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

// CelData holds fields which are common to all cel types.
#[derive(Debug)]
pub(crate) struct CelData {
    pub layer_index: u16,
    pub x: i16,
    pub y: i16,
    pub opacity: u8,
}
impl CelData {
    fn parse<R: Read>(reader: &mut AseReader<R>) -> Result<Self> {
        let layer_index = reader.word()?;
        let x = reader.short()?;
        let y = reader.short()?;
        let opacity = reader.byte()?;
        Ok(Self {
            layer_index,
            x,
            y,
            opacity,
        })
    }
}

pub(crate) struct ImageContent {
    pub size: ImageSize,
    pub pixels: Pixels,
}

// CelContent holds data specific to each type of cel.
#[derive(Debug)]
pub(crate) enum CelContent {
    Raw(ImageContent),
    Linked(u16),
    Tilemap(Tilemap),
}

impl CelContent {
    fn parse<R: Read>(
        mut reader: AseReader<R>,
        pixel_format: PixelFormat,
        cel_type: u16,
    ) -> Result<Self> {
        match cel_type {
            0 => parse_raw_cel(reader, pixel_format).map(CelContent::Raw),
            1 => reader.word().map(CelContent::Linked),
            2 => parse_compressed_cel(reader, pixel_format).map(CelContent::Raw),
            3 => Tilemap::parse_chunk(reader).map(CelContent::Tilemap),
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

#[derive(Debug)]
pub(crate) struct RawCel {
    pub data: CelData,
    pub content: CelContent,
    pub user_data: Option<UserData>,
}

fn parse_raw_cel<R: Read>(
    mut reader: AseReader<R>,
    pixel_format: PixelFormat,
) -> Result<ImageContent> {
    let size = ImageSize::parse(&mut reader)?;
    Pixels::from_raw(reader, pixel_format, size.pixel_count())
        .map(|pixels| ImageContent { size, pixels })
}

fn parse_compressed_cel<R: Read>(
    mut reader: AseReader<R>,
    pixel_format: PixelFormat,
) -> Result<ImageContent> {
    let size = ImageSize::parse(&mut reader)?;
    Pixels::from_compressed(reader, pixel_format, size.pixel_count())
        .map(|pixels| ImageContent { size, pixels })
}

pub(crate) fn parse_chunk(data: &[u8], pixel_format: PixelFormat) -> Result<RawCel> {
    let mut reader = AseReader::new(&data);
    let data = CelData::parse(&mut reader)?;
    let cel_type = reader.word()?;
    reader.skip_reserved(7)?;

    let content = CelContent::parse(reader, pixel_format, cel_type)?;
    Ok(RawCel {
        data,
        content,
        user_data: None,
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
