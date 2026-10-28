//! Quiescent publication owner state. External archives own immutable payloads.
use super::*;
use std::collections::{BTreeMap, HashSet};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntryCheckpoint {
    pub asset: AssetId,
    pub revision: u64,
    pub published_revision: u64,
    pub status: AssetStatus,
    pub publication: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetCatalogCheckpoint {
    pub version: u32,
    pub revision: u64,
    pub max_assets: usize,
    pub max_pending: usize,
    pub entries: Vec<CatalogEntryCheckpoint>,
}
impl<T> AssetCatalog<T> {
    /// Captures at an import barrier. Running worker recipes belong to the host.
    /// # Errors
    /// Rejects pending jobs, unidentified payloads and conflicting revision keys.
    pub fn checkpoint(
        &self,
        mut identify: impl FnMut(&Arc<T>) -> Option<String>,
    ) -> Result<AssetCatalogCheckpoint, AssetError> {
        if self.pending != 0 {
            return Err(AssetError::DependencyNotReady);
        }
        let mut keys: BTreeMap<String, &Arc<T>> = BTreeMap::new();
        let mut entries = Vec::with_capacity(self.entries.len());
        for (asset, entry) in &self.entries {
            let publication = entry
                .published
                .as_ref()
                .map(|payload| {
                    let key = identify(payload)
                        .filter(|k| !k.is_empty())
                        .ok_or(AssetError::InvalidId)?;
                    if let Some(prior) = keys.get(&key) {
                        if !Arc::ptr_eq(prior, payload) {
                            return Err(AssetError::InvalidId);
                        }
                    } else {
                        keys.insert(key.clone(), payload);
                    }
                    Ok(key)
                })
                .transpose()?;
            entries.push(CatalogEntryCheckpoint {
                asset: asset.clone(),
                revision: entry.revision,
                published_revision: entry.published_revision,
                status: entry.status.clone(),
                publication,
            });
        }
        Ok(AssetCatalogCheckpoint {
            version: 1,
            revision: self.revision,
            max_assets: self.max_assets,
            max_pending: self.max_pending,
            entries,
        })
    }
}
impl AssetCatalogCheckpoint {
    /// Restores exact revision allocation into a fresh catalog identity.
    /// Old import tickets cannot publish into the rebuilt owner.
    /// # Errors
    /// Rejects active jobs, malformed state, unknown payloads and caller limits.
    pub fn prepare<T>(
        &self,
        mut resolve: impl FnMut(&str) -> Option<Arc<T>>,
        max_assets: usize,
        max_pending: usize,
    ) -> Result<AssetCatalog<T>, AssetError> {
        if self.max_assets > max_assets
            || self.max_pending > max_pending
            || self.entries.len() > self.max_assets
        {
            return Err(AssetError::Capacity);
        }
        if self.version != 1 {
            return Err(AssetError::InvalidId);
        }
        let mut entries = BTreeMap::new();
        let mut revisions = HashSet::new();
        let mut publications: BTreeMap<String, Arc<T>> = BTreeMap::new();
        for saved in &self.entries {
            if saved.asset.0.is_empty()
                || saved.revision == 0
                || saved.revision > self.revision
                || !revisions.insert(saved.revision)
                || saved.published_revision > saved.revision
                || (saved.publication.is_none() != (saved.published_revision == 0))
                || saved.publication.as_ref().is_some_and(String::is_empty)
                || entries.contains_key(&saved.asset)
            {
                return Err(AssetError::InvalidId);
            }
            match saved.status {
                AssetStatus::Loading => return Err(AssetError::DependencyNotReady),
                AssetStatus::Ready
                    if saved.publication.is_none()
                        || saved.published_revision != saved.revision =>
                {
                    return Err(AssetError::InvalidId);
                }
                _ => {}
            }
            let published = if let Some(key) = &saved.publication {
                if !publications.contains_key(key) {
                    let value = resolve(key).ok_or(AssetError::UnknownAsset)?;
                    if publications
                        .values()
                        .any(|prior| Arc::ptr_eq(prior, &value))
                    {
                        return Err(AssetError::InvalidId);
                    }
                    publications.insert(key.clone(), value);
                }
                publications.get(key).cloned()
            } else {
                None
            };
            entries.insert(
                saved.asset.clone(),
                Entry {
                    revision: saved.revision,
                    status: saved.status.clone(),
                    published_revision: saved.published_revision,
                    published,
                },
            );
        }
        let mut catalog = AssetCatalog::new(self.max_assets, self.max_pending)?;
        catalog.revision = self.revision;
        catalog.entries = entries;
        Ok(catalog)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_reload_and_revision_allocator_survive_fresh_owner_restore() {
        let id = AssetId("tone".into());
        let mut source = AssetCatalog::new(3, 2).unwrap();
        let first = source.request(id.clone()).unwrap();
        source.complete(&first, Ok(17_u32)).unwrap();
        let pending = source.request(id.clone()).unwrap();
        assert_eq!(
            source.checkpoint(|_| Some("pcm-v1".into())),
            Err(AssetError::DependencyNotReady)
        );
        source
            .complete(&pending, Err("bad replacement".into()))
            .unwrap();
        let failed = source.request(AssetId("missing".into())).unwrap();
        source.complete(&failed, Err("not found".into())).unwrap();
        let saved = source.checkpoint(|_| Some("pcm-v1".into())).unwrap();
        let mut restored = saved.prepare(|_| Some(Arc::new(17_u32)), 3, 2).unwrap();
        assert_eq!(
            restored.checkpoint(|_| Some("pcm-v1".into())).unwrap(),
            saved
        );
        assert_eq!(restored.snapshot_with_revision(&id).unwrap().0, 1);
        assert_eq!(restored.status(&id), source.status(&id));
        assert_eq!(
            restored.complete(&pending, Ok(99)),
            Err(AssetError::StaleTicket)
        );
        let a = source.request(id.clone()).unwrap();
        let b = restored.request(id.clone()).unwrap();
        source.complete(&a, Ok(23)).unwrap();
        restored.complete(&b, Ok(23)).unwrap();
        assert_eq!(
            source.snapshot_with_revision(&id).unwrap().0,
            restored.snapshot_with_revision(&id).unwrap().0
        );
        assert_eq!(
            source.checkpoint(|_| Some("pcm-v2".into())).unwrap(),
            restored.checkpoint(|_| Some("pcm-v2".into())).unwrap()
        );
        source.invalidate(std::slice::from_ref(&id)).unwrap();
        restored.invalidate(std::slice::from_ref(&id)).unwrap();
        let dirty = source.checkpoint(|_| Some("pcm-v2".into())).unwrap();
        let loaded_dirty = dirty.prepare(|_| Some(Arc::new(23_u32)), 3, 2).unwrap();
        assert_eq!(loaded_dirty.status(&id), Some(&AssetStatus::Dirty));
        assert_eq!(
            loaded_dirty.checkpoint(|_| Some("pcm-v2".into())).unwrap(),
            dirty
        );
        assert_eq!(
            restored.checkpoint(|_| Some("pcm-v2".into())).unwrap(),
            dirty
        );
        assert!(saved.prepare::<u32>(|_| None, 3, 2).is_err());
        assert!(saved.prepare(|_| Some(Arc::new(17_u32)), 2, 2).is_err());
        let mut bad = saved;
        bad.entries[0].published_revision = u64::MAX;
        assert!(bad.prepare(|_| Some(Arc::new(17_u32)), 3, 2).is_err());
    }
}
