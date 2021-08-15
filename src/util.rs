//! Utilities not directly related to Aseprite, but useful for processing the
//! resulting image data.

use std::iter::once;
use image::RgbaImage;

/// Add a 1 pixel border around the input image by duplicating the outmost
/// pixels.
///
/// This can be useful when creating a texture atlas for sprites that represent
/// tiles. Without this, under certain zoom levels there might be small gaps
/// between tiles. For an example, see this [discussion of the problem on
/// StackOverflow][1].
///
/// [1]: https://gamedev.stackexchange.com/questions/148247/prevent-tile-layout-gaps
///
/// Many sprite atlas generation tools have this as a built-in feature. In that
/// case you don't need to use this function.
pub fn extrude_border(image: RgbaImage) -> RgbaImage {
    let (w, h) = image.dimensions();
    let w = w as usize;
    let h = h as usize;
    let src = image.as_raw();
    let bpp = 4;  // bytes per pixel
    let mut data: Vec<u8> = Vec::with_capacity(4 * (w + 2) * (h + 2));
    let bpp_w = bpp * w;
    for src_row in once(0).chain(0..h).chain(once(h-1)) {
        let ofs = src_row * bpp * w;
        data.extend_from_slice(&src[ofs..ofs + bpp]); // (0, r)
        data.extend_from_slice(&src[ofs..ofs + bpp_w]); // (0, r)..(w-1, r);
        data.extend_from_slice(&src[ofs + bpp_w - bpp..ofs + bpp_w]);
    }

    RgbaImage::from_raw((w + 2) as u32, (h + 2) as u32, data).unwrap()
}
