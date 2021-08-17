# Changelog

## 0.3.1 - 2021-08-17

This is mainly an administrative release.

- Build on docs.rs with all features enabled.
- Fix all clippy warnings.

## 0.3.0 - 2021-08-15

### Added

This release adds support for **Aseprite 1.3** features. Aseprite 1.3 is still in
beta, so things may still change. New supported features for Aseprite 1.3:

- **Tilemaps & Tilesets** ([#2](https://github.com/alpine-alpaca/asefile/pull/2)).
  This is a complex new feature, for details consult the [official Aseprite docs
  on tilemaps](https://www.aseprite.org/docs/tilemap/).

- External files. These are references to external files, used to reference
  external palettes or tilesets. This feature is currently not well-documented
  so we don't have any useful test cases. Please file an issue if you find a use
  case that we should support.

In addition, the following features were added:

- **User data**. Many of Aseprite's entities have additional user-defined meta data
  that you can set in the properties. We now have an API to access these.

- **Slices**. We now have basic support for accessing slices defined in your sprite.
  They can be used to define a sprite's pivot point, for example. The API is
  quite bare-bones at the moment, but you should be able to access all the
  data that you need.

- **Grayscale images**. You can now process sprites that use the grayscale color
  format.

- An optional new **`util` module**. (Must be enabled via the `utils` feature).
  
  It provides a function for extruding the border of images. This can be
  useful when building texture atlases.

  There's also a function for turning an `RgbaImage` into a vector of
  indexes into a color palette. This is a stop-gap until we have a more
  fleshed out low-level API.


### Changed

- No longer require the `png` feature from the `image` dependency.

### Contributors

Many thanks to [@B-Reif](https://github.com/B-Reif) who implemented virtually
all new features of this release.

## 0.2.0 - 2021-03-27

First public release


