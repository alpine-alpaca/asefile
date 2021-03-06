#![warn(clippy::all)]

//! Utilities for loading [Aseprite](https://www.aseprite.org/) files.
//!
//! # Basic Usage
//!
//! ## Load file
//!
//! The easiest way is to use [AsepriteFile::read_file] to load a file.
//!
//! ```
//! use asefile::AsepriteFile;
//! # use std::path::Path;
//! # let path = Path::new("./tests/data/basic-16x16.aseprite");
//! let ase = AsepriteFile::read_file(&path).unwrap();
//! println!("Size: {}x{}", ase.width(), ase.height());
//! println!("Layers: {}", ase.layers().len());
//! ```
//!
//! ## Save frame as image
//!
//! Aseprite files consist of multiple layers. Usually you just want the final
//! image. You can do this by using [Frame::image]. This will return
//! an `image::RgbaImage` from the [image](https://docs.rs/image) library.
//!
//! ```
//! use asefile::AsepriteFile;
//! # use std::path::Path;
//! # let asefile_path = Path::new("./tests/data/basic-16x16.aseprite");
//! # let output_dir = Path::new("./tests/data");
//! let ase = AsepriteFile::read_file(&asefile_path).unwrap();
//! let image = ase.frame(0).image().unwrap();
//! let output_path = output_dir.join("example.png");
//! image.save(&output_path).unwrap();
//! ```
//!
//! This blends together all visible layers the same way Aseprite would.
//!
//! ## Extract layer
//!
//!

pub mod blend;
pub mod cel;
pub mod color_profile;
pub mod error;
pub mod file;
pub mod layer;
pub mod palette;
pub(crate) mod parse;
pub mod rgba16;
pub mod slice;
pub mod tags;
#[cfg(test)]
mod tests;
pub mod user_data;

pub type Result<T> = std::result::Result<T, AsepriteParseError>;

pub use color_profile::ColorProfile;
pub use error::AsepriteParseError;
pub use file::{AsepriteFile, Frame, PixelFormat};
pub use layer::Layers;
pub use palette::ColorPalette;
pub use tags::{AnimationDirection, Tag};
