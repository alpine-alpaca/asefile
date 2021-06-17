use crate::{reader::AseReader, AsepriteParseError, Result};
use std::io::{Read, Seek};

pub enum TileSize {
    Byte,
    Word,
    DWord,
}
impl TileSize {
    pub(crate) fn from_bits_per_tile(bits_per_tile: usize) -> Result<Self> {
        match bits_per_tile {
            8 => Ok(Self::Byte),
            16 => Ok(Self::Word),
            32 => Ok(Self::DWord),
            _ => Err(AsepriteParseError::InvalidInput(format!(
                "Invalid number of bits per tile. Expected 8, 16, or 32, got: {}",
                bits_per_tile
            ))),
        }
    }
    fn to_bytes_per_tile(&self) -> usize {
        match self {
            TileSize::Byte => 1,
            TileSize::Word => 2,
            TileSize::DWord => 4,
        }
    }
}
#[derive(Debug)]
pub struct Tile8(u8);
#[derive(Debug)]
pub struct Tile16(u16);
impl Tile16 {
    fn new(chunk: &[u8]) -> Result<Self> {
        AseReader::new(chunk).word().map(Self)
    }
}
#[derive(Debug)]
pub struct Tile32(u32);
impl Tile32 {
    fn new(chunk: &[u8]) -> Result<Self> {
        AseReader::new(chunk).dword().map(Self)
    }
}
#[derive(Debug)]
pub enum Tiles {
    Byte(Vec<Tile8>),
    Word(Vec<Tile16>),
    DWord(Vec<Tile32>),
}
impl Tiles {
    pub(crate) fn unzip<T: Read + Seek>(
        reader: AseReader<T>,
        tile_size: TileSize,
        expected_tile_count: usize,
    ) -> Result<Self> {
        let expected_output_size = tile_size.to_bytes_per_tile() * expected_tile_count;
        let bytes = reader.unzip(expected_output_size)?;
        match tile_size {
            TileSize::Byte => {
                let tiles = bytes.iter().map(|byte| Tile8(*byte)).collect();
                Ok(Self::Byte(tiles))
            }
            TileSize::Word => {
                assert!(bytes.len() % 2 == 0);
                let tiles: Result<Vec<_>> = bytes.chunks_exact(2).map(Tile16::new).collect();
                tiles.map(Self::Word)
            }
            TileSize::DWord => {
                assert!(bytes.len() % 4 == 0);
                let tiles: Result<Vec<_>> = bytes.chunks_exact(4).map(Tile32::new).collect();
                tiles.map(Self::DWord)
            }
        }
    }
}
