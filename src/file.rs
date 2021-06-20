use std::{
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
};

use crate::{
    blend::{self, Color8},
    cel::{CelData, CelsData, ImageContent, ImageSize},
    external_file::{ExternalFile, ExternalFileId, ExternalFilesById},
    layer::{Layer, LayersData},
    tilemap::Tilemap,
    tileset::{Tileset, TilesetsById},
};
use crate::{cel::Cel, *};
use cel::{CelContent, RawCel};
use image::{Pixel, Rgba, RgbaImage};

/// A parsed Aseprite file.
#[derive(Debug)]
pub struct AsepriteFile {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) num_frames: u16,
    pub(crate) pixel_format: PixelFormat,
    pub(crate) palette: Option<ColorPalette>,
    pub(crate) layers: LayersData,
    // pub(crate) color_profile: Option<ColorProfile>,
    pub(crate) frame_times: Vec<u16>,
    pub(crate) tags: Vec<Tag>,
    pub(crate) framedata: CelsData, // Vec<Vec<cel::RawCel>>,
    pub(crate) external_files: ExternalFilesById,
    pub(crate) tilesets: TilesetsById,
}

/// A reference to a single frame.
#[derive(Debug)]
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
    #[allow(missing_docs)]
    Indexed { transparent_color_index: u8 },
}

impl PixelFormat {
    /// Number of bytes to store one pixel.
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

