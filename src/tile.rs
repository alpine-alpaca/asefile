use crate::{reader::AseReader, tilemap::TileBitmaskHeader, Result};
use std::{
    io::{Read, Seek},
    ops::Index,
};

#[derive(Debug)]
pub(crate) struct TileId(pub(crate) u32);

#[derive(Debug)]
pub(crate) struct Tile {
    pub id: TileId,
    pub flip_x: bool,
    pub flip_y: bool,
    pub rotate_90cw: bool,
}
impl Tile {
    pub(crate) fn new(chunk: &[u8], header: &TileBitmaskHeader) -> Result<Self> {
        AseReader::new(chunk)
            .dword()
            .map(|bits| Self::parse(bits, header))
    }
    fn parse(bits: u32, header: &TileBitmaskHeader) -> Self {
        Self {
            id: TileId(bits & header.tile_id),
            flip_x: as_bool(bits & header.x_flip),
            flip_y: as_bool(bits & header.y_flip),
            rotate_90cw: as_bool(bits & header.rotate_90cw),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Tiles(Vec<Tile>);
impl Tiles {
    pub(crate) fn unzip<T: Read + Seek>(
        reader: AseReader<T>,
        expected_tile_count: usize,
        header: &TileBitmaskHeader,
    ) -> Result<Self> {
        // Only 32-bit tiles supported for now
        let expected_output_size = 4 * expected_tile_count;
        let bytes = reader.unzip(expected_output_size)?;
        let tiles: Result<Vec<Tile>> = bytes
            .chunks_exact(4)
            .map(|bytes| Tile::new(bytes, header))
            .collect();
        tiles.map(Self)
    }
}
impl Index<usize> for Tiles {
    type Output = Tile;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

fn as_bool(bit: u32) -> bool {
    assert!(bit == 0 || bit == 1);
    bit == 1
}
