use std::io::{Read, Seek};
use std::iter::FromIterator;

use crate::{reader::AseReader, PixelFormat};
use crate::{ColorPalette, Result};

// From Aseprite file spec:
// PIXEL: One pixel, depending on the image pixel format:
// Grayscale: BYTE[2], each pixel have 2 bytes in the order Value, Alpha.
// Indexed: BYTE, Each pixel uses 1 byte (the index).
// RGBA: BYTE[4], each pixel have 4 bytes in this order Red, Green, Blue, Alpha.

pub struct Rgba {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}
impl Rgba {
    fn new(chunk: &[u8]) -> Self {
        Self {
            red: chunk[0],
            green: chunk[1],
            blue: chunk[2],
            alpha: chunk[3],
        }
    }
}
pub struct Grayscale {
    value: u8,
    alpha: u8,
}
impl Grayscale {
    fn new(chunk: &[u8]) -> Self {
        Self {
            value: chunk[0],
            alpha: chunk[1],
        }
    }
}
pub struct Indexed(u8);
impl Indexed {
    pub(crate) fn to_rgba(
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

pub enum Pixels {
    Rgba(Vec<Rgba>),
    Grayscale(Vec<Grayscale>),
    Indexed(Vec<Indexed>),
}
impl Pixels {
    pub(crate) fn unzip<T: Read + Seek>(
        reader: AseReader<T>,
        format: PixelFormat,
        expected_pixel_count: usize,
    ) -> Result<Self> {
        let bytes_per_pixel = format.bytes_per_pixel();
        let expected_output_size = bytes_per_pixel * expected_pixel_count;
        let bytes = reader.unzip(expected_output_size)?;
        Ok(match format {
            PixelFormat::Indexed { .. } => {
                let pixels = bytes.iter().map(|byte| Indexed(*byte)).collect();
                Self::Indexed(pixels)
            }
            PixelFormat::Grayscale => {
                assert!(bytes.len() % 2 == 0);
                let pixels = bytes.chunks_exact(2).map(Grayscale::new).collect();
                Self::Grayscale(pixels)
            }
            PixelFormat::Rgba => {
                assert!(bytes.len() % 4 == 0);
                let pixels = bytes.chunks_exact(4).map(Rgba::new).collect();
                Self::Rgba(pixels)
            }
        })
    }
}
