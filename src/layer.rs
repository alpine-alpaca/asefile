use crate::{
    cel::{Cel, CelId},
    reader::AseReader,
    tileset::TilesetsById,
    user_data::UserData,
    AsepriteFile, AsepriteParseError, Result,
};
use bitflags::bitflags;
use std::{io::Read, ops::Index};

/// Types of layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    /// A regular image layer. This is the normal layer type.
    Image,
    /// A layer that groups other layers and does not contain any image data.
    /// In Aseprite these are represented by a folder icon.
    Group,
    /// A tilemap layer. Contains the index of the tileset used for the tiles.
    ///
    /// In Aseprite these are represented by a grid icon.
    Tilemap(u32),
}

bitflags! {
    /// Various layer attributes.
    ///
    /// For checking whether a layer is visible prefer to use [Layer::is_visible]
    /// as that also takes into account any parent layer's visibility.
    #[derive(Debug, Copy, PartialEq, Eq, Clone, PartialOrd, Ord, Hash)]
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

        /// The is a background layer.
        const BACKGROUND_LAYER = Self::MOVEMENT_LOCKED.bits() | Self::BACKGROUND.bits();
    }
}

/// A reference to a single layer.
#[derive(Debug)]
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

    /// Describes whether this is a regular, group, or tilemap layer.
    pub fn layer_type(&self) -> LayerType {
        self.data().layer_type
    }

    /// Is this a tilemap layer?
    pub fn is_tilemap(&self) -> bool {
        matches!(self.layer_type(), LayerType::Tilemap(_))
    }

    /// The parent of this layer, if any. For layers that are part of a group
    /// this returns the parent layer.
    ///
    /// Does not indicate the blend order of layers (i.e., which layers are
    /// above or below).
    pub fn parent(&self) -> Option<Layer> {
        self.file.layers.parents[self.layer_id as usize].map(|id| Layer {
            file: self.file,
            layer_id: id,
        })
    }

    /// Returns if this layer is visible. This requires that this layer and all
    /// of its parent layers are visible.
    pub fn is_visible(&self) -> bool {
        let layer_is_visible = self.data().flags.contains(LayerFlags::VISIBLE);
        let parent_is_visible = self.parent().map(|p| p.is_visible()).unwrap_or(true);
        layer_is_visible && parent_is_visible
    }

    /// Get a reference to the Cel for this frame in the layer.
    pub fn frame(&self, frame_id: u32) -> Cel {
        assert!(frame_id < self.file.num_frames());
        let cel_id = CelId {
            frame: frame_id as u16,
            layer: self.layer_id as u16,
        };
        Cel {
            file: self.file,
            cel_id,
        }
    }

    /// Returns a reference to the layer's [UserData], if any exists.
    pub fn user_data(&self) -> Option<&UserData> {
        self.data().user_data.as_ref()
    }
}

#[derive(Debug)]
pub struct LayerData {
    pub(crate) flags: LayerFlags,
    pub(crate) name: String,
    pub(crate) blend_mode: BlendMode,
    pub(crate) opacity: u8,
    pub(crate) layer_type: LayerType,
    pub(crate) user_data: Option<UserData>,
    child_level: u16,
}

impl LayerData {
    pub(crate) fn is_background(&self) -> bool {
        self.flags.contains(LayerFlags::BACKGROUND)
    }
}

#[derive(Debug)]
pub(crate) struct LayersData {
    // Sorted back to front (or bottom to top in the GUI, but groups occur
    // before their children, i.e., lower index)
    pub(crate) layers: Vec<LayerData>,
    parents: Vec<Option<u32>>,
}

impl LayersData {
    pub(crate) fn validate(&self, tilesets: &TilesetsById) -> Result<()> {
        for l in &self.layers {
            if let LayerType::Tilemap(id) = l.layer_type {
                // Validate that all Tilemap layers reference an existing Tileset.
                tilesets.get(id).ok_or_else(|| {
                    AsepriteParseError::InvalidInput(format!(
                        "Tilemap layer references a missing tileset (id {}",
                        id
                    ))
                })?;
            }
        }
        Ok(())
    }

    pub(crate) fn from_vec(layers: Vec<LayerData>) -> Result<Self> {
        // TODO: Validate some properties
        let parents = compute_parents(&layers);
        Ok(LayersData { layers, parents })
    }
}

impl Index<u32> for LayersData {
    type Output = LayerData;

    fn index(&self, index: u32) -> &Self::Output {
        &self.layers[index as usize]
    }
}

/// Describes how the pixels from two layers are combined.
/// See also [Blend modes (Wikipedia)](https://en.wikipedia.org/wiki/Blend_modes)
///
/// Blend modes use Aseprite's "new layer blending method", i.e., we assume that
/// the source Aseprite has a checkmark under "Edit > Preferences > Experimental >
/// New Layer Blending Method (#1096)". This is the default as of Aseprite 1.2.25.
#[allow(missing_docs)]
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

pub(crate) fn parse_chunk(data: &[u8]) -> Result<LayerData> {
    let mut reader = AseReader::new(data);

    let flags = reader.word()?;
    let layer_type = reader.word()?;
    let child_level = reader.word()?;
    let _default_width = reader.word()?;
    let _default_height = reader.word()?;
    let blend_mode = reader.word()?;
    let opacity = reader.byte()?;
    let _reserved1 = reader.byte()?;
    let _reserved2 = reader.word()?;
    let name = reader.string()?;
    let layer_type = parse_layer_type(layer_type, &mut reader)?;

    let flags = LayerFlags::from_bits_truncate(flags as u32);

    let blend_mode = parse_blend_mode(blend_mode)?;

    // println!(
    //     "Layer {}: flags={:?} type={:?} blend_mode={:?}, opacity={}",
    //     name, flags, layer_type, blend_mode, opacity
    // );

    Ok(LayerData {
        flags,
        name,
        blend_mode,
        opacity,
        layer_type,
        child_level,
        user_data: None,
    })
}

fn parse_layer_type<R: Read>(id: u16, reader: &mut AseReader<R>) -> Result<LayerType> {
    match id {
        0 => Ok(LayerType::Image),
        1 => Ok(LayerType::Group),
        2 => reader.dword().map(LayerType::Tilemap),
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

fn compute_parents(layers: &[LayerData]) -> Vec<Option<u32>> {
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
