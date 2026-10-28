//! Accepted presentation frames are saved independently of playback sampling.
use super::*;

/// The enclosing owner selects the verified target rig, including retarget/IK output.
/// No clock advance or pose resampling happens during restore.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimatorFrameCheckpoint {
    version: u32,
    pose: PoseCheckpoint,
    properties: Properties,
    palette: Vec<[u32; 16]>,
    root_motion: [u32; 3],
    root_motion_joint: u16,
    transition_weight: u32,
}
impl AnimatorFrame {
    /// Capture the accepted frame after every owner-side correction has committed.
    /// # Errors
    /// Rejects foreign rigs, inconsistent cached palettes, invalid metadata and byte limits.
    pub fn checkpoint(
        &self,
        skeleton: &Skeleton,
        max_bytes: usize,
    ) -> Result<AnimatorFrameCheckpoint, AnimatorCheckpointError> {
        self.pose.skin_matrices(skeleton)?;
        let saved = AnimatorFrameCheckpoint {
            version: 1,
            pose: self.pose.checkpoint(),
            properties: encode(&self.properties),
            palette: self
                .skin_matrices
                .iter()
                .map(|m| m.to_cols_array().map(f32::to_bits))
                .collect(),
            root_motion: self.root_motion.to_array().map(f32::to_bits),
            root_motion_joint: self.root_motion_joint,
            transition_weight: self.transition_weight.to_bits(),
        };
        saved.prepare(skeleton, max_bytes)?;
        Ok(saved)
    }
}
impl AnimatorFrameCheckpoint {
    /// # Errors
    /// Rejects unsupported versions and encoded byte budgets.
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
    /// The caller must bound the underlying file read.
    /// # Errors
    /// Rejects oversized text before parsing, unknown fields and unsupported versions.
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
    /// Reconstruct a separate accepted frame, retaining every stored f32 bit.
    /// The stored palette must exactly match the validated pose on this backend;
    /// no tolerance silently admits a divergent first presentation frame.
    /// # Errors
    /// Rejects invalid pose/rig/palette, metadata, property schema and byte budgets.
    pub fn prepare(
        &self,
        skeleton: &Skeleton,
        max_bytes: usize,
    ) -> Result<AnimatorFrame, AnimatorCheckpointError> {
        self.to_json(max_bytes)?;
        let pose = self.pose.prepare(skeleton)?;
        let expected = pose.skin_matrices(skeleton)?;
        if expected.len() != self.palette.len()
            || expected
                .iter()
                .zip(&self.palette)
                .any(|(m, b)| m.to_cols_array().map(f32::to_bits) != *b)
        {
            return Err(AnimatorCheckpointError::State);
        }
        let root_motion = Vec3::from_array(self.root_motion.map(f32::from_bits));
        let transition_weight = f32::from_bits(self.transition_weight);
        if !root_motion.is_finite()
            || !transition_weight.is_finite()
            || !(0. ..=1.).contains(&transition_weight)
            || usize::from(self.root_motion_joint) >= skeleton.joints.len()
        {
            return Err(AnimatorCheckpointError::State);
        }
        let skin_matrices = self
            .palette
            .iter()
            .map(|b| Mat4::from_cols_array(&b.map(f32::from_bits)))
            .collect();
        Ok(AnimatorFrame {
            pose,
            skin_matrices,
            properties: decode(&self.properties)?,
            root_motion,
            root_motion_joint: self.root_motion_joint,
            transition_weight,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rig() -> Skeleton {
        Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            bind_local: Transform {
                translation: Vec3::new(0.3, -0.2, 0.1),
                rotation: Quat::from_rotation_y(0.3),
                scale: Vec3::new(-1.2, 0.8, 1.1),
            },
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap()
    }
    fn frame(skeleton: &Skeleton) -> AnimatorFrame {
        let mut pose = skeleton.bind_pose();
        pose.set_joint_translation(0, Vec3::new(0.7, 0.1, -0.4))
            .unwrap();
        let skin_matrices = pose.skin_matrices(skeleton).unwrap();
        AnimatorFrame {
            pose,
            skin_matrices,
            properties: PropertySet::new(vec![
                ("morph".into(), PropertyValue::Float(f32::from_bits(1))),
                ("signed_zero".into(), PropertyValue::Float(-0.)),
                ("color".into(), PropertyValue::Color([0.1, 0.2, 0.3, 0.4])),
                (
                    "vector".into(),
                    PropertyValue::Vec3(Vec3::new(0.1, 0.7, -0.3)),
                ),
                ("flag".into(), PropertyValue::Bool(true)),
            ])
            .unwrap(),
            root_motion: Vec3::new(0.03, -0., 0.01),
            root_motion_joint: 0,
            transition_weight: 0.731,
        }
    }
    #[test]
    fn accepted_pose_palette_properties_and_metadata_roundtrip_exactly() {
        let skeleton = rig();
        let original = frame(&skeleton);
        let saved = original.checkpoint(&skeleton, 65536).unwrap();
        let json = saved.to_json(65536).unwrap();
        let parsed = AnimatorFrameCheckpoint::from_json(&json, 65536).unwrap();
        let restored = parsed.prepare(&rig(), 65536).unwrap();
        assert_eq!(
            restored
                .checkpoint(&rig(), 65536)
                .unwrap()
                .to_json(65536)
                .unwrap(),
            json
        );
        assert_eq!(original.pose, restored.pose);
        assert_eq!(
            original.root_motion.to_array().map(f32::to_bits),
            restored.root_motion.to_array().map(f32::to_bits)
        );
        assert_eq!(encode(&original.properties), encode(&restored.properties));
    }
    #[test]
    fn corrupt_cached_palette_or_metadata_is_rejected_without_replacing_frame() {
        let skeleton = rig();
        let original = frame(&skeleton);
        let saved = original.checkpoint(&skeleton, 65536).unwrap();
        let json = saved.to_json(65536).unwrap();
        let mut bad = saved.clone();
        bad.palette[0][12] ^= 1;
        assert!(bad.prepare(&skeleton, 65536).is_err());
        let mut bad = saved.clone();
        bad.root_motion[0] = f32::NAN.to_bits();
        assert!(bad.prepare(&skeleton, 65536).is_err());
        let mut bad = saved.clone();
        bad.transition_weight = 1.1f32.to_bits();
        assert!(bad.prepare(&skeleton, 65536).is_err());
        let mut bad = saved.clone();
        bad.root_motion_joint = u16::MAX;
        assert!(bad.prepare(&skeleton, 65536).is_err());
        assert!(AnimatorFrameCheckpoint::from_json(&json, 1).is_err());
        let mut corrupt_live = original.clone();
        corrupt_live.skin_matrices[0].w_axis.x += 0.1;
        assert!(corrupt_live.checkpoint(&skeleton, 65536).is_err());
        assert_eq!(
            original
                .checkpoint(&skeleton, 65536)
                .unwrap()
                .to_json(65536)
                .unwrap(),
            json
        );
    }
}
