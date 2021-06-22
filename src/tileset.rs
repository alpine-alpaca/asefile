use std::{
    collections::HashMap,
    io::{Read, Seek},
};

use crate::{pixel::Pixels, PixelFormat, Result};
use bitflags::bitflags;

use crate::{external_file::ExternalFileId, reader::AseReader};
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TilesetId(pub(crate) u32);
impl TilesetId {
    pub(crate) fn new(value: u32) -> Self {
        Self(value)
    }
    pub fn value(&self) -> &u32 {
        &self.0
    }
}
bitflags! {
    struct TilesetFlags: u32 {
        // Include link to external file.
        const LINKS_EXTERNAL_FILE = 0x0001;
        // Include tiles inside this file.
        const FILE_INCLUDES_TILES = 0x0002;
        // From the spec:
        // Tilemaps using this tileset use tile ID=0 as empty tile
        // (this is the new format). In rare cases this bit is off,
        // the empty tile will be equal to 0xffffffff (used in
        // internal versions of Aseprite).
        const EMPTY_TILE_IS_ID_ZERO = 0x0004;
    }
}
#[derive(Debug)]
pub struct ExternalTilesetReference {
    external_file_id: ExternalFileId,
    tileset_id: TilesetId,
}
impl ExternalTilesetReference {
    /// Id of the external file.
    pub fn external_file_id(&self) -> &ExternalFileId {
        &self.external_file_id
    }
    /// Tileset ID in the external file.
    pub fn tileset_id(&self) -> &TilesetId {
        &self.tileset_id
    }
    fn parse<T: Read + Seek>(reader: &mut AseReader<T>) -> Result<Self> {
        let external_file_id = reader.dword().map(ExternalFileId::new)?;
        let tileset_id = reader.dword().map(TilesetId)?;
        Ok(ExternalTilesetReference {
            external_file_id,
            tileset_id,
        })
    }
}

#[derive(Debug)]
pub struct TileSize {
    width: u16,
    height: u16,
}
impl TileSize {
    pub fn width(&self) -> &u16 {
        &self.width
    }
    pub fn height(&self) -> &u16 {
        &self.height
    }
    pub fn pixels_per_tile(&self) -> u16 {
        self.width * self.height
    }
}

#[derive(Debug)]
pub struct Tileset {
    pub(crate) id: TilesetId,
    pub(crate) empty_tile_is_id_zero: bool,
    pub(crate) tile_count: u32,
    pub(crate) tile_size: TileSize,
    pub(crate) base_index: i16,
    pub(crate) name: String,
    pub(crate) external_file: Option<ExternalTilesetReference>,
    pub(crate) pixels: Option<Pixels>,
}
impl Tileset {
    /// Tileset id.
    pub fn id(&self) -> &TilesetId {
        &self.id
    }
    /// From the Aseprite file spec:
    /// When true, tilemaps using this tileset use tile ID=0 as empty tile.
    /// In rare cases this is false, the empty tile will be equal to 0xffffffff (used in internal versions of Aseprite).
    pub fn empty_tile_is_id_zero(&self) -> &bool {
        &self.empty_tile_is_id_zero
    }
    /// Number of tiles.
    pub fn tile_count(&self) -> &u32 {
        &self.tile_count
    }
    /// Tile width and height.
    pub fn tile_size(&self) -> &TileSize {
        &self.tile_size
    }
    /// Number to show in the UI for the tile with index=0. Default is 1.
    /// Only used for Aseprite UI purposes. Not used for data representation.
    pub fn base_index(&self) -> &i16 {
        &self.base_index
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    /// When Some, includes a link to an external file.
    pub fn external_file(&self) -> Option<&ExternalTilesetReference> {
        self.external_file.as_ref()
    }

    pub(crate) fn parse_chunk(data: &[u8], pixel_format: PixelFormat) -> Result<Tileset> {
        let mut reader = AseReader::new(data);
        let id = reader.dword().map(|val| TilesetId(val))?;
        let flags = reader.dword().map(|val| TilesetFlags { bits: val })?;
        let empty_tile_is_id_zero = flags.contains(TilesetFlags::EMPTY_TILE_IS_ID_ZERO);
        let tile_count = reader.dword()?;
        let tile_width = reader.word()?;
        let tile_height = reader.word()?;
        let tile_size = TileSize {
            width: tile_width,
            height: tile_height,
        };
        let base_index = reader.short()?;
        // Reserved bytes
        reader.skip_bytes(14)?;
        let name = reader.string()?;
        let external_file = {
            if !flags.contains(TilesetFlags::LINKS_EXTERNAL_FILE) {
                None
            } else {
                Some(ExternalTilesetReference::parse(&mut reader)?)
            }
        };
        let pixels = {
            if !flags.contains(TilesetFlags::FILE_INCLUDES_TILES) {
                None
            } else {
                let _compressed_length = reader.dword()?;
                let expected_pixel_count =
                    (tile_count * (tile_height as u32) * (tile_width as u32)) as usize;
                Pixels::from_compressed(reader, pixel_format, expected_pixel_count).map(Some)?
            }
        };
        Ok(Tileset {
            id,
            empty_tile_is_id_zero,
            tile_count,
            tile_size,
            base_index,
            name,
            external_file,
            pixels,
        })
    }
}
#[derive(Debug)]
pub struct TilesetsById(HashMap<TilesetId, Tileset>);
impl TilesetsById {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }
    pub(crate) fn add(&mut self, tileset: Tileset) {
        self.0.insert(*tileset.id(), tileset);
    }
    pub fn map(&self) -> &HashMap<TilesetId, Tileset> {
        &self.0
    }
    pub fn get(&self, id: &TilesetId) -> Option<&Tileset> {
        self.0.get(id)
    }
}
