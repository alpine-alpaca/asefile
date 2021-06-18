use std::io::{Read, Seek};

use crate::{cel::ImageSize, reader::AseReader, tile, Result};

#[derive(Debug)]
pub(crate) struct Tilemap {
    pub size: ImageSize,
    pub tiles: tile::Tiles,
    pub bits_per_tile: u16,
    pub bitmask_header: TileBitmaskHeader,
}
impl Tilemap {
    pub(crate) fn parse_chunk<R: Read + Seek>(mut reader: AseReader<R>) -> Result<Self> {
        let size = ImageSize::parse(&mut reader)?;
        let bits_per_tile = reader.word()?;
        let bitmask_header = TileBitmaskHeader::parse(&mut reader)?;
        // Reserved bytes
        reader.skip_bytes(10)?;
        let tile_size = tile::TileLength::from_bits_per_tile(bits_per_tile as usize)?;
        let tiles = tile::Tiles::unzip(reader, tile_size, size.pixel_count())?;
        Ok(Self {
            size,
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
    pub(crate) fn parse<R: Read + Seek>(mut reader: &mut AseReader<R>) -> Result<Self> {
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
