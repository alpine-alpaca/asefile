# Changelog

## Unreleased

### Added

This release adds support for Aseprite 1.3 features. Aseprite 1.3 is still in
beta, so things may still change. New supported features for Aseprite 1.3:

- Tilemaps/Tilesets ([#2](https://github.com/alpine-alpaca/asefile/pull/2)).
  This is a complex new feature, for details consult the [official Aseprite docs
  on tilemaps](https://www.aseprite.org/docs/tilemap/).
- External files. These are references to external files, used to referenc
  external palettes or tilesets. This feature is currently not well-documented
  so we don't have any useful test cases. Please file an issue if you find a use
  case that we should support.

### Changed

- No longer require the `png` feature from `image` dependency.

### Contributors

Many thanks to [Bruce Reif (Buswolley)](https://github.com/B-Reif) who
implemented virtually all new features of this release.

## 0.2.0 - 2021-03-27

First public release


