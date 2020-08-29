use crate::{AsepriteParseError, PixelFormat, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::io::{Cursor, Read};
use std::fmt;

#[derive(Debug)]
pub struct Cel {
    layer_index: u16,
    x: i16,
    y: i16,
    opacity: u8,
    data: CelData,
}

pub struct CelBytes(Vec<u8>);

#[derive(Debug)]
pub enum CelData {
    Raw { width: u16, height: u16, data: CelBytes },
    Linked(u16),
    ZlibData { width: u16, height: u16, data: CelBytes },
}

impl fmt::Debug for CelBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} bytes>", self.0.len())
    }
}


pub(crate) fn parse_cel_chunk(data: &[u8], pixel_format: PixelFormat) -> Result<Cel> {
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
            let width = input.read_u16::<LittleEndian>()?;
            let height = input.read_u16::<LittleEndian>()?;
            //println!("Raw data: {}x{}", width, height);
            let data_size = width as usize * height as usize * pixel_format.bytes_per_pixel();
            let mut output = Vec::with_capacity(data_size);
            input.take(data_size as u64).read_to_end(&mut output)?;
            if output.len() != data_size {
                return Err(AsepriteParseError::InvalidInput(format!(
                    "Invalid cel data size. Expected: {}, Actual: {}", data_size, output.len()
                )));
            }
            CelData::Raw {
                width, height,
                data: CelBytes(output)
            }
        }
        1 => {
            let linked = input.read_u16::<LittleEndian>()?;
            CelData::Linked(linked)
        }
        2 => {
            let width = input.read_u16::<LittleEndian>()?;
            let height = input.read_u16::<LittleEndian>()?;
            //let bytes_left = data.len() as u64 - input.position();
            //println!("Gzipped data: {}x{}, bytes:{}", width, height, bytes_left);
            //let d = &data[input.position() as usize..];
            //dump_bytes(d);
            let expected_output_size =
                width as usize * height as usize * pixel_format.bytes_per_pixel();
            //println!("Expected output: {}", expected_output_size);
            let decoded_data = unzip(input, expected_output_size)?;
            //println!("Gzipped bytes: {}", decoded_data.len());
            //dump_bytes(&decoded_data);
            CelData::Raw {
                width, height,
                data: CelBytes(decoded_data)
            }
        }
        _ => {
            return Err(AsepriteParseError::InvalidInput(format!(
                "Invalid/Unsupported Cel type: {}",
                cel_type
            )))
        }
    };

    Ok(Cel {
        layer_index,
        x,
        y,
        opacity,
        data: cel_data
    })
}

// fn read_raw_cell(input: Cursor<&[u8]>, pixel_format: PixelFormat) -> CelType {

// }

// Useful when debugging
#[allow(dead_code)]
fn dump_bytes(data: &[u8]) {
    let mut column = 0;
    for d in data {
        print!("{:02x} ", d);
        column += 1;
        if column >= 16 {
            column = 0;
            println!("");
        }
    }
}

pub(crate) fn unzip(input: Cursor<&[u8]>, expected_output_size: usize) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(input);
    let mut buffer = Vec::with_capacity(expected_output_size);
    decoder.read_to_end(&mut buffer)?;
    Ok(buffer)
}
