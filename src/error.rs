use std::{error::Error, fmt, io, string::FromUtf8Error};

/// An error occured while reading the Aseprite file.
#[derive(Debug)]
pub enum AsepriteParseError {
    InvalidInput(String),
    UnsupportedFeature(String),
    IoError(io::Error),
}

impl From<io::Error> for AsepriteParseError {
    fn from(err: io::Error) -> Self {
        AsepriteParseError::IoError(err)
    }
}

impl From<FromUtf8Error> for AsepriteParseError {
    fn from(err: FromUtf8Error) -> Self {
        AsepriteParseError::InvalidInput(format!("Could not decode utf8: {}", err))
    }
}

impl fmt::Display for AsepriteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsepriteParseError::InvalidInput(msg) => write!(f, "Invalid Aseprite input: {}", msg),
            AsepriteParseError::UnsupportedFeature(msg) => {
                write!(f, "Unsupported Aseprite feature: {}", msg)
            }
            AsepriteParseError::IoError(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl Error for AsepriteParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AsepriteParseError::IoError(err) => Some(err),
            _ => None,
        }
    }
}
