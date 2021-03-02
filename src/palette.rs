use crate::{parse::read_string, AsepriteParseError, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug)]
pub struct ColorPalette {
    pub entries: Vec<ColorPaletteEntry>,
}

#[derive(Debug)]
pub struct ColorPaletteEntry {
    pub id: u32,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
    pub name: Option<String>,
}

pub(crate) fn parse_palette_chunk(data: &[u8]) -> Result<ColorPalette> {
    let mut input = Cursor::new(data);

    let _num_total_entries = input.read_u32::<LittleEndian>()?;
    let first_color_index = input.read_u32::<LittleEndian>()?;
    let last_color_index = input.read_u32::<LittleEndian>()?;
    let _reserved = input.read_u64::<LittleEndian>()?;

    if last_color_index < first_color_index {
        return Err(AsepriteParseError::InvalidInput(format!(
            "Bad palette color indices: first={} last={}",
            first_color_index, last_color_index,
        )));
    }

    let count = last_color_index - first_color_index + 1;
    let mut entries = Vec::with_capacity(count as usize);
    for id in 0..count {
        let flags = input.read_u16::<LittleEndian>()?;
        let red = input.read_u8()?;
        let green = input.read_u8()?;
        let blue = input.read_u8()?;
        let alpha = input.read_u8()?;
        let name = if flags & 1 == 1 {
            let s = read_string(&mut input)?;
            Some(s)
        } else {
            None
        };
        entries.push(ColorPaletteEntry {
            id: id + first_color_index,
            red,
            green,
            blue,
            alpha,
            name,
        })
    }

    Ok(ColorPalette { entries })
}
