use crate::{read_string, AsepriteParseError, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt;
use std::io::Cursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    Image,
    Group,
}

#[derive(Debug)]
pub struct Layer {
    pub flags: LayerFlags,
    pub name: String,
    pub blend_mode: BlendMode,
    pub opacity: u8,
    pub layer_type: LayerType,
    child_level: u16,
}

#[derive(Debug)]
pub struct Layers {
    // Sorted back to front (or bottom to top in the GUI, but groups occur
    // before their children, i.e., lower index)
    layers: Vec<Layer>,
}

impl Layers {
    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }

    pub fn layer(&self, id: usize) -> &Layer {
        &self.layers[id]
    }

    pub fn find_layer_by_name(&self, name: &str) -> Option<usize> {
        for id in 0..self.num_layers() {
            if self.layer(id).name == name {
                return Some(id);
            }
        }
        None
    }

    pub fn parent(&self, id: usize) -> Option<usize> {
        // TODO: We could precompute all of this.
        let my_child_level = self.layer(id).child_level;
        if my_child_level == 0 {
            return None;
        }
        let mut parent_candidate = id - 1;
        while self.layer(parent_candidate).child_level >= my_child_level {
            assert!(parent_candidate > 0);
            parent_candidate -= 1;
        }
        Some(parent_candidate)
    }

    /// Check if layer is visible, taking into account parent visibility.
    pub fn is_visible(&self, id: usize) -> bool {
        // TODO: This could also be precomputed.
        let layer_is_visible = self.layer(id).flags.is_visible();
        let parent_is_visible = if let Some(parent) = self.parent(id) {
            self.is_visible(parent)
        } else {
            true
        };
        layer_is_visible && parent_is_visible
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
    pub fn is_visible(&self) -> bool {
        self.0 & 1 != 0
    }

    pub fn is_editable(&self) -> bool {
        self.0 & 2 != 0
    }

    pub fn is_movement_locked(&self) -> bool {
        self.0 & 4 != 0
    }

    pub fn is_background(&self) -> bool {
        self.0 & 8 != 0
    }

    pub fn prefer_linked_cels(&self) -> bool {
        self.0 & 16 != 0
    }

    pub fn is_collapsed(&self) -> bool {
        self.0 & 32 != 0
    }

    pub fn is_reference(&self) -> bool {
        self.0 & 64 != 0
    }
}

pub struct LayerFlags(u16);

impl fmt::Debug for LayerFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LayerFlags([")?;
        let mut sep = "";
        for (id, name) in FLAG_NAMES.iter().enumerate() {
            if self.0 & (1 << id) != 0 {
                write!(f, "{}{}", sep, name)?;
                sep = ","
            }
        }
        write!(f, "])")
    }
}

static FLAG_NAMES: [&str; 7] = [
    "Visible",
    "Editable",
    "LockMovement",
    "Background",
    "PreferLinkedCels",
    "DisplayCollapsed",
    "Reference",
];

pub(crate) fn parse_layer_chunk(data: &[u8]) -> Result<Layer> {
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

    let flags = LayerFlags(flags);

    let layer_type = parse_layer_type(layer_type)?;
    let blend_mode = parse_blend_mode(blend_mode)?;

    // println!(
    //     "Layer {}: flags={:?} type={:?} blend_mode={:?}, opacity={}",
    //     name, flags, layer_type, blend_mode, opacity
    // );

    Ok(Layer {
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

pub(crate) fn collect_layers(layers: Vec<Layer>) -> Result<Layers> {
    // TODO: Validate some properties
    Ok(Layers { layers })
}
