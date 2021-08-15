use crate::layer::LayerType;
use crate::pixel::{Pixels, RawPixels};
use crate::reader::AseReader;
use crate::tilemap::TilemapData;
use crate::user_data::UserData;
use crate::{
    layer::LayersData, AsepriteFile, AsepriteParseError, ColorPalette, PixelFormat, Result,
};

use image::RgbaImage;
use std::fmt;
use std::io::Read;
use std::sync::Arc;

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

    /// The frame coordinate of this cel.
    pub fn frame(&self) -> u32 {
        self.cel_id.frame as u32
    }

    /// The layer coordinate of this cel.
    pub fn layer(&self) -> u32 {
        self.cel_id.layer as u32
    }

    /// Returns the cel's user data, if any is present.
    pub fn user_data(&self) -> Option<&UserData> {
        self.file
            .framedata
            .cel(self.cel_id)
            .and_then(|c| c.user_data.as_ref())
    }

    /// Top-left corner of the non-empty rectangular area of the cel.
    ///
    /// In other words, the first component is the smallest x coordinate of a
    /// non-empty pixel. And the second is the same for y.
    ///
    /// These may be negative or outside of the visible area. This can happen if
    /// you drag a layer around.
    pub fn top_left(&self) -> (i32, i32) {
        self.raw_cel()
            .map_or_else(|| (0, 0), |raw| (raw.data.x as i32, raw.data.y as i32))
    }

    /// Does this cel include a tilemap.
    pub fn is_tilemap(&self) -> bool {
        if let Some(raw) = self.raw_cel() {
            if let CelContent::Tilemap(_) = raw.content {
                return true;
            }
        }
        false
    }

    pub(crate) fn raw_cel(&self) -> Option<&RawCel> {
        self.file.framedata.cel(self.cel_id)
    }
}

