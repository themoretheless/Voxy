//! Immutable source packages preserve the same root-relative observation IDs.
use crate::{AssetId, ImportInputs, ImportedAsset, SourcePath};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub struct PackageLimits {
    pub max_entries: usize,
    pub max_payload_bytes: usize,
    pub max_document_bytes: usize,
}
pub const PROJECT_PACKAGE_LIMITS: PackageLimits = PackageLimits {
    max_entries: 4096,
    max_payload_bytes: 64 * 1024 * 1024,
    max_document_bytes: 256 * 1024 * 1024,
};
pub const PROJECT_LAUNCH_SOURCE: &str = "__voxy_game.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDescriptor {
    pub version: u32,
    pub name: String,
    pub asset_manifest: std::path::PathBuf,
    pub startup_scene: std::path::PathBuf,
}

impl ProjectDescriptor {
    pub const MIN_SUPPORTED_VERSION: u32 = 1;
    pub const CURRENT_VERSION: u32 = 1;

    /// Parses and migrates a raw JSON project descriptor using built-in rules.
    /// # Errors
    /// Rejects malformed JSON, unsupported versions, missing migration steps, unknown fields, or empty names.
    pub fn parse_migrated(text: &str) -> Result<(Self, ProjectMigrationReport), String> {
        ProjectMigrationRegistry::builtin().parse(text)
    }
}

/// Summary of `project.voxy` version migration performed on load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectMigrationReport {
    pub original_version: u32,
    pub target_version: u32,
}

impl ProjectMigrationReport {
    #[must_use]
    pub const fn was_migrated(&self) -> bool {
        self.original_version != self.target_version
    }
}

/// Step transforming a project descriptor JSON map from version `V` to `V + 1`.
pub type ProjectMigrationStep =
    fn(&mut serde_json::Map<String, serde_json::Value>) -> Result<(), String>;

/// Ordered version migration chain for `project.voxy` descriptors.
#[derive(Clone)]
pub struct ProjectMigrationRegistry {
    min_supported: u32,
    current_version: u32,
    steps: BTreeMap<u32, ProjectMigrationStep>,
}

