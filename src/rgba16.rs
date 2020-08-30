use image::{ImageBuffer, Pixel, Rgba, RgbaImage};

pub type Rgba16Image = ImageBuffer<Rgba<u16>, Vec<u16>>;
pub type Rgba16 = Rgba<u16>;

pub fn into_rgba8_image(img: Rgba16Image) -> RgbaImage {
    let (width, height) = img.dimensions();
    let data16 = img.into_raw();
    let data8 = data16.iter().map(|c16| downscale(*c16)).collect();
    RgbaImage::from_raw(width, height, data8).unwrap()
}

pub fn rgba16_pixel(r: u8, g: u8, b: u8, a: u8) -> Rgba16 {
    Rgba::from_channels(scale(r), scale(g), scale(b), scale(a))
}

pub fn as_rgba8_pixel(p: Rgba16) -> Rgba<u8> {
    Rgba::from_channels(
        downscale(p.0[0]),
        downscale(p.0[1]),
        downscale(p.0[2]),
        downscale(p.0[3]),
    )
}

pub fn rgba16_as_fpixel(p: Rgba16) -> Rgba<f32> {
    Rgba::from_channels(
        u16_to_f32(p.0[0]),
        u16_to_f32(p.0[1]),
        u16_to_f32(p.0[2]),
        u16_to_f32(p.0[3]),
    )
}

fn u16_to_f32(x: u16) -> f32 {
    (x as f32) / (65535 as f32)
}

#[inline]
fn scale(x: u8) -> u16 {
    // (((x as u32) * 65535) / 255) as u16
    (x as u16) + ((x as u16) << 8)
}

#[inline]
fn downscale(x: u16) -> u8 {
    (((x as u32 + 128) * 255) / 65535) as u8
    //(x >> 8) as u8
}

#[test]
fn scaling() {
    for n in 0..=255 {
        println!("{:x} => {:x} => {:?}", n, scale(n), downscale(scale(n)));
        assert_eq!(n, downscale(scale(n)));
    }
}

#[test]
fn downscaling() {
    // Test that u16 -> u8 creates equal-sized buckets, except for 0 and 255 which
    // should be split evenly
    // Alternatively: We could just split into 256 equal-sized buckets.
    let mut buckets = vec![0_u16; 256];
    for n in 0..=65535 {
        let d = downscale(n);
        buckets[d as usize] += 1;
    }
    assert_eq!(buckets[0], buckets[255]);
    for i in 1..=254 {
        assert_eq!(buckets[i], 257);
    }
}
