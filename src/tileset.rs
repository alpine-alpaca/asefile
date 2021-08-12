use std::{collections::HashMap, error::Error, fmt, io::Read, sync::Arc};

use crate::{
    pixel::{Pixels, RawPixels},
    AsepriteParseError, ColorPalette, PixelFormat, Result,
};
use bitflags::bitflags;
use image::RgbaImage;

use crate::{external_file::ExternalFileId, reader::AseReader};

/// An id for a [Tileset].
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TilesetId(pub(crate) u32);

impl TilesetId {
    /// Create a new `TilesetId` from a raw `u32` value.
    pub fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Get the underlying `u32` value.
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for TilesetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TilesetId({})", self.0)
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

/// A [Tileset] reference to an [crate::ExternalFile].
#[derive(Debug, Clone)]
pub struct ExternalTilesetReference {
    external_file_id: ExternalFileId,
    tileset_id: TilesetId,
}

impl ExternalTilesetReference {
    /// The id of the [crate::ExternalFile].
    pub fn external_file_id(&self) -> ExternalFileId {
        self.external_file_id
    }

    /// The id of the [Tileset] in the [crate::ExternalFile].
    pub fn tileset_id(&self) -> TilesetId {
        self.tileset_id
    }

    fn parse<T: Read>(reader: &mut AseReader<T>) -> Result<Self> {
        Ok(ExternalTilesetReference {
            external_file_id: reader.dword().map(ExternalFileId::new)?,
            tileset_id: reader.dword().map(TilesetId)?,
        })
    }
}

/// The size of a tile in pixels.
#[derive(Debug, Clone, Copy)]
pub struct TileSize {
    width: u16,
    height: u16,
}

impl TileSize {
    /// Tile width in pixels.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Tile height in pixels.
    pub fn height(&self) -> u16 {
        self.height
    }

    pub(crate) fn pixels_per_tile(&self) -> u32 {
        self.width as u32 * self.height as u32
    }
}

/// A set of tiles of the same size.
///
/// In the GUI, this is the collection of tiles that you build up in the side
/// bar. Each tile has the same size and is identified by an Id.
///
/// See [official docs for tilemaps and tilesets](https://www.aseprite.org/docs/tilemap/)
/// for details.
#[derive(Debug)]
pub struct Tileset<P = Pixels> {
    pub(crate) id: TilesetId,
    pub(crate) empty_tile_is_id_zero: bool,
    pub(crate) tile_count: u32,
    pub(crate) tile_size: TileSize,
    pub(crate) base_index: i16,
    pub(crate) name: String,
    pub(crate) external_file: Option<ExternalTilesetReference>,
    pub(crate) pixels: Option<P>,
}

impl<P> Tileset<P> {
    /// Tileset id.
    pub fn id(&self) -> TilesetId {
        self.id
    }

    /// From the Aseprite file spec:
    /// When true, tilemaps using this tileset use tile ID=0 as empty tile.
    /// In rare cases this is false, the empty tile will be equal to 0xffffffff (used in internal versions of Aseprite).
    pub fn empty_tile_is_id_zero(&self) -> bool {
        self.empty_tile_is_id_zero
    }

    /// Number of tiles.
    pub fn tile_count(&self) -> u32 {
        self.tile_count
    }

    /// Tile width and height.
    pub fn tile_size(&self) -> TileSize {
        self.tile_size
    }

    /// Number to show in the UI for the tile with index=0. Default is 1.
    /// Only used for Aseprite UI purposes. Not used for data representation.
    pub fn base_index(&self) -> i16 {
        self.base_index
    }

