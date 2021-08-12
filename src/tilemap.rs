use std::io::Read;

use crate::{
    reader::AseReader,
    tile::{self, Tile},
    AsepriteParseError, Result,
};

/// A tilemap describes an image as a collection of tiles from a [crate::Tileset].
///
///
#[derive(Debug)]
pub struct Tilemap {
    width: u16,
    height: u16,
    //tileset_id: TilesetId,
    tiles: tile::Tiles,
    bits_per_tile: u16,
    bitmask_header: TileBitmaskHeader,
}

impl Tilemap {
    /// Width in number of tiles
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Height in number of tiles
    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn tile(&self, x: u16, y: u16) -> Option<&Tile> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y as usize * self.width as usize) + x as usize;
        Some(&self.tiles[index])
    }

    pub(crate) fn parse_chunk<R: Read>(mut reader: AseReader<R>) -> Result<Self> {
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
        reader.skip_reserved(10)?;
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
    pub(crate) fn parse<R: Read>(reader: &mut AseReader<R>) -> Result<Self> {
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