    /// Load Aseprite file from any input that implements `std::io::Read` and `std::io::Seek`.
    ///
    /// You can use this to read from an in-memory file.
    pub fn read<R: Read + Seek>(input: R) -> Result<AsepriteFile> {
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
    pub fn num_frames(&self) -> u32 {
        self.num_frames as u32
    }

    /// Number of layers.
    pub fn num_layers(&self) -> u32 {
        self.layers.layers.len() as u32
    }

    /// The pixel format used by the origal file. This library internally
    /// represents all images as RGBA.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    /// The color palette in the image.
    ///
    /// For indexed color images, this includes all colors used by individual
    /// cels. However, the final image after layer blending may contain colors
    /// outside of this palette (or with different transparency levels).
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
    pub fn layer_by_name(&self, name: &str) -> Option<Layer> {
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
    ///
    /// # Panics
    ///
    /// Panics if `index` is not less than `num_frames`.
    pub fn frame(&self, index: u32) -> Frame {
        assert!(index < self.num_frames as u32);
        Frame { file: self, index }
    }

    /// A HashMap of external files by id.
    pub fn external_files(&self) -> &ExternalFilesById {
        &self.external_files
    }

    /// Get a reference to an external file by ID.
    ///
    /// # Panics
    ///
    /// Panics if no external file is found for the given id.
    pub fn external_file_by_id(&self, id: &ExternalFileId) -> &ExternalFile {
        &self.external_files[*id]
    }

    /// Total number of tags.
    pub fn num_tags(&self) -> u32 {
        self.tags.len() as u32
    }

    /// Get a reference to the tag by ID.
    ///
    /// # Panics
    ///
    /// Panics if `tag_id` is not less than `num_tags`.
    pub fn tag(&self, tag_id: u32) -> &Tag {
        &self.tags[tag_id as usize]
    }

    /// Lookup tag by name.
    ///
    /// If multiple tags with the same name exist, returns the one with the
    /// lower ID.
    pub fn tag_by_name(&self, name: &str) -> Option<&Tag> {
        for tag in &self.tags {
            if tag.name() == name {
                return Some(tag);
            }
        }
        None
    }

    /// Access the file's Tilesets.
    pub fn tilesets(&self) -> &TilesetsById {
        &self.tilesets
    }

    // pub fn color_profile(&self) -> Option<&ColorProfile> {
    //     self.color_profile.as_ref()
    // }

    /// Construct the image belonging to the specific animation frame. Combines
    /// layers according to their blend mode. Skips invisible layers (i.e.,
    /// layers with a deactivated eye icon).
    ///
    /// Can fail if the `frame` does not exist, an unsupported feature is
    /// used, or the file is malformed.
    fn frame_image(&self, frame: u16) -> RgbaImage {
        let mut image = RgbaImage::new(self.width as u32, self.height as u32);

        for (layer_id, cel) in self.framedata.frame_cels(frame) {
            // TODO: Ensure this is always done in layer order (pre-sort Cels?)
            if !self.layer(layer_id).is_visible() {
                continue;
            }
            self.write_cel(&mut image, cel);
        }

        image
    }

    fn write_cel(&self, image: &mut RgbaImage, cel: &RawCel) {
        assert!(self.pixel_format != PixelFormat::Grayscale);
        let RawCel { data, content } = cel;
        let layer = self.layer(data.layer_index as u32);
        let blend_mode = layer.blend_mode();
        match &content {
            CelContent::Raw(image_content) => {
                let ImageContent { size, pixels } = image_content;
                match pixels {
                    pixel::Pixels::Rgba(pixels) => {
                        write_raw_cel_to_image(image, data, size, pixels, &blend_mode);
                    }
                    pixel::Pixels::Grayscale(_) => {
                        panic!("Grayscale cel. Should have been caught by validate()");
                    }
                    pixel::Pixels::Indexed(_) => {
                        panic!("Indexed data cel. Should have been caught by validate()");
                    }
                }
            }
            CelContent::Tilemap(tilemap_data) => {
                if let layer::LayerType::Tilemap(tileset_id) = layer.layer_type() {
                    let tileset = &self.tilesets()[tileset_id];
                    write_tilemap_cel_to_image(image, data, tilemap_data, tileset, &blend_mode)
                } else {
                    panic!("Tried to parse Tilemap Cel, but layer has no associated tileset");
                }
            }
            CelContent::Linked(frame) => {
                if let Some(cel) = self.framedata.cel(*frame, data.layer_index) {
                    if let CelContent::Linked(_) = cel.content {
                        panic!("Cel links to empty cel. Should have been caught by validate()");
                    } else {
                        // Recurse once with the source non-Linked cel
                        self.write_cel(image, cel);
                    }
                }
            }
        }
    }

    pub(crate) fn layer_image(&self, frame: u16, layer_id: usize) -> RgbaImage {
        let mut image = RgbaImage::new(self.width as u32, self.height as u32);
        for cel in self.framedata.cel(frame, layer_id as u16) {
            self.write_cel(&mut image, cel);
        }
        image
    }

    // fn frame_cels(&self, frame: u16, layer: u16) -> Vec<&RawCel> {
    //     self.framedata[frame as usize]
    //         .iter()
    //         .filter(|c| c.layer_index == layer)
    //         .collect()
    // }
}

/// An iterator over layers. See [AsepriteFile::layers].
#[derive(Debug)]
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
    pub fn image(&self) -> RgbaImage {
        self.file.frame_image(self.index as u16)
    }

    /// Get cel corresponding to the given layer in this frame.
    pub fn layer(&self, layer_id: u32) -> Cel {
        assert!(layer_id < self.file.num_layers());
        Cel {
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

type BlendFn = Box<dyn Fn(Color8, Color8, u8) -> Color8>;

fn blend_mode_to_blend_fn(mode: BlendMode) -> BlendFn {
    // TODO: Make these statically allocated
    match mode {
        BlendMode::Normal => Box::new(blend::normal),
        BlendMode::Multiply => Box::new(blend::multiply),
        BlendMode::Screen => Box::new(blend::screen),
        BlendMode::Overlay => Box::new(blend::overlay),
        BlendMode::Darken => Box::new(blend::darken),
        BlendMode::Lighten => Box::new(blend::lighten),
        BlendMode::ColorDodge => Box::new(blend::color_dodge),
        BlendMode::ColorBurn => Box::new(blend::color_burn),
        BlendMode::HardLight => Box::new(blend::hard_light),
        BlendMode::SoftLight => Box::new(blend::soft_light),
        BlendMode::Difference => Box::new(blend::difference),
        BlendMode::Exclusion => Box::new(blend::exclusion),
        BlendMode::Hue => Box::new(blend::hsl_hue),
        BlendMode::Saturation => Box::new(blend::hsl_saturation),
        BlendMode::Color => Box::new(blend::hsl_color),
        BlendMode::Luminosity => Box::new(blend::hsl_luminosity),
        BlendMode::Addition => Box::new(blend::addition),
        BlendMode::Subtract => Box::new(blend::subtract),
        BlendMode::Divide => Box::new(blend::divide),
    }
}

fn write_tilemap_cel_to_image(
    image: &mut RgbaImage,
    cel_data: &CelData,
    tilemap_data: &Tilemap,
    tileset: &Tileset,
    blend_mode: &BlendMode,
) {
    let CelData { x, y, opacity, .. } = cel_data;
    // tilemap dimensions
    let tilemap_width = tilemap_data.width;
    let tilemap_height = tilemap_data.height;
    let tiles = &tilemap_data.tiles;
    // tile dimensions
    let tile_size = tileset.tile_size();
    let tile_width = *tile_size.width();
    let tile_height = *tile_size.height();
    // pixel iteration
    let pixel_x_start = *x as i32;
    let pixel_x_end = pixel_x_start + (tile_width as i32);
    let pixel_y_start = *y as i32;
    let pixel_y_end = pixel_y_start + (tile_height as i32);
    // tile data
    // TODO: support external file reference
    let tiles_data = tileset
        .tiles_data()
        .as_ref()
        .expect("Tilesets with external file reference not yet implemented");
    for tile_y in 0..tilemap_height {
        for tile_x in 0..tilemap_width {
            // TODO: support tile transform flags
            let tile_idx = (tile_x + (tile_y * tilemap_width)) as usize;
            let tile = &tiles[tile_idx];
            let tile_id = &tile.id;
        }
    }
    let blend_fn = blend_mode_to_blend_fn(*blend_mode);

    todo!()
}

fn write_raw_cel_to_image(
    image: &mut RgbaImage,
    cel_data: &CelData,
    image_size: &ImageSize,
    pixels: &Vec<crate::pixel::Rgba>,
    blend_mode: &BlendMode,
) {
    let ImageSize { width, height } = image_size;
    let CelData { x, y, opacity, .. } = cel_data;
    let blend_fn = blend_mode_to_blend_fn(*blend_mode);
    let x0 = *x as i32;
    let y0 = *y as i32;
    let x_end = x0 + (*width as i32);
    let y_end = y0 + (*height as i32);
    let (img_width, img_height) = image.dimensions();

    for y in y0..y_end {
        if y < 0 || y >= img_height as i32 {
            continue;
        }
        for x in x0..x_end {
            if x < 0 || x >= img_width as i32 {
                continue;
            }
            let idx = (y - y0) as usize * *width as usize + (x - x0) as usize;
            let pixel = &pixels[idx];
            let image_pixel = Rgba::from_channels(pixel.red, pixel.green, pixel.blue, pixel.alpha);

            let src = *image.get_pixel(x as u32, y as u32);
            let new = blend_fn(src, image_pixel, *opacity);
            image.put_pixel(x as u32, y as u32, new);
        }
    }
}
