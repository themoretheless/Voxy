//! Bounded same-directory atomic replacement for trusted authoring paths.
use std::{
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_SAVE: AtomicU64 = AtomicU64::new(1);
#[derive(Debug)]
pub enum AtomicSaveError {
    InvalidPath,
    Capacity,
    Io(std::io::Error),
}
impl std::fmt::Display for AtomicSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "atomic save: {self:?}")
    }
}
impl std::error::Error for AtomicSaveError {}
/// Writes and syncs a unique sibling file before renaming over the destination.
/// Errors before rename retain the destination and attempt temporary-file cleanup.
/// Assumes trusted paths; directory durability and platform overwrite semantics
/// are not guaranteed. This API performs blocking filesystem IO.
/// # Errors
/// Rejects invalid paths, encoded byte limits, serial exhaustion and filesystem IO.
pub fn save_atomic_file(
    path: &Path,
    bytes: &[u8],
    max_bytes: usize,
) -> Result<(), AtomicSaveError> {
    if bytes.len() > max_bytes {
        return Err(AtomicSaveError::Capacity);
    }
    if path.file_name().is_none() {
        return Err(AtomicSaveError::InvalidPath);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let serial = NEXT_SAVE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| AtomicSaveError::Capacity)?;
    let temporary = parent.join(format!(
        ".voxy-authoring-{}-{serial}.tmp",
        std::process::id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(AtomicSaveError::Io)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(AtomicSaveError::Io)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_replace_and_failed_rename_preserve_destination_and_clean_temporary() {
        let root = std::env::temp_dir().join(format!(
            "voxy-atomic-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("settings.json");
        save_atomic_file(&path, b"old", 3).unwrap();
        assert!(save_atomic_file(&path, b"too large", 3).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"old");
        save_atomic_file(&path, b"new", 3).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        let directory = root.join("directory");
        std::fs::create_dir(&directory).unwrap();
        assert!(save_atomic_file(&directory, b"data", 4).is_err());
        assert!(directory.is_dir());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
}
