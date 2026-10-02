//! Stable logical asset identities resolve through owner-managed source locations.
use crate::AssetId;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourcePath(String);
impl SourcePath {
    /// A portable root-relative path, without traversal or ambiguous separators.
    /// # Errors
    /// Rejects empty/dot segments, absolute paths, backslashes, colons and NUL.
    pub fn new(path: impl Into<String>) -> Result<Self, LocationError> {
        let path = path.into();
        if path.contains(['\\', ':', '\0'])
            || path.split('/').any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(LocationError::InvalidPath);
        }
        Ok(Self(path))
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Bridges source observation APIs that currently use `AssetId` for source keys.
    #[must_use]
    pub fn observation_id(&self) -> AssetId {
        AssetId(self.0.clone())
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocationError {
    InvalidManifest,
    InvalidId,
    InvalidPath,
    Capacity,
    IdExists,
    SourceInUse,
    UnknownAsset,
}
impl std::fmt::Display for LocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asset location error: {self:?}")
    }
}
impl std::error::Error for LocationError {}
/// One project owner manages a bidirectional, byte/count-bounded identity table.
/// Logical IDs are caller-issued and must persist independently of paths/content.
/// This table neither renames physical files nor rewrites source observations.
#[derive(Debug)]
pub struct AssetLocations {
    sources: BTreeMap<AssetId, SourcePath>,
    assets: BTreeMap<SourcePath, AssetId>,
    max_assets: usize,
    max_bytes: usize,
    bytes: usize,
}
impl AssetLocations {
    #[must_use]
    pub fn new(max_assets: usize, max_bytes: usize) -> Self {
        Self {
            sources: BTreeMap::new(),
            assets: BTreeMap::new(),
            max_assets,
            max_bytes,
            bytes: 0,
        }
    }
    /// Iterates the bounded logical resource table in identity order.
    pub fn bindings(&self) -> impl Iterator<Item = (&AssetId, &SourcePath)> {
        self.sources.iter()
    }
    /// Registers a logical identity. Repeating the exact binding is a no-op.
    /// # Errors
    /// Rejects empty IDs, conflicting bindings and count/identity-byte overflow.
    pub fn bind(&mut self, asset: AssetId, source: SourcePath) -> Result<(), LocationError> {
        if asset.0.is_empty() {
            return Err(LocationError::InvalidId);
        }
        if let Some(old) = self.sources.get(&asset) {
            return if old == &source {
                Ok(())
            } else {
                Err(LocationError::IdExists)
            };
        }
        if self.assets.contains_key(&source) {
            return Err(LocationError::SourceInUse);
        }
        let bytes = self
            .bytes
            .checked_add(asset.0.len())
            .and_then(|n| n.checked_add(source.0.len()))
            .ok_or(LocationError::Capacity)?;
        if self.sources.len() >= self.max_assets || bytes > self.max_bytes {
            return Err(LocationError::Capacity);
        }
        self.assets.insert(source.clone(), asset.clone());
        self.sources.insert(asset, source);
        self.bytes = bytes;
        Ok(())
    }
    /// Changes a source location while preserving logical identity and references.
    /// # Errors
    /// Rejects unknown IDs, source collisions or byte overflow before mutation.
    pub fn relocate(&mut self, asset: &AssetId, source: SourcePath) -> Result<bool, LocationError> {
        let old = self.sources.get(asset).ok_or(LocationError::UnknownAsset)?;
        if old == &source {
            return Ok(false);
        }
        if self.assets.contains_key(&source) {
            return Err(LocationError::SourceInUse);
        }
        let bytes = (self.bytes - old.0.len())
            .checked_add(source.0.len())
            .ok_or(LocationError::Capacity)?;
        if bytes > self.max_bytes {
            return Err(LocationError::Capacity);
        }
        self.assets.remove(old);
        self.assets.insert(source.clone(), asset.clone());
        self.sources.insert(asset.clone(), source);
        self.bytes = bytes;
        Ok(true)
    }
    #[must_use]
    pub fn source(&self, asset: &AssetId) -> Option<&SourcePath> {
        self.sources.get(asset)
    }
    #[must_use]
    pub fn asset_at(&self, source: &SourcePath) -> Option<&AssetId> {
        self.assets.get(source)
    }
    pub fn remove(&mut self, asset: &AssetId) -> bool {
        let Some(source) = self.sources.remove(asset) else {
            return false;
        };
        self.assets.remove(&source);
        self.bytes -= asset.0.len() + source.0.len();
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn path(s: &str) -> SourcePath {
        SourcePath::new(s).unwrap()
    }
    #[test]
    fn relocation_preserves_identity_and_cleans_reverse_binding() {
        let id = AssetId("stable-quad-id".into());
        let mut locations = AssetLocations::new(1, 64);
        locations.bind(id.clone(), path("models/quad.obj")).unwrap();
        assert!(locations.relocate(&id, path("moved/quad.obj")).unwrap());
        assert_eq!(locations.asset_at(&path("moved/quad.obj")), Some(&id));
        assert_eq!(locations.asset_at(&path("models/quad.obj")), None);
        assert_eq!(locations.source(&id).unwrap().as_str(), "moved/quad.obj");
        assert!(!locations.relocate(&id, path("moved/quad.obj")).unwrap());
        assert!(locations.remove(&id));
        assert!(locations.asset_at(&path("moved/quad.obj")).is_none());
    }
    #[test]
    fn collisions_and_capacity_leave_both_directions_unchanged() {
        let a = AssetId("a".into());
        let b = AssetId("b".into());
        let mut locations = AssetLocations::new(2, 4);
        locations.bind(a.clone(), path("x")).unwrap();
        locations.bind(b.clone(), path("y")).unwrap();
        assert_eq!(
            locations.relocate(&a, path("y")),
            Err(LocationError::SourceInUse)
        );
        assert_eq!(
            locations.relocate(&a, path("long")),
            Err(LocationError::Capacity)
        );
        assert_eq!(
            locations.bind(a.clone(), path("z")),
            Err(LocationError::IdExists)
        );
        assert_eq!(locations.source(&a), Some(&path("x")));
        assert_eq!(locations.asset_at(&path("x")), Some(&a));
        locations.remove(&b);
        assert!(locations.relocate(&a, path("abc")).unwrap());
    }
    #[test]
    fn source_paths_reject_ambiguous_or_escaping_syntax() {
        for invalid in [
            "", "/a", "a/", "a//b", "../a", "./a", "a/../b", "C:/a", "a\\b", "a\0b",
        ] {
            assert_eq!(SourcePath::new(invalid), Err(LocationError::InvalidPath));
        }
        assert!(SourcePath::new("models/куб.obj").is_ok());
    }
}
