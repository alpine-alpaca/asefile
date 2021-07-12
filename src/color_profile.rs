use crate::{parse::ChunkContent, reader::AseReader, AsepriteParseError, Result};

#[derive(Debug)]
pub struct ColorProfile {
    pub profile_type: ColorProfileType,
    pub fixed_gamma: Option<f64>,
    // pub icc_profile: Option<Vec<u8>>,
}

#[derive(Debug, PartialEq)]
pub enum ColorProfileType {
    None,
    Srgb,
    ICC,
}

pub(crate) fn parse_chunk(chunk: ChunkContent) -> Result<ColorProfile> {
    let data = &chunk.data;
    let mut reader = AseReader::new(data);
    let profile_type = reader.word()?;
    let flags = reader.word()?;
    let _fixed_gamma = reader.dword()?;
    reader.skip_reserved(8)?;

    let profile_type = parse_color_profile_type(profile_type)?;
    let fixed_gamma = if flags & 1 != 0 {
        return Err(AsepriteParseError::UnsupportedFeature(
            "Custom gamma is currently not supported.".to_owned(),
        ));
    } else {
        None
    };

    if profile_type == ColorProfileType::ICC {
        return Err(AsepriteParseError::UnsupportedFeature(
            "Embedded ICC color profiles are currently not supported".to_owned(),
        ));
    }

    Ok(ColorProfile {
        profile_type,
        fixed_gamma,
    })
}

fn parse_color_profile_type(id: u16) -> Result<ColorProfileType> {
    match id {
        0x0000 => Ok(ColorProfileType::None),
        0x0001 => Ok(ColorProfileType::Srgb),
        0x0002 => Ok(ColorProfileType::ICC),
        _ => Err(AsepriteParseError::UnsupportedFeature(format!(
            "Unknown color profile type: {}",
            id
        ))),
    }
}
