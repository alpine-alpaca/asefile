use std::io::Read;

use image::RgbaImage;

use crate::{
    cel::CelContent,
    reader::AseReader,
    tile::{self, Tile, EMPTY_TILE},
    AsepriteParseError, Cel, Result, Tileset,
};

/// A reference to a tilemap.
///
/// A tilemap describes an image as a collection of tiles from a [Tileset].
///
/// Every non-empty cel in a tilemap layer corresponds to one tilemap.
pub struct Tilemap<'a> {
    pub(crate) cel: Cel<'a>,
    pub(crate) tileset: &'a Tileset,
    pub(crate) logical_size: (u16, u16),
}

impl<'a> Tilemap<'a> {
    fn tilemap(&self) -> &TilemapData {
        if let CelContent::Tilemap(ref tilemap_data) = self.cel.raw_cel().unwrap().content {
            tilemap_data
        } else {
            panic!("Tilemap cel does not contain a tilemap")
        }
    }

    /// Width in number of tiles
    pub fn width(&self) -> u32 {
        self.logical_size.0 as u32
    }

    /// Height in number of tiles
    pub fn height(&self) -> u32 {
        self.logical_size.1 as u32
    }

    /// Width and height of each tile in the tilemap.
    pub fn tile_size(&self) -> (u32, u32) {
        let sz = self.tileset.tile_size();
        (sz.width() as u32, sz.height() as u32)
    }

    /// The tileset used by this tilemap.
    pub fn tileset(&self) -> &Tileset {
        self.tileset
    }

    /// The tilemap as one large image.
    pub fn image(&self) -> RgbaImage {
        self.cel.image()
    }

    /// Lookup tile at given location.
    ///
    /// Tile coordinates start at (0, 0) in the top left.
    ///
    /// Note: Aseprite as of 1.3-beta5 labels tile coordinates relative to the
    /// tile offsets. I.e., if your first column is empty, then the GUI shows
    /// `-1` for the x coordinate of the top-left tile.
    pub fn tile(&self, x: u32, y: u32) -> &Tile {
        let (ofs_x, ofs_y) = self.tile_offsets();
        let x = x as i32 - ofs_x;
        let y = y as i32 - ofs_y;
        // The actual tilemap data may be smaller because it does not include
        // any data for empty tiles on the outer rows or columns.
        let w = self.tilemap().width() as i32;
        let h = self.tilemap().height() as i32;
        if x < 0 || y < 0 || x >= w || y >= h {
            return &EMPTY_TILE;
        }
        let index = (y as usize * self.width() as usize) + x as usize;
        &self.tilemap().tiles[index]
    }

    /// Describes first not-empty tile.
    pub fn tile_offsets(&self) -> (i32, i32) {
        let (x, y) = self.pixel_offsets();
        let size = self.tileset().tile_size();
        (x / size.width() as i32, y / size.height() as i32)
    }

    /// Describes first non-empty tile in pixel offsets.
    pub fn pixel_offsets(&self) -> (i32, i32) {
        self.cel.top_left()
    }
}

#[derive(Debug)]
pub struct TilemapData {
    width: u16,
    height: u16,
    //tileset_id: TilesetId,
    tiles: tile::Tiles,
    bits_per_tile: u16,
    bitmask_header: TileBitmaskHeader,
}

impl TilemapData {
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
