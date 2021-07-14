use crate::{reader::AseReader, Result};

/// UserData contains user-provided metadata which describes some other data in the sprite.
#[derive(Debug, Clone)]
pub struct UserData {
    /// User-provided string data.
    pub text: Option<String>,
    /// User-provided color in bytes [red, green, blue, alpha].
    pub color: Option<[u8; 4]>,
}

pub(crate) fn parse_userdata_chunk(data: &[u8]) -> Result<UserData> {
    let mut reader = AseReader::new(data);

    let flags = reader.dword()?;
    let text = if flags & 1 != 0 {
        let s = reader.string()?;
        Some(s)
    } else {
        None
    };
    let color = if flags & 2 != 0 {
        let red = reader.byte()?;
        let green = reader.byte()?;
        let blue = reader.byte()?;
        let alpha = reader.byte()?;
        Some([red, green, blue, alpha])
    } else {
        None
    };

    Ok(UserData { text, color })
}
