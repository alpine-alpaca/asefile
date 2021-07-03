use crate::{AsepriteParseError, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::io::{Cursor, Read};

fn to_ase(e: std::io::Error) -> AsepriteParseError {
    e.into()
}

pub(crate) struct AseReader<T: Read> {
    input: T,
}

impl AseReader<Cursor<&[u8]>> {
    pub(crate) fn new(data: &[u8]) -> AseReader<Cursor<&[u8]>> {
        let input = Cursor::new(data);
        AseReader { input }
    }
}

impl<T: Read> AseReader<T>
where
    T: Read,
{
    pub(crate) fn with(input: T) -> Self {
        Self { input }
    }

    pub(crate) fn byte(&mut self) -> Result<u8> {
        self.input.read_u8().map_err(to_ase)
    }

    pub(crate) fn word(&mut self) -> Result<u16> {
        self.input.read_u16::<LittleEndian>().map_err(to_ase)
    }

    pub(crate) fn short(&mut self) -> Result<i16> {
        self.input.read_i16::<LittleEndian>().map_err(to_ase)
    }

    pub(crate) fn dword(&mut self) -> Result<u32> {
        self.input.read_u32::<LittleEndian>().map_err(to_ase)
    }

    pub(crate) fn long(&mut self) -> Result<i32> {
        self.input.read_i32::<LittleEndian>().map_err(to_ase)
    }

    pub(crate) fn string(&mut self) -> Result<String> {
        let str_len = self.input.read_u16::<LittleEndian>()?;
        let mut str_bytes = vec![0_u8; str_len as usize];
        self.input.read_exact(&mut str_bytes)?;
        let s = String::from_utf8(str_bytes)?;
        Ok(s)
    }

    pub(crate) fn read_exact(&mut self, buffer: &mut [u8]) -> Result<()> {
        self.input.read_exact(buffer).map_err(to_ase)
    }

    pub(crate) fn skip_reserved(&mut self, count: usize) -> Result<()> {
        let mut ignored = vec![0_u8; count];
        self.input.read_exact(&mut ignored).map_err(to_ase)
    }

    pub(crate) fn take_bytes(self, limit: usize) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(limit);
        self.input.take(limit as u64).read_to_end(&mut output)?;
        if output.len() != limit {
            Err(AsepriteParseError::InvalidInput(format!(
                "Invalid data size. Expected: {}, Actual: {}",
                limit,
                output.len()
            )))
        } else {
            Ok(output)
        }
    }

    pub(crate) fn unzip(self, expected_output_size: usize) -> Result<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(self.input);
        let mut buffer = Vec::with_capacity(expected_output_size);
        decoder.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}
