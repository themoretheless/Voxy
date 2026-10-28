//! Contact hysteresis and support identity for an enclosing animation/game transaction.
use super::*;
use crate::SupportAnchorCheckpoint;
use voxy_scene::{NodeId, ObjectId, SceneGraph};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FootContactCheckpoint {
    version: u32,
    anchor: Option<SupportAnchorCheckpoint>,
    awaiting_swing: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum FootContactCheckpointError {
    #[error("foot contact checkpoint exceeds byte budget")]
    Capacity,
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Physics(#[from] PhysicsError),
}
impl FootContactState {
    /// # Errors
    /// Rejects unresolved, deleted or foreign support identities.
    pub fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<FootContactCheckpoint, PhysicsError> {
        Ok(FootContactCheckpoint {
            version: 1,
            anchor: self
                .anchor
                .as_ref()
                .map(|a| a.checkpoint(scene, &identify))
                .transpose()?,
            awaiting_swing: self.awaiting_swing,
        })
    }
}
impl FootContactCheckpoint {
    /// # Errors
    /// Rejects unsupported versions, inconsistent contact modes and byte limits.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, FootContactCheckpointError> {
        if self.version != 1 || self.anchor.is_some() && self.awaiting_swing {
            return Err(PhysicsError::InvalidBody.into());
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(FootContactCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// Bound the file read before calling this parser.
    /// # Errors
    /// Rejects oversized input before parsing, unknown fields and invalid contact modes.
    pub fn from_json(text: &str, max_bytes: usize) -> Result<Self, FootContactCheckpointError> {
        if text.len() > max_bytes {
            return Err(FootContactCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 || saved.anchor.is_some() && saved.awaiting_swing {
            return Err(PhysicsError::InvalidBody.into());
        }
        Ok(saved)
    }
    /// Rebuild the state privately; anchor geometry is checked by the normal next tick.
    /// # Errors
    /// Rejects invalid state, unresolved support identities and foreign handles.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    ) -> Result<FootContactState, PhysicsError> {
        if self.version != 1 || self.anchor.is_some() && self.awaiting_swing {
            return Err(PhysicsError::InvalidBody);
        }
        Ok(FootContactState {
            anchor: self
                .anchor
                .as_ref()
                .map(|a| a.prepare(scene, &resolve))
                .transpose()?,
            awaiting_swing: self.awaiting_swing,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DVec3, Vec3};
    use voxy_scene::{ComponentRegistry, SceneDocument, Transform};
    fn fixture() -> (ComponentRegistry, voxy_scene::LoadedScene) {
        let mut registry = ComponentRegistry::default();
        crate::register_components(&mut registry).unwrap();
        let doc = SceneDocument::from_json(r#"{"version":1,"objects":[{"id":"platform","parent":null,"name":"platform","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"game.box.v1":{"half_extents":[2,0.1,2]}}}]}"#).unwrap();
        let scene = doc.load(&registry, 8).unwrap();
        (registry, scene)
    }
    fn input() -> FootContactInput {
        FootContactInput {
            sole: DVec3::new(0.3, f64::from(0.1f32), 0.2),
            up: DVec3::Y,
            grounded: true,
            plant: true,
        }
    }
    fn budget() -> SupportQueryBudget {
        SupportQueryBudget::new(64).unwrap()
    }
    fn checkpoint(
        state: FootContactState,
        scene: &voxy_scene::LoadedScene,
    ) -> FootContactCheckpoint {
        state
            .checkpoint(&scene.graph, |n| scene.identity(n).cloned())
            .unwrap()
    }
    #[test]
    fn planted_contact_rebinds_and_follows_moving_platform_and_swing_exactly() {
        let (registry, mut source) = fixture();
        let settings = FootContactSettings::default();
        let world = SupportWorld::from_scene(&source.graph, 4).unwrap();
        let mut original = FootContactState::default()
            .prepare(&world, settings, input(), &mut budget())
            .unwrap()
            .state;
        assert!(original.anchor().is_some());
        let saved = checkpoint(original, &source);
        let json = saved.to_json(65536).unwrap();
        let saved = FootContactCheckpoint::from_json(&json, 65536).unwrap();
        let mut target = source
            .capture(&registry)
            .unwrap()
            .load(&registry, 8)
            .unwrap();
        let mut resumed = saved
            .prepare(&target.graph, |id| target.resolve(id))
            .unwrap();
        let source_id = source.resolve(&ObjectId("platform".into())).unwrap();
        let target_id = target.resolve(&ObjectId("platform".into())).unwrap();
        assert_ne!(source_id, target_id);
        assert!(
            SupportWorld::from_scene(&target.graph, 4)
                .unwrap()
                .resolve(original.anchor().unwrap(), &mut budget())
                .is_err()
        );
        for frame in 0..180 {
            let x = (frame % 20) as f32 * 0.001;
            let transform = Transform {
                translation: Vec3::new(x, 0., 0.),
                ..Default::default()
            };
            source.graph.set_local(source_id, transform).unwrap();
            target.graph.set_local(target_id, transform).unwrap();
            let mut request = input();
            request.sole.x += f64::from(x);
            request.plant = !(40..50).contains(&frame);
            request.grounded = !(90..100).contains(&frame);
            let a = original
                .prepare(
                    &SupportWorld::from_scene(&source.graph, 4).unwrap(),
                    settings,
                    request,
                    &mut budget(),
                )
                .unwrap();
            let b = resumed
                .prepare(
                    &SupportWorld::from_scene(&target.graph, 4).unwrap(),
                    settings,
                    request,
                    &mut budget(),
                )
                .unwrap();
            assert_eq!(a.status, b.status);
            assert_eq!(
                a.contact.map(|c| (
                    c.position.to_array().map(f64::to_bits),
                    c.normal.to_array().map(f64::to_bits)
                )),
                b.contact.map(|c| (
                    c.position.to_array().map(f64::to_bits),
                    c.normal.to_array().map(f64::to_bits)
                ))
            );
            original = a.state;
            resumed = b.state;
            assert_eq!(checkpoint(original, &source), checkpoint(resumed, &target));
        }
    }
    #[test]
    fn awaiting_swing_and_corrupt_anchor_preserve_contact_policy() {
        let (_, source) = fixture();
        let world = SupportWorld::from_scene(&source.graph, 4).unwrap();
        let state = FootContactState::default()
            .prepare(
                &world,
                FootContactSettings::default(),
                input(),
                &mut budget(),
            )
            .unwrap()
            .state;
        let saved = checkpoint(state, &source);
        let json = saved.to_json(65536).unwrap();
        for field in ["face", "sign", "local_position"] {
            let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
            match field {
                "face" => value["anchor"][field] = 99.into(),
                "sign" => value["anchor"][field] = f64::NAN.to_bits().into(),
                _ => value["anchor"][field][0] = f64::NAN.to_bits().into(),
            }
            let corrupt = FootContactCheckpoint::from_json(&value.to_string(), 65536).unwrap();
            assert!(
                corrupt
                    .prepare(&source.graph, |id| source.resolve(id))
                    .is_err()
            );
        }
        assert!(saved.prepare(&source.graph, |_| None).is_err());
        assert!(FootContactCheckpoint::from_json(&json, 1).is_err());
        assert!(saved.to_json(1).is_err());
        assert_eq!(checkpoint(state, &source), saved);
        let held = state.release_until_swing();
        let restored = checkpoint(held, &source)
            .prepare(&source.graph, |id| source.resolve(id))
            .unwrap();
        let a = restored
            .prepare(
                &world,
                FootContactSettings::default(),
                input(),
                &mut budget(),
            )
            .unwrap();
        assert_eq!(a.status, FootContactStatus::AwaitingSwing);
        let mut swing = input();
        swing.plant = false;
        let swing = a
            .state
            .prepare(&world, FootContactSettings::default(), swing, &mut budget())
            .unwrap();
        let next = swing
            .state
            .prepare(
                &world,
                FootContactSettings::default(),
                input(),
                &mut budget(),
            )
            .unwrap();
        assert_eq!(next.status, FootContactStatus::Planted);
    }
}
