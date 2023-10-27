//
// Collects all images into a single texture, commonly known as a texture atlas.
//
// There are a number of rectangle packer crates with different feature sets and
// with more or less complex APIs. For this example we picked rect_packer which
// is very easy to use.
//
// The rect packer tells us where to place each image and then we must create
// the final texture ourselves.
//
use asefile::AsepriteFile;
use image::{ImageFormat, RgbaImage};
use rect_packer::{Config, Packer, Rect};
use std::path::Path;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct SpriteInfo {
    name: String,
    source: Rect,
}

pub struct ImageInfo {
    location: Rect,
    image: RgbaImage,
}

fn main() {
    let config = Config {
        width: 64,
        height: 64,
        border_padding: 0,
        rectangle_padding: 1,
    };

    let mut packer = Packer::new(config);

    let basedir = Path::new("examples").join("atlas");

    let mut sprites: Vec<SpriteInfo> = Vec::new();
    let mut images: Vec<ImageInfo> = Vec::new();

    // Place all the sprites
    for basename in &["big", "small"] {
        let file = format!("{}.aseprite", basename);
        let ase = AsepriteFile::read_file(&basedir.join(&file)).unwrap();
        let (width, height) = ase.size();
        for frame in 0..ase.num_frames() {
            if let Some(rect) = packer.pack(width as i32, height as i32, false) {
                let name = format!("{}_{}", basename, frame);
                sprites.push(SpriteInfo { name, source: rect });
                images.push(ImageInfo {
                    location: rect,
                    image: ase.frame(frame).image(),
                });
            } else {
                panic!("Could not place {} frame {}", file, frame);
                // You could keep a list of packers and try to insert into
                // another one instead.
            }
        }
    }

    // Compute output size
    let (out_width, out_height) = images.iter().fold((1, 1), |(w, h), img| {
        (w.max(img.location.right()), h.max(img.location.bottom()))
    });

    // Build the final image
    let mut output = RgbaImage::new(out_width as u32, out_height as u32);
    for img in &images {
        image::imageops::replace(
            &mut output,
            &img.image,
            img.location.x as i64,
            img.location.y as i64,
        );
    }
    let output_file = basedir.join("atlas.png");
    output
        .save_with_format(&output_file, ImageFormat::Png)
        .unwrap();

    // Write sprite info. In a real use case should be serialized as json or similar.
    println!("{:#?}", sprites);
}
