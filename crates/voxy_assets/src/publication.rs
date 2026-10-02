//! Owner-thread publication of resource versions and their source invalidation map.
use crate::{
    AssetCatalog, AssetError, AssetStatus, AssetTicket, DependencyError, FailedImport,
    ImportOutcome, ImportedAsset, SourceDependencies,
};
#[derive(Debug, Eq, PartialEq)]
pub enum PublicationError {
    MissingResult,
    Asset(AssetError),
    Sources(DependencyError),
}
impl std::fmt::Display for PublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asset publication error: {self:?}")
    }
}
impl std::error::Error for PublicationError {}
impl<T> AssetCatalog<ImportedAsset<T>> {
    /// Publishes a current result and source mappings in one exclusive owner call.
    /// Failed imports retain last-good versions and register failed source reads.
    /// Source capacity rejection leaves both structures and pending status unchanged;
    /// the consumed result is released, so the owner must explicitly retry or settle
    /// the ticket with `complete`. Stale dependency tickets are settled as failures,
    /// matching `complete`, without recording obsolete source reads.
    /// # Errors
    /// Rejects stale/foreign/completed tickets or source index capacity overflow.
    pub fn complete_observed(
        &mut self,
        sources: &mut SourceDependencies,
        ticket: &AssetTicket,
        result: Result<ImportedAsset<T>, FailedImport<String>>,
    ) -> Result<(), PublicationError> {
        self.complete_pending(sources, ticket, &mut Some(result))
    }
    /// Applies a retained worker result, consuming it only after successful admission.
    /// Capacity and stale-ticket errors leave it available for inspection or retry.
    /// A changed compiled dependency still settles the ticket as failed; its retained
    /// result is obsolete and must be discarded rather than published on a new ticket.
    /// # Errors
    /// Rejects absent results and all `complete_observed` validation failures.
    pub fn complete_pending(
        &mut self,
        sources: &mut SourceDependencies,
        ticket: &AssetTicket,
        result: &mut Option<Result<ImportedAsset<T>, FailedImport<String>>>,
    ) -> Result<(), PublicationError> {
        let observation = result.as_ref().ok_or(PublicationError::MissingResult)?;
        if ticket.catalog != self.id
            || !self.entries.get(&ticket.asset).is_some_and(|entry| {
                entry.revision == ticket.revision && entry.status == AssetStatus::Loading
            })
        {
            return Err(PublicationError::Asset(AssetError::StaleTicket));
        }
        if !ticket.dependencies.iter().all(|(id, revision)| {
            self.entries.get(id).is_some_and(|entry| {
                entry.revision == *revision && entry.status == AssetStatus::Ready
            })
        }) {
            return self
                .complete(ticket, Err("dependency changed during import".into()))
                .map_err(PublicationError::Asset);
        }
        let (inputs, outcome) = match observation {
            Ok(value) => (value.inputs(), ImportOutcome::Published),
            Err(failed) => (&failed.inputs, ImportOutcome::Failed),
        };
        sources
            .record(ticket.asset.clone(), inputs, outcome)
            .map_err(PublicationError::Sources)?;
        // Exclusive borrows prevent owner mutation between validation and application.
        self.complete(
            ticket,
            result
                .take()
                .ok_or(PublicationError::MissingResult)?
                .map_err(|failure| failure.error),
        )
        .map_err(PublicationError::Asset)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetId, ImportInputs};
    fn id(s: &str) -> AssetId {
        AssetId(s.into())
    }
    fn value(source: &str, v: u8) -> ImportedAsset<u8> {
        let mut inputs = ImportInputs::new(1, 1);
        inputs.read(id(source), |_, _| Ok(vec![v])).unwrap();
        inputs.finish(v, |_, _| Ok(vec![v])).unwrap()
    }
    #[test]
    fn publication_failure_and_stale_results_keep_index_consistent() {
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let mut sources = SourceDependencies::new(1, 2);
        let ticket = catalog.request(id("output")).unwrap();
        catalog
            .complete_observed(&mut sources, &ticket, Ok(value("original", 1)))
            .unwrap();
        let old = catalog.snapshot(&id("output")).unwrap();
        let stale = catalog.request(id("output")).unwrap();
        let current = catalog.request(id("output")).unwrap();
        assert_eq!(
            catalog.complete_observed(&mut sources, &stale, Ok(value("obsolete", 2))),
            Err(PublicationError::Asset(AssetError::StaleTicket))
        );
        assert!(sources.affected([id("obsolete")]).is_empty());
        let mut inputs = ImportInputs::new(1, 1);
        let _ = inputs.read(id("missing"), |_, _| Err("absent".into()));
        catalog
            .complete_observed(
                &mut sources,
                &current,
                Err(FailedImport {
                    error: "decode failed".into(),
                    inputs,
                }),
            )
            .unwrap();
        assert_eq!(catalog.pending(), 0);
        assert!(std::sync::Arc::ptr_eq(
            &old,
            &catalog.snapshot(&id("output")).unwrap()
        ));
        assert_eq!(
            sources.affected([id("original"), id("missing")]),
            vec![id("output")]
        );
        let ticket = catalog.request(id("output")).unwrap();
        catalog
            .complete_observed(&mut sources, &ticket, Ok(value("new", 3)))
            .unwrap();
        assert!(sources.affected([id("original"), id("missing")]).is_empty());
        assert_eq!(sources.affected([id("new")]), vec![id("output")]);
    }
    #[test]
    fn retained_result_retries_publication_without_redecoding() {
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let ticket = catalog.request(id("output")).unwrap();
        let mut sources = SourceDependencies::new(1, 0);
        let artifact = value("source", 7);
        let bytes = artifact.inputs().observations()[&id("source")]
            .as_ref()
            .unwrap()
            .bytes
            .clone();
        let mut pending = Some(Ok(artifact));
        assert_eq!(
            catalog.complete_pending(&mut sources, &ticket, &mut pending),
            Err(PublicationError::Sources(DependencyError::Capacity))
        );
        assert!(pending.is_some());
        assert_eq!(catalog.pending(), 1);
        sources = SourceDependencies::new(1, 1);
        catalog
            .complete_pending(&mut sources, &ticket, &mut pending)
            .unwrap();
        assert!(pending.is_none());
        let published = catalog.snapshot(&id("output")).unwrap();
        assert_eq!(*published.value(), 7);
        assert!(std::sync::Arc::ptr_eq(
            &bytes,
            &published.inputs().observations()[&id("source")]
                .as_ref()
                .unwrap()
                .bytes
        ));
        assert_eq!(
            catalog.complete_pending(&mut sources, &ticket, &mut pending),
            Err(PublicationError::MissingResult)
        );
        assert_eq!(sources.affected([id("source")]), vec![id("output")]);
    }

