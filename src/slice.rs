use std::io::Read;

use crate::{reader::AseReader, user_data::UserData, Result};

/// A slice is a region of the sprite with some attributes.
///
/// They are created using the slice tool and can be animated over frames. See
/// the [official docs on slices](https://www.aseprite.org/docs/slices/) for
/// details.
#[derive(Debug, Clone)]
pub struct Slice {
    /// The name of the slice. Not guaranteed to be unique.
    pub name: String,
    /// A sequence of [SliceKey]s. Together, these describe the shape and
    /// position of a slice during animation.
    pub keys: Vec<SliceKey>,
    /// User data associated with this slice.
    pub user_data: Option<UserData>,
}

/// A devision of a [Slice] into nine regions for 9-slice scaling.
#[derive(Debug, Clone)]
pub struct Slice9 {
    /// X position of the center area (relative to slice bounds).
    pub center_x: i32,
    /// Y position of the center area (relative to slice bounds).
    pub center_y: i32,
    /// Width of the center area.
    pub center_width: u32,
    /// Height of the center area.
    pub center_height: u32,
}

impl Slice9 {
    fn read<R: Read>(reader: &mut AseReader<R>) -> Result<Self> {
        let center_x = reader.long()?;
        let center_y = reader.long()?;
        let center_width = reader.dword()?;
        let center_height = reader.dword()?;
        Ok(Self {
            center_x,
            center_y,
            center_width,
            center_height,
        })
    }
}

/// The position and shape of a [Slice], starting at a given frame.
#[derive(Debug, Clone)]
pub struct SliceKey {
    /// Starting frame number for this slice key. This slice is valid from this
    /// frame to the end of the animation or the next slice key.
    pub from_frame: u32,
    /// Origin of the slice.
    pub origin: (i32, i32),
    /// Size of the slice.
    pub size: (u32, u32),
    /// 9-slicing information.
    pub slice9: Option<Slice9>,
    /// Pivot information. Relative to the origin.
    pub pivot: Option<(i32, i32)>,
}

impl SliceKey {
    fn read<R: Read>(reader: &mut AseReader<R>, flags: u32) -> Result<Self> {
        let from_frame = reader.dword()?;
        let origin_x = reader.long()?;
        let origin_y = reader.long()?;
        let origin = (origin_x, origin_y);
        let slice_width = reader.dword()?;
        let slice_height = reader.dword()?;
        let size = (slice_width, slice_height);
        let slice9 = if flags & 1 != 0 {
            Some(Slice9::read(reader)?)
        } else {
            None
        };
        let pivot = if flags & 2 != 0 {
            let x = reader.long()?;
            let y = reader.long()?;
            Some((x, y))
        } else {
            None
        };

        Ok(Self {
            from_frame,
            origin,
            size,
            slice9,
            pivot,
        })
    }
}

pub(crate) fn parse_chunk(data: &[u8]) -> Result<Slice> {
    let mut reader = AseReader::new(data);

    let num_slice_keys = reader.dword()?;
    let flags = reader.dword()?;
    let _reserved = reader.dword()?;
    let name = reader.string()?;
    let slice_keys: Result<Vec<SliceKey>> = (0..num_slice_keys)
        .map(|_id| SliceKey::read(&mut reader, flags))
        .collect();

    Ok(Slice {
        name,
        keys: slice_keys?,
        user_data: None,
    })
}
