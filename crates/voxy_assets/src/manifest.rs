//! Versioned project identity manifest. File IO belongs outside the frame loop.
use crate::{
    AssetId, AssetLocations, JsonDocument, JsonDocumentError, LocationError, SourcePath,
    serialize_json_bounded,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    assets: Vec<Binding>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    asset: String,
    source: String,
}
#[derive(Serialize)]
struct ManifestRef<'a> {
    version: u32,
    assets: Vec<BindingRef<'a>>,
}
#[derive(Serialize)]
struct BindingRef<'a> {
    asset: &'a str,
    source: &'a str,
}
#[derive(Debug)]
pub enum ManifestError {
    Location(LocationError),
    Io(std::io::Error),
}
impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asset manifest error: {self:?}")
    }
}
impl std::error::Error for ManifestError {}
impl AssetLocations {
    /// Produces deterministic ID-sorted version-1 JSON, capped during serialization.
    /// # Errors
    /// Rejects encoded-document capacity overflow.
    pub fn to_json(&self, max_document_bytes: usize) -> Result<String, LocationError> {
        let manifest = ManifestRef {
            version: 1,
            assets: self
                .bindings()
                .map(|(asset, source)| BindingRef {
                    asset: &asset.0,
                    source: source.as_str(),
                })
                .collect(),
        };
        serialize_json_bounded(&manifest, max_document_bytes).map_err(|_| LocationError::Capacity)
    }
    /// Builds a detached validated registry; failures cannot modify a live table.
    /// Limits bound document bytes before parsing, then binding count/logical bytes.
    /// # Errors
    /// Rejects unknown schema, duplicate identities/paths, bad paths and capacities.
    pub fn from_json(
        json: &str,
        max_assets: usize,
        max_bytes: usize,
        max_document_bytes: usize,
    ) -> Result<Self, LocationError> {
        let manifest =
            Manifest::from_json_bounded(json, max_document_bytes).map_err(|err| match err {
                JsonDocumentError::CapacityExceeded => LocationError::Capacity,
                _ => LocationError::InvalidManifest,
            })?;
        Self::from_manifest(manifest, max_assets, max_bytes)
    }

