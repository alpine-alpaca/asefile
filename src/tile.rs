use crate::{reader::AseReader, tilemap::TileBitmaskHeader, Result};
use std::{io::Read, ops::Index};

#[derive(Debug, Clone, Copy)]
pub struct TileId(pub u32);

/// A tile is a reference to a single tile in a tilemap.
#[derive(Debug, Clone)]
pub struct Tile {
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
    pub(crate) fn unzip<T: Read>(
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
        Ok(Self(tiles?))
    }
}

impl Index<usize> for Tiles {
    type Output = Tile;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

fn as_bool(bitwise_and: u32) -> bool {
    bitwise_and != 0
}
