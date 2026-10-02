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
}
