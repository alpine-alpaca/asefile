use image::{Pixel, Rgba};

use crate::{reader::AseReader, AsepriteParseError, ColorPalette, PixelFormat, Result};
use std::{borrow::Cow, io::Read};

// From Aseprite file spec:
// PIXEL: One pixel, depending on the image pixel format:
// Grayscale: BYTE[2], each pixel have 2 bytes in the order Value, Alpha.
// Indexed: BYTE, Each pixel uses 1 byte (the index).
// RGBA: BYTE[4], each pixel have 4 bytes in this order Red, Green, Blue, Alpha.

fn read_rgba(chunk: &[u8]) -> Result<Rgba<u8>> {
    let mut reader = AseReader::new(chunk);
    let red = reader.byte()?;
    let green = reader.byte()?;
    let blue = reader.byte()?;
    let alpha = reader.byte()?;
    Ok(Rgba::from_channels(red, green, blue, alpha))
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Grayscale {
    value: u8,
    alpha: u8,
}

impl Grayscale {
    fn new(chunk: &[u8]) -> Result<Self> {
        let mut reader = AseReader::new(chunk);
        let value = reader.byte()?;
        let alpha = reader.byte()?;
        Ok(Self { value, alpha })
    }

    pub(crate) fn into_rgba(self) -> Rgba<u8> {
        let Self { value, alpha } = self;
        Rgba::from_channels(value, value, value, alpha)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Indexed(u8);

impl Indexed {
    pub(crate) fn value(&self) -> u8 {
        self.0
    }

    pub(crate) fn as_rgba(
        &self,
        palette: &ColorPalette,
        transparent_color_index: u8,
        layer_is_background: bool,
    ) -> Option<Rgba<u8>> {
        let index = self.0;
        palette.color(index as u32).map(|c| {
            let alpha = if transparent_color_index == index && !layer_is_background {
                0
            } else {
                c.alpha()
            };
            Rgba::from_channels(c.red(), c.green(), c.blue(), alpha)
        })
    }
}

fn output_size(pixel_format: PixelFormat, expected_pixel_count: usize) -> usize {
    pixel_format.bytes_per_pixel() * expected_pixel_count
}

#[derive(Debug)]
pub(crate) enum Pixels {
    Rgba(Vec<Rgba<u8>>),
    Grayscale(Vec<Grayscale>),
    Indexed(Vec<Indexed>),
}

impl Pixels {
    fn from_bytes(bytes: Vec<u8>, pixel_format: PixelFormat) -> Result<Self> {
        match pixel_format {
            PixelFormat::Indexed { .. } => {
                let pixels = bytes.iter().map(|byte| Indexed(*byte)).collect();
                Ok(Self::Indexed(pixels))
            }
            PixelFormat::Grayscale => {
                if bytes.len() % 2 != 0 {
                    return Err(AsepriteParseError::InvalidInput(
                        "Incorrect length of bytes for Grayscale image data".to_string(),
                    ));
                }
                let pixels: Result<Vec<_>> = bytes.chunks_exact(2).map(Grayscale::new).collect();
                pixels.map(Self::Grayscale)
            }
            PixelFormat::Rgba => {
                if bytes.len() % 4 != 0 {
                    return Err(AsepriteParseError::InvalidInput(
                        "Incorrect length of bytes for RGBA image data".to_string(),
                    ));
                }
                let pixels: Result<Vec<_>> = bytes.chunks_exact(4).map(read_rgba).collect();
                pixels.map(Self::Rgba)
            }
        }
    }

    pub(crate) fn from_raw<T: Read>(
        reader: AseReader<T>,
        pixel_format: PixelFormat,
        expected_pixel_count: usize,
    ) -> Result<Self> {
        let expected_output_size = output_size(pixel_format, expected_pixel_count);
        reader
            .take_bytes(expected_output_size)
            .and_then(|bytes| Self::from_bytes(bytes, pixel_format))
    }

    pub(crate) fn from_compressed<T: Read>(
        reader: AseReader<T>,
        pixel_format: PixelFormat,
        expected_pixel_count: usize,
    ) -> Result<Self> {
        let expected_output_size = output_size(pixel_format, expected_pixel_count);
        reader
            .unzip(expected_output_size)
            .and_then(|bytes| Self::from_bytes(bytes, pixel_format))
    }

    pub(crate) fn byte_count(&self) -> usize {
        match self {
            Pixels::Rgba(v) => v.len() * 4,
            Pixels::Grayscale(v) => v.len() * 2,
            Pixels::Indexed(v) => v.len(),
        }
    }

    // Returns a Borrowed Cow if the Pixels struct already contains Rgba pixels.
    // Otherwise clones them to create an Owned Cow.
    pub(crate) fn clone_as_image_rgba(
        &self,
        index_resolver_data: IndexResolverData<'_>,
    ) -> Cow<Vec<image::Rgba<u8>>> {
        match self {
            Pixels::Rgba(rgba) => Cow::Borrowed(rgba),
            Pixels::Grayscale(grayscale) => {
                Cow::Owned(grayscale.iter().map(|gs| gs.into_rgba()).collect())
            }
            Pixels::Indexed(indexed) => {
                let IndexResolverData {
                    palette,
                    transparent_color_index,
                    layer_is_background,
                } = index_resolver_data;
                let palette = palette.expect("Expected a palette when resolving indexed pixels.  Should have been caught in validation");
                let transparent_color_index = transparent_color_index.expect(
                    "Indexed pixels in non-indexed pixel format. Should have been caught in validation",
                );
                let resolver = |px: &Indexed| {
                    px.as_rgba(palette, transparent_color_index, layer_is_background)
                        .expect("Indexed pixel out of range. Should have been caught in validation")
                };
                Cow::Owned(indexed.iter().map(resolver).collect())
            }
        }
    }
}

pub(crate) struct IndexResolverData<'a> {
    pub(crate) palette: Option<&'a ColorPalette>,
    pub(crate) transparent_color_index: Option<u8>,
    pub(crate) layer_is_background: bool,
}
