use crate::{parse::read_string, AsepriteParseError, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub from_frame: u16,
    pub to_frame: u16,
    pub animation_direction: AnimationDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    Forward,
    Reverse,
    PingPong,
}

pub(crate) fn parse_tags_chunk(data: &[u8]) -> Result<Vec<Tag>> {
    let mut input = Cursor::new(data);

    let num_tags = input.read_u16::<LittleEndian>()?;
    let _reserved = input.read_u64::<LittleEndian>()?;

    let mut result = Vec::with_capacity(num_tags as usize);

    for _tag in 0..num_tags {
        let from_frame = input.read_u16::<LittleEndian>()?;
        let to_frame = input.read_u16::<LittleEndian>()?;
        let anim_dir = input.read_u8()?;
        let _reserved = input.read_u64::<LittleEndian>()?;
        let _color = input.read_u32::<LittleEndian>()?;
        let name = read_string(&mut input)?;
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
