//! Persist actual compiled mapping and calibrated reference, not a replacement profile.
use super::*;
use crate::{AnimatorCheckpointError, PoseCheckpoint};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    source: usize,
    target: usize,
    rotation: [u32; 4],
    translation: [u32; 4],
    scale: u32,
    root_nonzero: Option<[[bool; 3]; 3]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetargetBindingCheckpoint {
    version: u32,
    mapping: Vec<Mapping>,
    target_reference: Option<PoseCheckpoint>,
}
impl RetargetBinding {
    /// Rigs are externally identified immutable revisions. Canonical joint indices
    /// retain the compiled mapping even when authoring names are supplied elsewhere.
    /// # Errors
    /// Rejects an exceeded encoded byte budget.
    pub fn checkpoint(
        &self,
        max_bytes: usize,
    ) -> Result<RetargetBindingCheckpoint, AnimatorCheckpointError> {
        let reference = Pose {
            rig: self.target.clone(),
            local: self.target_reference.to_vec(),
        };
        let bind = Skeleton {
            joints: self.target.clone(),
        }
        .bind_pose();
        let reference = reference.checkpoint();
        let target_reference = if reference == bind.checkpoint() {
            None
        } else {
            Some(reference)
        };
        let saved = RetargetBindingCheckpoint {
            version: 1,
            mapping: self
                .joints
                .iter()
                .map(|j| Mapping {
                    source: j.source,
                    target: j.target,
                    rotation: j.rotation_basis.to_array().map(f32::to_bits),
                    translation: j.translation_basis.to_array().map(f32::to_bits),
                    scale: j.translation_scale.to_bits(),
                    root_nonzero: j.root_basis_nonzero,
                })
                .collect(),
            target_reference,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
impl RetargetBindingCheckpoint {
    /// # Errors
    /// Rejects unsupported versions and byte budgets.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, AnimatorCheckpointError> {
        if self.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// File reads must also be bounded by the caller.
    /// # Errors
    /// Rejects oversized text before parsing, unknown fields and versions.
    pub fn from_json(text: &str, max_bytes: usize) -> Result<Self, AnimatorCheckpointError> {
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(saved)
    }
    /// Rebuild via native mapping/calibration validation and recompute the exact
    /// root-similarity coefficient pattern. Serialized flags are only compared.
    /// # Errors
    /// Rejects invalid mapping/reference, stale root flags, rigs and byte budgets.
    pub fn prepare(
        &self,
        source: &Skeleton,
        target: &Skeleton,
        max_bytes: usize,
    ) -> Result<RetargetBinding, AnimatorCheckpointError> {
        self.to_json(max_bytes)?;
        if self.mapping.is_empty() || self.mapping.len() > MAX_JOINTS {
            return Err(AnimatorCheckpointError::State);
        }
        let mapping = self
            .mapping
            .iter()
            .map(|m| {
                let source = source
                    .joints
                    .get(m.source)
                    .ok_or(AnimatorCheckpointError::State)?
                    .name
                    .clone();
                let target = target
                    .joints
                    .get(m.target)
                    .ok_or(AnimatorCheckpointError::State)?
                    .name
                    .clone();
                Ok(RetargetJoint {
                    source,
                    target,
                    rotation_basis: Quat::from_array(m.rotation.map(f32::from_bits)),
                    translation_basis: Quat::from_array(m.translation.map(f32::from_bits)),
                    translation_scale: f32::from_bits(m.scale),
                })
            })
            .collect::<Result<Vec<_>, AnimatorCheckpointError>>()?;
        let mut binding = RetargetBinding::new(source, target, &mapping)?;
        if let Some(reference) = &self.target_reference {
            binding = binding.with_target_reference_pose(&reference.prepare(target)?)?;
        }
        if binding
            .joints
            .iter()
            .zip(&self.mapping)
            .any(|(j, m)| j.root_basis_nonzero != m.root_nonzero)
        {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Vec3};
    fn rig(name: &str) -> Skeleton {
        Skeleton::new(vec![Joint {
            name: name.into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap()
    }
    #[test]
    fn actual_mapping_and_calibrated_reference_survive_fresh_rigs_exactly() {
        for calibrated in [false, true] {
            let source = rig("source");
            let target = rig("target");
            let basis = Quat::from_rotation_y(0.31);
            let mut binding = RetargetBinding::new(
                &source,
                &target,
                &[RetargetJoint {
                    source: "source".into(),
                    target: "target".into(),
                    rotation_basis: basis,
                    translation_basis: basis,
                    translation_scale: 0.731,
                }],
            )
            .unwrap();
            if calibrated {
                let mut reference = target.bind_pose();
                reference
                    .set_joint_rotation(0, Quat::from_rotation_z(0.27))
                    .unwrap();
                binding = binding.with_target_reference_pose(&reference).unwrap();
            }
            let saved = binding.checkpoint(65536).unwrap();
            let json = saved.to_json(65536).unwrap();
            let decoded = RetargetBindingCheckpoint::from_json(&json, 65536).unwrap();
            let restored = decoded
                .prepare(&rig("source"), &rig("target"), 65536)
                .unwrap();
            assert_eq!(
                restored.checkpoint(65536).unwrap().to_json(65536).unwrap(),
                json
            );
            for step in 0..120 {
                let mut pose = source.bind_pose();
                pose.set_joint_translation(0, Vec3::new(step as f32 * 0.01, -0.3, 0.7))
                    .unwrap();
                pose.set_joint_rotation(0, Quat::from_rotation_x(step as f32 * 0.007))
                    .unwrap();
                assert_eq!(
                    binding.apply_pose(&pose).unwrap(),
                    restored.apply_pose(&pose).unwrap()
                );
                assert_eq!(
                    binding.source_root_motion_axes(0, [true; 3]).is_ok(),
                    restored.source_root_motion_axes(0, [true; 3]).is_ok()
                );
            }
            if !calibrated {
                let path = crate::RootRigidPath::from_twists(
                    &[(
                        crate::RootRigidTwist {
                            linear: glam::DVec3::X,
                            angular: glam::DVec3::Y * 0.3,
                        },
                        0.17,
                    )],
                    16,
                )
                .unwrap();
                assert_eq!(
                    binding
                        .apply_root_path(&path, 0, [true; 3])
                        .unwrap()
                        .checkpoint_words(),
                    restored
                        .apply_root_path(&path, 0, [true; 3])
                        .unwrap()
                        .checkpoint_words()
                );
            }
            let mut corrupt = decoded.clone();
            corrupt.mapping[0].root_nonzero = Some([[false; 3]; 3]);
            assert!(corrupt.prepare(&source, &target, 65536).is_err());
            let mut corrupt = decoded.clone();
            corrupt.mapping[0].target = 99;
            assert!(corrupt.prepare(&source, &target, 65536).is_err());
            assert!(RetargetBindingCheckpoint::from_json(&json, 1).is_err());
        }
    }
}
