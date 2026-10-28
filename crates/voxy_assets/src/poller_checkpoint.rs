//! Poll baseline and round-robin continuation; worker channels are separate owners.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceFingerprint {
    Content([u8; 32]),
    Unavailable,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePollerCheckpoint {
    pub version: u32,
    pub sources: Vec<(AssetId, Option<SourceFingerprint>)>,
    pub cursor: usize,
    pub max_sources: usize,
    pub max_file_bytes: usize,
}
impl SourcePoller {
    #[must_use]
    pub fn checkpoint(&self) -> SourcePollerCheckpoint {
        SourcePollerCheckpoint {
            version: 1,
            sources: self
                .sources
                .iter()
                .map(|(id, fingerprint)| {
                    (
                        id.clone(),
                        fingerprint.as_ref().map(|f| match f {
                            Fingerprint::Content(hash) => SourceFingerprint::Content(*hash),
                            Fingerprint::Unavailable => SourceFingerprint::Unavailable,
                        }),
                    )
                })
                .collect(),
            cursor: self.cursor,
            max_sources: self.max_sources,
            max_file_bytes: self.max_file_bytes,
        }
    }
}
impl SourcePollerCheckpoint {
    /// # Errors
    /// Rejects versions, unsorted/duplicate/empty identities, cursors and policy limits.
    pub fn prepare(
        &self,
        max_sources: usize,
        max_file_bytes: usize,
    ) -> Result<SourcePoller, DependencyError> {
        if self.max_sources > max_sources
            || self.max_file_bytes > max_file_bytes
            || self.sources.len() > self.max_sources
        {
            return Err(DependencyError::Capacity);
        }
        if self.version != 1
            || (self.sources.is_empty() && self.cursor != 0)
            || (!self.sources.is_empty() && self.cursor >= self.sources.len())
            || self.sources.iter().any(|(id, _)| id.0.is_empty())
            || self.sources.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err(DependencyError::InvalidId);
        }
        Ok(SourcePoller {
            sources: self
                .sources
                .iter()
                .map(|(id, fingerprint)| {
                    (
                        id.clone(),
                        fingerprint.as_ref().map(|f| match f {
                            SourceFingerprint::Content(hash) => Fingerprint::Content(*hash),
                            SourceFingerprint::Unavailable => Fingerprint::Unavailable,
                        }),
                    )
                })
                .collect(),
            cursor: self.cursor,
            max_sources: self.max_sources,
            max_file_bytes: self.max_file_bytes,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_unavailable_content_and_round_robin_cursor_continue_exactly() {
        let ids: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .map(|s| AssetId(s.into()))
            .collect();
        let mut original = SourcePoller::new(3, 4);
        original.reconcile_ids(ids).unwrap();
        original.poll(1, |_, _| Err("missing".into()));
        let saved = original.checkpoint();
        assert_eq!(saved.cursor, 1);
        let mut restored = saved.prepare(3, 4).unwrap();
        for tick in 0_u8..120 {
            let reader = |id: &AssetId, _: usize| {
                if tick % 7 == 0 && id.0 == "a" {
                    Err("missing".into())
                } else {
                    Ok(vec![tick / 5])
                }
            };
            assert_eq!(
                original.poll(usize::from(tick % 4), reader),
                restored.poll(usize::from(tick % 4), reader)
            );
            assert_eq!(original.checkpoint(), restored.checkpoint());
        }
        let mut bad = saved.clone();
        bad.cursor = 3;
        assert!(bad.prepare(3, 4).is_err());
        let mut bad = saved.clone();
        bad.sources.swap(0, 1);
        assert!(bad.prepare(3, 4).is_err());
        assert!(saved.prepare(2, 4).is_err());
    }
}
