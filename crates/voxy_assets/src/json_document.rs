//! Unified bounded JSON document serialization and atomic persistence contract.

use std::io::{Read, Write};
use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

use crate::{AtomicSaveError, save_atomic_file};

/// Error returned by bounded JSON document serialization or persistence.
#[derive(Debug)]
pub enum JsonDocumentError {
    CapacityExceeded,
    Parse(String),
    InvalidPath,
    Io(std::io::Error),
}

impl std::fmt::Display for JsonDocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "json document error: {self:?}")
    }
}

impl std::error::Error for JsonDocumentError {}

struct LimitedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("json document capacity exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Serializes `value` as pretty-printed JSON while enforcing `max_document_bytes` during writing.
///
/// # Errors
///
/// Returns [`JsonDocumentError::CapacityExceeded`] if the serialized output exceeds `max_document_bytes`.
pub fn serialize_json_bounded<T: Serialize>(
    value: &T,
    max_document_bytes: usize,
) -> Result<String, JsonDocumentError> {
    let mut writer = LimitedWriter {
        bytes: Vec::with_capacity(max_document_bytes.min(4096)),
        limit: max_document_bytes,
    };
    serde_json::to_writer_pretty(&mut writer, value)
        .map_err(|_| JsonDocumentError::CapacityExceeded)?;
    String::from_utf8(writer.bytes).map_err(|e| JsonDocumentError::Parse(e.to_string()))
}

/// Trait providing bounded JSON serialization, parsing, and atomic disk persistence.
pub trait JsonDocument: Serialize + DeserializeOwned {
    /// Serializes `self` to pretty JSON bounded by `max_document_bytes`.
    ///
    /// # Errors
    ///
    /// Rejects documents exceeding `max_document_bytes`.
    fn to_json_bounded(&self, max_document_bytes: usize) -> Result<String, JsonDocumentError> {
        serialize_json_bounded(self, max_document_bytes)
    }

    /// Parses `Self` from a JSON string bounded by `max_document_bytes`.
    ///
    /// # Errors
    ///
    /// Rejects oversized inputs or invalid JSON syntax/schema.
    fn from_json_bounded(
        json: &str,
        max_document_bytes: usize,
    ) -> Result<Self, JsonDocumentError> {
        if json.len() > max_document_bytes {
            return Err(JsonDocumentError::CapacityExceeded);
        }
        serde_json::from_str(json).map_err(|e| JsonDocumentError::Parse(e.to_string()))
    }

    /// Reads at most `max_document_bytes + 1` bytes from `path` and parses `Self`.
    ///
    /// # Errors
    ///
    /// Rejects I/O errors, oversized files, invalid UTF-8, or invalid JSON.
    fn load_json_bounded(
        path: &Path,
        max_document_bytes: usize,
    ) -> Result<Self, JsonDocumentError> {
        let limit = u64::try_from(max_document_bytes)
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(JsonDocumentError::CapacityExceeded)?;
        let file = std::fs::File::open(path).map_err(JsonDocumentError::Io)?;
        let mut bytes = Vec::new();
        file.take(limit)
            .read_to_end(&mut bytes)
            .map_err(JsonDocumentError::Io)?;
        if bytes.len() > max_document_bytes {
            return Err(JsonDocumentError::CapacityExceeded);
        }
        let text =
            std::str::from_utf8(&bytes).map_err(|e| JsonDocumentError::Parse(e.to_string()))?;
        Self::from_json_bounded(text, max_document_bytes)
    }

    /// Atomically writes `self` as bounded pretty JSON to `path`.
    ///
    /// # Errors
    ///
    /// Rejects oversized output, invalid target paths, or I/O failures.
    fn save_json_atomic(
        &self,
        path: &Path,
        max_document_bytes: usize,
    ) -> Result<(), JsonDocumentError> {
        let json = self.to_json_bounded(max_document_bytes)?;
        save_atomic_file(path, json.as_bytes(), max_document_bytes).map_err(|err| match err {
            AtomicSaveError::InvalidPath => JsonDocumentError::InvalidPath,
            AtomicSaveError::Capacity => JsonDocumentError::CapacityExceeded,
            AtomicSaveError::Io(io) => JsonDocumentError::Io(io),
        })
    }
}

impl<T: Serialize + DeserializeOwned> JsonDocument for T {}
