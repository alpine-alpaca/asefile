use crate::*;
use cel::{Cel, CelBytes, CelData};
use image::RgbaImage;

pub struct AsepriteFile {
    pub width: u16,
    pub height: u16,
    pub num_frames: u16,
    pub pixel_format: PixelFormat,
    pub transparent_color_index: u8, // only for PixelFormat::Indexed
    pub palette: Option<ColorPalette>,
    pub layers: Layers,
    pub color_profile: Option<ColorProfile>,
    pub frame_times: Vec<u16>,
    pub(crate) framedata: Vec<Vec<cel::Cel>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba,
    Grayscale,
    Indexed,
}

impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgba => 4,
            PixelFormat::Grayscale => 2,
            PixelFormat::Indexed => 1,
        }
    }
}

// pub struct ImageDataRgba {
//     width: u32,
//     height: u32,
//     bytes: Vec<u8>,
// }

// pub struct Rgba {
//     pub r: u8,
//     pub g: u8,
// }

// impl ImageDataRgba {
//     pub fn new(width: u32, height: u32) -> Self {
//         assert!(width <= 65536 && height <= 65536);
//         let num_bytes = width as u64 * height as u64 * 4;
//         assert!(num_bytes < usize::MAX as u64);
//         ImageDataRgba {
//             width, height,
//             bytes: vec![0; num_bytes as usize],
//         }
//     }

//     // pub fn pixel(&self, x: u32, y: u32) ->
// }

impl AsepriteFile {
    pub fn frame_image(&self, frame: u16) -> RgbaImage {
        let mut image = RgbaImage::new(self.width as u32, self.height as u32);

        for cel in &self.framedata[frame as usize] {
            // TODO: This must be done in layer order (pre-sort Cels?)
            if !self.layers.is_visible(cel.layer_index as usize) {
                println!("===> skipping invisible Cel: {:?}", cel);
                continue;
            }
            println!("====> Cel: {:?}", cel);
            assert!(cel.opacity == 255, "NYI: different Cel opacities");
            match &cel.data {
                CelData::Linked(frame) => assert!(false, "NYI: Linked Cels"),
                CelData::Raw {
                    width,
                    height,
                    data,
                } => {
                    let x0 = cel.x as i32;
                    let y0 = cel.y as i32;
                    let x_end = x0 + *width as i32;
                    let y_end = y0 + *height as i32;
                    assert!(x0 >= 0 && y0 >= 0);
                    assert!(x_end <= self.width as i32);
                    assert!(y_end <= self.height as i32);
                    println!(
                        "======> Writing cel: x:{}..{}, y:{}..{}",
                        x0, x_end, y0, y_end
                    );
                    for y in y0..y_end {
                        for x in x0..x_end {
                            let src = 4 * ((y - y0) as usize * *width as usize + (x - x0) as usize);
                            let alpha = data.0[src + 3];
                            if alpha == 0 {
                                continue;
                            };
                            assert!(alpha == 255);
                            image.put_pixel(
                                x as u32,
                                y as u32,
                                image::Rgba([data.0[src], data.0[src + 1], data.0[src + 2], alpha]),
                            )
                        }
                    }
                }
            }
        }

        image
    }

    // fn frame_cels(&self, frame: usize, layer: usize) -> Vec<Cel> {

    // }
}