    fn from_manifest(
        manifest: Manifest,
        max_assets: usize,
        max_bytes: usize,
    ) -> Result<Self, LocationError> {
        if manifest.version != 1 {
            return Err(LocationError::InvalidManifest);
        }
        if manifest.assets.len() > max_assets {
            return Err(LocationError::Capacity);
        }
        let mut locations = Self::new(max_assets, max_bytes);
        for binding in manifest.assets {
            let asset = AssetId(binding.asset);
            if locations.source(&asset).is_some() {
                return Err(LocationError::IdExists);
            }
            locations.bind(asset, SourcePath::new(binding.source)?)?;
        }
        Ok(locations)
    }
    /// Reads at most the document cap plus one sentinel before parsing.
    /// # Errors
    /// Reports IO, UTF-8, schema, binding and capacity failures.
    pub fn load_manifest(
        path: &Path,
        max_assets: usize,
        max_bytes: usize,
        max_document_bytes: usize,
    ) -> Result<Self, ManifestError> {
        let manifest =
            Manifest::load_json_bounded(path, max_document_bytes).map_err(|err| match err {
                JsonDocumentError::CapacityExceeded => {
                    ManifestError::Location(LocationError::Capacity)
                }
                JsonDocumentError::InvalidPath => {
                    ManifestError::Location(LocationError::InvalidPath)
                }
                JsonDocumentError::Io(io) => ManifestError::Io(io),
                JsonDocumentError::Parse(_) => {
                    ManifestError::Location(LocationError::InvalidManifest)
                }
            })?;
        Self::from_manifest(manifest, max_assets, max_bytes).map_err(ManifestError::Location)
    }
    /// Writes a new temporary file in the destination directory, syncs it, then renames.
    /// Pre-rename failure preserves the target and removes our temporary file.
    /// Assumes a trusted project directory. Directory-entry power-loss durability
    /// and cross-platform replacement semantics are not guaranteed by this API.
    /// # Errors
    /// Reports encoded capacity, temp creation/write/sync and rename errors.
    pub fn save_manifest(
        &self,
        path: &Path,
        max_document_bytes: usize,
    ) -> Result<(), ManifestError> {
        let json = self
            .to_json(max_document_bytes)
            .map_err(ManifestError::Location)?;
        crate::save_atomic_file(path, json.as_bytes(), max_document_bytes).map_err(|error| {
            match error {
                crate::AtomicSaveError::InvalidPath => {
                    ManifestError::Location(LocationError::InvalidPath)
                }
                crate::AtomicSaveError::Capacity => {
                    ManifestError::Location(LocationError::Capacity)
                }
                crate::AtomicSaveError::Io(error) => ManifestError::Io(error),
            }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_roundtrip_preserves_ids_and_rejects_invalid_manifests() {
        let mut locations = AssetLocations::new(2, 64);
        locations
            .bind(AssetId("z".into()), SourcePath::new("b.obj").unwrap())
            .unwrap();
        locations
            .bind(AssetId("a".into()), SourcePath::new("a.obj").unwrap())
            .unwrap();
        let json = locations.to_json(1024).unwrap();
        let restored = AssetLocations::from_json(&json, 2, 64, 1024).unwrap();
        assert_eq!(restored.to_json(1024).unwrap(), json);
        assert_eq!(locations.to_json(1), Err(LocationError::Capacity));
        for invalid in [
            r#"{"version":2,"assets":[]}"#,
            r#"{"version":1,"assets":[],"extra":1}"#,
            r#"{"version":1,"assets":[{"asset":"a","source":"../x"}]}"#,
            r#"{"version":1,"assets":[{"asset":"a","source":"x"},{"asset":"a","source":"x"}]}"#,
            r#"{"version":1,"assets":[{"asset":"a","source":"x"},{"asset":"b","source":"x"}]}"#,
        ] {
            assert!(AssetLocations::from_json(invalid, 2, 64, 1024).is_err());
        }
        assert!(matches!(
            AssetLocations::from_json(&json, 1, 64, 1024),
            Err(LocationError::Capacity)
        ));
        assert!(matches!(
            AssetLocations::from_json(&json, 2, 1, 1024),
            Err(LocationError::Capacity)
        ));
        assert!(matches!(
            AssetLocations::from_json(&json, 2, 64, 1),
            Err(LocationError::Capacity)
        ));
    }
    #[test]
    fn real_save_reload_and_preflight_failure_preserve_existing_file() {
        let root = std::env::temp_dir().join(format!(
            "voxy-manifest-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("assets.json");
        let asset = AssetId("stable-id".into());
        let mut locations = AssetLocations::new(1, 64);
        locations
            .bind(asset.clone(), SourcePath::new("old.obj").unwrap())
            .unwrap();
        locations.save_manifest(&path, 1024).unwrap();
        let previous = std::fs::read(&path).unwrap();
        locations
            .relocate(&asset, SourcePath::new("new.obj").unwrap())
            .unwrap();
        assert!(locations.save_manifest(&path, 1).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        locations.save_manifest(&path, 1024).unwrap();
        let restored = AssetLocations::load_manifest(&path, 1, 64, 1024).unwrap();
        assert_eq!(restored.source(&asset).unwrap().as_str(), "new.obj");
        assert!(matches!(
            AssetLocations::load_manifest(&path, 1, 64, 1),
            Err(ManifestError::Location(LocationError::Capacity))
        ));
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        let blocked = root.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("sentinel"), b"preserve").unwrap();
        assert!(matches!(
            locations.save_manifest(&blocked, 1024),
            Err(ManifestError::Io(_))
        ));
        assert_eq!(
            std::fs::read(blocked.join("sentinel")).unwrap(),
            b"preserve"
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2); // No abandoned temporary file.
        std::fs::remove_dir_all(root).unwrap();
    }
}
