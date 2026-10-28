//! One file for the complete scene/simulation owner, not all gameplay subsystems.
use super::*;
use crate::{
    BehaviorCheckpoint, BehaviorCheckpointError, BehaviorCheckpointRegistry,
    CommandCheckpointError, ComponentRegistry, DocumentError, LoadedScene,
    SceneAllocationCheckpoint, SceneCommandsCheckpoint, SceneDocument, SceneFileError,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum SceneCheckpointError {
    #[error("invalid scene checkpoint version/configuration")]
    Configuration,
    #[error("scene checkpoint exceeds its byte budget")]
    Capacity,
    #[error(transparent)]
    Simulation(#[from] SimulationError),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error(transparent)]
    Behavior(#[from] BehaviorCheckpointError),
    #[error(transparent)]
    Commands(#[from] CommandCheckpointError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    File(#[from] SceneFileError),
}
/// Scene data plus every SceneSimulation-owned participant. Physics, gameplay
/// input, animation, audio and external tick counters belong to the enclosing game.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneSimulationCheckpoint {
    version: u32,
    scene: SceneDocument,
    allocation: SceneAllocationCheckpoint,
    presentation: SimulationPresentationCheckpoint,
    behaviors: BehaviorCheckpoint,
    commands: SceneCommandsCheckpoint,
    max_behaviors: usize,
}
impl SceneSimulation {
    /// Capture matching scene and simulation state at an accepted call boundary.
    /// # Errors
    /// Rejects unsupported components/behaviors, missing IDs and byte limits.
    pub fn checkpoint(
        &self,
        scene: &LoadedScene,
        registry: &ComponentRegistry,
        max_bytes: usize,
    ) -> Result<SceneSimulationCheckpoint, SceneCheckpointError> {
        self.checkpoint_view(&scene.checkpoint_view(), registry, max_bytes)
    }
    /// Captures a host-owned graph through a validated borrowed identity view.
    pub fn checkpoint_view(
        &self,
        scene: &crate::SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        max_bytes: usize,
    ) -> Result<SceneSimulationCheckpoint, SceneCheckpointError> {
        self.validate(scene.graph)?;
        let saved = SceneSimulationCheckpoint {
            version: 1,
            scene: scene.capture(registry)?,
            allocation: scene.allocation_checkpoint(max_bytes)?,
            presentation: self
                .presentation_checkpoint(&scene.graph, |node| scene.identity(node).cloned())?,
            behaviors: self.behavior_checkpoint(
                &scene.graph,
                |node| scene.identity(node).cloned(),
                max_bytes,
            )?,
            commands: self.commands.checkpoint(
                &scene.graph,
                registry,
                |node| scene.identity(node).cloned(),
                max_bytes,
            )?,
            max_behaviors: self.max_behaviors,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
impl SceneSimulationCheckpoint {
    /// # Errors
    /// Rejects unknown versions and byte budgets before returning encoded data.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, SceneCheckpointError> {
        if self.version != 1 {
            return Err(SceneCheckpointError::Configuration);
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(SceneCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// # Errors
    /// Bounds bytes before parsing; complete compatibility validation happens in prepare.
    pub fn from_json(text: &str, max_bytes: usize) -> Result<Self, SceneCheckpointError> {
        if text.len() > max_bytes {
            return Err(SceneCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 {
            return Err(SceneCheckpointError::Configuration);
        }
        Ok(saved)
    }
    /// Build one privately staged scene/simulation pair. Publish the pair only
    /// after success; the existing live pair is never borrowed mutably here.
    /// `scene_capacity` is an admission ceiling; the archived arena capacity is
    /// restored exactly, including future allocation and exhaustion behavior.
    /// # Errors
    /// Rejects versions/configuration, corrupt scene/history/commands/behaviors,
    /// unsupported codecs and budgets before returning a candidate.
    pub fn prepare(
        &self,
        registry: &ComponentRegistry,
        behaviors: &BehaviorCheckpointRegistry,
        limits: SimulationLimits,
        scene_capacity: usize,
        max_bytes: usize,
    ) -> Result<(LoadedScene, SceneSimulation), SceneCheckpointError> {
        self.to_json(max_bytes)?;
        if self.max_behaviors != limits.max_behaviors {
            return Err(SceneCheckpointError::Configuration);
        }
        // The caller supplies an admission ceiling. Preserve the saved arena
        // capacity so subsequent spawn/full behavior remains identical.
        let saved_capacity = self.allocation.capacity();
        if saved_capacity > scene_capacity {
            return Err(SceneCheckpointError::Capacity);
        }
        let mut scene = self.scene.load(registry, saved_capacity)?;
        scene.restore_staged_allocation(&self.allocation, max_bytes)?;
        let mut simulation = SceneSimulation::new(&scene.graph, limits)?;
        simulation.restore_presentation_checkpoint(&scene.graph, &self.presentation, |id| {
            scene.resolve(id)
        })?;
        simulation.restore_behavior_checkpoint(
            &scene.graph,
            &self.behaviors,
            behaviors,
            |id| scene.resolve(id),
            max_bytes,
        )?;
        simulation.commands.restore_checkpoint(
            &scene.graph,
            registry,
            &self.commands,
            |id| scene.resolve(id),
            max_bytes,
        )?;
        Ok((scene, simulation))
    }
    /// Validate the entire candidate, then reuse the existing same-directory
    /// atomic scene writer. Post-rename durability errors retain committed=true.
    /// # Errors
    /// Invalid candidates or pre-publication IO errors retain the previous file.
    pub fn save_file(
        &self,
        path: &Path,
        registry: &ComponentRegistry,
        behaviors: &BehaviorCheckpointRegistry,
        limits: SimulationLimits,
        scene_capacity: usize,
        max_bytes: usize,
    ) -> Result<(), SceneCheckpointError> {
        let _candidate = self.prepare(registry, behaviors, limits, scene_capacity, max_bytes)?;
        let text = self.to_json(max_bytes)?;
        crate::file::write_text(path, &text)?;
        Ok(())
    }
    /// Read at most max_bytes+1 bytes and fully prepare a separate scene/simulation.
    /// # Errors
    /// Reports bounded IO, encoding, compatibility and participant errors.
    pub fn load_file(
        path: &Path,
        registry: &ComponentRegistry,
        behaviors: &BehaviorCheckpointRegistry,
        limits: SimulationLimits,
        scene_capacity: usize,
        max_bytes: usize,
    ) -> Result<(LoadedScene, SceneSimulation), SceneCheckpointError> {
        let text = crate::file::read_text(path, max_bytes)?;
        Self::from_json(&text, max_bytes)?.prepare(
            registry,
            behaviors,
            limits,
            scene_capacity,
            max_bytes,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Behavior, BehaviorState, NodeId, ObjectId, SceneGraph, Transform};
    use std::sync::atomic::{AtomicU64, Ordering};
    const BYTES: usize = 1024 * 1024;
    #[derive(Debug, Serialize, Deserialize)]
    struct Mover {
        awake: u32,
        start: u32,
        ticks: u64,
    }
    impl Behavior for Mover {
        fn checkpoint(&self) -> Result<BehaviorState, BehaviorCheckpointError> {
            BehaviorState::json("test.mover.v1", self)
        }
        fn awake(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.awake += 1;
        }
        fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.start += 1;
        }
        fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, dt: f64) {
            self.ticks += 1;
            let mut local = scene.local(owner).unwrap();
            local.translation.x += dt as f32;
            scene.set_local(owner, local).unwrap();
        }
    }
    fn limits() -> SimulationLimits {
        SimulationLimits {
            fixed_step: 1. / 60.,
            max_steps: 4,
            max_behaviors: 4,
            max_commands: 16,
        }
    }
    fn fixture() -> (
        ComponentRegistry,
        BehaviorCheckpointRegistry,
        LoadedScene,
        SceneSimulation,
    ) {
        let mut components = ComponentRegistry::default();
        components.register::<u32>("test.score.v1").unwrap();
        let mut behaviors = BehaviorCheckpointRegistry::default();
        behaviors.register::<Mover>("test.mover.v1").unwrap();
        let object = |id: &str, parent: Option<&str>| crate::SceneObject {
            id: ObjectId(id.into()),
            parent: parent.map(|p| ObjectId(p.into())),
            name: id.into(),
            active: true,
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
            components: Default::default(),
        };
        let doc = SceneDocument {
            version: 1,
            objects: vec![
                object("z-root", None),
                object("dead", None),
                object("a-child", Some("z-root")),
                object("y-second", Some("z-root")),
                object("spare", None),
            ],
        };
        let mut scene = doc.load(&components, 8).unwrap();
        let root = scene.resolve(&ObjectId("z-root".into())).unwrap();
        let child = scene.resolve(&ObjectId("a-child".into())).unwrap();
        scene
            .graph
            .remove_subtree(scene.resolve(&ObjectId("dead".into())).unwrap())
            .unwrap();
        scene
            .graph
            .remove_subtree(scene.resolve(&ObjectId("spare".into())).unwrap())
            .unwrap();
        scene.graph.reparent(child, None).unwrap();
        scene.graph.reparent(child, Some(root)).unwrap();
        scene.graph.insert_component(root, 17u32).unwrap();
        let mut sim = SceneSimulation::new(&scene.graph, limits()).unwrap();
        sim.attach(
            &mut scene.graph,
            root,
            Mover {
                awake: 0,
                start: 0,
                ticks: 0,
            },
        )
        .unwrap();
        sim.advance(&mut scene.graph, 0.023).unwrap();
        sim.commands().insert_component(root, 999u32).unwrap();
        sim.commands().set_name(child, "queued".into()).unwrap();
        sim.commands()
            .push(crate::SceneCommand::Spawn(None, Transform::default()))
            .unwrap();
        sim.commands()
            .push(crate::SceneCommand::Spawn(None, Transform::default()))
            .unwrap();
        let mut invalid = Transform::default();
        invalid.translation.x = f32::NAN;
        sim.commands()
            .push(crate::SceneCommand::SetLocal(child, invalid))
            .unwrap();
        (components, behaviors, scene, sim)
    }
    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "voxy-scene-session-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn identities(scene: &LoadedScene) -> Vec<ObjectId> {
        scene
            .graph
            .nodes()
            .map(|(n, _, _)| scene.identity(n).unwrap().clone())
            .collect()
    }
    fn label_spawns(scene: &mut LoadedScene, sim: &mut SceneSimulation) {
        for &(index, node) in sim.commands().spawned() {
            if scene.identity(node).is_none() {
                assert!(
                    scene.identify(node, ObjectId("dead".into())).is_err(),
                    "reserved deleted identity was lost"
                );
                scene
                    .identify(node, ObjectId(format!("spawn-{index}")))
                    .unwrap();
            }
        }
    }
    #[test]
    fn one_file_restores_scene_allocator_history_behaviors_commands_and_continuation() {
        let (components, behaviors, mut source, mut original) = fixture();
        let snapshot = original.checkpoint(&source, &components, BYTES).unwrap();
        let dir = Temp::new();
        let path = dir.0.join("slot.json");
        snapshot
            .save_file(&path, &components, &behaviors, limits(), 8, BYTES)
            .unwrap();
        let (mut restored_scene, mut restored) = SceneSimulationCheckpoint::load_file(
            &path,
            &components,
            &behaviors,
            limits(),
            8,
            BYTES,
        )
        .unwrap();
        assert_eq!(identities(&source), identities(&restored_scene));
        let root = source.resolve(&ObjectId("z-root".into())).unwrap();
        let target = restored_scene.resolve(&ObjectId("z-root".into())).unwrap();
        assert_ne!(root, target);
        assert!(restored_scene.graph.local(root).is_err());
        assert_eq!(source.graph.component::<u32>(root).unwrap(), Some(&17));
        assert_eq!(
            restored_scene.graph.component::<u32>(target).unwrap(),
            Some(&17)
        );
        let children = |scene: &LoadedScene, node| {
            scene
                .graph
                .node(node)
                .unwrap()
                .children
                .iter()
                .map(|n| scene.identity(*n).unwrap().clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(children(&source, root), children(&restored_scene, target));
        assert_eq!(
            original
                .render_world(&source.graph, root)
                .unwrap()
                .to_cols_array()
                .map(f32::to_bits),
            restored
                .render_world(&restored_scene.graph, target)
                .unwrap()
                .to_cols_array()
                .map(f32::to_bits)
        );
        for i in 0..120 {
            let dt = [0.003, 0.021, 0.25][i % 3];
            let a = original.advance(&mut source.graph, dt).unwrap();
            let b = restored.advance(&mut restored_scene.graph, dt).unwrap();
            assert_eq!(a.commands, b.commands);
            assert_eq!(a.time.steps, b.time.steps);
            assert_eq!(a.time.alpha.to_bits(), b.time.alpha.to_bits());
            label_spawns(&mut source, &mut original);
            label_spawns(&mut restored_scene, &mut restored);
            assert_eq!(identities(&source), identities(&restored_scene));
            assert_eq!(
                source.capture(&components).unwrap(),
                restored_scene.capture(&components).unwrap()
            );
            for id in identities(&source) {
                let a = original
                    .render_world(&source.graph, source.resolve(&id).unwrap())
                    .unwrap();
                let b = restored
                    .render_world(&restored_scene.graph, restored_scene.resolve(&id).unwrap())
                    .unwrap();
                assert_eq!(
                    a.to_cols_array().map(f32::to_bits),
                    b.to_cols_array().map(f32::to_bits)
                );
            }
            assert_eq!(
                original
                    .behavior_checkpoint(&source.graph, |n| source.identity(n).cloned(), BYTES)
                    .unwrap(),
                restored
                    .behavior_checkpoint(
                        &restored_scene.graph,
                        |n| restored_scene.identity(n).cloned(),
                        BYTES
                    )
                    .unwrap()
            );
        }
        assert_eq!(source.graph.component::<u32>(root).unwrap(), Some(&999));
    }
    #[test]
    fn invalid_candidate_never_overwrites_good_file_or_live_pair() {
        let (components, behaviors, source, original) = fixture();
        let saved = original.checkpoint(&source, &components, BYTES).unwrap();
        let before = saved.to_json(BYTES).unwrap();
        let dir = Temp::new();
        let path = dir.0.join("slot.json");
        saved
            .save_file(&path, &components, &behaviors, limits(), 8, BYTES)
            .unwrap();
        let good = std::fs::read(&path).unwrap();
        let mut wrong = saved.clone();
        wrong.version = 2;
        assert!(
            wrong
                .save_file(&path, &components, &behaviors, limits(), 8, BYTES)
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), good);
        assert!(
            saved
                .save_file(&path, &components, &behaviors, limits(), 8, 1)
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), good);
        assert!(
            saved
                .prepare(
                    &components,
                    &BehaviorCheckpointRegistry::default(),
                    limits(),
                    8,
                    BYTES
                )
                .is_err()
        );
        assert!(
            SceneSimulationCheckpoint::load_file(&path, &components, &behaviors, limits(), 8, 1)
                .is_err()
        );
        assert_eq!(
            original
                .checkpoint(&source, &components, BYTES)
                .unwrap()
                .to_json(BYTES)
                .unwrap(),
            before
        );
    }
    #[test]
    fn corrupt_allocation_or_configuration_is_rejected_before_candidate_publication() {
        let (components, behaviors, source, original) = fixture();
        let saved = original.checkpoint(&source, &components, BYTES).unwrap();
        let before = saved.to_json(BYTES).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&before).unwrap();
        value["allocation"]["free"] = serde_json::json!([1, 1]);
        let bad = SceneSimulationCheckpoint::from_json(&value.to_string(), BYTES).unwrap();
        assert!(
            bad.prepare(&components, &behaviors, limits(), 8, BYTES)
                .is_err()
        );
        let mut changed = limits();
        changed.max_behaviors += 1;
        assert!(
            saved
                .prepare(&components, &behaviors, changed, 8, BYTES)
                .is_err()
        );
        assert!(matches!(
            saved.prepare(&components, &behaviors, limits(), 7, BYTES),
            Err(SceneCheckpointError::Capacity)
        ));
        let (restored, simulation) = saved
            .prepare(&components, &behaviors, limits(), 9, BYTES)
            .unwrap();
        assert_eq!(
            simulation
                .checkpoint(&restored, &components, BYTES)
                .unwrap()
                .to_json(BYTES)
                .unwrap(),
            before
        );
        assert_eq!(
            original
                .checkpoint(&source, &components, BYTES)
                .unwrap()
                .to_json(BYTES)
                .unwrap(),
            before
        );
    }
}

#[cfg(test)]
mod capacity_admission_tests {
    use super::*;
    #[test]
    fn larger_admission_ceiling_preserves_exact_arena_and_future_exhaustion() {
        let registry = ComponentRegistry::default();
        let document = SceneDocument {
            version: 1,
            objects: vec![],
        };
        let mut source = document.load(&registry, 4).unwrap();
        let old = source.graph.spawn(None, Transform::default()).unwrap();
        source.graph.remove_subtree(old).unwrap();
        let limits = SimulationLimits {
            fixed_step: 1. / 60.,
            max_steps: 8,
            max_behaviors: 4,
            max_commands: 8,
        };
        let simulation = SceneSimulation::new(&source.graph, limits).unwrap();
        let saved = simulation.checkpoint(&source, &registry, 100000).unwrap();
        let (mut restored, _) = saved
            .prepare(
                &registry,
                &BehaviorCheckpointRegistry::default(),
                limits,
                16,
                100000,
            )
            .unwrap();
        assert_eq!(restored.graph.capacity, 4);
        for _ in 0..6 {
            let first = source.graph.spawn(None, Transform::default());
            let second = restored.graph.spawn(None, Transform::default());
            assert_eq!(first.is_ok(), second.is_ok());
            if let (Ok(first), Ok(second)) = (first, second) {
                assert_eq!(
                    (first.slot, first.generation),
                    (second.slot, second.generation)
                );
            }
        }
    }
}
