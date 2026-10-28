//! Exact source invalidation index, including failed-attempt dependencies.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceReadsCheckpoint {
    pub output: AssetId,
    pub published: Vec<AssetId>,
    pub attempted: Vec<AssetId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDependenciesCheckpoint {
    pub version: u32,
    pub max_outputs: usize,
    pub max_edges: usize,
    pub outputs: Vec<SourceReadsCheckpoint>,
}
impl SourceDependencies {
    #[must_use]
    pub fn checkpoint(&self) -> SourceDependenciesCheckpoint {
        SourceDependenciesCheckpoint {
            version: 1,
            max_outputs: self.max_outputs,
            max_edges: self.max_edges,
            outputs: self
                .outputs
                .iter()
                .map(|(output, reads)| SourceReadsCheckpoint {
                    output: output.clone(),
                    published: reads.published.iter().cloned().collect(),
                    attempted: reads.attempted.iter().cloned().collect(),
                })
                .collect(),
        }
    }
}
impl SourceDependenciesCheckpoint {
    /// Rebuilds a staged invalidation index without changing any live owner.
    /// # Errors
    /// Rejects versions, duplicate/empty IDs and aggregate policy limits.
    pub fn prepare(
        &self,
        max_outputs: usize,
        max_edges: usize,
    ) -> Result<SourceDependencies, DependencyError> {
        if self.max_outputs > max_outputs
            || self.max_edges > max_edges
            || self.outputs.len() > self.max_outputs
        {
            return Err(DependencyError::Capacity);
        }
        if self.version != 1 {
            return Err(DependencyError::InvalidId);
        }
        let mut index = SourceDependencies::new(self.max_outputs, self.max_edges);
        let mut edges = 0_usize;
        for saved in &self.outputs {
            if saved.output.0.is_empty() || index.outputs.contains_key(&saved.output) {
                return Err(DependencyError::InvalidId);
            }
            let collect = |ids: &[AssetId]| -> Result<BTreeSet<AssetId>, DependencyError> {
                let mut set = BTreeSet::new();
                for id in ids {
                    if id.0.is_empty() || !set.insert(id.clone()) {
                        return Err(DependencyError::InvalidId);
                    }
                }
                Ok(set)
            };
            let reads = Reads {
                published: collect(&saved.published)?,
                attempted: collect(&saved.attempted)?,
            };
            edges = edges
                .checked_add(reads.count())
                .ok_or(DependencyError::Capacity)?;
            if edges > self.max_edges {
                return Err(DependencyError::Capacity);
            }
            index.outputs.insert(saved.output.clone(), reads);
        }
        Ok(index)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn inputs(id: &str, ok: bool) -> ImportInputs {
        let mut inputs = ImportInputs::new(1, 4);
        let _ = inputs.read(AssetId(id.into()), |_, _| {
            if ok {
                Ok(vec![1])
            } else {
                Err("missing".into())
            }
        });
        inputs
    }
    #[test]
    fn failed_attempt_edges_and_following_replacement_survive_restore() {
        let output = AssetId("tone".into());
        let mut original = SourceDependencies::new(2, 4);
        original
            .record(
                output.clone(),
                &inputs("published.wav", true),
                ImportOutcome::Published,
            )
            .unwrap();
        original
            .record(
                output.clone(),
                &inputs("missing.cfg", false),
                ImportOutcome::Failed,
            )
            .unwrap();
        let saved = original.checkpoint();
        let mut restored = saved.prepare(2, 4).unwrap();
        assert_eq!(restored.checkpoint(), saved);
        for key in ["published.wav", "missing.cfg", "other"] {
            let changed = [AssetId(key.into())];
            assert_eq!(
                original.affected(changed.clone()),
                restored.affected(changed)
            );
        }
        let replacement = inputs("new.wav", true);
        original
            .record(output.clone(), &replacement, ImportOutcome::Published)
            .unwrap();
        restored
            .record(output, &replacement, ImportOutcome::Published)
            .unwrap();
        assert_eq!(original.checkpoint(), restored.checkpoint());
        assert!(
            restored
                .affected([AssetId("missing.cfg".into())])
                .is_empty()
        );
        assert!(saved.prepare(1, 4).is_err());
        let mut bad = saved.clone();
        bad.outputs[0].attempted.push(AssetId("missing.cfg".into()));
        assert!(bad.prepare(2, 4).is_err());
        let mut bad = saved;
        bad.max_edges = 1;
        assert!(bad.prepare(2, 4).is_err());
    }
}
