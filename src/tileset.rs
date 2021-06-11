use bitflags::bitflags;

use crate::external_file::ExternalFileId;
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TilesetId(u32);
impl TilesetId {
    fn new(id: u32) -> Self {
        Self(id)
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

pub struct ExternalTilesetReference {
    external_file_id: ExternalFileId,
    tileset_id: TilesetId,
}

pub struct TilesData {
    length: u32,
}

pub struct TileSize {
    width: u16,
    height: u16,
}

pub struct Tileset {
    id: TilesetId,
    empty_tile_is_id_zero: bool,
    tile_count: u32,
    tile_size: TileSize,
    base_index: u8,
    name: String,
    external_file: Option<ExternalTilesetReference>,
    tiles_data: Option<TilesData>,
}
