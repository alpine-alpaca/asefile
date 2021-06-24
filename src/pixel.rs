use crate::{reader::AseReader, AsepriteParseError, ColorPalette, PixelFormat, Result};
use std::io::{Read, Seek};

// From Aseprite file spec:
// PIXEL: One pixel, depending on the image pixel format:
// Grayscale: BYTE[2], each pixel have 2 bytes in the order Value, Alpha.
// Indexed: BYTE, Each pixel uses 1 byte (the index).
// RGBA: BYTE[4], each pixel have 4 bytes in this order Red, Green, Blue, Alpha.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rgba {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}
impl Rgba {
    fn new(chunk: &[u8]) -> Result<Self> {
        let mut reader = AseReader::new(chunk);
        let red = reader.byte()?;
        let green = reader.byte()?;
        let blue = reader.byte()?;
        let alpha = reader.byte()?;
        Ok(Self {
            red,
            green,
            blue,
            alpha,
        })
    }
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
    ) -> Option<Rgba> {
        let index = self.0;
        palette.color(index as u32).map(|c| {
            let alpha = if transparent_color_index == index && !layer_is_background {
                0
            } else {
                c.alpha()
            };
            Rgba {
                red: c.red(),
                green: c.green(),
                blue: c.blue(),
                alpha,
            }
        })
    }
}

fn output_size(pixel_format: PixelFormat, expected_pixel_count: usize) -> usize {
    pixel_format.bytes_per_pixel() * expected_pixel_count
}
#[derive(Debug)]
pub(crate) enum Pixels {
    Rgba(Vec<Rgba>),
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
                let pixels: Result<Vec<_>> = bytes.chunks_exact(4).map(Rgba::new).collect();
                pixels.map(Self::Rgba)
            }
        }
    }
    pub(crate) fn from_raw<T: Read + Seek>(
        reader: AseReader<T>,
        pixel_format: PixelFormat,
        expected_pixel_count: usize,
    ) -> Result<Self> {
        let expected_output_size = output_size(pixel_format, expected_pixel_count);
        reader
            .take_bytes(expected_output_size)
            .and_then(|bytes| Self::from_bytes(bytes, pixel_format))
    }
    pub(crate) fn from_compressed<T: Read + Seek>(
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
}

pub(crate) fn resolve_indexed(
    pixel: &Indexed,
    palette: &ColorPalette,
    transparent_color_index: u8,
    layer_is_background: bool,
) -> Result<Rgba> {
    pixel
        .as_rgba(palette, transparent_color_index, layer_is_background)
        .ok_or_else(|| {
            AsepriteParseError::InvalidInput(format!(
                "Index out of range: {} (max: {})",
                pixel.value(),
                palette.num_colors()
            ))
        })
}

pub(crate) fn resolve_indexed_pixels(
    pixels: &[Indexed],
    palette: &ColorPalette,
    transparent_color_index: u8,
    layer_is_background: bool,
) -> Result<Vec<Rgba>> {
    pixels
        .iter()
        .map(|px| resolve_indexed(px, palette, transparent_color_index, layer_is_background))
        .collect()
}
