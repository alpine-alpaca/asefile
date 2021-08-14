#![warn(clippy::all)]
#![warn(missing_docs)]
/*!

Utilities for loading [Aseprite](https://www.aseprite.org/) files. This
library directly reads the binary Aseprite files ([file format
specification][spec]) and does not require you to export files to JSON. This
should make it fast enough to load your assets when the game boots up. You can
also use it to build your own asset pipelines.

Note that this library can be rather slow when compiled without optimizations.
We recommend that you override the optimization settings for this dependency
in dev mode by adding the following to your `Cargo.toml`:

```text
[profile.dev.package.asefile]
opt-level = 2  # or 3
```

This is not necessary if you already have a wildcard override. See
[Cargo profile overrides][overrides] for more info.

[spec]: https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md
[overrides]: https://doc.rust-lang.org/cargo/reference/profiles.html#overrides

# Basic Usage

## Load file

The easiest way is to use [AsepriteFile::read_file] to load a file.

```
use asefile::AsepriteFile;
# use std::path::Path;
# let path = Path::new("./tests/data/basic-16x16.aseprite");
let ase = AsepriteFile::read_file(&path).unwrap();

println!("Size: {}x{}", ase.width(), ase.height());
println!("Frames: {}", ase.num_frames());
println!("Layers: {}", ase.num_layers());
```

## Save frame as image

Aseprite files consist of multiple layers. Usually you just want the final
image. You can do this by using [Frame::image]. This will return an
`image::RgbaImage` from the [image](https://docs.rs/image) library.

```
# use asefile::AsepriteFile;
# use std::path::Path;
# let asefile_path = Path::new("./tests/data/basic-16x16.aseprite");
# let output_dir = Path::new("./tests/data");
# let ase = AsepriteFile::read_file(&asefile_path).unwrap();
let image = ase.frame(0).image();
let output_path = output_dir.join("example.png");
image.save(&output_path).unwrap();
```

This blends together all visible layers the same way Aseprite would.

## Layers

You can access a [Layer] by name or by ID.

```
# use asefile::AsepriteFile;
# use std::path::Path;
# let path = Path::new("./tests/data/basic-16x16.aseprite");
# let ase = AsepriteFile::read_file(&path).unwrap();
let layer = ase.layer(0);
println!("Name of layer 0: {}", layer.name());
let layer = ase.layer_by_name("Layer 1").unwrap();
println!("Layer 1 is visible? {}", layer.is_visible());
```

## Cels

A cel is the intersection of a frame and a layer. Because of this there are
multiple ways to access a cel:

```
# use asefile::AsepriteFile;
# use std::path::Path;
# let path = Path::new("./tests/data/basic-16x16.aseprite");
# let ase = AsepriteFile::read_file(&path).unwrap();

let layer0 = ase.layer(0);
let cel1 = layer0.frame(0);
let cel2 = ase.frame(0).layer(0);

let image = cel1.image();
```

## Tilesets

Since Aseprite 1.3 you can also create tilesets and tilemaps layers.

You access each tile separately, or export them all as one image which is one
tile wide.

```
# use asefile::AsepriteFile;
# use std::path::Path;
# use image::RgbaImage;
# let path = Path::new("./tests/data/tileset.aseprite");
# let ase = AsepriteFile::read_file(&path).unwrap();

let num_tilesets = ase.tilesets().len();
let tileset = ase.tilesets().get(0).unwrap();

let all_tiles: RgbaImage = tileset.image();
let single_tile: RgbaImage = tileset.tile_image(1);
// Note: tile 0 is usually the empty tile
assert_eq!(
    all_tiles.dimensions().0,
    tileset.tile_size().width() as u32
);
assert_eq!(
    all_tiles.dimensions().1,
    tileset.tile_size().height() as u32 * tileset.tile_count()
)
```

## Tilemaps

Aseprite also supports tilemaps which are layers that are composed entirely out
of tiles from a tileset.

You can export those layers as a single large image or you can do some custom
processing by looking at the tile indexes in the layer.

```
// todo!()
```

*/

pub(crate) mod blend;
pub(crate) mod cel;
pub(crate) mod color_profile;
pub(crate) mod error;
pub(crate) mod external_file;
pub(crate) mod file;
pub(crate) mod layer;
pub(crate) mod palette;
pub(crate) mod parse;
mod pixel;
mod reader;
pub(crate) mod slice;
pub(crate) mod tags;
#[cfg(test)]
mod tests;
mod tile;
mod tilemap;
mod tileset;
pub(crate) mod user_data;

/// A specialized `Result` type for Aseprite parsing functions.
pub type Result<T> = std::result::Result<T, AsepriteParseError>;

pub use cel::Cel;
// pub use color_profile::ColorProfile;
pub use error::AsepriteParseError;
pub use external_file::{ExternalFile, ExternalFileId, ExternalFilesById};
pub use file::{AsepriteFile, Frame, LayersIter, PixelFormat};
pub use layer::{BlendMode, Layer, LayerFlags};
pub use palette::{ColorPalette, ColorPaletteEntry};
pub use slice::{Slice, Slice9, SliceKey, SliceOrigin, SlicePivot, SliceSize};
pub use tags::{AnimationDirection, Tag};
pub use tileset::{ExternalTilesetReference, TileSize, Tileset, TilesetImageError, TilesetsById};
pub use user_data::UserData;
