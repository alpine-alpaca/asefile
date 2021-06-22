use std::io::{Read, Seek};

use crate::{reader::AseReader, tile, AsepriteParseError, Result};

#[derive(Debug)]
pub(crate) struct Tilemap {
    pub width: u16,  // width in number of tiles
    pub height: u16, // height in number of tiles
    pub tiles: tile::Tiles,
    pub bits_per_tile: u16,
    pub bitmask_header: TileBitmaskHeader,
}
impl Tilemap {
    pub(crate) fn parse_chunk<R: Read + Seek>(mut reader: AseReader<R>) -> Result<Self> {
        let width = reader.word()?;
        let height = reader.word()?;
        let bits_per_tile = reader.word()?;
        if bits_per_tile != 32 {
            return Err(AsepriteParseError::UnsupportedFeature(format!(
                "Asefile only supports 32 bits per tile, got input with {} bits per tile",
                bits_per_tile
            )));
        }
        let bitmask_header = TileBitmaskHeader::parse(&mut reader)?;
        // Reserved bytes
        reader.skip_bytes(10)?;
        let expected_tile_count = width as usize * height as usize;
        let tiles = tile::Tiles::unzip(reader, expected_tile_count, &bitmask_header)?;
        Ok(Self {
            width,
            height,
            tiles,
            bits_per_tile,
            bitmask_header,
        })
    }
}

#[derive(Debug)]
pub(crate) struct TileBitmaskHeader {
    pub tile_id: u32,
    pub x_flip: u32,
    pub y_flip: u32,
    pub rotate_90cw: u32,
}
impl TileBitmaskHeader {
    pub(crate) fn parse<R: Read + Seek>(reader: &mut AseReader<R>) -> Result<Self> {
        let tile_id = reader.dword()?;
        let x_flip = reader.dword()?;
        let y_flip = reader.dword()?;
        let rotate_90cw = reader.dword()?;
        Ok(Self {
            tile_id,
            x_flip,
            y_flip,
            rotate_90cw,
        })
    }
}