impl std::fmt::Debug for ProjectMigrationRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectMigrationRegistry")
            .field("min_supported", &self.min_supported)
            .field("current_version", &self.current_version)
            .field(
                "registered_from_versions",
                &self.steps.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Default for ProjectMigrationRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ProjectMigrationRegistry {
    #[must_use]
    pub fn builtin() -> Self {
        Self {
            min_supported: ProjectDescriptor::MIN_SUPPORTED_VERSION,
            current_version: ProjectDescriptor::CURRENT_VERSION,
            steps: BTreeMap::new(),
        }
    }

    /// Creates a migration registry covering `min_supported..=current_version`.
    /// # Errors
    /// Rejects zero minimum version or inverted ranges.
    pub fn with_range(min_supported: u32, current_version: u32) -> Result<Self, String> {
        if min_supported == 0 || min_supported > current_version {
            return Err("invalid project version range".into());
        }
        Ok(Self {
            min_supported,
            current_version,
            steps: BTreeMap::new(),
        })
    }

    /// Registers a schema migration step from `from_version` to `from_version + 1`.
    /// # Errors
    /// Rejects out-of-range or duplicate source versions.
    pub fn register_step(
        &mut self,
        from_version: u32,
        step: ProjectMigrationStep,
    ) -> Result<(), String> {
        if from_version < self.min_supported || from_version >= self.current_version {
            return Err(format!(
                "migration step source version {from_version} outside {}..{}",
                self.min_supported, self.current_version
            ));
        }
        if self.steps.insert(from_version, step).is_some() {
            return Err(format!(
                "duplicate project migration step from v{from_version}"
            ));
        }
        Ok(())
    }

    /// Parses, validates version bounds, runs ordered upgrades, and decodes `ProjectDescriptor`.
    /// # Errors
    /// Rejects malformed JSON, unsupported versions, missing steps, unknown fields, and empty names.
    pub fn parse(&self, text: &str) -> Result<(ProjectDescriptor, ProjectMigrationReport), String> {
        let mut raw: serde_json::Value =
            serde_json::from_str(text).map_err(|error| error.to_string())?;
        let obj = raw
            .as_object_mut()
            .ok_or_else(|| "project descriptor must be a JSON object".to_owned())?;
        let version_raw = obj
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "project descriptor missing integer version".to_owned())?;
        let version = u32::try_from(version_raw)
            .map_err(|_| "project descriptor version out of range".to_owned())?;
        if version == 0 || version < self.min_supported || version > self.current_version {
            return Err(format!(
                "unsupported project version {version} (supported: {}..={})",
                self.min_supported, self.current_version
            ));
        }
        for v in version..self.current_version {
            let step = self
                .steps
                .get(&v)
                .ok_or_else(|| format!("missing project migration step from v{v} to v{}", v + 1))?;
            step(obj)?;
            obj.insert("version".into(), serde_json::Value::from(v + 1));
        }
        let descriptor: ProjectDescriptor =
            serde_json::from_value(raw).map_err(|error| error.to_string())?;
        if descriptor.name.trim().is_empty() {
            return Err("project name must not be empty".into());
        }
        Ok((
            descriptor,
            ProjectMigrationReport {
                original_version: version,
                target_version: self.current_version,
            },
        ))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectPackageLaunch {
    pub version: u32,
    pub project: String,
}
impl ProjectPackageLaunch {
    /// # Errors
    /// Rejects unsupported versions and non-root project descriptors.
    pub fn descriptor_path(&self) -> Result<SourcePath, String> {
        if self.version != 2 {
            return Err("unsupported project package launch version".into());
        }
        let path = SourcePath::new(self.project.clone()).map_err(|error| error.to_string())?;
        if path.as_str().contains('/') {
            return Err("package bootstrap must be at its root".into());
        }
        Ok(path)
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    bytes: Vec<u8>,
    digest: [u8; 32],
}
#[derive(Debug, Serialize)]
pub struct ResourcePackage {
    version: u32,
    entries: BTreeMap<String, Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageDocument {
    version: u32,
    #[serde(deserialize_with = "unique_entries")]
    entries: BTreeMap<String, Entry>,
}
fn unique_entries<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Entry>, D::Error> {
    struct Entries;
    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = BTreeMap<String, Entry>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("unique source entries")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut entries = BTreeMap::new();
            while let Some((path, entry)) = map.next_entry::<String, Entry>()? {
                if entries.insert(path, entry).is_some() {
                    return Err(serde::de::Error::custom("duplicate package source"));
                }
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_map(Entries)
}
impl ResourcePackage {
    /// Captures and revalidates all inputs through the regular importer protocol.
    /// # Errors
    /// Rejects duplicates, quotas, missing files and changes during capture.
    pub fn capture(
        paths: &[SourcePath],
        limits: PackageLimits,
        mut read: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
    ) -> Result<ImportedAsset<Self>, String> {
        let mut inputs = ImportInputs::new(limits.max_entries, limits.max_payload_bytes);
        let mut entries = BTreeMap::new();
        for path in paths {
            if entries.contains_key(path.as_str()) {
                return Err("duplicate package source".into());
            }
            let input = inputs
                .read(path.observation_id(), &mut read)
                .map_err(|error| format!("package input: {error:?}"))?;
            entries.insert(
                path.as_str().to_owned(),
                Entry {
                    digest: *blake3::hash(&input.bytes).as_bytes(),
                    bytes: input.bytes.to_vec(),
                },
            );
        }
        let package = Self {
            version: 1,
            entries,
        };
        package.validate(limits)?;
        package.to_bytes(limits)?;
        inputs
            .finish(package, read)
            .map_err(|error| format!("package inputs changed: {error:?}"))
    }
    fn validate(&self, limits: PackageLimits) -> Result<(), String> {
        if self.version != 1 || self.entries.len() > limits.max_entries {
            return Err("invalid package version/count".into());
        }
        let mut bytes = 0_usize;
        for (path, entry) in &self.entries {
            SourcePath::new(path.clone()).map_err(|error| error.to_string())?;
            bytes = bytes
                .checked_add(entry.bytes.len())
                .ok_or("package byte overflow")?;
            if bytes > limits.max_payload_bytes {
                return Err("package payload budget exceeded".into());
            }
            if blake3::hash(&entry.bytes).as_bytes() != &entry.digest {
                return Err(format!("package integrity failure: {path}"));
            }
        }
        Ok(())
    }
    /// # Errors
    /// Rejects invalid contents and serialized document quotas.
    pub fn to_bytes(&self, limits: PackageLimits) -> Result<Vec<u8>, String> {
        self.validate(limits)?;
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        if bytes.len() > limits.max_document_bytes {
            return Err("package document budget exceeded".into());
        }
        Ok(bytes)
    }
    /// # Errors
    /// Rejects untrusted paths, damaged data, schema/version errors and quotas.
    pub fn from_bytes(bytes: &[u8], limits: PackageLimits) -> Result<Self, String> {
        if bytes.len() > limits.max_document_bytes {
            return Err("package document budget exceeded".into());
        }
        let document: PackageDocument =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        let package = Self {
            version: document.version,
            entries: document.entries,
        };
        package.validate(limits)?;
        Ok(package)
    }
    /// Validated root-relative source paths in deterministic order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
    /// Uses the same provider contract as `FileInputs`; no filesystem fallback.
    /// # Errors
    /// Rejects invalid/missing source IDs and per-read quotas.
    pub fn read(&self, id: &AssetId, max_bytes: usize) -> Result<Vec<u8>, String> {
        SourcePath::new(id.0.clone()).map_err(|error| error.to_string())?;
        let entry = self
            .entries
            .get(&id.0)
            .ok_or_else(|| format!("missing packaged source {}", id.0))?;
        if entry.bytes.len() > max_bytes {
            return Err("package read budget exceeded".into());
        }
        Ok(entry.bytes.clone())
    }
    /// Returns the pre-verified BLAKE3 digest for a packaged source after enforcing
    /// path normalization and byte budget, without re-hashing the payload.
    /// # Errors
    /// Rejects invalid/missing source IDs and per-read quotas.
    pub fn fingerprint(&self, id: &AssetId, max_bytes: usize) -> Result<[u8; 32], String> {
        SourcePath::new(id.0.clone()).map_err(|error| error.to_string())?;
        let entry = self
            .entries
            .get(&id.0)
            .ok_or_else(|| format!("missing packaged source {}", id.0))?;
        if entry.bytes.len() > max_bytes {
            return Err("package read budget exceeded".into());
        }
        Ok(entry.digest)
    }
    /// Resolves and parses the root project bootstrap (`__voxy_game.json` -> `ProjectDescriptor`)
    /// directly from memory without extracting files to the filesystem.
    /// # Errors
    /// Rejects missing/malformed launch or descriptor payloads, non-root bootstrap paths, and quotas.
    pub fn open_project_descriptor(&self) -> Result<(SourcePath, ProjectDescriptor), String> {
        let launch_bytes = self.read(&AssetId(PROJECT_LAUNCH_SOURCE.into()), 65_536)?;
        let launch: ProjectPackageLaunch =
            serde_json::from_slice(&launch_bytes).map_err(|error| error.to_string())?;
        let bootstrap = launch.descriptor_path()?;
        let descriptor_bytes = self.read(&bootstrap.observation_id(), 8 * 1024 * 1024)?;
        let descriptor_text = std::str::from_utf8(&descriptor_bytes)
            .map_err(|error| format!("project descriptor UTF-8: {error}"))?;
        let (descriptor, _) = ProjectDescriptor::parse_migrated(descriptor_text)?;
        Ok((bootstrap, descriptor))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_roundtrip_quota_integrity_and_source_change() {
        let limits = PackageLimits {
            max_entries: 2,
            max_payload_bytes: 8,
            max_document_bytes: 2048,
        };
        let paths = [
            SourcePath::new("scene.json").unwrap(),
            SourcePath::new("mesh.obj").unwrap(),
        ];
        let imported =
            ResourcePackage::capture(&paths, limits, |id, _| Ok(id.0.as_bytes()[..4].to_vec()))
                .unwrap();
        let bytes = imported.value().to_bytes(limits).unwrap();
        let package = ResourcePackage::from_bytes(&bytes, limits).unwrap();
        assert!(
            ResourcePackage::from_bytes(
                &bytes,
                PackageLimits {
                    max_entries: 1,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            ResourcePackage::from_bytes(
                &bytes,
                PackageLimits {
                    max_payload_bytes: 7,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            ResourcePackage::from_bytes(
                &bytes,
                PackageLimits {
                    max_document_bytes: bytes.len() - 1,
                    ..limits
                }
            )
            .is_err()
        );
        let mut invalid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        invalid["version"] = serde_json::json!(2);
        assert!(
            ResourcePackage::from_bytes(&serde_json::to_vec(&invalid).unwrap(), limits).is_err()
        );
        invalid["version"] = serde_json::json!(1);
        let entry = invalid["entries"]
            .as_object_mut()
            .unwrap()
            .remove("mesh.obj")
            .unwrap();
        invalid["entries"]["../mesh.obj"] = entry;
        assert!(
            ResourcePackage::from_bytes(&serde_json::to_vec(&invalid).unwrap(), limits).is_err()
        );

        assert_eq!(
            package.read(&AssetId("mesh.obj".into()), 4).unwrap(),
            b"mesh"
        );
        assert!(package.read(&AssetId("../mesh.obj".into()), 4).is_err());
        assert!(package.read(&AssetId("missing".into()), 4).is_err());
        assert!(package.read(&AssetId("mesh.obj".into()), 3).is_err());
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let entry = original["entries"]["mesh.obj"].to_string();
        let duplicate =
            format!("{{\"version\":1,\"entries\":{{\"mesh.obj\":{entry},\"mesh.obj\":{entry}}}}}");
        assert!(ResourcePackage::from_bytes(duplicate.as_bytes(), limits).is_err());
        let mut damaged: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        damaged["entries"]["mesh.obj"]["bytes"][0] = serde_json::json!(0);
        assert!(
            ResourcePackage::from_bytes(&serde_json::to_vec(&damaged).unwrap(), limits).is_err()
        );
        let mut reads = 0;
        assert!(
            ResourcePackage::capture(&paths[..1], limits, |_, _| {
                reads += 1;
                Ok(vec![u8::from(reads > 1)])
            })
            .is_err()
        );
        assert!(
            ResourcePackage::capture(&[paths[0].clone(), paths[0].clone()], limits, |_, _| Ok(
                vec![1]
            ))
            .is_err()
        );
    }

    #[test]
    fn project_descriptor_migration_chain_and_bounds() {
        let v1_json = serde_json::json!({
            "version": 1,
            "name": "Legacy",
            "asset_manifest": "assets.json",
            "startup_scene": "main.scene.json"
        })
        .to_string();
        let (desc, report) = ProjectDescriptor::parse_migrated(&v1_json).unwrap();
        assert_eq!(desc.version, ProjectDescriptor::CURRENT_VERSION);
        assert!(!report.was_migrated());

        let mut chain = ProjectMigrationRegistry::with_range(1, 3).unwrap();
        chain
            .register_step(1, |obj| {
                if let Some(legacy_title) = obj.remove("legacy_title") {
                    obj.insert("name".into(), legacy_title);
                }
                Ok(())
            })
            .unwrap();
        chain
            .register_step(2, |obj| {
                obj.entry("startup_scene".to_owned())
                    .or_insert_with(|| serde_json::json!("main.scene.json"));
                Ok(())
            })
            .unwrap();
        let legacy_v1 = serde_json::json!({
            "version": 1,
            "legacy_title": "Upgraded Project",
            "asset_manifest": "assets.json",
            "startup_scene": "main.scene.json"
        })
        .to_string();
        let (migrated, mig_report) = chain.parse(&legacy_v1).unwrap();
        assert_eq!(migrated.version, 3);
        assert_eq!(migrated.name, "Upgraded Project");
        assert!(mig_report.was_migrated());
        assert_eq!(mig_report.original_version, 1);
        assert_eq!(mig_report.target_version, 3);

        let v0 = serde_json::json!({"version": 0, "name": "Bad", "asset_manifest": "a", "startup_scene": "b"}).to_string();
        assert!(chain.parse(&v0).is_err());
        let v99 = serde_json::json!({"version": 99, "name": "Future", "asset_manifest": "a", "startup_scene": "b"}).to_string();
        assert!(chain.parse(&v99).is_err());
        let empty = serde_json::json!({"version": 1, "name": "  ", "asset_manifest": "a", "startup_scene": "b"}).to_string();
        assert!(ProjectDescriptor::parse_migrated(&empty).is_err());

        let incomplete = ProjectMigrationRegistry::with_range(1, 2).unwrap();
        assert!(incomplete.parse(&v1_json).is_err());
    }
}
