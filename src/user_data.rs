use crate::{read_string, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug)]
pub struct UserData {
    pub text: Option<String>,
    pub color: Option<[u8; 4]>,
}

pub(crate) fn parse_userdata_chunk(data: &[u8]) -> Result<UserData> {
    let mut input = Cursor::new(data);

    let flags = input.read_u32::<LittleEndian>()?;
    let text = if flags & 1 != 0 {
        let s = read_string(&mut input)?;
        Some(s)
    } else {
        None
    };
    let color = if flags & 2 != 0 {
        let red = input.read_u8()?;
        let green = input.read_u8()?;
        let blue = input.read_u8()?;
        let alpha = input.read_u8()?;
        Some([red, green, blue, alpha])
    } else {
        None
    };

    Ok(UserData { text, color })
}
