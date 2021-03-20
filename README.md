# asefile

[![crates.io](https://img.shields.io/crates/v/asefile.svg)](https://crates.io/crates/asefile)
[![Documentation](https://docs.rs/asefile/badge.svg)](https://docs.rs/asefile)
<!-- [![Build Status](https://github.com/alpine-alpaca/asefile/workflows/Rust%20CI/badge.svg)](https://github.com/alpine-alpaca/asefile/actions) -->

Utilities for loading [Aseprite](https://www.aseprite.org/) files. This
library directly reads the binary Aseprite files ([file format
specification][spec]) and does not require you to export files to JSON. This
should make it fast enough to load your assets when the game boots up. You can
also use it to build your own asset pipelines.

[Documentation](https://docs.rs/asefile/)

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
        // Create 
        let img = ase.frame(frame).image();
        img.save_with_format(output, ImageFormat::Png).unwrap();
    }
}
```

# Unsupported Features

The following features of Aseprite 1.2.25 are not supported:

- grayscale images
- color profiles