    #[test]
    fn index_capacity_cannot_publish_unwatched_version() {
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let mut sources = SourceDependencies::new(1, 0);
        let ticket = catalog.request(id("output")).unwrap();
        assert_eq!(
            catalog.complete_observed(&mut sources, &ticket, Ok(value("source", 1))),
            Err(PublicationError::Sources(DependencyError::Capacity))
        );
        assert_eq!(catalog.pending(), 1);
        assert_eq!(catalog.status(&id("output")), Some(&AssetStatus::Loading));
        assert!(catalog.snapshot(&id("output")).is_none());
        assert!(sources.affected([id("source")]).is_empty());
        catalog.complete(&ticket, Err("index full".into())).unwrap();
        assert_eq!(catalog.pending(), 0);
    }
    #[test]
    fn changed_compiled_dependency_settles_ticket_without_obsolete_sources() {
        let mut catalog = AssetCatalog::new(2, 2).unwrap();
        let mut sources = SourceDependencies::new(2, 4);
        let dep = catalog.request(id("dependency")).unwrap();
        catalog
            .complete_observed(&mut sources, &dep, Ok(value("dep-source", 1)))
            .unwrap();
        let output = catalog
            .request_with_dependencies(id("output"), &[id("dependency")])
            .unwrap();
        let dep = catalog.request(id("dependency")).unwrap();
        catalog
            .complete_observed(&mut sources, &dep, Ok(value("new-dep-source", 2)))
            .unwrap();
        assert_eq!(
            catalog.complete_observed(&mut sources, &output, Ok(value("out-source", 3))),
            Err(PublicationError::Asset(AssetError::StaleTicket))
        );
        assert_eq!(catalog.pending(), 0);
        assert!(sources.affected([id("out-source")]).is_empty());
        assert!(matches!(
            catalog.status(&id("output")),
            Some(AssetStatus::Failed(_))
        ));
    }
}
