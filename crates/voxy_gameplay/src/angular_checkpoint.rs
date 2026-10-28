//! Angular batch continuation policy. Pose/history belongs to the saved scene;
//! motions/staging/edits are scratch, overwritten before every fixed-step use.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AngularMotionCheckpoint {
    version: u32,
    capacity: usize,
}
impl AngularMotionBatch {
    #[must_use]
    pub fn checkpoint(&self) -> AngularMotionCheckpoint {
        AngularMotionCheckpoint {
            version: 1,
            capacity: self.capacity,
        }
    }
}
impl AngularMotionCheckpoint {
    /// Binds a fresh scratch owner to the staged scene. Descriptor admission remains
    /// at the normal reconcile boundary, including pending invalid scene edits.
    /// # Errors
    /// Rejects unsupported versions and caller capacity limits.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        max_motions: usize,
    ) -> Result<AngularMotionBatch, GameplayError> {
        if self.version != 1 {
            return Err("unsupported angular motion checkpoint".into());
        }
        if self.capacity > max_motions {
            return Err("angular motion checkpoint capacity".into());
        }
        Ok(AngularMotionBatch {
            scene: scene.identity(),
            capacity: self.capacity,
            motions: Vec::new(),
            staging: Vec::new(),
            edits: Vec::new(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::Transform;
    #[test]
    fn fresh_scratch_continues_scene_pose_and_pending_descriptor_failures_exactly() {
        let mut original_scene = SceneGraph::new(1);
        let owner = original_scene.spawn(None, Transform::default()).unwrap();
        let motion = AngularMotion {
            axis: [0., 1., 0.],
            radians_per_second: 0.73,
        };
        original_scene.insert_component(owner, motion).unwrap();
        let mut original = AngularMotionBatch::new(&original_scene, 1).unwrap();
        for _ in 0..23 {
            original.fixed_step(&mut original_scene, 1. / 60.).unwrap();
        }
        let mut fresh_scene = SceneGraph::new(2);
        fresh_scene.spawn(None, Transform::default()).unwrap();
        let fresh = fresh_scene
            .spawn(None, original_scene.local(owner).unwrap())
            .unwrap();
        // A pending invalid edit is not yet admitted by the live batch.
        let invalid = AngularMotion {
            axis: [0.; 3],
            ..motion
        };
        original_scene.insert_component(owner, invalid).unwrap();
        fresh_scene.insert_component(fresh, invalid).unwrap();
        let saved = original.checkpoint();
        let json = serde_json::to_string(&saved).unwrap();
        let saved: AngularMotionCheckpoint = serde_json::from_str(&json).unwrap();
        let mut restored = saved.prepare(&fresh_scene, 1).unwrap();
        let before = original_scene.local(owner).unwrap();
        assert!(original.fixed_step(&mut original_scene, 1. / 60.).is_err());
        assert!(restored.fixed_step(&mut fresh_scene, 1. / 60.).is_err());
        assert_eq!(original_scene.local(owner).unwrap(), before);
        assert_eq!(fresh_scene.local(fresh).unwrap(), before);
        assert!(restored.fixed_step(&mut original_scene, 1. / 60.).is_err());
        for frame in 0..120 {
            let updated = AngularMotion {
                radians_per_second: if frame < 40 { 0.73 } else { -1.19 },
                ..motion
            };
            original_scene.insert_component(owner, updated).unwrap();
            fresh_scene.insert_component(fresh, updated).unwrap();
            let active = !(60..80).contains(&frame);
            original_scene.set_active(owner, active).unwrap();
            fresh_scene.set_active(fresh, active).unwrap();
            let dt = [1. / 60., 1. / 120., 0.03][frame % 3];
            original.fixed_step(&mut original_scene, dt).unwrap();
            restored.fixed_step(&mut fresh_scene, dt).unwrap();
            assert_eq!(
                original_scene
                    .local(owner)
                    .unwrap()
                    .rotation
                    .to_array()
                    .map(f32::to_bits),
                fresh_scene
                    .local(fresh)
                    .unwrap()
                    .rotation
                    .to_array()
                    .map(f32::to_bits)
            );
        }
        assert!(saved.prepare(&fresh_scene, 0).is_err());
        let bad = AngularMotionCheckpoint {
            version: 2,
            ..saved
        };
        assert!(bad.prepare(&fresh_scene, 1).is_err());
    }
}
