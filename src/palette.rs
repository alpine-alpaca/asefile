use crate::{parse::read_string, AsepriteParseError, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use nohash::IntMap;
use std::io::Cursor;

/// The color palette embedded in the file.
#[derive(Debug)]
pub struct ColorPalette {
    //entries: Vec<ColorPaletteEntry>,
    entries: IntMap<u32, ColorPaletteEntry>,
}

/// A single entry in a [ColorPalette].
#[derive(Debug)]
pub struct ColorPaletteEntry {
    id: u32,
    rgba8: [u8; 4],
    name: Option<String>,
}

impl ColorPalette {
    /// Total number of colors in the palette.
    pub fn num_colors(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Look up entry at given
    pub fn get(&self, index: u32) -> Option<&ColorPaletteEntry> {
        self.entries.get(&index)
    }
}

impl ColorPaletteEntry {
    /// The id of this entry is the same as its index in the palette.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the RGBA components as an array. Most color libraries allow you to
    /// build an instance of their color type from such an array.
    pub fn raw_rgba8(&self) -> [u8; 4] {
        self.rgba8
    }

    pub fn red(&self) -> u8 {
        self.rgba8[0]
    }

    pub fn green(&self) -> u8 {
        self.rgba8[1]
    }

    pub fn blue(&self) -> u8 {
        self.rgba8[2]
    }

    /// Alpha value of this color (0 = fully transparent, 255 = fully opaque).
    pub fn alpha(&self) -> u8 {
        self.rgba8[3]
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
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
    //let mut entries = Vec::with_capacity(count as usize);
    let mut entries = IntMap::default();

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
        let id = id + first_color_index;
        entries.insert(
            id,
            ColorPaletteEntry {
                id,
                rgba8: [red, green, blue, alpha],
                name,
            },
        );
    }

    Ok(ColorPalette { entries })
}
