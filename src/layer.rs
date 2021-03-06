use crate::{parse::read_string, AsepriteFile, AsepriteParseError, Result};
use bitflags::bitflags;
use byteorder::{LittleEndian, ReadBytesExt};
use image::RgbaImage;
use std::{io::Cursor, ops::Index};

/// Types of layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    /// A regular image layer. This is the normal layer type.
    Image,
    /// A layer that groups other layers and does not contain any image data.
    /// In Aseprite these are represented by a folder icon.
    Group,
}

bitflags! {
    pub struct LayerFlags: u32 {
        /// Layer is visible (eye icon is enabled).
        const VISIBLE = 0x0001;
        /// Layer can be modified (lock icon is disabled).
        const EDITABLE = 0x0002;
        /// Layer cannot be moved.
        const MOVEMENT_LOCKED = 0x0004;
        /// Layer is background (stack order cannot be changed).
        const BACKGROUND = 0x0008;
        /// Prefer to link cels when the user copies them.
        const CONTINUOUS = 0x0010;
        /// Prefer to show this group layer collapsed.
        const COLLAPSED = 0x0020;
        /// This is a reference layer.
        const REFERENCE = 0x0040;

        const BACKGROUND_LAYER = Self::MOVEMENT_LOCKED.bits | Self::BACKGROUND.bits;
    }
}

/// A reference to a single layer.
pub struct Layer<'a> {
    pub(crate) file: &'a AsepriteFile,
    pub(crate) layer_id: u32,
}

impl<'a> Layer<'a> {
    fn data(&self) -> &LayerData {
        &self.file.layers[self.layer_id]
    }

    /// This layer's ID.
    pub fn id(&self) -> u32 {
        self.layer_id
    }

    /// Layer's flags
    pub fn flags(&self) -> LayerFlags {
        self.data().flags
    }

    /// Name of the layer
    pub fn name(&self) -> &str {
        &self.data().name
    }

    /// Blend mode of the layer. Describes how this layer is combined with the
    /// layers underneath it. See [BlendMode] for details.
    pub fn blend_mode(&self) -> BlendMode {
        self.data().blend_mode
    }

    /// Layer opacity describes
    pub fn opacity(&self) -> u8 {
        self.data().opacity
    }

    /// Describes whether this is a regular layer or a group layer.
    pub fn layer_type(&self) -> LayerType {
        self.data().layer_type
    }

    /// The parent of this layer, if any. For layers that are part of a group
    /// this returns the parent layer.
    ///
    /// Does not indicate the blend order of layers (i.e., which layers are
    /// above or below).
    pub fn parent(&self) -> Option<Layer> {
        match self.file.layers.parents[self.layer_id as usize] {
            None => None,
            Some(id) => Some(Layer {
                file: self.file,
                layer_id: id,
            }),
        }
    }

    /// Returns if this layer is visible. This requires that this layer and all
    /// of its parent layers are visible.
    pub fn is_visible(&self) -> bool {
        let layer_is_visible = self.data().flags.contains(LayerFlags::VISIBLE);
        let parent_is_visible = self.parent().map(|p| p.is_visible()).unwrap_or(true);
        layer_is_visible && parent_is_visible
    }

    /// Get a reference to the Cel for this frame in the layer.
    pub fn frame(&self, frame_id: u32) -> CelRef {
        assert!((frame_id as usize) < self.file.num_frames());
        CelRef {
            file: self.file,
            layer: self.layer_id as u32,
            frame: frame_id,
        }
    }
}

/// A reference to a single Cel. This contains the image data at a specific
/// layer and frame. In the timeline view these dots.
pub struct CelRef<'a> {
    pub(crate) file: &'a AsepriteFile,
    pub(crate) layer: u32,
    pub(crate) frame: u32,
}

impl<'a> CelRef<'a> {
    pub fn image(&self) -> Result<RgbaImage> {
        self.file
            .layer_image(self.frame as u16, self.layer as usize)
    }
}

#[derive(Debug)]
pub struct LayerData {
    pub(crate) flags: LayerFlags,
    pub(crate) name: String,
    pub(crate) blend_mode: BlendMode,
    pub(crate) opacity: u8,
    pub(crate) layer_type: LayerType,
    child_level: u16,
}

