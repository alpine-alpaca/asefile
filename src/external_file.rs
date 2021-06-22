use crate::reader::AseReader;
use crate::Result;
use core::str;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ExternalFileId(u32);
impl ExternalFileId {
    pub(crate) fn new(id: u32) -> Self {
        Self(id)
    }
    pub fn value(&self) -> &u32 {
        &self.0
    }
}

#[derive(Debug)]
pub struct ExternalFile {
    id: ExternalFileId,
    name: String,
}
impl ExternalFile {
    pub(crate) fn new(id: ExternalFileId, name: String) -> Self {
        Self { id, name }
    }
    pub fn id(&self) -> &ExternalFileId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn parse_chunk(data: &[u8]) -> Result<Vec<Self>> {
        let mut reader = AseReader::new(data);
        let entry_ct = reader.dword()?;
        let mut results = Vec::with_capacity(entry_ct as usize);
        // Reserved bytes
        reader.skip_bytes(8)?;
        for _ in 0..entry_ct {
            let id = ExternalFileId::new(reader.dword()?);
            // Reserved bytes
            reader.skip_bytes(8)?;
            let name = reader.string()?;
            results.push(Self::new(id, name))
        }
        Ok(results)
    }
}
#[derive(Debug)]
pub struct ExternalFilesById(HashMap<ExternalFileId, ExternalFile>);
impl ExternalFilesById {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }
    pub(crate) fn add(&mut self, external_file: ExternalFile) {
        self.0.insert(*external_file.id(), external_file);
    }
    pub fn map(&self) -> &HashMap<ExternalFileId, ExternalFile> {
        &self.0
    }
    pub fn get(&self, id: &ExternalFileId) -> Option<&ExternalFile> {
        let map = self.map();
        map.get(id)
    }
}
