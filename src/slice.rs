use crate::{parse::read_string, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug)]
pub struct Slice {
    pub name: String,
    pub keys: Vec<SliceKey>,
}

#[derive(Debug)]
pub struct SliceKey {
    pub from_frame: u32,
    pub origin: (i32, i32),
    pub size: (u32, u32),
    pub slice9: Option<(i32, i32, u32, u32)>,
    pub pivot: Option<(i32, i32)>,
}

pub(crate) fn parse_slice_chunk(data: &[u8]) -> Result<Slice> {
    let mut input = Cursor::new(data);

    let num_slice_keys = input.read_u32::<LittleEndian>()?;
    let flags = input.read_u32::<LittleEndian>()?;
    let _reserved = input.read_u32::<LittleEndian>()?;
    let name = read_string(&mut input)?;

    let mut slice_keys: Vec<SliceKey> = Vec::with_capacity(num_slice_keys as usize);
    for _id in 0..num_slice_keys {
        let from_frame = input.read_u32::<LittleEndian>()?;
        let origin_x = input.read_i32::<LittleEndian>()?;
        let origin_y = input.read_i32::<LittleEndian>()?;
        let width = input.read_u32::<LittleEndian>()?;
        let height = input.read_u32::<LittleEndian>()?;
        let slice9 = if flags & 1 != 0 {
            let center_x = input.read_i32::<LittleEndian>()?;
            let center_y = input.read_i32::<LittleEndian>()?;
            let center_width = input.read_u32::<LittleEndian>()?;
            let center_height = input.read_u32::<LittleEndian>()?;
            Some((center_x, center_y, center_width, center_height))
        } else {
            None
        };
        let pivot = if flags & 2 != 0 {
            let pivot_x = input.read_i32::<LittleEndian>()?;
            let pivot_y = input.read_i32::<LittleEndian>()?;
            Some((pivot_x, pivot_y))
        } else {
            None
        };

        slice_keys.push(SliceKey {
            from_frame,
            origin: (origin_x, origin_y),
            size: (width, height),
            slice9,
            pivot,
        });
    }

    Ok(Slice {
        name,
        keys: slice_keys,
    })
}
