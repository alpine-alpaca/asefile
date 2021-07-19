use std::{collections::HashMap, fmt, io::Read};

use crate::{pixel::Pixels, AsepriteParseError, ColorPalette, PixelFormat, Result};
use bitflags::bitflags;
use image::{Rgba, RgbaImage};

use crate::{external_file::ExternalFileId, reader::AseReader};

/// An id for a [Tileset].
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TilesetId(pub(crate) u32);

impl TilesetId {
    /// Create a new TilesetId over a raw u32 value.
    pub fn new(value: u32) -> Self {
        Self(value)
    }
    /// Get a reference to the underlying u32 value.
    pub fn value(&self) -> &u32 {
        &self.0
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

/// A [Tileset] reference to an [ExternalFile].
#[derive(Debug)]
pub struct ExternalTilesetReference {
    external_file_id: ExternalFileId,
    tileset_id: TilesetId,
}

impl ExternalTilesetReference {
    /// The id of the [ExternalFile].
    pub fn external_file_id(&self) -> &ExternalFileId {
        &self.external_file_id
    }

    /// The id of the [Tileset] in the [ExternalFile].
    pub fn tileset_id(&self) -> &TilesetId {
        &self.tileset_id
    }

    fn parse<T: Read>(reader: &mut AseReader<T>) -> Result<Self> {
        Ok(ExternalTilesetReference {
            external_file_id: reader.dword().map(ExternalFileId::new)?,
            tileset_id: reader.dword().map(TilesetId)?,
        })
    }
}

/// The size of a tile in pixels.
#[derive(Debug)]
pub struct TileSize {
    width: u16,
    height: u16,
}

impl TileSize {
    /// Tile width in pixels.
    pub fn width(&self) -> &u16 {
        &self.width
    }

    /// Tile height in pixels.
    pub fn height(&self) -> &u16 {
        &self.height
    }

    pub(crate) fn pixels_per_tile(&self) -> u16 {
        self.width * self.height
    }
}

/// Various attributes of a tileset.
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

    /// Tileset name. May not be unique among tilesets.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// When Some, includes a link to an external file.
    pub fn external_file(&self) -> Option<&ExternalTilesetReference> {
        self.external_file.as_ref()
    }

    pub(crate) fn write_to_image(&self, image_pixels: &[Rgba<u8>]) -> RgbaImage {
        let Tileset {
            tile_size,
            tile_count,
            ..
        } = self;
        let TileSize { width, height } = tile_size;
        let tile_width = *width as u32;
        let tile_height = *height as u32;
        let pixels_per_tile = tile_size.pixels_per_tile() as u32;
        let image_height = tile_count * tile_height;
        let mut image = RgbaImage::new(tile_width, image_height);
        for tile_idx in 0..*tile_count {
            let pixel_idx_offset = tile_idx * pixels_per_tile;
            // tile_y and tile_x are positions relative to the current tile.
            for tile_y in 0..tile_height {
                // pixel_y is the absolute y position of the pixel on the image.
                let pixel_y = tile_y + (tile_idx * tile_height);
                for tile_x in 0..tile_width {
                    let sub_index = (tile_y * tile_width) + tile_x;
                    let pixel_idx = sub_index + pixel_idx_offset;
                    let image_pixel = image_pixels[pixel_idx as usize];
                    // Absolute pixel x is equal to tile_x.
                    image.put_pixel(tile_x, pixel_y, image_pixel);
                }
            }
        }

        image
    }

    pub(crate) fn parse_chunk(data: &[u8], pixel_format: PixelFormat) -> Result<Tileset> {
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

/// A map of [TilesetId] values to [Tileset] instances.
#[derive(Debug)]
pub struct TilesetsById(HashMap<TilesetId, Tileset>);

impl TilesetsById {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn add(&mut self, tileset: Tileset) {
        self.0.insert(*tileset.id(), tileset);
    }

    /// Returns a reference to the underlying HashMap value.
    pub fn map(&self) -> &HashMap<TilesetId, Tileset> {
        &self.0
    }

    /// Get a reference to a [Tileset] from a [TilesetId], if the entry exists.
    pub fn get(&self, id: &TilesetId) -> Option<&Tileset> {
        self.0.get(id)
    }

    pub(crate) fn validate(
        &self,
        pixel_format: &PixelFormat,
        palette: &Option<ColorPalette>,
    ) -> Result<()> {
        for tileset in self.0.values() {
            // Validates that all Tilesets contain their own pixel data.
            // External file references currently not supported.
            let pixels = tileset.pixels.as_ref().ok_or_else(|| {
                AsepriteParseError::UnsupportedFeature(
                    "Expected Tileset data to contain pixels. External file Tilesets not supported"
                        .into(),
                )
            })?;

            if let Pixels::Indexed(indexed_pixels) = pixels {
                let palette = palette.as_ref().ok_or_else(|| {
                    AsepriteParseError::InvalidInput(
                        "Expected a palette present when resolving indexed image".into(),
                    )
                })?;
                palette.validate_indexed_pixels(indexed_pixels)?;

                // Validates that the file PixelFormat is indexed if the Tileset is indexed.
                if let PixelFormat::Indexed { .. } = pixel_format {
                    // Format matches tileset content, ok
                } else {
                    return Err(AsepriteParseError::InvalidInput(
                        "Found indexed tileset pixels in non-indexed pixel format.".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// An error occured while generating a tileset image.
#[derive(Debug)]
pub enum TilesetImageError {
    /// No tileset was found for the provided id.
    MissingTilesetId(TilesetId),
    /// No pixel data is contained in the tileset with the provided id.
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
