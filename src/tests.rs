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
    let ref_rgba = ref_image.to_rgba8();

    assert_eq!(img.dimensions(), ref_rgba.dimensions());
    img.save(&actual_path).unwrap();

    for (x, y, expected_color) in ref_rgba.enumerate_pixels() {
        let actual_color = img.get_pixel(x, y);
        if actual_color == expected_color {
            continue;
        } else if is_transparent(expected_color) && is_transparent(actual_color) {
            continue;
        } else {
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

fn is_transparent(col: &image::Rgba<u8>) -> bool {
    col.0[3] == 0
}

#[test]
fn basic() {
    let f = load_test_file("basic-16x16");
    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);
    assert!(f.layer(0).flags().contains(LayerFlags::VISIBLE));

    compare_with_reference_image(f.frame(0).image(), "basic-16x16");
}

#[test]
fn layers_and_tags() {
    let f = load_test_file("layers_and_tags");

    assert_eq!(f.num_frames, 4);
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.num_layers(), 6);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);
    assert_eq!(f.tags.len(), 3);

    compare_with_reference_image(f.frame(0).image(), "layers_and_tags_01");
    compare_with_reference_image(f.frame(1).image(), "layers_and_tags_02");
    compare_with_reference_image(f.frame(2).image(), "layers_and_tags_03");
    compare_with_reference_image(f.frame(3).image(), "layers_and_tags_04");
}

#[test]
fn big() {
    let f = load_test_file("big");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "big");
}

#[test]
fn transparency() {
    let f = load_test_file("transparency");

    assert_eq!(f.num_frames(), 2);
    assert_eq!(f.size(), (16, 16));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format(), PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "transparency_01");
    compare_with_reference_image(f.frame(1).image(), "transparency_02");
}

#[test]
fn cels_basic() {
    use std::path::Path;
    let path = Path::new("./tests/data/basic-16x16.aseprite");
    let ase = AsepriteFile::read_file(&path).unwrap();

    let layer0 = ase.layer(0);
    let cel1 = layer0.frame(0);
    let _cel2 = ase.frame(0).layer(0);

    let _image = cel1.image();
}

#[test]
fn background() {
    let f = load_test_file("background");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 1);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);
    println!("{:#?}", f.layers);

    compare_with_reference_image(f.frame(0).image(), "background");
}

#[test]
fn blend_normal() {
    let f = load_test_file("blend_normal");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "blend_normal");
}

#[test]
fn blend_multiply() {
    let f = load_test_file("blend_multiply");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "blend_multiply");
}

#[test]
fn blend_screen() {
    let f = load_test_file("blend_screen");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "blend_screen");
}

#[test]
fn blend_darken() {
    let f = load_test_file("blend_darken");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "blend_darken");
}

#[test]
fn blend_lighten() {
    let f = load_test_file("blend_lighten");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "blend_lighten");
}

#[test]
fn blend_overlay() {
    let f = load_test_file("blend_overlay");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "blend_overlay");
}

#[test]
fn blend_color_dodge() {
    let f = load_test_file("blend_colordodge");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (256, 256));
    assert_eq!(f.num_layers(), 2);
    assert_eq!(f.pixel_format, PixelFormat::Rgba);

    compare_with_reference_image(f.frame(0).image(), "blend_colordodge");
}

#[test]
fn blend_color_burn() {
    blend_test("blend_colorburn");
}

#[test]
fn blend_hard_light() {
    blend_test("blend_hardlight");
}

#[test]
fn blend_soft_light() {
    blend_test("blend_softlight");
}

fn blend_test(name: &str) {
    let f = load_test_file(name);
    compare_with_reference_image(f.frame(0).image(), name);
}

#[test]
fn blend_divide() {
    blend_test("blend_divide");
}

#[test]
fn blend_difference() {
    blend_test("blend_difference");
}

#[test]
fn blend_exclusion() {
    blend_test("blend_exclusion");
}

#[test]
fn blend_addition() {
    blend_test("blend_addition");
}

#[test]
fn blend_subtract() {
    blend_test("blend_subtract");
}

#[test]
fn blend_hue() {
    blend_test("blend_hue");
}

#[test]
fn blend_saturation() {
    blend_test("blend_saturation");
}

#[test]
fn blend_saturation_bug() {
    blend_test("blend_saturation_bug");
}

#[test]
fn blend_color() {
    blend_test("blend_color");
}

#[test]
fn blend_luminosity() {
    blend_test("blend_luminosity");
}

