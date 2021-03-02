#![warn(clippy::all)]

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

type Result<T> = std::result::Result<T, AsepriteParseError>;

pub use color_profile::ColorProfile;
use error::AsepriteParseError;
pub use file::{AsepriteFile, PixelFormat};
pub use layer::Layers;
pub use palette::ColorPalette;
pub use tags::{AnimationDirection, Tag};
