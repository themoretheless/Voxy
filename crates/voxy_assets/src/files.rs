//! Root-scoped file input provider for trusted project directories.
use crate::AssetId;
use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub struct FileInputs {
    root: PathBuf,
}
impl FileInputs {
    /// # Errors
    /// Fails if the source root cannot be resolved or is not a directory.
    pub fn new(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().canonicalize()?;
        if !root.is_dir() {
            return Err(std::io::Error::other("source root is not a directory"));
        }
        Ok(Self { root })
    }
    /// Reads a relative path, checking resolved symlinks remain inside the root.
    /// Reads at most limit + one sentinel byte to detect oversized files.
    /// This portable path check assumes a trusted project directory: concurrent
    /// symlink replacement between resolution and opening is not prevented.
    /// # Errors
    /// Rejects non-normal paths, escaping symlinks, non-files, IO and byte limits.
    pub fn read(&self, id: &AssetId, limit: usize) -> Result<Vec<u8>, String> {
        let relative = Path::new(&id.0);
        if id.0.is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("input must be a normal relative path".into());
        }
        let path = self
            .root
            .join(relative)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&self.root) {
            return Err("input escapes source root".into());
        }
        let file = File::open(path).map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("input is not a file".into());
        }
        let read_limit = u64::try_from(limit)
            .map_err(|e| e.to_string())?
            .checked_add(1)
            .ok_or("input limit overflow")?;
        let mut bytes = Vec::new();
        file.take(read_limit)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > limit {
            return Err("input byte limit exceeded".into());
        }
        Ok(bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ImportInputs, InputError};
    #[cfg(unix)]
    #[test]
    fn symlink_resolution_keeps_reads_in_the_source_root() {
        let parent = std::env::temp_dir().join(format!(
            "voxy-input-links-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = parent.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("inside"), [1]).unwrap();
        std::fs::write(parent.join("outside"), [2]).unwrap();
        std::os::unix::fs::symlink("inside", root.join("local-link")).unwrap();
        std::os::unix::fs::symlink("../outside", root.join("escape-link")).unwrap();
        let provider = FileInputs::new(&root).unwrap();
        assert_eq!(
            provider.read(&AssetId("local-link".into()), 1).unwrap(),
            vec![1]
        );
        assert_eq!(
            provider.read(&AssetId("escape-link".into()), 1),
            Err("input escapes source root".into())
        );
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn actual_files_are_bounded_and_changes_detected() {
        let root = std::env::temp_dir().join(format!(
            "voxy-inputs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("source"), [1, 2]).unwrap();
        let provider = FileInputs::new(&root).unwrap();
        let id = AssetId("source".into());
        let mut inputs = ImportInputs::new(1, 2);
        inputs
            .read(id.clone(), |id, limit| provider.read(id, limit))
            .unwrap();
        assert_eq!(
            inputs.validate(|id, limit| provider.read(id, limit)),
            Ok(())
        );
        std::fs::write(root.join("source"), [2, 1]).unwrap();
        assert_eq!(
            inputs.validate(|id, limit| provider.read(id, limit)),
            Err(InputError::Changed(id.clone()))
        );
        assert!(provider.read(&id, 1).is_err());
        assert!(provider.read(&AssetId("../source".into()), 2).is_err());
        assert!(
            provider
                .read(&AssetId(root.to_string_lossy().into_owned()), 2)
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