/// Organizes all Cels into a 2d array.
pub(crate) struct CelsData<P> {
    // Mapping: frame_id -> layer_id -> Option<RawCel>
    data: Vec<Vec<Option<RawCel<P>>>>,
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

impl<P> fmt::Debug for CelsData<P>
where
    P: fmt::Debug,
{
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

impl<P> CelsData<P> {
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

    pub(crate) fn add_cel(&mut self, frame_id: u16, cel: RawCel<P>) -> Result<()> {
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

    pub(crate) fn frame_cels(&self, frame_id: u16) -> impl Iterator<Item = (u32, &RawCel<P>)> {
        self.data[frame_id as usize]
            .iter()
            .enumerate()
            .filter_map(|(layer_id, cel)| cel.as_ref().map(|c| (layer_id as u32, c)))
    }

    // Frame ID must be valid. If Layer ID is out of bounds always returns an
    // empty Vec.
    pub(crate) fn cel(&self, cel_id: CelId) -> Option<&RawCel<P>> {
        let CelId { frame, layer } = cel_id;
        let layers = &self.data[frame as usize];
        if (layer as usize) >= layers.len() {
            None
        } else {
            layers[layer as usize].as_ref()
        }
    }

    pub(crate) fn cel_mut(&mut self, cel_id: &CelId) -> Option<&mut RawCel<P>> {
        let frame = cel_id.frame;
        let layer = cel_id.layer;
        let layers = &mut self.data[frame as usize];
        if (layer as usize) >= layers.len() {
            None
        } else {
            layers[layer as usize].as_mut()
        }
    }
}

impl RawCel<RawPixels> {
    pub(crate) fn validate<F>(
        self,
        cel_id: CelId,
        layers: &LayersData,
        pixel_format: &PixelFormat,
        palette: Option<Arc<ColorPalette>>,
        validate_ref: &F,
    ) -> Result<RawCel<Pixels>>
    where
        F: Fn(CelId) -> Result<()>,
    {
        let content = match self.content {
            CelContent::Raw(image_content) => {
                let layer_is_background = layers[cel_id.layer as u32].is_background();
                let image_content =
                    image_content.validate(palette, pixel_format, layer_is_background)?;
                CelContent::Raw(image_content)
            }
            CelContent::Linked(other_frame) => {
                let ref_cel_id = CelId {
                    frame: other_frame as u16,
                    layer: cel_id.layer,
                };
                validate_ref(ref_cel_id)?;
                CelContent::Linked(other_frame)
            }
            CelContent::Tilemap(tilemap) => {
                if let LayerType::Tilemap(_) = layers[cel_id.layer as u32].layer_type {
                    // all good
                } else {
                    return Err(AsepriteParseError::InvalidInput(format!(
                        "Invalid cel. Tilemap Cel ({}) outside of tilemap layer.",
                        cel_id
                    )));
                }
                CelContent::Tilemap(tilemap)
            }
        };
        Ok(RawCel {
            data: self.data,
            content,
            user_data: self.user_data,
        })
    }
}

impl CelsData<RawPixels> {
    pub(crate) fn validate(
        self,
        layers: &LayersData,
        pixel_format: &PixelFormat,
        palette: Option<Arc<ColorPalette>>,
    ) -> Result<CelsData<Pixels>> {
        let num_frames = self.num_frames;
        let num_layers = layers.layers.len();
        let mut result = CelsData {
            data: Vec::with_capacity(self.data.len()),
            num_frames,
        };
        // Mapping from CelId -> bool. True if the cel can be used as a target
        // for a linked cel. That means it must exist, and it must be a raw cel.
        // We copy it out here, so we can consume the actual data in the
        // validation/transformation step.
        let mut is_linkable_cel: Vec<bool> = Vec::with_capacity(num_frames as usize * num_layers);
        for frame in 0..num_frames {
            for layer in 0..num_layers {
                let cel_id = CelId {
                    frame: frame as u16,
                    layer: layer as u16,
                };
                is_linkable_cel.push(self.cel(cel_id).map_or(false, |c| c.content.is_raw()));
            }
        }
        let validate_ref = |id: CelId| {
            let index = id.frame as usize * num_layers + id.layer as usize;
            if is_linkable_cel[index] {
                Ok(())
            } else {
                Err(AsepriteParseError::InvalidInput(format!(
                    "Cel {} is not a valid target for a linked cel",
                    id
                )))
            }
        };

        // Validate and transform each cel. Consumes input arrays.
        for (frame, cels_by_layer) in self.data.into_iter().enumerate() {
            result.data.push(Vec::with_capacity(cels_by_layer.len()));
            for (layer, opt_cel) in cels_by_layer.into_iter().enumerate() {
                let cel = if let Some(cel) = opt_cel {
                    let cel_id = CelId {
                        frame: frame as u16,
                        layer: layer as u16,
                    };
                    Some(cel.validate(
                        cel_id,
                        layers,
                        pixel_format,
                        palette.clone(),
                        &validate_ref,
                    )?)
                } else {
                    None
                };
                result.data[frame as usize].push(cel);
            }
        }

        Ok(result)
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
pub(crate) struct CelCommon {
    pub layer_index: u16,
    pub x: i16,
    pub y: i16,
    pub opacity: u8,
}

impl CelCommon {
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

#[derive(Debug)]
pub(crate) struct ImageContent<P> {
    pub size: ImageSize,
    pub pixels: P,
}

impl ImageContent<RawPixels> {
    fn validate(
        self,
        palette: Option<Arc<ColorPalette>>,
        pixel_format: &PixelFormat,
        layer_is_background: bool,
    ) -> Result<ImageContent<Pixels>> {
        let size = self.size;
        let pixels = self
            .pixels
            .validate(palette, pixel_format, layer_is_background)?;
        Ok(ImageContent { size, pixels })
    }
}

// CelContent holds data specific to each type of cel.
#[derive(Debug)]
pub(crate) enum CelContent<P> {
    Raw(ImageContent<P>),
    Linked(u16),
    Tilemap(TilemapData),
}

impl<P> CelContent<P> {
    fn is_raw(&self) -> bool {
        matches!(self, CelContent::Raw(_))
    }
}

impl CelContent<RawPixels> {
    fn parse<R: Read>(
        mut reader: AseReader<R>,
        pixel_format: PixelFormat,
        cel_type: u16,
    ) -> Result<Self> {
        match cel_type {
            0 => parse_raw_cel(reader, pixel_format).map(CelContent::Raw),
            1 => reader.word().map(CelContent::Linked),
            2 => parse_compressed_cel(reader, pixel_format).map(CelContent::Raw),
            3 => TilemapData::parse_chunk(reader).map(CelContent::Tilemap),
            _ => Err(AsepriteParseError::InvalidInput(format!(
                "Invalid/Unsupported Cel type: {}",
                cel_type
            ))),
        }
    }
}

// impl fmt::Debug for ImageContent<RawPixels> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "<{} bytes>", self.pixels.byte_count())
//     }
// }

#[derive(Debug)]
pub(crate) struct RawCel<P = Pixels> {
    pub data: CelCommon,
    pub content: CelContent<P>,
    pub user_data: Option<UserData>,
}

fn parse_raw_cel<R: Read>(
    mut reader: AseReader<R>,
    pixel_format: PixelFormat,
) -> Result<ImageContent<RawPixels>> {
    let size = ImageSize::parse(&mut reader)?;
    RawPixels::from_raw(reader, pixel_format, size.pixel_count())
        .map(|pixels| ImageContent { size, pixels })
}

fn parse_compressed_cel<R: Read>(
    mut reader: AseReader<R>,
    pixel_format: PixelFormat,
) -> Result<ImageContent<RawPixels>> {
    let size = ImageSize::parse(&mut reader)?;
    RawPixels::from_compressed(reader, pixel_format, size.pixel_count())
        .map(|pixels| ImageContent { size, pixels })
}

pub(crate) fn parse_chunk(data: &[u8], pixel_format: PixelFormat) -> Result<RawCel<RawPixels>> {
    let mut reader = AseReader::new(data);
    let data = CelCommon::parse(&mut reader)?;
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
