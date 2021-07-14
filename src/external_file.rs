use crate::reader::AseReader;
use crate::Result;
use core::str;
use std::collections::HashMap;

/// Unique identifier of a reference to an [ExternalFile].
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ExternalFileId(u32);

impl ExternalFileId {
    /// Converts a raw u32 value to an ExternalFileId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns a reference to the id's underlying u32 value.
    pub fn value(&self) -> &u32 {
        &self.0
    }
}

/// An external file. Used to reference external palettes or tilesets.
#[derive(Debug)]
pub struct ExternalFile {
    id: ExternalFileId,
    name: String,
}

impl ExternalFile {
    pub(crate) fn new(id: ExternalFileId, name: String) -> Self {
        Self { id, name }
    }

    /// Returns a reference to the external file's id.
    pub fn id(&self) -> &ExternalFileId {
        &self.id
    }

    /// Returns a reference to the external file's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn parse_chunk(data: &[u8]) -> Result<Vec<Self>> {
        let mut reader = AseReader::new(data);
        let entry_ct = reader.dword()?;
        reader.skip_reserved(8)?;

        let mut results = Vec::with_capacity(entry_ct as usize);
        for _ in 0..entry_ct {
            let id = ExternalFileId::new(reader.dword()?);
            reader.skip_reserved(8)?;
            let name = reader.string()?;
            results.push(Self::new(id, name))
        }

        Ok(results)
    }
}

/// A map of [ExternalFileId] values to [ExternalFile] instances.
#[derive(Debug)]
pub struct ExternalFilesById(HashMap<ExternalFileId, ExternalFile>);

impl ExternalFilesById {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn add(&mut self, external_file: ExternalFile) {
        self.0.insert(*external_file.id(), external_file);
    }

    /// Returns a reference to the underlying HashMap value.
    pub fn map(&self) -> &HashMap<ExternalFileId, ExternalFile> {
        &self.0
    }

    /// Get a reference to an [ExternalFile] from an [ExternalFileId], if the entry exists.
    pub fn get(&self, id: &ExternalFileId) -> Option<&ExternalFile> {
        self.0.get(id)
    }
}
