//! Version migration chains and component schema upgrade reporting for scene documents.

use crate::DocumentError;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt;

/// Step transforming a scene or prefab JSON object map from version `V` to `V + 1`.
pub type DocumentMigrationStep = fn(&mut Map<String, Value>) -> Result<(), DocumentError>;

/// Summary of document version and component schema upgrades applied during load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneMigrationReport {
    pub original_version: u32,
    pub target_version: u32,
    pub migrated_components: usize,
}

impl SceneMigrationReport {
    /// Returns true if either the document envelope version or any component payload was upgraded.
    #[must_use]
    pub const fn was_migrated(&self) -> bool {
        self.original_version != self.target_version || self.migrated_components > 0
    }
}

/// Ordered JSON document version migration chain (`V -> V + 1`) for scenes and prefabs.
#[derive(Clone)]
pub struct DocumentMigrationChain {
    min_supported: u32,
    current_version: u32,
    steps: BTreeMap<u32, DocumentMigrationStep>,
}

impl fmt::Debug for DocumentMigrationChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentMigrationChain")
            .field("min_supported", &self.min_supported)
            .field("current_version", &self.current_version)
            .field(
                "registered_from_versions",
                &self.steps.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Default for DocumentMigrationChain {
    fn default() -> Self {
        Self::scene_builtin()
    }
}

impl DocumentMigrationChain {
    /// Built-in migration policy for authored `SceneDocument` files.
    #[must_use]
    pub fn scene_builtin() -> Self {
        Self {
            min_supported: crate::SceneDocument::MIN_SUPPORTED_VERSION,
            current_version: crate::SceneDocument::CURRENT_VERSION,
            steps: BTreeMap::new(),
        }
    }

    /// Built-in migration policy for authored `PrefabSceneDocument` files.
    #[must_use]
    pub fn prefab_builtin() -> Self {
        Self {
            min_supported: crate::PrefabSceneDocument::MIN_SUPPORTED_VERSION,
            current_version: crate::PrefabSceneDocument::CURRENT_VERSION,
            steps: BTreeMap::new(),
        }
    }

    /// Creates a custom migration chain covering `min_supported..=current_version`.
    /// # Errors
    /// Rejects zero minimum version or inverted ranges.
    pub fn with_range(min_supported: u32, current_version: u32) -> Result<Self, DocumentError> {
        if min_supported == 0 || min_supported > current_version {
            return Err(DocumentError::Invalid(
                "invalid document migration version range".into(),
            ));
        }
        Ok(Self {
            min_supported,
            current_version,
            steps: BTreeMap::new(),
        })
    }

    /// Registers a migration step from `from_version` to `from_version + 1`.
    /// # Errors
    /// Rejects out-of-range or duplicate source versions.
    pub fn register_step(
        &mut self,
        from_version: u32,
        step: DocumentMigrationStep,
    ) -> Result<(), DocumentError> {
        if from_version < self.min_supported || from_version >= self.current_version {
            return Err(DocumentError::Invalid(format!(
                "document migration step v{from_version} outside {}..{}",
                self.min_supported, self.current_version
            )));
        }
        if self.steps.insert(from_version, step).is_some() {
            return Err(DocumentError::Invalid(format!(
                "duplicate document migration step from v{from_version}"
            )));
        }
        Ok(())
    }

    /// Parses raw JSON, validates version bounds, runs ordered upgrades, and returns `(upgraded_json, original_version, target_version)`.
    /// # Errors
    /// Rejects malformed JSON, unsupported versions, missing intermediate steps, or failing step hooks.
    pub fn upgrade_json(&self, text: &str) -> Result<(Value, u32, u32), DocumentError> {
        let mut raw: Value = serde_json::from_str(text)?;
        let obj = raw
            .as_object_mut()
            .ok_or_else(|| DocumentError::Invalid("scene document must be a JSON object".into()))?;
        let raw_ver = obj.get("version").and_then(Value::as_u64).ok_or_else(|| {
            DocumentError::Invalid("scene document missing integer version".into())
        })?;
        let version = u32::try_from(raw_ver)
            .map_err(|_| DocumentError::Invalid("scene document version out of range".into()))?;
        if version == 0 || version < self.min_supported || version > self.current_version {
            return Err(DocumentError::Invalid(format!(
                "unsupported document version {version} (supported: {}..={})",
                self.min_supported, self.current_version
            )));
        }
        for v in version..self.current_version {
            let step = self.steps.get(&v).ok_or_else(|| {
                DocumentError::Invalid(format!(
                    "missing document migration step from v{v} to v{}",
                    v + 1
                ))
            })?;
            step(obj)?;
            obj.insert("version".into(), Value::from(v + 1));
        }
        Ok((raw, version, self.current_version))
    }
}