#[derive(Debug)]
pub struct LayersData {
    // Sorted back to front (or bottom to top in the GUI, but groups occur
    // before their children, i.e., lower index)
    pub(crate) layers: Vec<LayerData>,
    parents: Vec<Option<u32>>,
}

impl Index<u32> for LayersData {
    type Output = LayerData;

    fn index(&self, index: u32) -> &Self::Output {
        &self.layers[index as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Addition,
    Subtract,
    Divide,
}

impl LayerFlags {
    /// Shortcut for `.contains(LayerFlags::VISIBLE)`.
    pub fn is_visible(&self) -> bool {
        self.contains(LayerFlags::VISIBLE)
    }
}

pub(crate) fn parse_layer_chunk(data: &[u8]) -> Result<LayerData> {
    let mut input = Cursor::new(data);

    let flags = input.read_u16::<LittleEndian>()?;
    let layer_type = input.read_u16::<LittleEndian>()?;
    let child_level = input.read_u16::<LittleEndian>()?;
    let _default_width = input.read_u16::<LittleEndian>()?;
    let _default_height = input.read_u16::<LittleEndian>()?;
    let blend_mode = input.read_u16::<LittleEndian>()?;
    let opacity = input.read_u8()?;
    let _reserved1 = input.read_u8()?;
    let _reserved2 = input.read_u16::<LittleEndian>()?;
    let name = read_string(&mut input)?;

    let flags = LayerFlags::from_bits_truncate(flags as u32);

    let layer_type = parse_layer_type(layer_type)?;
    let blend_mode = parse_blend_mode(blend_mode)?;

    // println!(
    //     "Layer {}: flags={:?} type={:?} blend_mode={:?}, opacity={}",
    //     name, flags, layer_type, blend_mode, opacity
    // );

    Ok(LayerData {
        name,
        flags,
        blend_mode,
        opacity,
        layer_type,
        child_level,
    })
}

fn parse_layer_type(id: u16) -> Result<LayerType> {
    match id {
        0 => Ok(LayerType::Image),
        1 => Ok(LayerType::Group),
        _ => Err(AsepriteParseError::InvalidInput(format!(
            "Invalid layer type: {}",
            id
        ))),
    }
}

fn parse_blend_mode(id: u16) -> Result<BlendMode> {
    match id {
        0 => Ok(BlendMode::Normal),
        1 => Ok(BlendMode::Multiply),
        2 => Ok(BlendMode::Screen),
        3 => Ok(BlendMode::Overlay),
        4 => Ok(BlendMode::Darken),
        5 => Ok(BlendMode::Lighten),
        6 => Ok(BlendMode::ColorDodge),
        7 => Ok(BlendMode::ColorBurn),
        8 => Ok(BlendMode::HardLight),
        9 => Ok(BlendMode::SoftLight),
        10 => Ok(BlendMode::Difference),
        11 => Ok(BlendMode::Exclusion),
        12 => Ok(BlendMode::Hue),
        13 => Ok(BlendMode::Saturation),
        14 => Ok(BlendMode::Color),
        15 => Ok(BlendMode::Luminosity),
        16 => Ok(BlendMode::Addition),
        17 => Ok(BlendMode::Subtract),
        18 => Ok(BlendMode::Divide),
        _ => Err(AsepriteParseError::InvalidInput(format!(
            "Invalid/Unsupported blend mode: {}",
            id
        ))),
    }
}

fn compute_parents(layers: &Vec<LayerData>) -> Vec<Option<u32>> {
    let mut result = Vec::with_capacity(layers.len());

    for id in 0..layers.len() {
        let parent = {
            let my_child_level = layers[id].child_level;
            if my_child_level == 0 {
                None
            } else {
                // Find first layer with a lower id and a lower child_level.
                let mut parent_candidate = id - 1;
                while layers[parent_candidate].child_level >= my_child_level {
                    assert!(parent_candidate > 0);
                    parent_candidate -= 1;
                }
                Some(parent_candidate as u32)
            }
        };
        result.push(parent);
    }
    result
}

pub(crate) fn collect_layers(layers: Vec<LayerData>) -> Result<LayersData> {
    // TODO: Validate some properties
    let parents = compute_parents(&layers);
    Ok(LayersData { layers, parents })
}