    /// Tileset name. May not be unique among tilesets.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// When non-empty, describes a link to an external file.
    pub fn external_file(&self) -> Option<&ExternalTilesetReference> {
        self.external_file.as_ref()
    }
}

impl Tileset<RawPixels> {
    pub(crate) fn parse_chunk(
        data: &[u8],
        pixel_format: PixelFormat,
    ) -> Result<Tileset<RawPixels>> {
        let mut reader = AseReader::new(data);
        let id = reader.dword().map(TilesetId)?;
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
        reader.skip_reserved(14)?;
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
                RawPixels::from_compressed(reader, pixel_format, expected_pixel_count).map(Some)?
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

impl Tileset<Pixels> {
    /// Get the image for the given tile.
    pub fn tile_image(&self, tile_index: u32) -> RgbaImage {
        assert!(tile_index < self.tile_count());
        let width = self.tile_size.width() as u32;
        let height = self.tile_size.height() as u32;
        let pixels = self.pixels.as_ref().expect("No pixel data in tileset");
        let pixels_per_tile = (width * height) as usize;
        let start_ofs = tile_index as usize * pixels_per_tile;
        let raw: Vec<u8> = pixels
            .clone_as_image_rgba()
            .into_owned()
            .into_iter()
            .skip(start_ofs)
            .take(pixels_per_tile)
            .flat_map(|pixel| pixel.0)
            .collect();
        RgbaImage::from_raw(width, height, raw).expect("Mismatched image size")
    }

    // Collect all tiles into one long vertical image.
    pub(crate) fn image(&self) -> RgbaImage {
        let width = self.tile_size.width() as u32;
        let tile_height = self.tile_size.height() as u32;
        let image_height = tile_height * self.tile_count;
        let pixels = self.pixels.as_ref().expect("No pixel data in tileset");

        let raw: Vec<u8> = pixels
            .clone_as_image_rgba()
            .into_owned()
            .into_iter()
            .flat_map(|pixel| pixel.0)
            .collect();
        RgbaImage::from_raw(width, image_height, raw).expect("Mismatched image size")
    }
}

/// A map from [TilesetId]s to [Tileset]s.
#[derive(Debug)]
pub struct TilesetsById<P = Pixels>(HashMap<TilesetId, Tileset<P>>);

impl<P> TilesetsById<P> {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn add(&mut self, tileset: Tileset<P>) {
        self.0.insert(tileset.id(), tileset);
    }

    /// Returns a reference to the underlying HashMap value.
    pub fn map(&self) -> &HashMap<TilesetId, Tileset<P>> {
        &self.0
    }

    /// Get a reference to a [Tileset] from a [TilesetId], if the entry exists.
    pub fn get(&self, id: TilesetId) -> Option<&Tileset<P>> {
        self.0.get(&id)
    }
}

impl TilesetsById<RawPixels> {
    pub(crate) fn validate(
        self,
        pixel_format: &PixelFormat,
        palette: Option<Arc<ColorPalette>>,
    ) -> Result<TilesetsById<Pixels>> {
        let mut result = HashMap::with_capacity(self.0.capacity());
        for (id, tileset) in self.0.into_iter() {
            // Validates that all Tilesets contain their own pixel data.
            // External file references currently not supported.
            let _ = tileset.pixels.as_ref().ok_or_else(|| {
                AsepriteParseError::UnsupportedFeature(
                    "Expected Tileset data to contain pixels. External file Tilesets not supported"
                        .into(),
                )
            })?;

            let pixels = tileset
                .pixels
                .unwrap()
                .validate(palette.clone(), pixel_format, false)?;

            result.insert(
                id,
                Tileset {
                    pixels: Some(pixels),
                    id: tileset.id,
                    empty_tile_is_id_zero: tileset.empty_tile_is_id_zero,
                    tile_count: tileset.tile_count,
                    tile_size: tileset.tile_size,
                    base_index: tileset.base_index,
                    name: tileset.name,
                    external_file: tileset.external_file,
                },
            );
        }
        Ok(TilesetsById(result))
    }
}

/// An error occured while generating a tileset image.
#[derive(Debug, Clone)]
pub enum TilesetImageError {
    /// No tileset was found for the given id.
    MissingTilesetId(TilesetId),
    /// No pixel data contained in the tileset with the given id.
    NoPixelsInTileset(TilesetId),
}

impl fmt::Display for TilesetImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TilesetImageError::MissingTilesetId(tileset_id) => {
                write!(f, "No tileset found with id: {}", tileset_id)
            }
            TilesetImageError::NoPixelsInTileset(tileset_id) => {
                write!(f, "No pixel data for tileset with id: {}", tileset_id)
            }
        }
    }
}

impl Error for TilesetImageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            _ => None,
        }
    }
}
