use crate::*;
use std::path::PathBuf;

fn load_test_file(name: &str) -> AsepriteFile {
    let mut path = PathBuf::new();
    path.push("tests");
    path.push("data");
    path.push(format!("{}.aseprite", name));
    println!("Loading file: {}", path.display());
    AsepriteFile::read_file(&path).unwrap()
    // let file = File::open(&path).unwrap();
    // let reader = BufReader::new(file);
    // parse::read_aseprite(reader).unwrap()
}

fn compare_with_reference_image(img: image::RgbaImage, filename: &str) {
    let mut reference_path = PathBuf::new();
    reference_path.push("tests");
    reference_path.push("data");
    let mut actual_path = reference_path.clone();
    reference_path.push(format!("{}.png", filename));
    actual_path.push(format!("{}.actual.png", filename));
    let ref_image = image::open(&reference_path).unwrap();
    let ref_rgba = ref_image.to_rgba();

    assert_eq!(img.dimensions(), ref_rgba.dimensions());
    img.save(&actual_path).unwrap();

    for (x, y, expected_color) in ref_rgba.enumerate_pixels() {
        let actual_color = img.get_pixel(x, y);
        if actual_color != expected_color {
            println!(
                "Pixel difference in {}:\nlocation: {},{}\nexpected: {:?}\n  actual: {:?}",
                actual_path.display(),
                x,
                y,
                expected_color,
                actual_color
            );
            panic!("Found pixel difference");
        }
    }
}

#[test]
fn basic() {
    let f = load_test_file("basic-16x16");
    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.layers.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);
    assert!(f.layers.layer(0).flags.is_visible());

    compare_with_reference_image(f.frame_image(0).unwrap(), "basic-16x16");
}

#[test]
fn layers_and_tags() {
    let f = load_test_file("layers_and_tags");

    assert_eq!(f.num_frames, 4);
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.layers.num_layers(), 6);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);
    assert_eq!(f.tags.len(), 3);

    compare_with_reference_image(f.frame_image(0).unwrap(), "layers_and_tags_01");
    compare_with_reference_image(f.frame_image(1).unwrap(), "layers_and_tags_02");
    compare_with_reference_image(f.frame_image(2).unwrap(), "layers_and_tags_03");
    compare_with_reference_image(f.frame_image(3).unwrap(), "layers_and_tags_04");
}

#[test]
fn big() {
    let f = load_test_file("big");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.layers.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame_image(0).unwrap(), "big");
}

#[test]
fn transparency() {
    let f = load_test_file("transparency");

    assert_eq!(f.num_frames, 2);
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.layers.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame_image(0).unwrap(), "transparency_01");
    compare_with_reference_image(f.frame_image(1).unwrap(), "transparency_02");
}

#[test]
fn background() {
    let f = load_test_file("background");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.layers.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);
    println!("{:#?}", f.layers);

    compare_with_reference_image(f.frame_image(0).unwrap(), "background");
}

#[test]
fn blend_normal() {
    let f = load_test_file("blend_normal");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.layers.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame_image(0).unwrap(), "blend_normal");
}

#[test]
fn single_layer() {
    let f = load_test_file("layers_and_tags");

    assert_eq!(f.num_frames, 4);
    assert_eq!(f.layers.num_layers(), 6);
    assert_eq!(f.layers.find_layer_by_name("Layer 1"), Some(1));

    compare_with_reference_image(f.layer_image(2, 1).unwrap(), "single_layer");
}

/*
#[test]
fn gen_random_pixels() {
    use rand::Rng;
    use image::{Rgba};
    use std::path::Path;
    let mut rng = rand::thread_rng();

    let (width, height) = (256, 256);
    let mut img = image::RgbaImage::new(width, height);
    for y in 0..width {
        for x in 0..height {
            let r: u8 = rng.gen();
            let g: u8 = rng.gen();
            let b: u8 = rng.gen();
            let a: u8 = rng.gen();
            img.put_pixel(x, y, Rgba([r, g, b, a]));
        }
    }
    img.save(&Path::new("tests/data/random-256x256.png")).unwrap();
}
// */
