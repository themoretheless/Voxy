//! Complete native UI input adapter; GPU-visible snapshots remain UiLive-owned.
use super::*;
use serde::{Deserialize, Serialize};
use voxy_gameplay::{SceneUiCheckpoint, SceneUiSnapshotCheckpoint};
use voxy_scene::{NodeId, ObjectId};
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GameUiInputCheckpoint {
    version: u32,
    runtime: SceneUiCheckpoint,
    activation_keys: u8,
    admitted: Option<SceneUiSnapshotCheckpoint>,
}
#[allow(dead_code)]
impl GameUiInput {
    pub(crate) fn checkpoint_historical(
        &self,
        view: &voxy_scene::SceneCheckpointView<'_>,
        max_bytes: usize,
    ) -> Result<GameUiInputCheckpoint, EditorError> {
        let Self {
            runtime,
            activation_keys,
            admitted,
        } = self;
        let identify = |node| view.reserved_identity(node).cloned();
        let saved = GameUiInputCheckpoint {
            version: 1,
            runtime: runtime.checkpoint(identify, max_bytes)?,
            activation_keys: *activation_keys,
            admitted: admitted
                .as_ref()
                .map(|snapshot| snapshot.checkpoint(identify, max_bytes))
                .transpose()?,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
    pub(crate) fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<GameUiInputCheckpoint, EditorError> {
        let Self {
            runtime,
            activation_keys,
            admitted,
        } = self;
        let identify = |node| {
            scene.local(node).ok()?;
            identify(node)
        };
        let saved = GameUiInputCheckpoint {
            version: 1,
            runtime: runtime.checkpoint(identify, max_bytes)?,
            activation_keys: *activation_keys,
            admitted: admitted
                .as_ref()
                .map(|snapshot| snapshot.checkpoint(identify, max_bytes))
                .transpose()?,
        };
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
}
#[allow(dead_code)]
impl GameUiInputCheckpoint {
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        if self.version != 1 || self.activation_keys & !3 != 0 {
            return Err("invalid native UI input checkpoint".into());
        }
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("native UI input checkpoint byte budget".into());
        }
        Ok(json)
    }
    pub(crate) fn from_json(json: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if json.len() > max_bytes {
            return Err("native UI input checkpoint byte budget".into());
        }
        let saved: Self = serde_json::from_str(json)?;
        saved.to_json(max_bytes)?;
        Ok(saved)
    }
    pub(crate) fn prepare_historical(
        &self,
        view: &voxy_scene::SceneCheckpointView<'_>,
        max_elements: usize,
        max_bytes: usize,
    ) -> Result<GameUiInput, EditorError> {
        self.to_json(max_bytes)?;
        let runtime = self
            .runtime
            .prepare_historical(view, max_elements, max_bytes)?;
        let admitted = self
            .admitted
            .as_ref()
            .map(|snapshot| snapshot.prepare_historical(view, max_elements, max_bytes))
            .transpose()?;
        Ok(GameUiInput {
            runtime,
            activation_keys: self.activation_keys,
            admitted,
        })
    }
    pub(crate) fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_elements: usize,
        max_bytes: usize,
    ) -> Result<GameUiInput, EditorError> {
        self.to_json(max_bytes)?;
        let runtime = self
            .runtime
            .prepare(scene, &resolve, max_elements, max_bytes)?;
        let admitted = self
            .admitted
            .as_ref()
            .map(|snapshot| snapshot.prepare(scene, &resolve, max_elements, max_bytes))
            .transpose()?;
        Ok(GameUiInput {
            runtime,
            activation_keys: self.activation_keys,
            admitted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_gameplay::UiElement;
    use voxy_scene::Transform;
    fn fixture(padding: bool) -> (SceneGraph, NodeId) {
        let mut scene = SceneGraph::new(4);
        if padding {
            scene.spawn(None, Transform::default()).unwrap();
        }
        let node = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                node,
                UiElement {
                    origin: [0.; 2],
                    size: [1.; 2],
                    color: [1.; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("start".into()),
                    text: None,
                },
            )
            .unwrap();
        (scene, node)
    }
    fn capture(input: &GameUiInput, scene: &SceneGraph, node: NodeId) -> String {
        input
            .checkpoint(
                scene,
                |n| (n == node).then(|| ObjectId("button".into())),
                100000,
            )
            .unwrap()
            .to_json(100000)
            .unwrap()
    }
    fn identify(event: Option<UiActionEvent>, node: NodeId) -> Option<(bool, String)> {
        event.map(|event| (event.owner == node, event.action))
    }
    fn key(
        input: &mut GameUiInput,
        scene: &SceneGraph,
        node: NodeId,
        code: KeyCode,
        pressed: bool,
        viewport: [f32; 2],
    ) -> (bool, Option<(bool, String)>) {
        let (consumed, event) = input
            .key(
                scene,
                viewport,
                code,
                if pressed {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                },
                false,
                false,
            )
            .unwrap();
        (consumed, identify(event, node))
    }
    #[test]
    fn native_adapter_rebinds_both_activation_keys_pointer_and_admitted_layout_for_120_cycles() {
        for held_pointer in [false, true] {
            let (mut source, node) = fixture(false);
            let mut input = GameUiInput::new();
            let snapshot = voxy_gameplay::extract_scene_ui(&source, [100.; 2], 128).unwrap();
            input
                .admit_presented(&source, [100.; 2], Some(&snapshot))
                .unwrap();
            key(&mut input, &source, node, KeyCode::Tab, true, [100.; 2]);
            key(&mut input, &source, node, KeyCode::Enter, true, [100.; 2]);
            key(&mut input, &source, node, KeyCode::Space, true, [100.; 2]);
            if held_pointer {
                input.pointer(&source, [100.; 2], [50.; 2], true).unwrap();
            }
            assert_eq!(input.activation_keys, 3);
            let json = capture(&input, &source, node);
            let saved = GameUiInputCheckpoint::from_json(&json, 100000).unwrap();
            let (mut target, fresh) = fixture(true);
            let mut resumed = saved
                .prepare(
                    &target,
                    |id| (id.0 == "button").then_some(fresh),
                    128,
                    100000,
                )
                .unwrap();
            assert_ne!(source.identity(), target.identity());
            assert_eq!(capture(&resumed, &target, fresh), json);
            assert_eq!(
                key(&mut input, &source, node, KeyCode::Enter, false, [100.; 2]),
                key(
                    &mut resumed,
                    &target,
                    fresh,
                    KeyCode::Enter,
                    false,
                    [100.; 2]
                )
            );
            let a = key(&mut input, &source, node, KeyCode::Space, false, [100.; 2]);
            let b = key(
                &mut resumed,
                &target,
                fresh,
                KeyCode::Space,
                false,
                [100.; 2],
            );
            assert_eq!(a, b);
            if !held_pointer {
                assert_eq!(a.1, Some((true, "start".into())));
            }
            if held_pointer {
                assert_eq!(
                    identify(
                        input.pointer(&source, [100.; 2], [50.; 2], false).unwrap(),
                        node
                    ),
                    identify(
                        resumed
                            .pointer(&target, [100.; 2], [50.; 2], false)
                            .unwrap(),
                        fresh
                    )
                );
            }
            let mut activations = 0;
            for i in 0..120 {
                let viewport = if i < 60 { [100.; 2] } else { [200.; 2] };
                for (scene, owner) in [(&mut source, node), (&mut target, fresh)] {
                    if i == 40 {
                        scene
                            .component_mut::<UiElement>(owner)
                            .unwrap()
                            .unwrap()
                            .action = Some("changed".into());
                    }
                    if i == 80 {
                        scene
                            .component_mut::<UiElement>(owner)
                            .unwrap()
                            .unwrap()
                            .enabled = false;
                    }
                    if i == 100 {
                        scene
                            .component_mut::<UiElement>(owner)
                            .unwrap()
                            .unwrap()
                            .enabled = true;
                    }
                }
                let a_snapshot = voxy_gameplay::extract_scene_ui(&source, viewport, 128).unwrap();
                let b_snapshot = voxy_gameplay::extract_scene_ui(&target, viewport, 128).unwrap();
                let a_presented = if i == 40 {
                    input.admitted.clone()
                } else {
                    Some(a_snapshot)
                };
                let b_presented = if i == 40 {
                    resumed.admitted.clone()
                } else {
                    Some(b_snapshot)
                };
                let a = input
                    .admit_presented(&source, viewport, a_presented.as_ref())
                    .unwrap();
                let b = resumed
                    .admit_presented(&target, viewport, b_presented.as_ref())
                    .unwrap();
                assert_eq!(a, b);
                if a {
                    let operation = |input: &mut GameUiInput,
                                     scene: &SceneGraph,
                                     owner|
                     -> Option<(bool, String)> {
                        match i % 8 {
                            0 | 7 => key(input, scene, owner, KeyCode::Tab, true, viewport).1,
                            1 => key(input, scene, owner, KeyCode::Enter, true, viewport).1,
                            2 => key(input, scene, owner, KeyCode::Space, true, viewport).1,
                            3 => key(input, scene, owner, KeyCode::Enter, false, viewport).1,
                            4 => key(input, scene, owner, KeyCode::Space, false, viewport).1,
                            5 => identify(
                                input.pointer(scene, viewport, [50.; 2], true).unwrap(),
                                owner,
                            ),
                            _ => identify(
                                input.pointer(scene, viewport, [50.; 2], false).unwrap(),
                                owner,
                            ),
                        }
                    };
                    let a = operation(&mut input, &source, node);
                    let b = operation(&mut resumed, &target, fresh);
                    assert_eq!(a, b);
                    activations += usize::from(a.is_some());
                }
                assert_eq!(
                    capture(&input, &source, node),
                    capture(&resumed, &target, fresh)
                );
            }
            assert!(activations > 10);
        }
    }
    #[test]
    fn tab_cancellation_and_empty_admitted_layout_remain_distinct_from_physical_key_holds() {
        let (scene, node) = fixture(false);
        let mut input = GameUiInput::new();
        key(&mut input, &scene, node, KeyCode::Tab, true, [100.; 2]);
        key(&mut input, &scene, node, KeyCode::Enter, true, [100.; 2]);
        key(&mut input, &scene, node, KeyCode::Space, true, [100.; 2]);
        key(&mut input, &scene, node, KeyCode::Tab, true, [100.; 2]);
        assert_eq!(input.activation_keys, 3);
        let json = capture(&input, &scene, node);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["runtime"]["focus"]["held"], false);
        let saved = GameUiInputCheckpoint::from_json(&json, 100000).unwrap();
        let (target, fresh) = fixture(true);
        let mut restored = saved
            .prepare(&target, |_| Some(fresh), 128, 100000)
            .unwrap();
        assert_eq!(
            key(
                &mut restored,
                &target,
                fresh,
                KeyCode::Enter,
                false,
                [100.; 2]
            )
            .1,
            None
        );
        assert_eq!(
            key(
                &mut restored,
                &target,
                fresh,
                KeyCode::Space,
                false,
                [100.; 2]
            )
            .1,
            None
        );
        let empty = SceneGraph::new(4);
        let snapshot = voxy_gameplay::extract_scene_ui(&empty, [100.; 2], 128).unwrap();
        let mut input = GameUiInput::new();
        input
            .admit_presented(&empty, [100.; 2], Some(&snapshot))
            .unwrap();
        let saved = input.checkpoint(&empty, |_| None, 100000).unwrap();
        let restored = saved.prepare(&empty, |_| None, 128, 100000).unwrap();
        assert_eq!(restored.admitted, Some(snapshot));
        assert_eq!(
            input
                .checkpoint(&empty, |_| None, 100000)
                .unwrap()
                .to_json(100000)
                .unwrap(),
            restored
                .checkpoint(&empty, |_| None, 100000)
                .unwrap()
                .to_json(100000)
                .unwrap()
        );
    }

    #[test]
    fn pending_layout_change_is_not_reextracted_during_load_and_invalid_data_preserves_input() {
        let (mut source, node) = fixture(false);
        let mut input = GameUiInput::new();
        let snapshot = voxy_gameplay::extract_scene_ui(&source, [100.; 2], 128).unwrap();
        input
            .admit_presented(&source, [100.; 2], Some(&snapshot))
            .unwrap();
        key(&mut input, &source, node, KeyCode::Tab, true, [100.; 2]);
        key(&mut input, &source, node, KeyCode::Enter, true, [100.; 2]);
        source
            .component_mut::<UiElement>(node)
            .unwrap()
            .unwrap()
            .action = Some("new".into());
        let json = capture(&input, &source, node);
        let saved = GameUiInputCheckpoint::from_json(&json, 100000).unwrap();
        let (mut target, fresh) = fixture(true);
        target
            .component_mut::<UiElement>(fresh)
            .unwrap()
            .unwrap()
            .action = Some("new".into());
        let mut resumed = saved
            .prepare(
                &target,
                |id| (id.0 == "button").then_some(fresh),
                128,
                100000,
            )
            .unwrap();
        assert_eq!(capture(&resumed, &target, fresh), json);
        assert_eq!(
            resumed.admitted.as_ref().unwrap().elements[0]
                .descriptor
                .action
                .as_deref(),
            Some("start")
        );
        assert!(
            !input
                .admit_presented(&source, [100.; 2], Some(&snapshot))
                .unwrap()
        );
        let old = resumed.admitted.clone();
        assert!(
            !resumed
                .admit_presented(&target, [100.; 2], old.as_ref())
                .unwrap()
        );
        assert_eq!(
            capture(&input, &source, node),
            capture(&resumed, &target, fresh)
        );
        assert!(saved.prepare(&target, |_| None, 128, 100000).is_err());
        assert!(saved.prepare(&target, |_| Some(fresh), 0, 100000).is_err());
        assert!(GameUiInputCheckpoint::from_json(&json, json.len() - 1).is_err());
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("activation_keys", serde_json::json!(4)),
        ] {
            let mut bad: serde_json::Value = serde_json::from_str(&json).unwrap();
            bad[field] = value;
            assert!(GameUiInputCheckpoint::from_json(&bad.to_string(), 100000).is_err());
        }
        let mut bad: serde_json::Value = serde_json::from_str(&json).unwrap();
        bad["admitted"]["layout"]["pointer"]["held"] = serde_json::json!(true);
        let bad = GameUiInputCheckpoint::from_json(&bad.to_string(), 100000).unwrap();
        assert!(bad.prepare(&target, |_| Some(fresh), 128, 100000).is_err());
        let before = capture(&input, &source, node);
        assert!(input.checkpoint(&source, |_| None, 100000).is_err());
        assert_eq!(capture(&input, &source, node), before);
    }
}

#[cfg(test)]
mod historical_tests {
    use super::*;
    use voxy_scene::{ObjectId, SceneDocument, SceneSimulation, SimulationLimits, Transform};
    #[test]
    fn deleted_pressed_owner_keeps_history_but_cannot_activate_a_recycled_slot() {
        let mut registry = voxy_scene::ComponentRegistry::default();
        voxy_gameplay::register_components(&mut registry).unwrap();
        let mut source = SceneDocument {
            version: 1,
            objects: Vec::new(),
        }
        .load(&registry, 4)
        .unwrap();
        let owner = source.graph.spawn(None, Transform::default()).unwrap();
        source.identify(owner, ObjectId("old".into())).unwrap();
        let descriptor = voxy_gameplay::UiElement {
            origin: [0.; 2],
            size: [1.; 2],
            color: [1.; 4],
            layer: 0,
            enabled: true,
            action: Some("old-action".into()),
            text: None,
        };
        source
            .graph
            .insert_component(owner, descriptor.clone())
            .unwrap();
        let layout = voxy_gameplay::extract_scene_ui(&source.graph, [100.; 2], 128).unwrap();
        let mut input = GameUiInput::new();
        input
            .admit_presented(&source.graph, [100.; 2], Some(&layout))
            .unwrap();
        input
            .key(
                &source.graph,
                [100.; 2],
                KeyCode::Tab,
                ElementState::Pressed,
                false,
                false,
            )
            .unwrap();
        input
            .key(
                &source.graph,
                [100.; 2],
                KeyCode::Enter,
                ElementState::Pressed,
                false,
                false,
            )
            .unwrap();
        input
            .pointer(&source.graph, [100.; 2], [50.; 2], true)
            .unwrap();
        source.graph.remove_subtree(owner).unwrap();
        let replacement = source.graph.spawn(None, Transform::default()).unwrap();
        source
            .identify(replacement, ObjectId("replacement".into()))
            .unwrap();
        source
            .graph
            .insert_component(
                replacement,
                voxy_gameplay::UiElement {
                    action: Some("new-action".into()),
                    ..descriptor
                },
            )
            .unwrap();
        let saved = input
            .checkpoint_historical(&source.checkpoint_view(), 100000)
            .unwrap();
        let json = saved.to_json(100000).unwrap();
        assert!(
            input
                .checkpoint(&source.graph, |node| source.identity(node).cloned(), 100000)
                .is_err()
        );
        let limits = SimulationLimits {
            fixed_step: 0.01,
            max_steps: 4,
            max_behaviors: 4,
            max_commands: 4,
        };
        let sim = SceneSimulation::new(&source.graph, limits).unwrap();
        let scene_record = sim.checkpoint(&source, &registry, 100000).unwrap();
        let (target, _) = scene_record
            .prepare(
                &registry,
                &voxy_scene::BehaviorCheckpointRegistry::default(),
                limits,
                4,
                100000,
            )
            .unwrap();
        let view = target.checkpoint_view();
        let dead = target.reserved_node(&ObjectId("old".into())).unwrap();
        assert!(target.graph.local(dead).is_err());
        assert_eq!(target.graph.len(), 1);
        assert!(
            saved
                .prepare(&target.graph, |id| target.reserved_node(id), 128, 100000)
                .is_err()
        );
        let mut resumed = saved.prepare_historical(&view, 128, 100000).unwrap();
        assert_eq!(
            resumed
                .checkpoint_historical(&view, 100000)
                .unwrap()
                .to_json(100000)
                .unwrap(),
            json
        );
        assert!(
            resumed
                .admitted
                .as_ref()
                .unwrap()
                .elements
                .iter()
                .any(|element| element.owner == dead)
        );
        let mut direct = saved.prepare_historical(&view, 128, 100000).unwrap();
        assert_eq!(
            input
                .key(
                    &source.graph,
                    [100.; 2],
                    KeyCode::Enter,
                    ElementState::Released,
                    false,
                    false
                )
                .unwrap()
                .1,
            direct
                .key(
                    &target.graph,
                    [100.; 2],
                    KeyCode::Enter,
                    ElementState::Released,
                    false,
                    false
                )
                .unwrap()
                .1
        );
        assert_eq!(
            input
                .pointer(&source.graph, [100.; 2], [50.; 2], false)
                .unwrap(),
            direct
                .pointer(&target.graph, [100.; 2], [50.; 2], false)
                .unwrap()
        );
        assert_eq!(
            input
                .checkpoint_historical(&source.checkpoint_view(), 100000)
                .unwrap()
                .to_json(100000)
                .unwrap(),
            direct
                .checkpoint_historical(&view, 100000)
                .unwrap()
                .to_json(100000)
                .unwrap()
        );
        let old = resumed.admitted.clone();
        assert!(
            !resumed
                .admit_presented(&target.graph, [100.; 2], old.as_ref())
                .unwrap()
        );
        assert_eq!(
            resumed
                .key(
                    &target.graph,
                    [100.; 2],
                    KeyCode::Enter,
                    ElementState::Released,
                    false,
                    false
                )
                .unwrap()
                .1,
            None
        );
        assert_eq!(
            resumed
                .pointer(&target.graph, [100.; 2], [50.; 2], false)
                .unwrap(),
            None
        );
        assert_eq!(target.graph.len(), 1);
        assert!(target.graph.local(dead).is_err());
    }
}
