//! Root-scoped file input provider for trusted project directories.
use crate::AssetId;
use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug)]
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
        let file = self.open_bounded(id, limit)?;
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
    /// Hashes all file bytes with a fixed 64 KiB scratch buffer. The byte budget
    /// is enforced both against opened-file metadata and while reading, so a
    /// concurrently growing file cannot bypass admission. Same trusted-root
    /// assumptions as `read` apply; concurrent edits are not an atomic snapshot.
    /// # Errors
    /// Reports the same path, IO and byte-limit errors as `read`.
    pub fn fingerprint(&self, id: &AssetId, limit: usize) -> Result<[u8; 32], String> {
        let file = self.open_bounded(id, limit)?;
        let read_limit = u64::try_from(limit)
            .map_err(|e| e.to_string())?
            .checked_add(1)
            .ok_or("input limit overflow")?;
        let mut reader = file.take(read_limit);
        let mut scratch = vec![0_u8; 65536];
        let mut remaining = limit;
        let mut hash = blake3::Hasher::new();
        loop {
            let count = match reader.read(&mut scratch) {
                Ok(count) => count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.to_string()),
            };
            if count == 0 {
                return Ok(*hash.finalize().as_bytes());
            }
            remaining = remaining
                .checked_sub(count)
                .ok_or("input byte limit exceeded")?;
            hash.update(&scratch[..count]);
        }
    }
    fn open_bounded(&self, id: &AssetId, limit: usize) -> Result<File, String> {
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
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("input is not a file".into());
        }
        if metadata.len() > u64::try_from(limit).map_err(|e| e.to_string())? {
            return Err("input byte limit exceeded".into());
        }
        Ok(file)
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
    fn streaming_fingerprint_matches_full_bytes_across_block_boundaries_and_limits() {
        let root = std::env::temp_dir().join(format!(
            "voxy-streaming-input-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let provider = FileInputs::new(&root).unwrap();
        let id = AssetId("source".into());
        for len in [0, 1, 65535, 65536, 65537, 200003] {
            let bytes: Vec<_> = (0..len).map(|i| u8::try_from(i % 251).unwrap()).collect();
            std::fs::write(root.join("source"), &bytes).unwrap();
            assert_eq!(
                provider.fingerprint(&id, len).unwrap(),
                *blake3::hash(&bytes).as_bytes()
            );
            assert_eq!(provider.read(&id, len).unwrap(), bytes);
            if len > 0 {
                assert_eq!(
                    provider.fingerprint(&id, len - 1),
                    Err("input byte limit exceeded".into())
                );
                assert_eq!(
                    provider.read(&id, len - 1),
                    Err("input byte limit exceeded".into())
                );
                let mut changed = bytes;
                changed[len - 1] ^= 1;
                std::fs::write(root.join("source"), &changed).unwrap();
                assert_eq!(
                    provider.fingerprint(&id, len).unwrap(),
                    *blake3::hash(&changed).as_bytes()
                );
            }
        }
        std::fs::remove_file(root.join("source")).unwrap();
        assert!(provider.fingerprint(&id, 200003).is_err());
        assert!(
            provider
                .fingerprint(&AssetId("../escape".into()), 1)
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
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
