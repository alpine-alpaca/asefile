use crate::{reader::AseReader, AsepriteParseError, Result};

/// A tag is a grouping of one or more frames.
///
/// Tag ranges may overlap each other. Tag names are _not_ guaranteed to be
/// unique.
#[derive(Debug, Clone)]
pub struct Tag {
    name: String,
    from_frame: u16,
    to_frame: u16,
    animation_direction: AnimationDirection,
}

impl Tag {
    /// Tag name. May not be unique among all tags.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// First frame included in the tag.
    pub fn from_frame(&self) -> u32 {
        self.from_frame as u32
    }

    /// Last frame included in the tag.
    pub fn to_frame(&self) -> u32 {
        self.to_frame as u32
    }

    /// See [AnimationDirection] for details.
    pub fn animation_direction(&self) -> AnimationDirection {
        self.animation_direction
    }
}

/// Describes how the tag's frames should be animated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    /// Start at `from_frame` and count up to `to_frame`.
    Forward,
    /// Start at `to_frame` and count down to `from_frame`.
    Reverse,
    /// Start at `from_frame`, count up to `to_frame`, then back down to `from_frame`.
    PingPong,
}

pub(crate) fn parse_tags_chunk(data: &[u8]) -> Result<Vec<Tag>> {
    let mut reader = AseReader::new(data);

    let num_tags = reader.word()?;
    reader.skip_reserved(8)?;

    let mut result = Vec::with_capacity(num_tags as usize);

    for _tag in 0..num_tags {
        let from_frame = reader.word()?;
        let to_frame = reader.word()?;
        let anim_dir = reader.byte()?;
        reader.skip_reserved(8)?;
        let _color = reader.dword()?;
        let name = reader.string()?;
        let animation_direction = parse_animation_direction(anim_dir)?;
        result.push(Tag {
            name,
            from_frame,
            to_frame,
            animation_direction,
        });
    }

    Ok(result)
}

fn parse_animation_direction(id: u8) -> Result<AnimationDirection> {
    match id {
        0 => Ok(AnimationDirection::Forward),
        1 => Ok(AnimationDirection::Reverse),
        2 => Ok(AnimationDirection::PingPong),
        _ => Err(AsepriteParseError::InvalidInput(format!(
            "Unknown animation direction: {}",
            id
        ))),
    }
}
