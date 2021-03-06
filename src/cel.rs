use crate::{AsepriteParseError, PixelFormat, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::fmt;
use std::io::{Cursor, Read};

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
    Raw {
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
            CelData::Raw {
                width,
                height,
                data: CelBytes(output),
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
            CelData::Raw {
                width,
                height,
                data: CelBytes(decoded_data),
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
