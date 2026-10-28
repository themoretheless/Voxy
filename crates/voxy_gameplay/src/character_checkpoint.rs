//! Durable controller checkpoint. Scene data and other runtime owners are saved separately.
use super::*;
use voxy_scene::ObjectId;

/// Versioned accepted controller state, addressed by durable scene identities.
/// Float bit patterns avoid loss of solver precision through JSON number parsing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CharacterPhysicsCheckpoint {
    version: u32,
    depenetration: bool,
    angular_iterations: usize,
    trajectory_queries: usize,
    bodies: Vec<(ObjectId, BodyWire)>,
}
/// Bounded checkpoint codec errors. Decoding never changes a live controller.
#[derive(Debug, thiserror::Error)]
pub enum CharacterCheckpointError {
    #[error("controller checkpoint exceeds its byte budget")]
    TooLarge,
    #[error("invalid controller checkpoint encoding: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("invalid controller checkpoint state: {0}")]
    State(#[from] PhysicsError),
}
impl CharacterPhysicsCheckpoint {
    fn validate(&self, max_bodies: usize) -> Result<(), PhysicsError> {
        if self.version != 1
            || self.bodies.len() > max_bodies
            || !(1..=4096).contains(&self.angular_iterations)
            || !(1..=1_048_576).contains(&self.trajectory_queries)
        {
            return Err(PhysicsError::Capacity);
        }
        let mut identities = std::collections::HashSet::new();
        for (id, wire) in &self.bodies {
            if id.0.is_empty() || !identities.insert(id) {
                return Err(PhysicsError::InvalidBody);
            }
            wire.decode()?;
        }
        Ok(())
    }
    /// Parse a byte-bounded JSON record and admit every cached physical state.
    /// The outer save system must also bound its file read before creating text.
    /// # Errors
    /// Rejects byte/body budgets, unknown fields/versions and corrupt body data.
    pub fn from_json(
        text: &str,
        max_bytes: usize,
        max_bodies: usize,
    ) -> Result<Self, CharacterCheckpointError> {
        if text.len() > max_bytes {
            return Err(CharacterCheckpointError::TooLarge);
        }
        let saved: Self = serde_json::from_str(text)?;
        saved.validate(max_bodies)?;
        Ok(saved)
    }
    /// Encode exact bit representations; never publish an oversized record.
    /// # Errors
    /// Rejects invalid state, encoding failure or an exceeded byte budget.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, CharacterCheckpointError> {
        self.validate(self.bodies.len())?;
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(CharacterCheckpointError::TooLarge);
        }
        Ok(text)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct BodyWire {
    anchor: [i64; 3],
    min: [u64; 3],
    max: [u64; 3],
    velocity: [u64; 3],
    grounded: bool,
    published: [u32; 3],
    extents: [u32; 3],
    speed: u64,
    gravity: u64,
    jump: u64,
    edges: [[u64; 3]; 3],
    rest_edges: [[u64; 3]; 3],
    orientation: [u64; 4],
    rotation: [u32; 4],
}
impl From<RuntimeBody> for BodyWire {
    fn from(body: RuntimeBody) -> Self {
        Self {
            anchor: [
                body.state.body.anchor.x,
                body.state.body.anchor.y,
                body.state.body.anchor.z,
            ],
            min: body.state.body.min.map(f64::to_bits),
            max: body.state.body.max.map(f64::to_bits),
            velocity: body.state.velocity.map(f64::to_bits),
            grounded: body.state.grounded,
            published: body.published.to_array().map(f32::to_bits),
            extents: body.descriptor.half_extents.map(f32::to_bits),
            speed: body.descriptor.speed.to_bits(),
            gravity: body.descriptor.gravity.to_bits(),
            jump: body.descriptor.jump_speed.to_bits(),
            edges: body.edges.map(|v| v.to_array().map(f64::to_bits)),
            rest_edges: body.rest_edges.map(|v| v.to_array().map(f64::to_bits)),
            orientation: body.orientation.to_array().map(f64::to_bits),
            rotation: body.published_rotation.to_array().map(f32::to_bits),
        }
    }
}
impl BodyWire {
    fn decode(&self) -> Result<RuntimeBody, PhysicsError> {
        let min = self.min.map(f64::from_bits);
        let max = self.max.map(f64::from_bits);
        let velocity = self.velocity.map(f64::from_bits);
        let descriptor = CharacterBody {
            half_extents: self.extents.map(f32::from_bits),
            speed: f64::from_bits(self.speed),
            gravity: f64::from_bits(self.gravity),
            jump_speed: f64::from_bits(self.jump),
        };
        validate_body(descriptor)?;
        let edges = self
            .edges
            .map(|v| glam::DVec3::from_array(v.map(f64::from_bits)));
        let rest_edges = self
            .rest_edges
            .map(|v| glam::DVec3::from_array(v.map(f64::from_bits)));
        let orientation = glam::DQuat::from_array(self.orientation.map(f64::from_bits));
        let rotation = Quat::from_array(self.rotation.map(f32::from_bits));
        let published = Vec3::from_array(self.published.map(f32::from_bits));
        if min
            .iter()
            .chain(&max)
            .chain(&velocity)
            .any(|v| !v.is_finite())
            || (0..3).any(|i| min[i] >= max[i])
            || !published.is_finite()
            || !orientation.is_finite()
            || (orientation.length_squared() - 1.).abs() > 1e-10
            || !rotation.is_finite()
            || (rotation.length_squared() - 1.).abs() > 1e-3
            || edges
                .iter()
                .chain(&rest_edges)
                .any(|v| !v.is_finite() || v.length_squared() == 0.)
        {
            return Err(PhysicsError::InvalidBody);
        }
        if !glam::DMat3::from_cols(edges[0], edges[1], edges[2])
            .inverse()
            .is_finite()
            || !glam::DMat3::from_cols(rest_edges[0], rest_edges[1], rest_edges[2])
                .inverse()
                .is_finite()
        {
            return Err(PhysicsError::InvalidBody);
        }
        let body = AnchoredAabb {
            anchor: Origin {
                x: self.anchor[0],
                y: self.anchor[1],
                z: self.anchor[2],
            },
            min,
            max,
        };
        center_of(body)?;
        let half = edges.iter().map(|v| v.abs()).sum::<glam::DVec3>();
        for i in 0..3 {
            let tolerance =
                32. * f64::EPSILON * min[i].abs().max(max[i].abs()).max(half[i]).max(1.);
            if ((max[i] - min[i]) * 0.5 - half[i]).abs() > tolerance {
                return Err(PhysicsError::InvalidBody);
            }
        }
        Ok(RuntimeBody {
            state: CharacterState {
                body,
                velocity,
                grounded: self.grounded,
            },
            published,
            descriptor,
            edges,
            rest_edges,
            orientation,
            published_rotation: rotation,
        })
    }
}
impl CharacterPhysics {
    /// Capture all cached controller states, including inactive owners. The caller
    /// must capture matching scene data at the same accepted tick boundary.
    /// # Errors
    /// Rejects foreign scenes and missing/duplicate durable identities.
    pub fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<CharacterPhysicsCheckpoint, PhysicsError> {
        self.validate_scene(scene)?;
        let mut bodies = Vec::with_capacity(self.states.len());
        for (&owner, &body) in &self.states {
            scene.local(owner)?;
            let id = identify(owner).ok_or(PhysicsError::InvalidBody)?;
            if id.0.is_empty() {
                return Err(PhysicsError::InvalidBody);
            }
            bodies.push((id, BodyWire::from(body)));
        }
        bodies.sort_by(|a, b| a.0.0.cmp(&b.0.0));
        if bodies.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(PhysicsError::InvalidBody);
        }
        Ok(CharacterPhysicsCheckpoint {
            version: 1,
            depenetration: self.depenetration,
            angular_iterations: self.angular_iterations,
            trajectory_queries: self.trajectory_queries,
            bodies,
        })
    }
    /// Restore into a matching scene after resolving durable IDs to its NEW handles.
    /// The caller owns restoration of scene transforms, input, clocks and other
    /// runtime systems. This method replaces no scene data and commits atomically.
    /// # Errors
    /// Rejects versions, budgets, corrupt states, unresolved/duplicate bindings,
    /// missing character components and invalid target scene descriptors.
    pub fn restore_checkpoint(
        &mut self,
        scene: &SceneGraph,
        saved: &CharacterPhysicsCheckpoint,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    ) -> Result<(), PhysicsError> {
        self.validate(scene)?;
        saved.validate(self.max_bodies)?;
        let mut candidate = HashMap::with_capacity(saved.bodies.len());
        let mut identities = std::collections::HashSet::new();
        for (id, wire) in &saved.bodies {
            if id.0.is_empty() || !identities.insert(id.clone()) {
                return Err(PhysicsError::InvalidBody);
            }
            let owner = resolve(id).ok_or(PhysicsError::InvalidBody)?;
            if scene.component::<CharacterBody>(owner)?.is_none() {
                return Err(PhysicsError::InvalidBody);
            }
            if candidate.insert(owner, wire.decode()?).is_some() {
                return Err(PhysicsError::InvalidBody);
            }
        }
        self.states = candidate;
        self.depenetration = saved.depenetration;
        self.angular_iterations = saved.angular_iterations;
        self.trajectory_queries = saved.trajectory_queries;
        self.validated_structure = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::{ComponentRegistry, SceneDocument};
    fn fixture() -> (ComponentRegistry, voxy_scene::LoadedScene) {
        let mut registry = ComponentRegistry::default();
        register_components(&mut registry).unwrap();
        let document = SceneDocument::from_json(r#"{"version":1,"objects":[
          {"id":"body","parent":null,"name":"body","active":true,"translation":[0,2,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"game.character.v1":{"half_extents":[0.1,0.1,0.1],"speed":1,"gravity":-9.8,"jump_speed":3}}},
          {"id":"floor","parent":null,"name":"floor","active":true,"translation":[0,-0.1,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"game.box.v1":{"half_extents":[10,0.1,10]}}}
        ]}"#).unwrap();
        let scene = document.load(&registry, 8).unwrap();
        (registry, scene)
    }
    #[test]
    fn controller_checkpoint_json_rebinds_new_scene_and_matches_full_fall_cycle() {
        let (registry, mut original) = fixture();
        let mut physics = CharacterPhysics::new(&original.graph, 4, 4);
        let mut input = player_input().unwrap();
        for _ in 0..17 {
            physics
                .fixed_step(&mut original.graph, &mut input, 1. / 60.)
                .unwrap();
        }
        let saved = physics
            .checkpoint(&original.graph, |node| original.identity(node).cloned())
            .unwrap();
        let encoded = saved.to_json(1024 * 1024).unwrap();
        let saved = CharacterPhysicsCheckpoint::from_json(&encoded, 1024 * 1024, 4).unwrap();
        let mut document = original.capture(&registry).unwrap();
        document.objects.reverse();
        let mut resumed = document.load(&registry, 8).unwrap();
        let mut restored = CharacterPhysics::new(&resumed.graph, 4, 4);
        restored
            .restore_checkpoint(&resumed.graph, &saved, |id| resumed.resolve(id))
            .unwrap();
        assert_eq!(
            restored
                .checkpoint(&resumed.graph, |node| resumed.identity(node).cloned())
                .unwrap(),
            saved
        );
        let mut resumed_input = player_input().unwrap();
        let mut landed = false;
        for _ in 0..180 {
            physics
                .fixed_step(&mut original.graph, &mut input, 1. / 60.)
                .unwrap();
            restored
                .fixed_step(&mut resumed.graph, &mut resumed_input, 1. / 60.)
                .unwrap();
            assert_eq!(
                physics
                    .checkpoint(&original.graph, |node| original.identity(node).cloned())
                    .unwrap(),
                restored
                    .checkpoint(&resumed.graph, |node| resumed.identity(node).cloned())
                    .unwrap()
            );
            let a = original.capture(&registry).unwrap();
            let b = resumed.capture(&registry).unwrap();
            for object in &a.objects {
                assert_eq!(
                    Some(object),
                    b.objects.iter().find(|other| other.id == object.id)
                );
            }
            landed |= physics
                .state(
                    &original.graph,
                    original.resolve(&ObjectId("body".into())).unwrap(),
                )
                .unwrap()
                .unwrap()
                .grounded;
        }
        assert!(landed);
    }
    #[test]
    fn bounded_checkpoint_codec_rejects_limits_unknown_schema_and_corrupt_bits() {
        let (_, mut scene) = fixture();
        let mut physics = CharacterPhysics::new(&scene.graph, 4, 4);
        physics
            .fixed_step(&mut scene.graph, &mut player_input().unwrap(), 1. / 60.)
            .unwrap();
        let saved = physics
            .checkpoint(&scene.graph, |node| scene.identity(node).cloned())
            .unwrap();
        let text = saved.to_json(1024 * 1024).unwrap();
        assert_eq!(
            CharacterPhysicsCheckpoint::from_json(&text, text.len(), 1).unwrap(),
            saved
        );
        assert!(matches!(
            CharacterPhysicsCheckpoint::from_json(&text, text.len() - 1, 1),
            Err(CharacterCheckpointError::TooLarge)
        ));
        assert!(saved.to_json(text.len() - 1).is_err());
        assert!(CharacterPhysicsCheckpoint::from_json(&text, text.len(), 0).is_err());
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["unknown"] = true.into();
        assert!(CharacterPhysicsCheckpoint::from_json(&value.to_string(), 1024 * 1024, 1).is_err());
        let mut corrupt = saved.clone();
        corrupt.bodies[0].1.velocity[0] = f64::INFINITY.to_bits();
        let invalid = serde_json::to_string(&corrupt).unwrap();
        assert!(CharacterPhysicsCheckpoint::from_json(&invalid, 1024 * 1024, 1).is_err());
        assert!(corrupt.to_json(1024 * 1024).is_err());
    }

