use crate::{reader::AseReader, Result};

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

pub(crate) fn parse_chunk(data: &[u8]) -> Result<Slice> {
    let mut reader = AseReader::new(data);

    let num_slice_keys = reader.dword()?;
    let flags = reader.dword()?;
    let _reserved = reader.dword()?;
    let name = reader.string()?;

    let mut slice_keys: Vec<SliceKey> = Vec::with_capacity(num_slice_keys as usize);
    for _id in 0..num_slice_keys {
        let from_frame = reader.dword()?;
        let origin_x = reader.long()?;
        let origin_y = reader.long()?;
        let width = reader.dword()?;
        let height = reader.dword()?;
        let slice9 = if flags & 1 != 0 {
            let center_x = reader.long()?;
            let center_y = reader.long()?;
            let center_width = reader.dword()?;
            let center_height = reader.dword()?;
            Some((center_x, center_y, center_width, center_height))
        } else {
            None
        };
        let pivot = if flags & 2 != 0 {
            let pivot_x = reader.long()?;
            let pivot_y = reader.long()?;
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
