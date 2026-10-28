//! Staged scene, simulation, controller and input save transaction.
//! Animation, audio, UI and other application owners remain outer participants.
use crate::{
    CharacterPhysics, CharacterPhysicsCheckpoint, InputCheckpointCodecError, PhysicsError,
    decode_input_checkpoint, encode_input_checkpoint,
};
use serde::{Deserialize, Serialize};
use voxy_input::InputMap;
use voxy_scene::{
    BehaviorCheckpointRegistry, ComponentRegistry, LoadedScene, SceneCheckpointError,
    SceneSimulation, SceneSimulationCheckpoint, SimulationLimits,
};

/// Caller-selected admission policy. Limits are never taken from untrusted save data.
#[derive(Clone, Copy, Debug)]
pub struct GameplayCheckpointLimits {
    pub simulation: SimulationLimits,
    pub scene_capacity: usize,
    pub max_bodies: usize,
    pub max_colliders: usize,
    pub max_actions: usize,
    pub max_bindings: usize,
    pub max_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::{ObjectId, SceneDocument};

    fn limits() -> GameplayCheckpointLimits {
        GameplayCheckpointLimits {
            simulation: SimulationLimits {
                fixed_step: 1. / 60.,
                max_steps: 4,
                max_behaviors: 8,
                max_commands: 16,
            },
            scene_capacity: 8,
            max_bodies: 4,
            max_colliders: 4,
            max_actions: 131,
            max_bindings: 5,
            max_bytes: 1024 * 1024,
        }
    }
    fn fixture() -> (ComponentRegistry, PreparedGameplayCheckpoint) {
        let mut registry = ComponentRegistry::default();
        crate::register_components(&mut registry).unwrap();
        let doc = SceneDocument::from_json(r#"{"version":1,"objects":[
          {"id":"body","parent":null,"name":"body","active":true,"translation":[0,2,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"game.character.v1":{"half_extents":[0.1,0.1,0.1],"speed":1,"gravity":-9.8,"jump_speed":3}}},
          {"id":"floor","parent":null,"name":"floor","active":true,"translation":[0,-0.1,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"game.box.v1":{"half_extents":[10,0.1,10]}}}
        ]}"#).unwrap();
        let scene = doc.load(&registry, 8).unwrap();
        let simulation = SceneSimulation::new(&scene.graph, limits().simulation).unwrap();
        let physics = CharacterPhysics::new(&scene.graph, 4, 4);
        (
            registry,
            PreparedGameplayCheckpoint {
                scene,
                simulation,
                physics,
                input: crate::player_input().unwrap(),
                fixed_ticks: 0,
            },
        )
    }
    fn advance(game: &mut PreparedGameplayCheckpoint, dt: f64) {
        let PreparedGameplayCheckpoint {
            scene,
            simulation,
            physics,
            input,
            fixed_ticks,
        } = game;
        simulation
            .advance_with(&mut scene.graph, dt, |graph, step| {
                physics.fixed_step(graph, input, step)?;
                *fixed_ticks += 1;
                Ok::<_, PhysicsError>(())
            })
            .unwrap();
    }
    #[test]
    fn combined_json_restores_midfall_and_matches_full_landing_and_jump_cycle() {
        let (registry, mut source) = fixture();
        for _ in 0..17 {
            advance(&mut source, 0.019);
        }
        source.input.event(crate::RIGHT, 0.375).unwrap();
        source.input.event(crate::JUMP, 1.).unwrap();
        source.input.event(crate::JUMP, 0.).unwrap();
        let old = source.scene.resolve(&ObjectId("body".into())).unwrap();
        let saved = source
            .checkpoint(&registry, "project/assets/adapter-v1", limits())
            .unwrap();
        let text = saved.to_json(limits().max_bytes).unwrap();
        let decoded = GameplayCheckpoint::from_json(&text, limits().max_bytes).unwrap();
        let mut resumed = decoded
            .prepare(
                &registry,
                &BehaviorCheckpointRegistry::default(),
                "project/assets/adapter-v1",
                limits(),
            )
            .unwrap();
        assert!(resumed.scene.graph.local(old).is_err());
        assert_eq!(resumed.input.checkpoint(), source.input.checkpoint());
        let mut landed = false;
        let mut jumped = false;
        for frame in 0..240 {
            if frame == 120 {
                source.input.event(crate::JUMP, 1.).unwrap();
                resumed.input.event(crate::JUMP, 1.).unwrap();
            }
            if frame == 121 {
                source.input.event(crate::JUMP, 0.).unwrap();
                resumed.input.event(crate::JUMP, 0.).unwrap();
            }
            let dt = [0.003, 0.019, 0.031][frame % 3];
            advance(&mut source, dt);
            advance(&mut resumed, dt);
            assert_eq!(
                source.scene.capture(&registry).unwrap(),
                resumed.scene.capture(&registry).unwrap()
            );
            assert_eq!(source.input.checkpoint(), resumed.input.checkpoint());
            assert_eq!(source.fixed_ticks, resumed.fixed_ticks);
            assert_eq!(
                source
                    .simulation
                    .checkpoint(&source.scene, &registry, limits().max_bytes)
                    .unwrap()
                    .to_json(limits().max_bytes)
                    .unwrap(),
                resumed
                    .simulation
                    .checkpoint(&resumed.scene, &registry, limits().max_bytes)
                    .unwrap()
                    .to_json(limits().max_bytes)
                    .unwrap()
            );
            let a = source
                .physics
                .checkpoint(&source.scene.graph, |n| source.scene.identity(n).cloned())
                .unwrap();
            let b = resumed
                .physics
                .checkpoint(&resumed.scene.graph, |n| resumed.scene.identity(n).cloned())
                .unwrap();
            assert_eq!(a, b);
            let body = source.scene.resolve(&ObjectId("body".into())).unwrap();
            let state = source
                .physics
                .state(&source.scene.graph, body)
                .unwrap()
                .unwrap();
            landed |= state.grounded;
            jumped |= frame > 120 && state.velocity[1] > 0.;
        }
        assert!(landed && jumped);
    }
    #[test]
    fn late_physics_failure_or_compatibility_mismatch_preserves_source() {
        let (registry, mut source) = fixture();
        advance(&mut source, 0.023);
        let original = source
            .checkpoint(&registry, "compatible", limits())
            .unwrap();
        let text = original.to_json(limits().max_bytes).unwrap();
        let codecs = BehaviorCheckpointRegistry::default();
        assert!(
            original
                .prepare(&registry, &codecs, "other", limits())
                .is_err()
        );
        let mut wire: serde_json::Value = serde_json::from_str(&text).unwrap();
        wire["physics"]["bodies"][0][0] = "missing-owner".into();
        let bad = GameplayCheckpoint::from_json(&wire.to_string(), limits().max_bytes).unwrap();
        assert!(
            bad.prepare(&registry, &codecs, "compatible", limits())
                .is_err()
        );
        assert!(GameplayCheckpoint::from_json(&text, 1).is_err());
        assert_eq!(
            source
                .checkpoint(&registry, "compatible", limits())
                .unwrap()
                .to_json(limits().max_bytes)
                .unwrap(),
            text
        );
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GameplayCheckpointError {
    #[error("incompatible gameplay checkpoint version or compatibility key")]
    Compatibility,
    #[error("gameplay checkpoint exceeds byte budget")]
    Capacity,
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Scene(#[from] SceneCheckpointError),
    #[error(transparent)]
    Physics(#[from] PhysicsError),
    #[error(transparent)]
    Input(#[from] InputCheckpointCodecError),
    #[error(transparent)]
    InputState(#[from] voxy_input::InputError),
}

/// A single publishable ownership unit. Old handles cannot address its fresh scene.
#[derive(Debug)]
pub struct PreparedGameplayCheckpoint {
    pub scene: LoadedScene,
    pub simulation: SceneSimulation,
    pub physics: CharacterPhysics,
    pub input: InputMap,
    pub fixed_ticks: u64,
}

/// Compatibility must identify the project's data, assets, executable schemas and
/// physical input adapter. This type compares the supplied key; it does not derive it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameplayCheckpoint {
    version: u32,
    compatibility: String,
    scene: SceneSimulationCheckpoint,
    physics: CharacterPhysicsCheckpoint,
    input: String,
    fixed_ticks: u64,
}
impl PreparedGameplayCheckpoint {
    /// Capture every owned participant at the same accepted call boundary.
    /// # Errors
    /// Rejects unsupported scene state, unidentified bodies, invalid input and budgets.
    pub fn checkpoint(
        &self,
        registry: &ComponentRegistry,
        compatibility: &str,
        limits: GameplayCheckpointLimits,
    ) -> Result<GameplayCheckpoint, GameplayCheckpointError> {
        if compatibility.is_empty() {
            return Err(GameplayCheckpointError::Compatibility);
        }
        let saved = GameplayCheckpoint {
            version: 1,
            compatibility: compatibility.into(),
            scene: self
                .simulation
                .checkpoint(&self.scene, registry, limits.max_bytes)?,
            physics: self
                .physics
                .checkpoint(&self.scene.graph, |n| self.scene.identity(n).cloned())?,
            input: encode_input_checkpoint(
                &self.input.checkpoint(),
                limits.max_bytes,
                limits.max_actions,
                limits.max_bindings,
            )?,
            fixed_ticks: self.fixed_ticks,
        };
        saved.to_json(limits.max_bytes)?;
        Ok(saved)
    }
}
impl GameplayCheckpoint {
    /// # Errors
    /// Rejects invalid envelope metadata and oversized output.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, GameplayCheckpointError> {
        if self.version != 1 || self.compatibility.is_empty() {
            return Err(GameplayCheckpointError::Compatibility);
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(GameplayCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// The caller must also bound the underlying file read.
    /// # Errors
    /// Rejects oversized text before parsing, unknown fields and invalid envelope metadata.
    pub fn from_json(text: &str, max_bytes: usize) -> Result<Self, GameplayCheckpointError> {
        if text.len() > max_bytes {
            return Err(GameplayCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 || saved.compatibility.is_empty() {
            return Err(GameplayCheckpointError::Compatibility);
        }
        Ok(saved)
    }
    /// Prepare all owners privately. Publish the returned unit only after success.
    /// No live scene, simulation, input or controller is borrowed mutably here.
    /// # Errors
    /// Rejects compatibility differences, invalid participants and admission limits.
    pub fn prepare(
        &self,
        registry: &ComponentRegistry,
        behaviors: &BehaviorCheckpointRegistry,
        compatibility: &str,
        limits: GameplayCheckpointLimits,
    ) -> Result<PreparedGameplayCheckpoint, GameplayCheckpointError> {
        self.to_json(limits.max_bytes)?;
        if self.compatibility != compatibility {
            return Err(GameplayCheckpointError::Compatibility);
        }
        let input_state = decode_input_checkpoint(
            &self.input,
            limits.max_bytes,
            limits.max_actions,
            limits.max_bindings,
        )?;
        let mut input = InputMap::new(limits.max_actions, limits.max_bindings);
        input.restore_checkpoint(&input_state)?;
        let (scene, simulation) = self.scene.prepare(
            registry,
            behaviors,
            limits.simulation,
            limits.scene_capacity,
            limits.max_bytes,
        )?;
        let mut physics =
            CharacterPhysics::new(&scene.graph, limits.max_bodies, limits.max_colliders);
        physics.restore_checkpoint(&scene.graph, &self.physics, |id| scene.resolve(id))?;
        Ok(PreparedGameplayCheckpoint {
            scene,
            simulation,
            physics,
            input,
            fixed_ticks: self.fixed_ticks,
        })
    }
}
