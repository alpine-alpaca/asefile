//! Utilities not directly related to Aseprite, but useful for processing the
//! resulting image data. (Requires feature `utils`.)
//!
//! This module is not available by default. To use it, you must enable the
//! feature `utils` in your `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! asefile = { version = "0.3", features = ["utils"] }
//! ```

use image::RgbaImage;
use nohash::IntMap;
use std::iter::once;

use crate::ColorPalette;

/// Add a 1 pixel border around the input image by duplicating the outmost
/// pixels.
///
/// This can be useful when creating a texture atlas for sprites that represent
/// tiles. Without this, under certain zoom levels there might be small gaps
/// between tiles. For an example, see this [discussion of the problem on
/// StackOverflow][1].
///
/// [1]: https://gamedev.stackexchange.com/questions/148247/prevent-tile-layout-gaps
///
/// Many sprite atlas generation tools have this as a built-in feature. In that
/// case you don't need to use this function.
pub fn extrude_border(image: RgbaImage) -> RgbaImage {
    let (w, h) = image.dimensions();
    let w = w as usize;
    let h = h as usize;
    let src = image.as_raw();
    let bpp = 4; // bytes per pixel
    let mut data: Vec<u8> = Vec::with_capacity(4 * (w + 2) * (h + 2));
    let bpp_w = bpp * w;
    for src_row in once(0).chain(0..h).chain(once(h - 1)) {
        let ofs = src_row * bpp * w;
        data.extend_from_slice(&src[ofs..ofs + bpp]); // (0, r)
        data.extend_from_slice(&src[ofs..ofs + bpp_w]); // (0, r)..(w-1, r);
        data.extend_from_slice(&src[ofs + bpp_w - bpp..ofs + bpp_w]);
    }
    RgbaImage::from_raw((w + 2) as u32, (h + 2) as u32, data).unwrap()
}

/// A helper for mapping `Rgba` values into indexes in a color palette.
pub struct PaletteMapper {
    map: IntMap<u32, u8>,
    transparent: u8,
    failure: u8,
}

/// Configuration of palette mapping.
pub struct MappingOptions {
    /// If pixel is not in the palette, use this index.
    pub failure: u8,
    /// If pixel is transparent (`alpha != 255`), use this index. If `None`
    /// transparent pixels are treated as failures.
    pub transparent: Option<u8>,
}

impl PaletteMapper {
    /// Create a new mapper from a color palette.
    pub fn new(palette: &ColorPalette, options: MappingOptions) -> PaletteMapper {
        let mut map = IntMap::default();
        for (idx, entry) in palette.entries.iter() {
            let m =
                entry.red() as u32 + ((entry.green() as u32) << 8) + ((entry.blue() as u32) << 16);
            let col = if *idx < 256 {
                *idx as u8
            } else {
                options.failure
            };
            let _ = map.insert(m, col);
        }
        PaletteMapper {
            map,
            transparent: options.transparent.unwrap_or(options.failure),
            failure: options.failure,
        }
    }

    /// Look up a color in the palette.
    ///
    /// An `alpha` other than `255` is considered transparent. If the color
    /// is not in the palette returns the failure color.
    pub fn lookup(&self, r: u8, g: u8, b: u8, alpha: u8) -> u8 {
        if alpha != 255 {
            return self.transparent;
        }
        let m = r as u32 + ((g as u32) << 8) + ((b as u32) << 16);
        *self.map.get(&m).unwrap_or(&self.failure)
    }
}

/// Turn an `RgbaImage` into an indexed image.
///
/// Returns image dimensions and raw index data.
///
/// # Example
///
/// ```
/// # use asefile::AsepriteFile;
/// # use std::path::Path;
/// # let asefile_path = Path::new("./tests/data/util_indexed.aseprite");
/// # let output_dir = Path::new("./tests/data");
/// # let ase = AsepriteFile::read_file(&asefile_path).unwrap();
/// use asefile::util::{PaletteMapper, MappingOptions, to_indexed_image};
/// let img = ase.frame(0).image();
/// assert!(ase.is_indexed_color());
/// let mapper = PaletteMapper::new(
///     ase.palette().unwrap(),
///     MappingOptions {
///         transparent: ase.transparent_color_index(),
///         failure: 0,
///     }
/// );
/// let ((w, h), data) = to_indexed_image(img, &mapper);
/// assert_eq!(data.len(), (w * h) as usize);
/// ```
pub fn to_indexed_image(image: RgbaImage, mapper: &PaletteMapper) -> ((u32, u32), Vec<u8>) {
    let data = image
        .pixels()
        .map(|c| mapper.lookup(c.0[0], c.0[1], c.0[2], c.0[3]))
        .collect();
    (image.dimensions(), data)
}
