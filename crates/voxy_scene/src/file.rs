//! Scene file IO with bounded reads and atomic replacement on the same filesystem.
use crate::{ComponentRegistry, DocumentError, SceneDocument};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_SAVE: AtomicU64 = AtomicU64::new(1);
#[derive(Debug)]
pub enum SceneFileError {
    Document(DocumentError),
    TooLarge,
    /// committed=true means rename succeeded but a later durability check failed.
    Io {
        committed: bool,
        error: std::io::Error,
    },
}
impl std::fmt::Display for SceneFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scene file error: {self:?}")
    }
}
impl std::error::Error for SceneFileError {}
impl From<DocumentError> for SceneFileError {
    fn from(e: DocumentError) -> Self {
        Self::Document(e)
    }
}
fn io(error: std::io::Error) -> SceneFileError {
    SceneFileError::Io {
        committed: false,
        error,
    }
}
struct Temporary {
    path: PathBuf,
    committed: bool,
}
impl Drop for Temporary {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}
/// Reads at most `max_bytes+1` bytes, even if the file grows during the read.
/// # Errors
/// Reports IO, quota, UTF-8 and document syntax errors.
pub fn read_scene_file(path: &Path, max_bytes: usize) -> Result<SceneDocument, SceneFileError> {
    SceneDocument::from_json(&read_text(path, max_bytes)?).map_err(Into::into)
}

pub(crate) fn read_text(path: &Path, max_bytes: usize) -> Result<String, SceneFileError> {
    let limit = u64::try_from(max_bytes)
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or(SceneFileError::TooLarge)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io)?
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > max_bytes {
        return Err(SceneFileError::TooLarge);
    }
    let text = String::from_utf8(bytes)
        .map_err(|e| io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    Ok(text)
}
/// Validates and writes a private sibling file, flushes it, then renames over the
/// destination. On Unix the parent directory is synced after publication. Other
/// targets do not yet provide a directory-durability guarantee. No parent directory
/// is created automatically. Existing destination permissions are preserved.
/// Ownership and extended attributes are not copied to the replacement file.
/// # Errors
/// Before publication, errors leave the old destination untouched. A directory
/// sync failure after rename is explicitly reported with committed=true.
pub fn save_scene_file(
    path: &Path,
    document: &SceneDocument,
    registry: &ComponentRegistry,
    scene_capacity: usize,
    max_bytes: usize,
) -> Result<(), SceneFileError> {
    let text = document.to_json()?;
    if text.len() > max_bytes {
        return Err(SceneFileError::TooLarge);
    }
    document.load(registry, scene_capacity)?;
    write_text(path, &text)
}

pub(crate) fn write_text(path: &Path, text: &str) -> Result<(), SceneFileError> {
    let permissions = match fs::metadata(path) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(io(error)),
    };
    let name = path.file_name().ok_or_else(|| {
        io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "scene destination needs a filename",
        ))
    })?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut created = None;
    for _ in 0..16 {
        let mut temporary_name = name.to_os_string();
        let sequence = NEXT_SAVE.fetch_add(1, Ordering::Relaxed);
        temporary_name.push(format!(".voxy-save-{}-{sequence}.tmp", std::process::id()));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => {
                created = Some((
                    Temporary {
                        path: temporary_path,
                        committed: false,
                    },
                    file,
                ));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(io(error)),
        }
    }
    let (mut temporary, mut file) = created.ok_or_else(|| {
        io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "temporary scene filename collisions",
        ))
    })?;
    file.write_all(text.as_bytes()).map_err(io)?;
    if let Some(permissions) = permissions {
        file.set_permissions(permissions).map_err(io)?;
    }
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(&temporary.path, path).map_err(io)?;
    temporary.committed = true;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| SceneFileError::Io {
            committed: true,
            error,
        })?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_failure_removes_temporary_file_and_preserves_destination() {
        let folder = std::env::temp_dir().join(format!(
            "voxy-scene-rename-{}-{}",
            std::process::id(),
            NEXT_SAVE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&folder).unwrap();
        let path = folder.join("scene.json");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("retained"), "original").unwrap();
        assert!(matches!(
            write_text(&path, "replacement"),
            Err(SceneFileError::Io {
                committed: false,
                ..
            })
        ));
        assert_eq!(
            fs::read_to_string(path.join("retained")).unwrap(),
            "original"
        );
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        fs::remove_dir_all(folder).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn replacement_preserves_destination_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let folder = std::env::temp_dir().join(format!(
            "voxy-scene-mode-{}-{}",
            std::process::id(),
            NEXT_SAVE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&folder).unwrap();
        let path = folder.join("scene.json");
        write_text(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        write_text(&path, "new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn replacement_roundtrip_and_invalid_save_preserve_last_good_file() {
        let folder = std::env::temp_dir().join(format!(
            "voxy-scene-{}-{}",
            std::process::id(),
            NEXT_SAVE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&folder).unwrap();
        let path = folder.join("scene.json");
        let registry = ComponentRegistry::default();
        let document = SceneDocument {
            version: 1,
            objects: vec![],
        };
        save_scene_file(&path, &document, &registry, 0, 4096).unwrap();
        assert_eq!(read_scene_file(&path, 4096).unwrap(), document);
        let before = fs::read(&path).unwrap();
        let invalid = SceneDocument {
            version: 2,
            objects: vec![],
        };
        assert!(save_scene_file(&path, &invalid, &registry, 0, 4096).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(matches!(
            save_scene_file(&path, &document, &registry, 0, 1),
            Err(SceneFileError::TooLarge)
        ));
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(matches!(
            read_scene_file(&path, 1),
            Err(SceneFileError::TooLarge)
        ));
        save_scene_file(&path, &document, &registry, 0, 4096).unwrap();
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        fs::remove_dir_all(folder).unwrap();
    }
}
