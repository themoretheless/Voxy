//! Process-local mutation observations; never serialized as asset revisions.
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(1);
/// Opaque observation of one native catalog or dependency-index state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetChangeToken(u64);
impl AssetChangeToken {
    pub(crate) fn fresh() -> Self {
        Self(
            NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .expect("asset change token space exhausted"),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn catalog_observations_cover_requests_completions_restore_and_removal() {
        let mut catalog = AssetCatalog::new(4, 1).unwrap();
        let id = AssetId("model".into());
        let initial = catalog.change_token();
        let stale = catalog.request(id.clone()).unwrap();
        assert_ne!(initial, catalog.change_token());
        let ticket = catalog.request(id.clone()).unwrap();
        let loading = catalog.change_token();
        assert!(catalog.complete(&stale, Ok(3_u8)).is_err());
        assert_eq!(loading, catalog.change_token());
        catalog.complete(&ticket, Ok(7_u8)).unwrap();
        assert_ne!(loading, catalog.change_token());
        let published = catalog.change_token();
        let value = catalog.snapshot(&id).unwrap();
        assert_eq!(*value, 7);
        assert_eq!(published, catalog.change_token());
        let saved = catalog.checkpoint(|_| Some("version".into())).unwrap();
        let restored = saved.prepare(|_| Some(value.clone()), 4, 1).unwrap();
        assert_eq!(
            saved,
            restored.checkpoint(|_| Some("version".into())).unwrap()
        );
        assert_ne!(published, restored.change_token());
        catalog.invalidate(std::slice::from_ref(&id)).unwrap();
        assert_ne!(published, catalog.change_token());
        let ticket = catalog.request(id.clone()).unwrap();
        let loading = catalog.change_token();
        catalog.complete(&ticket, Err("rejected".into())).unwrap();
        assert_ne!(loading, catalog.change_token());
        let before = catalog.change_token();
        assert!(catalog.remove(&id));
        assert_ne!(before, catalog.change_token());
        let before = catalog.change_token();
        assert!(!catalog.remove(&id));
        assert_eq!(before, catalog.change_token());
    }
    #[test]
    fn source_index_observations_do_not_reuse_tokens_for_equal_mappings() {
        let mut sources = SourceDependencies::new(2, 4);
        let id = AssetId("output".into());
        let reads = ImportInputs::new(0, 0);
        let initial = sources.change_token();
        sources
            .record(id.clone(), &reads, ImportOutcome::Published)
            .unwrap();
        assert_ne!(initial, sources.change_token());
        let before = sources.change_token();
        let saved = sources.checkpoint();
        sources
            .record(id.clone(), &reads, ImportOutcome::Published)
            .unwrap();
        assert_eq!(saved, sources.checkpoint());
        assert_ne!(before, sources.change_token());
        let restored = saved.prepare(2, 4).unwrap();
        assert_eq!(saved, restored.checkpoint());
        assert_ne!(restored.change_token(), sources.change_token());
        let before = sources.change_token();
        sources.remove(&id);
        assert_ne!(before, sources.change_token());
        let before = sources.change_token();
        assert!(
            sources
                .record(AssetId(String::new()), &reads, ImportOutcome::Failed)
                .is_err()
        );
        assert_eq!(before, sources.change_token());
    }
}
