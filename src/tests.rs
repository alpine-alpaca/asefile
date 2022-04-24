use image::Pixel;

use crate::*;
use std::path::PathBuf;

fn load_test_file(name: &str) -> AsepriteFile {
    let mut path = PathBuf::new();
    path.push("tests");
    path.push("data");
    path.push(format!("{}.aseprite", name));
    println!("Loading file: {}", path.display());
    AsepriteFile::read_file(&path).unwrap()
}

// Takes the `img` and saves it under `tests/data/<filename>.actual.png`. Then
// compares it against the reference image `tests/data/<filename>.png`.
fn compare_with_reference_image(img: image::RgbaImage, filename: &str) {
    let mut reference_path = PathBuf::new();
    reference_path.push("tests");
    reference_path.push("data");
    let mut actual_path = reference_path.clone();
    reference_path.push(format!("{}.png", filename));
    actual_path.push(format!("{}.actual.png", filename));

    // If no reference image exists we still write the actual image and give the
    // user the option to make that the reference image.
    if !reference_path.is_file() {
        img.save(&actual_path).unwrap();
        panic!(
            "No reference image found: {}\n\nTo accept the current result run `cp {:?} {:?}` (or similar)",
            reference_path.display(),
            actual_path.display(),
            reference_path.display(),
        );
    }

    let ref_image = image::open(&reference_path).unwrap();
    let ref_rgba = ref_image.to_rgba8();
    // println!("Loaded reference image: {}", reference_path.display());

    // dbg!(img.dimensions(), ref_rgba.dimensions());
    assert_eq!(img.dimensions(), ref_rgba.dimensions());
    // println!("saving image");
    img.save(&actual_path).unwrap();
    // println!("done saving");

    for (x, y, expected_color) in ref_rgba.enumerate_pixels() {
        let actual_color = img.get_pixel(x, y);
        if actual_color == expected_color
            || (is_transparent(expected_color) && is_transparent(actual_color))
        {
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

fn test_user_data(s: &str, c: [u8; 4]) -> UserData {
    UserData {
        text: Some(s.to_string()),
        color: Some(image::Rgba::from_channels(c[0], c[1], c[2], c[3])),
    }
}

const COLOR_GREEN: [u8; 4] = [0, 255, 0, 255];
const COLOR_RED: [u8; 4] = [255, 0, 0, 255];

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
    let ase = AsepriteFile::read_file(path).unwrap();

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

    compare_with_reference_image(f.frame(2).layer(1).image(), "single_layer");
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
    let ts = f.tilesets().get(0).expect("No tileset found");
    assert_eq!(ts.name(), "test_tileset");

    compare_with_reference_image(img, "tilemap");
}

#[test]
fn tilemap_indexed() {
    let f = load_test_file("tilemap_indexed");
    let img = f.frame(0).image();
    assert_eq!(f.size(), (32, 32));
    let ts = f.tilesets().get(0).expect("No tileset found");
    assert_eq!(ts.name(), "test_tileset");

    compare_with_reference_image(img, "tilemap_indexed");
}

#[test]
fn tilemap_grayscale() {
    let f = load_test_file("tilemap_grayscale");
    let img = f.frame(0).image();
    assert_eq!(f.size(), (32, 32));
    let ts = f.tilesets().get(0).expect("No tileset found");
    assert_eq!(ts.name(), "test_tileset");

    compare_with_reference_image(img, "tilemap_grayscale");
}

#[test]
fn tilemap_empty_edges() {
    let f = load_test_file("tilemap_empty_edges");
    let tilemap = f.tilemap(0, 0).unwrap();
    assert_eq!(tilemap.tile(0, 0).id(), 1);
    let tile_0_0_img = tilemap.tileset().tile_image(tilemap.tile(0, 0).id());
    compare_with_reference_image(tile_0_0_img, "tilemap_empty_edges_0_0");

    //assert_eq!(tilemap.tile(0, 1).id(), 4);
    let tile_0_1_img = tilemap.tileset().tile_image(tilemap.tile(0, 1).id());
    compare_with_reference_image(tile_0_1_img, "tilemap_empty_edges_0_1");
}

#[test]
fn tileset_export() {
    let f = load_test_file("tileset");
    let tileset = f.tilesets().get(0).expect("No tileset found");
    let img = tileset.image();

    compare_with_reference_image(img, "tileset");
}

#[test]
fn tileset_export_single() {
    let f = load_test_file("tileset");
    let tileset = f.tilesets().get(0).expect("No tileset found");

    let img = tileset.tile_image(1);

    compare_with_reference_image(img, "tileset_1");
}

#[test]
fn tileset_multi() {
    let f = load_test_file("tilemap_multi");
    //let tileset = f.tilesets().get(0).expect("No tileset found");
    let img = f.frame(0).image();
    compare_with_reference_image(img, "tilemap_multi");

    let tilemap = f.layer_by_name("Tilemap 1").unwrap();
    let img = tilemap.frame(0).image();
    compare_with_reference_image(img, "tilemap_multi_map1");

    let tilemap = f.layer_by_name("Tilemap 2").unwrap();
    let img = tilemap.frame(0).image();
    compare_with_reference_image(img, "tilemap_multi_map2");
}

#[test]
fn tileset_single_tile() {
    let f = load_test_file("tilemap_multi");
    let map_layer = f.layer_by_name("Tilemap 1").unwrap().id();
    let tilemap = f.tilemap(map_layer, 0).unwrap();

    dbg!(tilemap.tile_offsets());
    assert_eq!(tilemap.width(), 13);
    assert_eq!(tilemap.height(), 16);

    assert_eq!(tilemap.tile(0, 0).id(), 0);
    assert_eq!(tilemap.tile(0, 2).id(), 4);
    assert_eq!(tilemap.tile(0, 3).id(), 2);
    assert_eq!(tilemap.tile(11, 5).id(), 3);
    assert_eq!(tilemap.tile(12, 15).id(), 0);
    assert_eq!(tilemap.tile(4, 7).id(), 3);

    let img = tilemap.tileset().tile_image(3);
    compare_with_reference_image(img, "tilemap_single_tile_1");
}

#[test]
fn slices() {
    let f = load_test_file("slice_advanced");
    let slices = f.slices();
    assert_eq!(slices.len(), 2);
    let slice_1 = &f.slices()[0];
    assert_eq!(slice_1.name, "Slice 1");
    assert_eq!(
        slice_1
            .keys
            .iter()
            .map(|k| k.from_frame)
            .collect::<Vec<_>>(),
        &[0, 1, 2, 3]
    );
    assert_eq!(slice_1.keys[0].pivot.unwrap().0, 4);
    let slice_2 = &f.slices()[1];
    assert_eq!(
        slice_2
            .keys
            .iter()
            .map(|k| k.from_frame)
            .collect::<Vec<_>>(),
        &[0]
    );
    let slice9 = slice_2.keys[0].slice9.as_ref().unwrap();
    assert_eq!(slice9.center_x, 3);
    assert_eq!(slice9.center_y, 3);
    assert_eq!(slice9.center_width, 2);
    assert_eq!(slice9.center_height, 2);
}

#[test]
fn user_data_sprite() {
    let f = load_test_file("user_data");
    let user_data = f.sprite_user_data().unwrap();
    let expected = test_user_data("test_user_data_sprite", COLOR_GREEN);
    assert_eq!(*user_data, expected);
}

#[test]
fn user_data_layer() {
    let f = load_test_file("user_data");
    let layer = f.layer(0);
    let user_data = layer.user_data().unwrap();
    let expected = test_user_data("test_user_data_layer", COLOR_RED);
    assert_eq!(*user_data, expected);
}

#[test]
fn user_data_cel() {
    let f = load_test_file("user_data");
    let raw_cel = f.framedata.cel(cel::CelId { frame: 0, layer: 0 }).unwrap();
    let user_data = raw_cel.user_data.as_ref().unwrap();
    let expected = test_user_data("test_user_data_cel", COLOR_GREEN);
    assert_eq!(*user_data, expected);
}

#[test]
fn user_data_tags() {
    let f = load_test_file("user_data");
    let tags = f.tags;
    let first = tags.get(0).and_then(|t| t.user_data()).unwrap();
    let second = tags.get(1).and_then(|t| t.user_data()).unwrap();
    let third = tags.get(2).and_then(|t| t.user_data()).unwrap();

    let expected_first = test_user_data("test_user_data_tag_0", COLOR_GREEN);
    assert_eq!(*first, expected_first);

    let expected_second = UserData {
        text: None,
        color: Some(image::Rgba::from_channels(0, 0, 0, 255)),
    };
    assert_eq!(*second, expected_second);

    let expected_third = test_user_data("test_user_data_tag_2", COLOR_RED);
    assert_eq!(*third, expected_third);
}

#[test]
fn cel_overflow() {
    let file = load_test_file("cel_overflow");
    let frame = file.frame(0);
    let img = frame.image();
    assert_eq!(file.width as u32, img.width());
    assert_eq!(file.height as u32, img.height());
}

#[test]
fn old_palette_chunk_04() {
    let f = load_test_file("256_color_old_palette_chunk");

    assert_eq!(f.num_frames, 1);
    assert_eq!((f.width, f.height), (64, 64));
    assert_eq!(f.num_layers(), 1);
    assert_eq!(
        f.pixel_format,
        PixelFormat::Indexed {
            transparent_color_index: 0
        }
    );
    assert!(f.palette().is_some());
    assert_eq!(f.palette().unwrap().num_colors(), 256);

    compare_with_reference_image(f.frame(0).image(), "256_color_old_palette_chunk");
}

#[cfg(feature = "utils")]
#[test]
fn extrude_border() {
    use crate::util::extrude_border;
    let f = load_test_file("util_extrude");
    let img = f.frame(0).image();
    let img = extrude_border(img);
    compare_with_reference_image(img, "util_extrude");
}

#[cfg(feature = "utils")]
#[test]
fn compute_indexed() {
    use crate::util;
    let f = load_test_file("util_indexed");
    let img = f.frame(0).image();
    let palette = f.palette().unwrap();
    let mapper = util::PaletteMapper::new(
        palette,
        util::MappingOptions {
            transparent: f.transparent_color_index(),
            failure: 0,
        },
    );
    let ((w, h), data) = util::to_indexed_image(img, &mapper);
    assert_eq!((w, h), (4, 4));
    assert_eq!(data.len(), 4 * 4);
    assert_eq!(data[0], 8);
    assert_eq!(data[1], 0);
    assert_eq!(data[5], 11);
    assert_eq!(data[7], 13);
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
