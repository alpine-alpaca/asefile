use crate::*;
use image;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

fn load_test_file(name: &str) -> AsepriteFile {
    let mut path = PathBuf::new();
    path.push("tests");
    path.push("data");
    path.push(format!("{}.aseprite", name));
    println!("Loading file: {}", path.display());
    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    read_aseprite(reader).unwrap()
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
                "Pixel difference in {}: {},{} expected: {:?} actual: {:?}",
                actual_path.display(),
                x, y, expected_color, actual_color
            );
            assert!(false, "Found pixel difference");
        }
    }
}

#[test]
fn basic_1() {
    let f = load_test_file("basic-16x16");
    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.layers.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);
    assert!(f.layers.layer(0).flags.is_visible());

    compare_with_reference_image(f.frame_image(0), "basic-16x16");
}

#[test]
fn basic_2() {
    let f = load_test_file("layers_and_tags");

    assert_eq!(f.num_frames, 4);
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.layers.num_layers(), 6);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame_image(0), "layers_and_tags_01");
    compare_with_reference_image(f.frame_image(1), "layers_and_tags_02");
    compare_with_reference_image(f.frame_image(2), "layers_and_tags_03");
    compare_with_reference_image(f.frame_image(3), "layers_and_tags_04");
}

#[test]
fn basic_3() {
    let f = load_test_file("big");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.layers.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame_image(0), "big");
}
