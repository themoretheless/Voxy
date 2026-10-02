//! Generic disk artifact bytes; format-specific verification belongs to importers.
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAGIC: &[u8; 8] = b"VOXYCA01";
const HEADER: usize = 72;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum ArtifactCacheError {
    Io(std::io::Error),
    TooLarge,
    Corrupt,
}
impl From<std::io::Error> for ArtifactCacheError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl std::fmt::Display for ArtifactCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "artifact cache error: {self:?}")
    }
}
impl std::error::Error for ArtifactCacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Root-scoped cache for trusted project directories. Keys are existing build-key
/// bytes; the cache does not define a parallel import identity or certify payloads.
#[derive(Debug)]
pub struct ArtifactCache {
    root: PathBuf,
    entry_limit: usize,
}
impl ArtifactCache {
    /// Creates/resolves the cache directory. Limit applies to payload bytes.
    /// # Errors
    /// Rejects IO failures and limits that cannot include the header/sentinel.
    pub fn new(root: impl AsRef<Path>, entry_limit: usize) -> Result<Self, ArtifactCacheError> {
        u64::try_from(entry_limit)
            .ok()
            .and_then(|limit| limit.checked_add(HEADER as u64 + 1))
            .ok_or(ArtifactCacheError::TooLarge)?;
        fs::create_dir_all(root.as_ref())?;
        let root = root.as_ref().canonicalize()?;
        if !root.is_dir() {
            return Err(std::io::Error::other("cache root is not a directory").into());
        }
        Ok(Self { root, entry_limit })
    }
    fn path(&self, key: &[u8; 32]) -> PathBuf {
        self.root
            .join(format!("{}.artifact", blake3::Hash::from(*key).to_hex()))
    }
    /// Bounded read with version/key/content integrity checks. Misses return None.
    /// Importers must still verify decoded format/geometry and current source identity.
    /// # Errors
    /// Rejects non-file entries, IO, excessive bytes and corrupt envelopes/payloads.
    pub fn load(&self, key: &[u8; 32]) -> Result<Option<Vec<u8>>, ArtifactCacheError> {
        let path = self.path(key);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file() {
            return Err(ArtifactCacheError::Corrupt);
        }
        // Like FileInputs, this assumes no hostile concurrent directory/symlink swaps.
        let file = File::open(path)?;
        let limit = u64::try_from(self.entry_limit).map_err(|_| ArtifactCacheError::TooLarge)?
            + HEADER as u64
            + 1;
        let mut bytes = Vec::new();
        file.take(limit).read_to_end(&mut bytes)?;
        if bytes.len() < HEADER {
            return Err(ArtifactCacheError::Corrupt);
        }
        if bytes.len() - HEADER > self.entry_limit {
            return Err(ArtifactCacheError::TooLarge);
        }
        if &bytes[..8] != MAGIC
            || &bytes[8..40] != key
            || blake3::hash(&bytes[HEADER..]).as_bytes() != &bytes[40..72]
        {
            return Err(ArtifactCacheError::Corrupt);
        }
        bytes.drain(..HEADER);
        Ok(Some(bytes))
    }
    /// Writes/syncs a unique same-directory temporary file, then renames it over
    /// the key entry. Failure before successful rename preserves the previous file.
    /// Rename replacement semantics follow the platform/filesystem; unsupported
    /// replacement returns IO rather than deleting the previous entry.
    /// This is atomic publication, not a claim of directory crash durability.
    /// # Errors
    /// Rejects excessive payloads, exhausted temporary identities and IO failures.
    pub fn store(&self, key: &[u8; 32], bytes: &[u8]) -> Result<(), ArtifactCacheError> {
        if bytes.len() > self.entry_limit {
            return Err(ArtifactCacheError::TooLarge);
        }
        let (pending, mut file) = self.temporary(key)?;
        file.write_all(MAGIC)?;
        file.write_all(key)?;
        file.write_all(blake3::hash(bytes).as_bytes())?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&pending.0, self.path(key))?;
        Ok(())
    }
    fn temporary(&self, key: &[u8; 32]) -> Result<(Pending, File), ArtifactCacheError> {
        for _ in 0..16 {
            let id = NEXT_TEMP
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .map_err(|_| std::io::Error::other("temporary identity exhausted"))?;
            let path = self.root.join(format!(
                ".{}-{}-{id}.part",
                blake3::Hash::from(*key).to_hex(),
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((Pending(path), file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }
        Err(std::io::Error::other("temporary collisions exhausted").into())
    }
}
struct Pending(PathBuf);
impl Drop for Pending {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("voxy-artifact-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn disk_roundtrip_limit_and_last_good() {
        let directory = Directory::new();
        let cache = ArtifactCache::new(&directory.0, 4).unwrap();
        let key = [1; 32];
        assert!(cache.load(&key).unwrap().is_none());
        cache.store(&key, b"good").unwrap();
        assert_eq!(cache.load(&key).unwrap(), Some(b"good".to_vec()));
        assert!(matches!(
            cache.store(&key, b"oversized"),
            Err(ArtifactCacheError::TooLarge)
        ));
        assert_eq!(cache.load(&key).unwrap(), Some(b"good".to_vec()));
        cache.store(&key, b"new").unwrap();
        assert_eq!(cache.load(&key).unwrap(), Some(b"new".to_vec()));
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }
    #[test]
    fn corruption_and_foreign_key_are_not_hits() {
        let directory = Directory::new();
        let cache = ArtifactCache::new(&directory.0, 4).unwrap();
        let key = [1; 32];
        cache.store(&key, b"good").unwrap();
        let path = cache.path(&key);
        let mut bytes = fs::read(&path).unwrap();
        bytes[HEADER] ^= 1;
        fs::write(&path, &bytes).unwrap();
        assert!(matches!(cache.load(&key), Err(ArtifactCacheError::Corrupt)));
        cache.store(&key, b"good").unwrap();
        fs::rename(&path, cache.path(&[2; 32])).unwrap();
        assert!(matches!(
            cache.load(&[2; 32]),
            Err(ArtifactCacheError::Corrupt)
        ));
    }
    #[test]
    fn build_key_changes_miss_without_losing_old_entry() {
        let directory = Directory::new();
        let cache = ArtifactCache::new(&directory.0, 8).unwrap();
        let key = |source: &[u8], options: &[u8]| {
            let mut inputs = crate::ImportInputs::new(1, 8);
            inputs
                .read(
                    crate::AssetId("mesh.obj".into()),
                    |_, _| Ok(source.to_vec()),
                )
                .unwrap();
            inputs
                .build_key("voxy.lod", "verifier-1", "native", options)
                .unwrap()
        };
        let original = key(b"source", b"depth=1");
        cache.store(&original, b"artifact").unwrap();
        for changed in [key(b"changed", b"depth=1"), key(b"source", b"depth=2")] {
            assert!(cache.load(&changed).unwrap().is_none());
        }
        assert_eq!(cache.load(&original).unwrap(), Some(b"artifact".to_vec()));
    }
    #[test]
    fn rename_failure_cleans_pending_file_and_oversized_reads_reject() {
        let directory = Directory::new();
        let cache = ArtifactCache::new(&directory.0, 4).unwrap();
        let key = [3; 32];
        fs::create_dir(cache.path(&key)).unwrap();
        assert!(matches!(
            cache.store(&key, b"good"),
            Err(ArtifactCacheError::Io(_))
        ));
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
        assert!(cache.path(&key).is_dir());
        fs::remove_dir(cache.path(&key)).unwrap();
        fs::write(cache.path(&key), vec![0; HEADER + 5]).unwrap();
        assert!(matches!(
            cache.load(&key),
            Err(ArtifactCacheError::TooLarge)
        ));
    }
}
