//! Repeatable native render/input/physics/barrier/lifecycle acceptance.
use super::{
    App, Arc, Behavior, CharacterBody, ElementState, KeyCode, ModelInstance, NodeId, SceneGraph,
    Transform, Vec3,
};
use voxy_scene::SceneCommand;
#[derive(Debug)]
struct Lifecycle(Arc<std::sync::Mutex<Vec<&'static str>>>);
impl Behavior for Lifecycle {
    fn awake(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.0.lock().unwrap().push("awake");
    }
    fn on_enable(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.0.lock().unwrap().push("enable");
    }
    fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.0.lock().unwrap().push("start");
    }
    fn on_disable(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.0.lock().unwrap().push("disable");
    }
    fn on_destroy(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.0.lock().unwrap().push("destroy");
    }
}
#[derive(Debug)]
pub(super) struct Smoke {
    phase: u8,
    tick: u64,
    frame: u64,
    owner: Option<NodeId>,
    original: Option<Transform>,
    log: Arc<std::sync::Mutex<Vec<&'static str>>>,
}
impl Smoke {
    pub(super) fn new() -> Self {
        Self {
            phase: 0,
            tick: 0,
            frame: 0,
            owner: None,
            original: None,
            log: Arc::default(),
        }
    }
}
impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn gameplay_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut smoke = self.gameplay_smoke.take().ok_or("missing game smoke")?;
        let mut done = false;
        match smoke.phase {
            0 if self.frames >= 3
                && self.instances.iter().all(|owner| {
                    self.scene
                        .component::<ModelInstance>(*owner)
                        .ok()
                        .flatten()
                        .is_some_and(|model| self.catalog.snapshot(&model.asset).is_some())
                }) =>
            {
                self.toggle_play()?;
                let owner = self
                    .scene
                    .components::<CharacterBody>()
                    .next()
                    .map(|(owner, _)| owner)
                    .ok_or("game fixture needs a character")?;
                self.selected = self
                    .instances
                    .iter()
                    .position(|node| *node == owner)
                    .ok_or("missing game actor")?;
                self.play.simulation
                    .as_mut()
                    .ok_or("missing simulation")?
                    .attach(&mut self.scene, owner, Lifecycle(Arc::clone(&smoke.log)))?;
                smoke.owner = Some(owner);
                smoke.frame = self.frames;
                smoke.phase = 1;
            }
            1 => {
                let owner = smoke.owner.ok_or("missing owner")?;
                if self.play
                    .physics
                    .as_ref()
                    .ok_or("missing physics")?
                    .state(&self.scene, owner)?
                    .is_some_and(|state| state.grounded)
                {
                    smoke.original = Some(self.scene.local(owner)?);
                    self.game_key(KeyCode::ArrowRight, ElementState::Pressed)?;
                    // A complete press/release before a tick must still jump once.
                    self.game_key(KeyCode::Space, ElementState::Pressed)?;
                    self.game_key(KeyCode::Space, ElementState::Released)?;
                    smoke.tick = self.play.simulation_ticks;
                    smoke.phase = 2;
                }
            }
            2 if self.play.simulation_ticks >= smoke.tick + 3 => {
                let owner = smoke.owner.ok_or("missing owner")?;
                let state = self.play
                    .physics
                    .as_ref()
                    .ok_or("missing physics")?
                    .state(&self.scene, owner)?
                    .ok_or("missing body")?;
                let local = self.scene.local(owner)?;
                let original = smoke.original.ok_or("missing original")?;
                if state.velocity[1] <= 0.0
                    || local.translation.y <= original.translation.y
                    || local.translation.x <= original.translation.x
                {
                    return Err("named movement/quick-tap jump did not advance physics".into());
                }
                println!(
                    "GAME INPUT PASS: native key mapping, fixed-tick movement, quick-tap jump"
                );
                smoke.phase = 3;
            }
            3 if self.play.simulation_ticks >= smoke.tick + 20 => {
                self.game_key(KeyCode::ArrowRight, ElementState::Released)?;
                let owner = smoke.owner.ok_or("missing owner")?;
                self.play.simulation
                    .as_mut()
                    .ok_or("missing simulation")?
                    .commands()
                    .push(SceneCommand::SetActive(owner, false))?;
                smoke.original = Some(self.scene.local(owner)?);
                smoke.tick = self.play.simulation_ticks;
                smoke.phase = 4;
            }
            4 if self.play.simulation_ticks >= smoke.tick + 4 => {
                let owner = smoke.owner.ok_or("missing owner")?;
                if self.scene.local(owner)? != smoke.original.ok_or("missing pose")?
                    || self
                        .extraction
                        .instances()
                        .iter()
                        .any(|instance| instance.owner == owner)
                {
                    return Err("inactive body moved or remained rendered".into());
                }
                self.play.simulation
                    .as_mut()
                    .ok_or("missing simulation")?
                    .commands()
                    .push(SceneCommand::SetActive(owner, true))?;
                smoke.tick = self.play.simulation_ticks;
                smoke.phase = 5;
            }
            5 if self.play.simulation_ticks >= smoke.tick + 2 => {
                self.play.simulation
                    .as_mut()
                    .ok_or("missing simulation")?
                    .commands()
                    .push(SceneCommand::RemoveSubtree(
                        smoke.owner.ok_or("missing owner")?,
                    ))?;
                smoke.frame = self.frames;
                smoke.phase = 6;
            }
            6 if self.frames > smoke.frame + 1 => {
                let old = smoke.owner.ok_or("missing owner")?;
                if self.scene.local(old).is_ok()
                    || self.play.physics.as_ref().ok_or("missing physics")?.body_count() != 0
                    || self
                        .extraction
                        .instances()
                        .iter()
                        .any(|instance| instance.owner == old)
                    || self
                        .graphics
                        .as_ref()
                        .ok_or("missing graphics")?
                        .transforms
                        .keys()
                        .any(|(_, owner)| *owner == old)
                {
                    return Err("deleted owner retained scene/physics/render state".into());
                }
                if *smoke.log.lock().unwrap()
                    != [
                        "awake", "enable", "start", "disable", "enable", "disable", "destroy",
                    ]
                {
                    return Err("unexpected lifecycle transitions".into());
                }
                self.play.simulation
                    .as_mut()
                    .ok_or("missing simulation")?
                    .commands()
                    .push(SceneCommand::Spawn(
                        None,
                        Transform {
                            translation: Vec3::new(0.0, 0.0, 0.5),
                            ..Transform::default()
                        },
                    ))?;
                smoke.frame = self.frames;
                smoke.phase = 7;
            }
            7 if self.frames > smoke.frame => {
                let commands = self.play
                    .simulation
                    .as_mut()
                    .ok_or("missing simulation")?
                    .commands();
                let owner = commands.spawned().first().ok_or("spawn did not execute")?.1;
                if Some(owner) == smoke.owner {
                    return Err("reused slot retained old identity".into());
                }
                commands.insert_component(
                    owner,
                    ModelInstance {
                        asset: self.id.clone(),
                    },
                )?;
                commands.insert_component(
                    owner,
                    CharacterBody {
                        gravity: 0.0,
                        ..CharacterBody::default()
                    },
                )?;
                smoke.owner = Some(owner);
                smoke.frame = self.frames;
                smoke.phase = 8;
            }
            8 if self.frames > smoke.frame + 1 => {
                let owner = smoke.owner.ok_or("missing replacement")?;
                if !self
                    .extraction
                    .instances()
                    .iter()
                    .any(|instance| instance.owner == owner)
                    || !self
                        .graphics
                        .as_ref()
                        .ok_or("missing graphics")?
                        .transforms
                        .keys()
                        .any(|(_, node)| *node == owner)
                    || self.play
                        .physics
                        .as_ref()
                        .ok_or("missing physics")?
                        .state(&self.scene, owner)?
                        .is_none_or(|state| {
                            state
                                .velocity
                                .iter()
                                .any(|value| value.abs() > f64::EPSILON)
                        })
                {
                    return Err(
                        "replacement did not acquire independent render/physics state".into(),
                    );
                }
                println!(
                    "GAME OWNERS PASS: activity, queued deletion/creation, generation reuse, GPU cache and lifecycle"
                );
                self.toggle_play()?;
                smoke.frame = self.frames;
                smoke.phase = 9;
            }
            9 if self.frames > smoke.frame => {
                if self.play.physics.is_some() || self.play.simulation.is_some() || self.play.playing.is_some() {
                    return Err("Stop retained runtime state".into());
                }
                println!(
                    "GAMEPLAY WINDOW PASS: ticks={} native_frames={} authoring_restored=true",
                    self.play.simulation_ticks, self.frames
                );
                smoke.phase = 10;
                done = true;
            }
            10 => {
                done = true;
            }
            _ => {}
        }
        self.gameplay_smoke = Some(smoke);
        Ok(done)
    }
}
