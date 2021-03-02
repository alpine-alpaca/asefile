use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use crate::blend;
use crate::*;
use cel::{Cel, CelData};
use image::{Pixel, Rgba, RgbaImage};

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
    pub tags: Vec<Tag>,
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
    /// Load Aseprite file. Loads full file into memory.
    pub fn read_file(path: &Path) -> Result<Self> {
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        parse::read_aseprite(reader)
    }

    /// Load Aseprite file from any input that implements `std::io::Read`.
    ///
    /// You can use this to read from an in memory file.
    pub fn read<R: Read>(input: R) -> Result<AsepriteFile> {
        parse::read_aseprite(input)
    }

    /// Construct the image belonging to the specific animation frame. Combines
    /// layers according to their blend mode. Skips invisible layers (i.e.,
    /// layers with a deactivated eye icon).
    ///
    /// Can fail if the `frame` does not exist, an unsupported feature is
    /// used, or the file is malformed.
    pub fn frame_image(&self, frame: u16) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(self.width as u32, self.height as u32);

        for cel in &self.framedata[frame as usize] {
            // TODO: This must be done in layer order (pre-sort Cels?)
            if !self.layers.is_visible(cel.layer_index as usize) {
                // println!("===> skipping invisible Cel: {:?}", cel);
                continue;
            }
            // println!("====> Cel: {:?}", cel);
            //assert!(cel.opacity == 255, "NYI: different Cel opacities");
            self.copy_cel(&mut image, cel)?;
        }

        //into_rgba8_image(image)
        Ok(image)
    }

    fn copy_cel(&self, image: &mut RgbaImage, cel: &Cel) -> Result<()> {
        assert!(self.pixel_format == PixelFormat::Rgba);
        match &cel.data {
            CelData::Linked(frame) => {
                //assert!(false, "NYI: Linked Cels"),
                for cel in self.frame_cels(*frame, cel.layer_index) {
                    match &cel.data {
                        CelData::Linked(_) => {
                            return Err(AsepriteParseError::InvalidInput(
                                "Linked cel points to another linked cel".into(),
                            ));
                        }
                        CelData::Raw {
                            width,
                            height,
                            data,
                        } => {
                            copy_cel_to_image(
                                image,
                                cel.x as i32,
                                cel.y as i32,
                                *width as i32,
                                *height as i32,
                                cel.opacity,
                                &data.0,
                            );
                        }
                    }
                }
            }
            CelData::Raw {
                width,
                height,
                data,
            } => {
                copy_cel_to_image(
                    image,
                    cel.x as i32,
                    cel.y as i32,
                    *width as i32,
                    *height as i32,
                    cel.opacity,
                    &data.0,
                );
            }
        }
        Ok(())
    }

    pub fn layer_image(&self, frame: u16, layer_id: usize) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(self.width as u32, self.height as u32);
        for cel in &self.framedata[frame as usize] {
            if cel.layer_index as usize == layer_id {
                self.copy_cel(&mut image, cel)?;
            }
        }
        Ok(image)
    }

    fn frame_cels(&self, frame: u16, layer: u16) -> Vec<&Cel> {
        self.framedata[frame as usize]
            .iter()
            .filter(|c| c.layer_index == layer)
            .collect()
    }
}

fn copy_cel_to_image(
    image: &mut RgbaImage,
    x0: i32,
    y0: i32,
    width: i32,
    height: i32,
    opacity: u8,
    rgba_data: &[u8],
) {
    let x_end = x0 + width;
    let y_end = y0 + height;
    // let x0 = x0.max(0);
    // let y0 = y0.max(0);
    //assert!(x0 >= 0 && y0 >= 0);
    let (img_width, img_height) = image.dimensions();
    // assert!(x_end <= img_width as i32);
    // assert!(y_end <= img_height as i32);
    // println!(
    //     "======> Writing cel: x:{}..{}, y:{}..{}",
    //     x0, x_end, y0, y_end
    // );

    for y in y0..y_end {
        if y < 0 || y >= img_height as i32 {
            continue;
        }
        for x in x0..x_end {
            if x < 0 || x >= img_width as i32 {
                continue;
            }
            let src = 4 * ((y - y0) as usize * width as usize + (x - x0) as usize);

            let pixel = Rgba::from_channels(
                rgba_data[src],
                rgba_data[src + 1],
                rgba_data[src + 2],
                rgba_data[src + 3],
            );

            let src = *image.get_pixel(x as u32, y as u32);
            let new = blend::normal(src, pixel, opacity);
            image.put_pixel(x as u32, y as u32, new);

            // let new = image.get_pixel(x as u32, y as u32);
            // if x == 5 && y == 8 {
            //     println!(
            //         "**** src={:?},\n   pixel={:?}, opacity={},\n     new={:?}",
            //         src, pixel, opacity, new
            //     );
            // }
        }
    }
}
