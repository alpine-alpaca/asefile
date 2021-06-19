use std::{
    collections::HashMap,
    io::{Read, Seek},
    ops::Index,
};

use crate::Result;
use bitflags::bitflags;

use crate::{external_file::ExternalFileId, reader::AseReader};
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TilesetId(u32);
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
        /// Include link to external file.
        const LINKS_EXTERNAL_FILE = 0x0001;
        /// Include tiles inside this file.
        const FILE_INCLUDES_TILES = 0x0002;
        /// Tilemaps using this tileset use tile ID=0 as empty tile
        /// (this is the new format). In rare cases this bit is off,
        /// the empty tile will be equal to 0xffffffff (used in
        /// internal versions of Aseprite).
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
    fn parse<T: Read + Seek>(
        reader: &mut AseReader<T>,
        flags: TilesetFlags,
    ) -> Result<Option<Self>> {
        if !flags.contains(TilesetFlags::LINKS_EXTERNAL_FILE) {
            return Ok(None);
        }
        let external_file_id = reader.dword().map(ExternalFileId::new)?;
        let tileset_id = reader.dword().map(TilesetId)?;
        Ok(Some(ExternalTilesetReference {
            external_file_id,
            tileset_id,
        }))
    }
}
#[derive(Debug)]
pub struct TilesData(Vec<u8>);
impl TilesData {
    pub fn data(&self) -> &Vec<u8> {
        &self.0
    }
    fn parse<T: Read + Seek>(
        reader: AseReader<T>,
        flags: TilesetFlags,
        tile_size: &TileSize,
        tile_count: &u32,
    ) -> Result<Option<Self>> {
        if !flags.contains(TilesetFlags::FILE_INCLUDES_TILES) {
            return Ok(None);
        }

        let data_length =
            tile_size.width as usize * (tile_size.height as usize * *tile_count as usize);
        // Currently does not work; needs PIXEL[] refactor from cel module
        // TODO: Fix this
        reader.unzip(data_length).map(TilesData).map(Some)
    }
}

#[derive(Debug)]
pub struct TileSize {
    width: u16,
    height: u16,
}

#[derive(Debug)]
pub struct Tileset {
    id: TilesetId,
    empty_tile_is_id_zero: bool,
    tile_count: u32,
    tile_size: TileSize,
    base_index: i16,
    name: String,
    external_file: Option<ExternalTilesetReference>,
    tiles_data: Option<TilesData>,
}
impl Tileset {
    /// Tileset id.
    pub fn id(&self) -> &TilesetId {
        &self.id
    }
    /// When true, tilemaps using this tileset use tile ID=0 as empty tile.
    /// In rare cases this is false, the empty tile will be equal to 0xffffffff.
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
    pub fn external_file(&self) -> &Option<ExternalTilesetReference> {
        &self.external_file
    }
    /// When Some, a tileset image.
    pub fn tiles_data(&self) -> &Option<TilesData> {
        &self.tiles_data
    }

    pub(crate) fn parse_chunk(data: &[u8]) -> Result<Tileset> {
        let mut reader = AseReader::new(data);
        let id = reader.dword().map(|val| TilesetId(val))?;
        let flags = reader.dword().map(|val| TilesetFlags { bits: val })?;
        let empty_tile_is_id_zero = flags.contains(TilesetFlags::EMPTY_TILE_IS_ID_ZERO);
        let tile_count = reader.dword()?;
        let width = reader.word()?;
        let height = reader.word()?;
        let tile_size = TileSize { width, height };
        let base_index = reader.short()?;
        // Reserved bytes
        reader.skip_bytes(14)?;
        let name = reader.string()?;
        let external_file = ExternalTilesetReference::parse(&mut reader, flags)?;
        let tiles_data = TilesData::parse(reader, flags, &tile_size, &tile_count)?;
        Ok(Tileset {
            id,
            empty_tile_is_id_zero,
            tile_count,
            tile_size,
            base_index,
            name,
            external_file,
            tiles_data,
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
}
impl Index<TilesetId> for TilesetsById {
    type Output = Tileset;
    fn index(&self, id: TilesetId) -> &Self::Output {
        let map = self.map();
        if map.contains_key(&id) {
            return &self.map()[&id];
        }
        panic!("no external file found for id")
    }
}