#[test]
fn single_layer() {
    let f = load_test_file("layers_and_tags");

    assert_eq!(f.num_frames, 4);
    assert_eq!(f.num_layers(), 6);
    assert_eq!(f.layer_by_name("Layer 1").map(|l| l.id()), Some(1));

    compare_with_reference_image(f.layer_image(2, 1), "single_layer");
}

#[test]
fn linked_cels() {
    let f = load_test_file("linked_cels");

    assert_eq!(f.num_frames, 3);
    assert_eq!(f.num_layers(), 3);
    //assert_eq!(f.named_layer("Layer 1").map(|l| l.id()), Some(1));

    compare_with_reference_image(f.frame(0).image(), "linked_cels_01");
    compare_with_reference_image(f.frame(1).image(), "linked_cels_02");
    compare_with_reference_image(f.frame(2).image(), "linked_cels_03");
}

#[test]
fn indexed() {
    let f = load_test_file("indexed");

    assert_eq!(f.size(), (64, 64));

    compare_with_reference_image(f.frame(0).image(), "indexed_01");
}

#[test]
fn grayscale() {
    let f = load_test_file("grayscale");
    assert_eq!(f.size(), (64, 64));

    compare_with_reference_image(f.frame(0).image(), "grayscale");
}

#[test]
fn palette() {
    let f = load_test_file("palette");

    let pal = f.palette().unwrap();
    assert_eq!(pal.num_colors(), 85);
    assert_eq!(pal.color(0).unwrap().raw_rgba8(), [46, 34, 47, 255]);
    assert_eq!(pal.color(71).unwrap().raw_rgba8(), [0, 0, 0, 83]);
}

#[test]
fn tilemap() {
    let f = load_test_file("tilemap");
    let img = f.frame(0).image();
    assert_eq!(f.size(), (32, 32));
    let ts = f
        .tilesets()
        .get(&tileset::TilesetId::new(0))
        .expect("No tileset found");
    assert_eq!(ts.name(), "test_tileset");

    compare_with_reference_image(img, "tilemap");
}

#[test]
fn tilemap_indexed() {
    let f = load_test_file("tilemap_indexed");
    let img = f.frame(0).image();
    assert_eq!(f.size(), (32, 32));
    let ts = f
        .tilesets()
        .get(&tileset::TilesetId::new(0))
        .expect("No tileset found");
    assert_eq!(ts.name(), "test_tileset");

    compare_with_reference_image(img, "tilemap_indexed");
}

#[test]
fn tilemap_grayscale() {
    let f = load_test_file("tilemap_grayscale");
    let img = f.frame(0).image();
    assert_eq!(f.size(), (32, 32));
    let ts = f
        .tilesets()
        .get(&tileset::TilesetId::new(0))
        .expect("No tileset found");
    assert_eq!(ts.name(), "test_tileset");

    compare_with_reference_image(img, "tilemap_grayscale");
}

#[test]
fn tileset_export() {
    let f = load_test_file("tileset");
    let tileset = f
        .tilesets()
        .get(&tileset::TilesetId::new(0))
        .expect("No tileset found");
    let img = f.tileset_image(tileset.id()).unwrap();

    compare_with_reference_image(img, "tileset");
}

#[test]
fn user_data_sprite() {
    let f = load_test_file("user_data");
    let text = f.sprite_user_data().and_then(|d| d.text.as_ref()).unwrap();
    assert_eq!(text, "test_user_data_sprite");
}

#[test]
fn user_data_layer() {
    let f = load_test_file("user_data");
    let layer = f.layer(0);
    let text = layer.user_data().and_then(|d| d.text.as_ref()).unwrap();
    assert_eq!(text, "test_user_data_layer");
}

#[test]
fn user_data_cel() {
    let f = load_test_file("user_data");
    let raw_cel = f.framedata.cel(cel::CelId { frame: 0, layer: 0 }).unwrap();
    let text = raw_cel
        .user_data
        .as_ref()
        .and_then(|d| d.text.as_ref())
        .unwrap();
    assert_eq!(text, "test_user_data_cel");
}

#[test]
fn user_data_tags() {
    let f = load_test_file("user_data");
    let tags = f.tags;
    let first = tags.get(0).and_then(|t| t.user_data()).unwrap();
    let second = tags.get(1).and_then(|t| t.user_data()).unwrap();
    let third = tags.get(2).and_then(|t| t.user_data()).unwrap();
    assert_eq!(first.text, Some("test_user_data_tag_0".into()));
    assert_eq!(second.text, None);
    assert_eq!(third.text, Some("test_user_data_tag_2".into()));
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
