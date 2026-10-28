//! Persist shared snapshot identity independently of coincident pose values.
use crate::{AnimatorCheckpointError, Pose, PoseCheckpoint, Skeleton};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    rig: String,
    pose: PoseCheckpoint,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoseSnapshotArchive {
    version: u32,
    entries: Vec<Entry>,
}

/// Keeps source Arcs alive while interning, preventing address reuse/ABA.
#[derive(Debug)]
pub struct PoseSnapshotBuilder {
    poses: Vec<Arc<Pose>>,
    entries: Vec<Entry>,
    max_poses: usize,
}
impl PoseSnapshotBuilder {
    #[must_use]
    pub fn new(max_poses: usize) -> Self {
        Self {
            poses: Vec::new(),
            entries: Vec::new(),
            max_poses,
        }
    }
    /// Reuses an ID only for the same allocation, not for equal pose contents.
    /// The rig key must identify the same immutable rig revision on reload.
    /// # Errors
    /// Rejects empty rig keys, inconsistent identities and snapshot count limits.
    pub fn intern(
        &mut self,
        pose: &Arc<Pose>,
        rig: &str,
    ) -> Result<String, AnimatorCheckpointError> {
        if rig.is_empty() {
            return Err(AnimatorCheckpointError::State);
        }
        if let Some(index) = self.poses.iter().position(|old| Arc::ptr_eq(old, pose)) {
            if self.entries[index].rig != rig {
                return Err(AnimatorCheckpointError::State);
            }
            return Ok(self.entries[index].id.clone());
        }
        if self.poses.len() >= self.max_poses {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let id = format!("pose-{}", self.entries.len());
        self.entries.push(Entry {
            id: id.clone(),
            rig: rig.into(),
            pose: pose.checkpoint(),
        });
        self.poses.push(pose.clone());
        Ok(id)
    }
    /// # Errors
    /// Rejects an archive exceeding its encoded byte budget.
    pub fn finish(self, max_bytes: usize) -> Result<PoseSnapshotArchive, AnimatorCheckpointError> {
        let archive = PoseSnapshotArchive {
            version: 1,
            entries: self.entries,
        };
        archive.to_json(max_bytes, self.max_poses)?;
        Ok(archive)
    }
}
#[derive(Debug)]
pub struct PreparedPoseSnapshots {
    poses: HashMap<String, Arc<Pose>>,
}
impl PreparedPoseSnapshots {
    #[must_use]
    pub fn resolve(&self, key: &str) -> Option<Arc<Pose>> {
        self.poses.get(key).cloned()
    }
}
impl PoseSnapshotArchive {
    fn validate(&self, max_poses: usize) -> Result<(), AnimatorCheckpointError> {
        if self.entries.len() > max_poses {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let mut keys = std::collections::HashSet::new();
        if self.version != 1
            || self
                .entries
                .iter()
                .any(|e| e.id.is_empty() || e.rig.is_empty() || !keys.insert(&e.id))
        {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(())
    }
    /// # Errors
    /// Rejects schema/count limits and oversized output.
    pub fn to_json(
        &self,
        max_bytes: usize,
        max_poses: usize,
    ) -> Result<String, AnimatorCheckpointError> {
        self.validate(max_poses)?;
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// Bound file reads before passing text to this parser.
    /// # Errors
    /// Rejects bytes before parsing, unknown fields, duplicate IDs, versions and counts.
    pub fn from_json(
        text: &str,
        max_bytes: usize,
        max_poses: usize,
    ) -> Result<Self, AnimatorCheckpointError> {
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let archive: Self = serde_json::from_str(text)?;
        archive.validate(max_poses)?;
        Ok(archive)
    }
    /// Build every snapshot before returning any usable table. Rig keys are
    /// externally verified immutable revisions; content equality does not merge IDs.
    /// # Errors
    /// Rejects unknown rigs, invalid poses/palettes and admission budgets.
    pub fn prepare(
        &self,
        resolve: impl Fn(&str) -> Option<Skeleton>,
        max_bytes: usize,
        max_poses: usize,
    ) -> Result<PreparedPoseSnapshots, AnimatorCheckpointError> {
        self.to_json(max_bytes, max_poses)?;
        let mut rigs = HashMap::new();
        let mut poses = HashMap::new();
        for entry in &self.entries {
            if !rigs.contains_key(&entry.rig) {
                rigs.insert(
                    entry.rig.clone(),
                    resolve(&entry.rig).ok_or(AnimatorCheckpointError::State)?,
                );
            }
            let pose = entry.pose.prepare(&rigs[&entry.rig])?;
            poses.insert(entry.id.clone(), Arc::new(pose));
        }
        Ok(PreparedPoseSnapshots { poses })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Joint, Transform};
    fn rig() -> Skeleton {
        Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: glam::Mat4::IDENTITY,
        }])
        .unwrap()
    }
    #[test]
    fn archive_preserves_sharing_but_keeps_equal_distinct_allocations_separate() {
        let rig = rig();
        let a = Arc::new(rig.bind_pose());
        let b = Arc::new(rig.bind_pose());
        let mut builder = PoseSnapshotBuilder::new(2);
        let first = builder.intern(&a, "rig-v1").unwrap();
        assert_eq!(builder.intern(&a, "rig-v1").unwrap(), first);
        assert!(builder.intern(&a, "other-rig").is_err());
        let second = builder.intern(&b, "rig-v1").unwrap();
        assert_ne!(first, second);
        assert!(
            builder
                .intern(&Arc::new(rig.bind_pose()), "rig-v1")
                .is_err()
        );
        let archive = builder.finish(65536).unwrap();
        let text = archive.to_json(65536, 2).unwrap();
        let archive = PoseSnapshotArchive::from_json(&text, 65536, 2).unwrap();
        let ready = archive
            .prepare(|key| (key == "rig-v1").then(|| rig.clone()), 65536, 2)
            .unwrap();
        assert!(Arc::ptr_eq(
            &ready.resolve(&first).unwrap(),
            &ready.resolve(&first).unwrap()
        ));
        assert!(!Arc::ptr_eq(
            &ready.resolve(&first).unwrap(),
            &ready.resolve(&second).unwrap()
        ));
        assert_eq!(
            *ready.resolve(&first).unwrap(),
            *ready.resolve(&second).unwrap()
        );
        assert!(ready.resolve("missing").is_none());
        assert!(archive.prepare(|_| None, 65536, 2).is_err());
        assert!(PoseSnapshotArchive::from_json(&text, 1, 2).is_err());
        let first_arc = ready.resolve(&first).unwrap();
        let mut corrupt: serde_json::Value = serde_json::from_str(&text).unwrap();
        corrupt["entries"][1]["pose"][0][7] = 0.into();
        let corrupt = PoseSnapshotArchive::from_json(&corrupt.to_string(), 65536, 2).unwrap();
        assert!(
            corrupt
                .prepare(|key| (key == "rig-v1").then(|| rig.clone()), 65536, 2)
                .is_err()
        );
        assert!(Arc::ptr_eq(&first_arc, &ready.resolve(&first).unwrap()));

        let mut wire: serde_json::Value = serde_json::from_str(&text).unwrap();
        wire["entries"][1]["id"] = wire["entries"][0]["id"].clone();
        assert!(PoseSnapshotArchive::from_json(&wire.to_string(), 65536, 2).is_err());
    }
}