    #[test]
    fn corrupt_checkpoint_and_ambiguous_bindings_preserve_live_controller() {
        let (_, mut loaded) = fixture();
        let mut physics = CharacterPhysics::new(&loaded.graph, 4, 4);
        physics
            .fixed_step(&mut loaded.graph, &mut player_input().unwrap(), 1. / 60.)
            .unwrap();
        let owner = loaded.resolve(&ObjectId("body".into())).unwrap();
        physics.states.get_mut(&owner).unwrap().state.velocity[0] =
            f64::from_bits(0x3ff0000000000001);
        let saved = physics
            .checkpoint(&loaded.graph, |node| loaded.identity(node).cloned())
            .unwrap();
        let encoded = serde_json::to_string(&saved).unwrap();
        assert_eq!(
            serde_json::from_str::<CharacterPhysicsCheckpoint>(&encoded).unwrap(),
            saved
        );
        let mut invalid = saved.clone();
        invalid.version = 2;
        let mut nonfinite = saved.clone();
        nonfinite.bodies[0].1.velocity[0] = f64::NAN.to_bits();
        let mut duplicate = saved.clone();
        duplicate.bodies.push(duplicate.bodies[0].clone());
        for bad in [invalid, nonfinite, duplicate] {
            assert!(
                physics
                    .restore_checkpoint(&loaded.graph, &bad, |id| loaded.resolve(id))
                    .is_err()
            );
            assert_eq!(
                physics
                    .checkpoint(&loaded.graph, |node| loaded.identity(node).cloned())
                    .unwrap(),
                saved
            );
        }
        assert!(
            physics
                .restore_checkpoint(&loaded.graph, &saved, |_| None)
                .is_err()
        );
        assert_eq!(
            physics
                .checkpoint(&loaded.graph, |node| loaded.identity(node).cloned())
                .unwrap(),
            saved
        );
    }
}
