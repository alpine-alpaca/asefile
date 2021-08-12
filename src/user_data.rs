use crate::{reader::AseReader, Result};
use image::Pixel;

/// User-provided metadata which can be attached to various items.
///
/// Aseprite allows attaching user data to several entities, both via the GUI
/// and via extensions. For an example see the discussion
/// [How to associate data to each cel](https://community.aseprite.org/t/how-to-associate-data-to-each-cel-frame/6307).
#[derive(Debug, Clone, PartialEq)]
pub struct UserData {
    /// User-provided string data.
    pub text: Option<String>,
    /// User-provided color.
    pub color: Option<image::Rgba<u8>>,
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
        let rgba = image::Rgba::from_channels(red, green, blue, alpha);
        Some(rgba)
    } else {
        None
    };

    Ok(UserData { text, color })
}
