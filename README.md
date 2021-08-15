# asefile

[![Build status](https://github.com/alpine-alpaca/asefile/actions/workflows/rust.yml/badge.svg)](https://github.com/alpine-alpaca/asefile/actions/)
[![crates.io](https://img.shields.io/crates/v/asefile.svg)](https://crates.io/crates/asefile)
[![Documentation](https://docs.rs/asefile/badge.svg)](https://docs.rs/asefile)
<!-- [![Build Status](https://github.com/alpine-alpaca/asefile/workflows/Rust%20CI/badge.svg)](https://github.com/alpine-alpaca/asefile/actions) -->

Utilities for loading [Aseprite](https://www.aseprite.org/) files. This library
directly reads the binary Aseprite files ([specification][spec]) and does not
require you to export files to JSON. This should make it fast enough to load
your assets when the game boots up (during development). You can also use it to
build your own asset pipelines.

[Documentation](https://docs.rs/asefile/) | [Changelog](CHANGELOG.md)

[spec]: https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md

# Example

```rust
use std::path::Path;

use asefile::AsepriteFile;
use image::{self, ImageFormat};

fn main() {
    let file = Path::new("input.aseprite");
    // Read file into memory
    let ase = AsepriteFile::read_file(&file).unwrap();
    // Write one output image for each frame in the Aseprite file.
    for frame in 0..ase.num_frames() {
        let output = format!("output_{}.png", frame);
        // Create image in memory, then write it to disk as PNG.
        let img = ase.frame(frame).image();
        img.save_with_format(output, ImageFormat::Png).unwrap();
    }
}
```

# Unsupported Features

The following features of Aseprite 1.2.25 are currently not supported:

- color profiles

# Bug compatibility

- For indexed color files Aseprite supports blend modes, but ignores them when
  exporting the image. The images constructed by `asefile` currently match the
  in-editor preview.

- Aseprite has a bug in its luminance and color blend modes. Since this is the
  same in editor and in exported files, `asefile` reproduces this bug. (If
  Aseprite fixes this, `asefile` will fix this bug based on the version that
  the file was generated with.)
