use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use crate::{
    blend,
    layer::{Layer, LayersData},
};
use crate::{layer::CelRef, *};
use cel::{CelData, RawCel};
use image::{Pixel, Rgba, RgbaImage};

/// A parsed Aseprite file.
pub struct AsepriteFile {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) num_frames: u16,
    pub(crate) pixel_format: PixelFormat,
    pub(crate) palette: Option<ColorPalette>,
    pub(crate) layers: LayersData,
    pub(crate) color_profile: Option<ColorProfile>,
    pub(crate) frame_times: Vec<u16>,
    pub(crate) tags: Vec<Tag>,
    pub(crate) framedata: Vec<Vec<cel::RawCel>>,
}

/// A reference to a single frame.
pub struct Frame<'a> {
    file: &'a AsepriteFile,
    index: u32,
}

/// Pixel format of the source Aseprite file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// Red, green, blue, and alpha with 8 bits each.
    Rgba,
    /// 8 bit grayscale and 8 bit alpha,
    Grayscale,
    /// Indexed color. Color is determined by palette.
    /// The `transparent_color_index` is used to indicate a
    /// transparent pixel in any non-background layer.
    Indexed { transparent_color_index: u8 },
}

impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgba => 4,
            PixelFormat::Grayscale => 2,
            PixelFormat::Indexed { .. } => 1,
        }
    }
}

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

    /// Width in pixels.
    pub fn width(&self) -> usize {
        self.width as usize
    }

    /// Height in pixels.
    pub fn height(&self) -> usize {
        self.height as usize
    }

    /// Width and height in pixels.
    pub fn size(&self) -> (usize, usize) {
        (self.width(), self.height())
    }

    /// Number of animation frames.
    pub fn num_frames(&self) -> usize {
        self.num_frames as usize
    }

    /// Number of layers.
    pub fn num_layers(&self) -> u32 {
        self.layers.layers.len() as u32
    }

    /// The color palette in the image.
    pub fn palette(&self) -> Option<&ColorPalette> {
        self.palette.as_ref()
    }

    /// Access a layer by ID.
    ///
    /// # Panics
    ///
    /// Panics if the ID is not valid. ID must be less than number of layers.
    pub fn layer(&self, id: u32) -> Layer {
        assert!(id < self.num_layers());
        Layer {
            file: &self,
            layer_id: id,
        }
    }

    /// Access a layer by name.
    ///
    /// If multiple layers with the same name exist returns the layer with
    /// the lower ID.
    pub fn named_layer(&self, name: &str) -> Option<Layer> {
        for layer_id in 0..self.num_layers() {
            let l = self.layer(layer_id);
            if l.name() == name {
                return Some(l);
            }
        }
        None
    }

    /// An iterator over all layers.
    pub fn layers(&self) -> LayersIter {
        LayersIter {
            file: self,
            next: 0,
        }
    }

    /// A reference to a single frame.
    pub fn frame(&self, index: u32) -> Frame {
        assert!(index < self.num_frames as u32);
        Frame { file: self, index }
    }

    /// The pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn num_tags(&self) -> u32 {
        self.tags.len() as u32
    }

    pub fn tag(&self, tag_id: u32) -> &Tag {
        &self.tags[tag_id as usize]
    }

    pub fn color_profile(&self) -> Option<&ColorProfile> {
        self.color_profile.as_ref()
    }

    /// Construct the image belonging to the specific animation frame. Combines
    /// layers according to their blend mode. Skips invisible layers (i.e.,
    /// layers with a deactivated eye icon).
    ///
    /// Can fail if the `frame` does not exist, an unsupported feature is
    /// used, or the file is malformed.
    fn frame_image(&self, frame: u16) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(self.width as u32, self.height as u32);

        for cel in &self.framedata[frame as usize] {
            // TODO: This must be done in layer order (pre-sort Cels?)
            if !self.layer(cel.layer_index as u32).is_visible() {
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

    fn copy_cel(&self, image: &mut RgbaImage, cel: &RawCel) -> Result<()> {
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

    pub(crate) fn layer_image(&self, frame: u16, layer_id: usize) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(self.width as u32, self.height as u32);
        for cel in &self.framedata[frame as usize] {
            if cel.layer_index as usize == layer_id {
                self.copy_cel(&mut image, cel)?;
            }
        }
        Ok(image)
    }

    fn frame_cels(&self, frame: u16, layer: u16) -> Vec<&RawCel> {
        self.framedata[frame as usize]
            .iter()
            .filter(|c| c.layer_index == layer)
            .collect()
    }
}

pub struct LayersIter<'a> {
    file: &'a AsepriteFile,
    next: u32,
}

impl<'a> Iterator for LayersIter<'a> {
    type Item = Layer<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.file.num_layers() {
            let item = self.file.layer(self.next);
            self.next += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl<'a> Frame<'a> {
    /// Construct the image belonging to the specific animation frame. Combines
    /// layers according to their blend mode. Skips invisible layers (i.e.,
    /// layers with a deactivated eye icon).
    ///
    /// Can fail if an unsupported feature is used, or the file is malformed.
    ///
    /// TODO: Check for unspported features at parse time and make this function
    /// infallible.
    pub fn image(&self) -> Result<RgbaImage> {
        self.file.frame_image(self.index as u16)
    }

    /// Get cel corresponding to the given layer in this frame.
    pub fn layer(&self, layer_id: u32) -> CelRef {
        assert!(layer_id < self.file.num_layers());
        CelRef {
            file: self.file,
            layer: layer_id,
            frame: self.index,
        }
    }

    /// Frame duration in milliseconds.
    pub fn duration(&self) -> u32 {
        self.file.frame_times[self.index as usize] as u32
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
