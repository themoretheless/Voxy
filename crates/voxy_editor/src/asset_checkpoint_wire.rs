//! Editor-owned adapters keep native asset identities codec-neutral.
use serde::{Deserialize, Serialize};
use voxy_assets::{
    AssetId, SourceDependenciesCheckpoint, SourceFingerprint, SourcePollerCheckpoint,
    SourceReadsCheckpoint,
};
pub(super) mod ids {
    use super::*;
    pub fn serialize<S: serde::Serializer>(ids: &[AssetId], s: S) -> Result<S::Ok, S::Error> {
        ids.iter().map(|id| &id.0).collect::<Vec<_>>().serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<AssetId>, D::Error> {
        Ok(Vec::<String>::deserialize(d)?
            .into_iter()
            .map(AssetId)
            .collect())
    }
}
pub(super) mod recipes {
    use super::*;
    use std::collections::BTreeMap;
    pub fn serialize<S: serde::Serializer>(
        map: &BTreeMap<AssetId, Option<String>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter()
            .map(|(id, value)| (&id.0, value))
            .collect::<Vec<_>>()
            .serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<AssetId, Option<String>>, D::Error> {
        let entries = Vec::<(String, Option<String>)>::deserialize(d)?;
        let mut result = BTreeMap::new();
        for (id, value) in entries {
            let id = AssetId(id);
            if result.contains_key(&id) {
                return Err(serde::de::Error::custom("duplicate audio recipe"));
            }
            result.insert(id, value);
        }
        Ok(result)
    }
}
pub(super) mod sources {
    use super::*;
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        version: u32,
        max_outputs: usize,
        max_edges: usize,
        outputs: Vec<(String, Vec<String>, Vec<String>)>,
    }
    pub fn serialize<S: serde::Serializer>(
        state: &SourceDependenciesCheckpoint,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        Wire {
            version: state.version,
            max_outputs: state.max_outputs,
            max_edges: state.max_edges,
            outputs: state
                .outputs
                .iter()
                .map(|r| {
                    (
                        r.output.0.clone(),
                        r.published.iter().map(|id| id.0.clone()).collect(),
                        r.attempted.iter().map(|id| id.0.clone()).collect(),
                    )
                })
                .collect(),
        }
        .serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> Result<SourceDependenciesCheckpoint, D::Error> {
        let w = Wire::deserialize(d)?;
        Ok(SourceDependenciesCheckpoint {
            version: w.version,
            max_outputs: w.max_outputs,
            max_edges: w.max_edges,
            outputs: w
                .outputs
                .into_iter()
                .map(|(output, published, attempted)| SourceReadsCheckpoint {
                    output: AssetId(output),
                    published: published.into_iter().map(AssetId).collect(),
                    attempted: attempted.into_iter().map(AssetId).collect(),
                })
                .collect(),
        })
    }
}
pub(super) mod watcher {
    use super::*;
    #[derive(Serialize, Deserialize)]
    enum Fingerprint {
        Unknown,
        Unavailable,
        Content([u8; 32]),
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        version: u32,
        max_sources: usize,
        max_file_bytes: usize,
        cursor: usize,
        sources: Vec<(String, Fingerprint)>,
    }
    pub fn serialize<S: serde::Serializer>(
        state: &SourcePollerCheckpoint,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        Wire {
            version: state.version,
            max_sources: state.max_sources,
            max_file_bytes: state.max_file_bytes,
            cursor: state.cursor,
            sources: state
                .sources
                .iter()
                .map(|(id, f)| {
                    (
                        id.0.clone(),
                        match f {
                            None => Fingerprint::Unknown,
                            Some(SourceFingerprint::Unavailable) => Fingerprint::Unavailable,
                            Some(SourceFingerprint::Content(hash)) => Fingerprint::Content(*hash),
                        },
                    )
                })
                .collect(),
        }
        .serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> Result<SourcePollerCheckpoint, D::Error> {
        let w = Wire::deserialize(d)?;
        Ok(SourcePollerCheckpoint {
            version: w.version,
            max_sources: w.max_sources,
            max_file_bytes: w.max_file_bytes,
            cursor: w.cursor,
            sources: w
                .sources
                .into_iter()
                .map(|(id, f)| {
                    (
                        AssetId(id),
                        match f {
                            Fingerprint::Unknown => None,
                            Fingerprint::Unavailable => Some(SourceFingerprint::Unavailable),
                            Fingerprint::Content(hash) => Some(SourceFingerprint::Content(hash)),
                        },
                    )
                })
                .collect(),
        })
    }
}

// Shared quiescent catalog wire format for audio and model publications.
use crate::EditorError;
use voxy_assets::{AssetCatalogCheckpoint, AssetStatus, CatalogEntryCheckpoint};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogWire {
    version: u32,
    revision: u64,
    max_assets: usize,
    max_pending: usize,
    entries: Vec<EntryWire>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryWire {
    asset: String,
    revision: u64,
    published_revision: u64,
    status: StatusWire,
    publication: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum StatusWire {
    Ready,
    Dirty,
    Failed(String),
}
impl TryFrom<AssetCatalogCheckpoint> for CatalogWire {
    type Error = EditorError;
    fn try_from(c: AssetCatalogCheckpoint) -> Result<Self, EditorError> {
        let entries = c
            .entries
            .into_iter()
            .map(|e| {
                Ok(EntryWire {
                    asset: e.asset.0,
                    revision: e.revision,
                    published_revision: e.published_revision,
                    publication: e.publication,
                    status: match e.status {
                        AssetStatus::Ready => StatusWire::Ready,
                        AssetStatus::Dirty => StatusWire::Dirty,
                        AssetStatus::Failed(error) => StatusWire::Failed(error),
                        AssetStatus::Loading => {
                            return Err("asset import barrier not reached".into());
                        }
                    },
                })
            })
            .collect::<Result<_, EditorError>>()?;
        Ok(Self {
            version: c.version,
            revision: c.revision,
            max_assets: c.max_assets,
            max_pending: c.max_pending,
            entries,
        })
    }
}
impl From<CatalogWire> for AssetCatalogCheckpoint {
    fn from(c: CatalogWire) -> Self {
        Self {
            version: c.version,
            revision: c.revision,
            max_assets: c.max_assets,
            max_pending: c.max_pending,
            entries: c
                .entries
                .into_iter()
                .map(|e| CatalogEntryCheckpoint {
                    asset: AssetId(e.asset),
                    revision: e.revision,
                    published_revision: e.published_revision,
                    publication: e.publication,
                    status: match e.status {
                        StatusWire::Ready => AssetStatus::Ready,
                        StatusWire::Dirty => AssetStatus::Dirty,
                        StatusWire::Failed(error) => AssetStatus::Failed(error),
                    },
                })
                .collect(),
        }
    }
}
