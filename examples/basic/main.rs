//
// Reads a file and writes out a PNG file for every frame.
//
use std::path::Path;

use asefile::AsepriteFile;
use image::{self, ImageFormat};

fn main() {
    let basedir = Path::new("examples").join("basic");
    let file = basedir.join("input.aseprite");
    let ase = AsepriteFile::read_file(&file).unwrap();
    for frame in 0..ase.num_frames() {
        let output = format!("output_{}.png", frame);
        let outpath = basedir.join(&output);
        let img = ase.frame(frame).image();
        img.unwrap()
            .save_with_format(outpath, ImageFormat::Png)
            .unwrap();
    }
}
