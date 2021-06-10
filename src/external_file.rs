use crate::parse::read_string;
use crate::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use core::str;
use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};
use std::ops::Index;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ExternalFileId(u32);
impl ExternalFileId {
    fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub struct ExternalFile {
    id: ExternalFileId,
    name: String,
}
impl ExternalFile {
    pub fn new(id: ExternalFileId, name: String) -> Self {
        Self { id, name }
    }
    pub fn id(&self) -> &ExternalFileId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn parse_chunk(data: &[u8]) -> Result<Vec<Self>> {
        let mut input = Cursor::new(data);
        let entry_ct = input.read_u32::<LittleEndian>()?;
        let mut results = Vec::with_capacity(entry_ct as usize);
        input.seek(SeekFrom::Current(8))?;
        for _ in 0..entry_ct {
            let id = ExternalFileId::new(input.read_u32::<LittleEndian>()?);
            input.seek(SeekFrom::Current(8))?;
            let name = read_string(&mut input)?;
            results.push(Self::new(id, name))
        }
        Ok(results)
    }
}
#[derive(Debug)]
pub struct ExternalFilesById(HashMap<ExternalFileId, ExternalFile>);
impl ExternalFilesById {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn add(&mut self, external_file: ExternalFile) {
        self.0.insert(*external_file.id(), external_file);
    }
    pub fn map(&self) -> &HashMap<ExternalFileId, ExternalFile> {
        &self.0
    }
}
impl Index<ExternalFileId> for ExternalFilesById {
    type Output = ExternalFile;
    fn index(&self, id: ExternalFileId) -> &Self::Output {
        let map = self.map();
        if map.contains_key(&id) {
            return &self.map()[&id];
        }
        panic!("no external file found for id")
    }
}
