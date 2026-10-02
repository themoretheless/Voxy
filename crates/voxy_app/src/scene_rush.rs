//! Winit input and scene lifecycle adapter for object scripts.
use voxy_input::{Binding, Control, InputMap};
use voxy_rush::{ScriptEvent, ScriptManager, rush::StateValue};
use voxy_scene::{NodeId, SceneGraph};
use winit::keyboard::KeyCode;
#[derive(Debug)]
pub(crate) struct SceneRush {
    pub scripts: ScriptManager,
    input: InputMap,
    owner: NodeId,
    failed: bool,
    character: physics::CharacterState,
    physics_enabled: bool,
    pub contacts: usize,
    pub messages: std::collections::VecDeque<String>,
    pub panel_visible: bool,
    pub selected_message: usize,
    pub line_offset: usize,
}
impl SceneRush {
    pub fn new(
        scene: &mut SceneGraph,
        owner: NodeId,
        path: impl AsRef<std::path::Path>,
        selected: Vec<String>,
    ) -> Result<Self, String> {
        let mut scripts = ScriptManager::new(1024, 1024);
        scripts.attach(scene, owner, path, selected)?;
        let initial = scene.local(owner).map_err(|e| e.to_string())?.translation;
        let mut input = InputMap::new(3, 4);
        for (name, keys) in [
            ("move", vec![(KeyCode::KeyD, 1.0), (KeyCode::KeyA, -1.0)]),
            ("spawn", vec![(KeyCode::KeyF, 1.0)]),
            ("reward", vec![(KeyCode::KeyE, 1.0)]),
        ] {
            input
                .bind(
                    name,
                    keys.into_iter()
                        .map(|(key, scale)| Binding {
                            control: Control {
                                device: 0,
                                code: key as u32,
                            },
                            scale,
                        })
                        .collect(),
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(Self {
            scripts,
            input,
            owner,
            failed: false,
            character: physics::CharacterState {
                body: physics::AnchoredAabb {
                    anchor: physics::Origin { x: 0, y: 0, z: 0 },
                    min: [
                        initial.x as f64 - 0.5,
                        initial.y as f64 - 0.5,
                        initial.z as f64 - 0.5,
                    ],
                    max: [
                        initial.x as f64 + 0.5,
                        initial.y as f64 + 0.5,
                        initial.z as f64 + 0.5,
                    ],
                },
                velocity: [0.0; 3],
                grounded: false,
            },
            physics_enabled: false,
            contacts: 0,
            messages: Default::default(),
            panel_visible: true,
            selected_message: 0,
            line_offset: 0,
        })
    }
    pub fn enable_character(&mut self, scene: &mut SceneGraph) -> Result<(), String> {
        if self.physics_enabled {
            return Ok(());
        }
        scene
            .apply_atomic(&[voxy_scene::SceneMutation::SpawnNamed(
                "floor".into(),
                voxy_scene::Transform {
                    translation: glam::Vec3::new(0.0, -1.0, 0.0),
                    scale: glam::Vec3::new(14.0, 1.0, 10.0),
                    ..Default::default()
                },
            )])
            .map_err(|e| e.to_string())?;
        self.physics_enabled = true;
        Ok(())
    }
    pub fn keyboard(&mut self, key: KeyCode, down: bool) {
        if down {
            match key {
                KeyCode::ArrowDown => self.line_offset = (self.line_offset + 1).min(512),
                KeyCode::ArrowUp => self.line_offset = self.line_offset.saturating_sub(1),
                KeyCode::F1 => self.panel_visible = !self.panel_visible,
                KeyCode::PageUp => {
                    self.selected_message =
                        (self.selected_message + 1).min(self.messages.len().saturating_sub(1))
                }
                KeyCode::PageDown => {
                    self.selected_message = self.selected_message.saturating_sub(1)
                }
                _ => {}
            }
        }
        let _ = self.input.event(
            Control {
                device: 0,
                code: key as u32,
            },
            if down { 1.0 } else { 0.0 },
        );
    }
    pub fn focused(&mut self, focused: bool) {
        self.input.set_focused(focused);
    }
    pub fn poll_reload(&mut self, scene: &SceneGraph) {
        let was_paused = self.scripts.paused(self.owner);
        for error in self.scripts.reload_changed(scene) {
            self.record(error);
        }
        if was_paused && !self.scripts.paused(self.owner) {
            self.record(
                "Reload succeeded; instance resumed. Earlier errors remain in history.".into(),
            );
        }
    }
    pub fn begin_frame(&mut self, scene: &mut SceneGraph) -> Result<(), String> {
        self.scripts
            .set_input(&self.input, &["move", "spawn", "reward"])?;
        if self.input.state("reward").is_some_and(|s| s.pressed) {
            self.scripts.event(
                scene,
                ScriptEvent {
                    target: Some(self.owner),
                    name: "scene.reward".into(),
                    payload: StateValue::Null,
                },
            )?;
        }
        Ok(())
    }
    pub fn fixed_update(&mut self, scene: &mut SceneGraph, delta: f64) -> Result<(), String> {
        self.scripts.fixed_update(scene, delta)?;
        if !self.physics_enabled {
            return Ok(());
        }
        let Ok(mut transform) = scene.local(self.owner) else {
            return Ok(());
        };
        if !scene.active_in_hierarchy(self.owner).unwrap_or(false) {
            return Ok(());
        }
        let mut world = crate::rush_physics::ScriptCollisionWorld { boxes: Vec::new() };
        for (id, t, _) in scene.nodes() {
            if id == self.owner || !scene.active_in_hierarchy(id).unwrap_or(false) {
                continue;
            }
            let name = scene.name(id).unwrap_or_default();
            if name != "floor" && name != "crate" {
                continue;
            }
            let p = t.translation.as_dvec3();
            let half = t.scale.abs().as_dvec3() * 0.5;
            world
                .boxes
                .push((name.into(), (p - half).to_array(), (p + half).to_array()));
        }
        let origin = [
            self.character.body.anchor.x as f64,
            self.character.body.anchor.y as f64,
            self.character.body.anchor.z as f64,
        ];
        let center: [f64; 3] = std::array::from_fn(|i| {
            origin[i] + (self.character.body.min[i] + self.character.body.max[i]) * 0.5
        });
        let desired = transform.translation.as_dvec3().to_array();
        let mut candidate = self.character;
        let input = physics::CharacterInput {
            planar_velocity: [
                (desired[0] - center[0]) / delta,
                (desired[2] - center[2]) / delta,
            ],
            jump_pressed: false,
        };
        let step = physics::step_character(
            &world,
            &mut candidate,
            input,
            delta,
            physics::CharacterConfig {
                step_height: 0.0,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
        let origin = [
            candidate.body.anchor.x as f64,
            candidate.body.anchor.y as f64,
            candidate.body.anchor.z as f64,
        ];
        transform.translation = glam::Vec3::from_array(std::array::from_fn(|i| {
            (origin[i] + (candidate.body.min[i] + candidate.body.max[i]) * 0.5) as f32
        }));
        scene
            .apply_atomic(&[voxy_scene::SceneMutation::SetLocal(self.owner, transform)])
            .map_err(|e| e.to_string())?;
        self.character = candidate;
        self.contacts += step.contacts.len();
        self.scripts
            .character_events(scene, self.owner, &step, |name| {
                StateValue::String(name.clone())
            })?;
        Ok(())
    }
    fn record(&mut self, message: String) {
        if self.messages.back() == Some(&message) {
            return;
        }
        eprintln!("{message}");
        if self.messages.back() != Some(&message) {
            if self.messages.len() == 32 {
                self.messages.pop_front();
            }
            self.messages.push_back(message);
        }
        self.selected_message = 0;
        self.line_offset = 0;
    }
    pub fn panel_text(&self) -> String {
        let status = if self.scripts.paused(self.owner) {
            "PAUSED"
        } else {
            "RUNNING"
        };
        let message = self
            .messages
            .iter()
            .rev()
            .nth(self.selected_message)
            .map_or("No script errors", String::as_str);
        format!(
            "Rush diagnostics | {status} | F1 hide | PgUp/PgDn history | Up/Down scroll\nA/D move  F spawn  E event | Physics: {}\n{}",
            if !self.physics_enabled {
                "disabled"
            } else if self.contacts > 0 {
                "contacts received"
            } else {
                "waiting for contact"
            },
            message
                .lines()
                .skip(self.line_offset)
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    pub fn smoke_input(&mut self, frame: u32) {
        self.input.set_focused(true);
        match frame {
            0 => self.keyboard(KeyCode::KeyD, true),
            20 => self.keyboard(KeyCode::KeyF, true),
            21 => self.keyboard(KeyCode::KeyF, false),
            40 => self.keyboard(KeyCode::KeyE, true),
            41 => self.keyboard(KeyCode::KeyE, false),
            60 => self.keyboard(KeyCode::KeyD, false),
            _ => {}
        }
    }
    pub fn verify_smoke(&mut self, scene: &SceneGraph) -> Result<(), String> {
        let transform = scene.local(self.owner).map_err(|e| e.to_string())?;
        let state = self.scripts.save(self.owner)?;
        if !matches!(state.get("contacts"),Some(StateValue::Number(n)) if *n>0.0) {
            return Err("Rush did not receive real physics contacts".into());
        }
        if self.failed
            || self.scripts.paused(self.owner)
            || self.contacts == 0
            || transform.translation.x <= 0.0
            || transform.scale != glam::Vec3::splat(2.0)
            || scene.find_named("crate").count() != 1
        {
            return Err(format!(
                "Rush gameplay smoke failed: {transform:?}, errors={}",
                self.failed
            ));
        }
        Ok(())
    }
    pub fn finish_frame(&mut self) {
        self.input.finish_frame();
        let diagnostics: Vec<_> = self
            .scripts
            .diagnostics
            .drain(..)
            .map(|d| d.to_string())
            .collect();
        for diagnostic in diagnostics {
            self.failed = true;
            self.record(diagnostic);
        }
        let errors: Vec<_> = self.scripts.command_errors.drain(..).collect();
        for error in errors {
            self.failed = true;
            self.record(format!("Rush command: {error}"));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_fixed_loop_resolves_collisions_and_delivers_contacts_to_script() {
        let file = std::env::temp_dir().join(format!("rush-physics-loop-{}.r", std::process::id()));
        std::fs::write(&file, include_str!("../../voxy_rush/examples/player.r")).unwrap();
        let mut scene = SceneGraph::new(8);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let mut runtime =
            SceneRush::new(&mut scene, owner, &file, vec!["contacts".into()]).unwrap();
        scene
            .apply_atomic(&[voxy_scene::SceneMutation::SpawnNamed(
                "crate".into(),
                voxy_scene::Transform {
                    translation: glam::Vec3::new(2.0, 0.0, 0.0),
                    ..Default::default()
                },
            )])
            .unwrap();
        runtime.enable_character(&mut scene).unwrap();
        runtime.keyboard(KeyCode::KeyD, true);
        for _ in 0..90 {
            runtime.begin_frame(&mut scene).unwrap();
            runtime.fixed_update(&mut scene, 1.0 / 60.0).unwrap();
            runtime.scripts.update(&mut scene, 1.0 / 60.0).unwrap();
            runtime.finish_frame();
        }
        let t = scene.local(owner).unwrap();
        assert!(t.translation.x > 0.9 && t.translation.x <= 1.0001, "{t:?}");
        assert!(t.translation.y.abs() < 0.0001);
        assert!(runtime.contacts > 0);
        assert!(!runtime.failed);
        assert!(
            matches!(runtime.scripts.save(owner).unwrap()["contacts"],StateValue::Number(n) if n>0.0)
        );
        runtime.keyboard(KeyCode::KeyD, false);
        runtime.keyboard(KeyCode::KeyA, true);
        for _ in 0..90 {
            runtime.begin_frame(&mut scene).unwrap();
            runtime.fixed_update(&mut scene, 1.0 / 60.0).unwrap();
            runtime.finish_frame();
        }
        let t = scene.local(owner).unwrap();
        assert!(t.translation.x < -4.8, "{t:?}");
        assert!(t.translation.z.abs() < 0.0001);
        assert!(t.translation.y.abs() < 0.0001);
        std::fs::remove_file(file).unwrap();
    }
}
#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn panel_keeps_stack_and_reports_recovery_without_repeating_the_error() {
        let file =
            std::env::temp_dir().join(format!("rush-diagnostic-panel-{}.r", std::process::id()));
        std::fs::write(
            &file,
            "fn fail() { return 1 / 0 }\nfn update(delta) { fail() }",
        )
        .unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let mut runtime = SceneRush::new(&mut scene, owner, &file, vec![]).unwrap();
        runtime.scripts.update(&mut scene, 1.0).unwrap();
        runtime.finish_frame();
        let text = runtime.panel_text();
        assert!(text.contains("PAUSED"));
        assert!(text.contains("at fail"));
        assert!(text.contains(file.to_str().unwrap()));
        runtime.scripts.update(&mut scene, 1.0).unwrap();
        runtime.finish_frame();
        assert_eq!(runtime.messages.len(), 1);
        std::fs::write(&file, "mut n = 0\nfn update(delta) { n += 1 }").unwrap();
        runtime.poll_reload(&scene);
        assert!(runtime.panel_text().contains("RUNNING"));
        assert!(runtime.panel_text().contains("instance resumed"));
        runtime.scripts.update(&mut scene, 1.0).unwrap();
        runtime.finish_frame();
        assert_eq!(runtime.messages.len(), 2);
        runtime.keyboard(KeyCode::PageUp, true);
        assert!(runtime.panel_text().contains("at fail"));
        std::fs::remove_file(file).unwrap();
    }
}
