//! Native acceptance uses presented frames and the ordinary editor Play/Stop path.
use crate::{App, ModelAnimation};
use winit::keyboard::KeyCode;

#[derive(Debug, Default)]
pub(super) struct Smoke {
    phase: u8,
    since: u64,
    authoring: Option<voxy_scene::SceneDocument>,
}
impl App {
    pub(super) fn animation_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(smoke) = &self.animation_smoke else {
            return Ok(false);
        };
        if self.frames < smoke.since + 3 {
            return Ok(false);
        }
        match smoke.phase {
            0 => {
                let Some(asset) = self.catalog.snapshot(&self.id) else {
                    return Ok(false);
                };
                let model = asset
                    .value()
                    .animated
                    .as_ref()
                    .ok_or("animation acceptance requires an animated model")?;
                if model.animations.is_empty() {
                    return Err("animation acceptance requires a clip".into());
                }
                if self
                    .graphics
                    .as_ref()
                    .is_none_or(|g| !g.models.contains_key(&self.id))
                {
                    return Ok(false);
                }
                self.edit_key(KeyCode::KeyD)?;
                if self.instances.len() != 2 {
                    return Err("native animation setup did not duplicate the model".into());
                }
                self.scene
                    .insert_component(self.instances[0], ModelAnimation::default())?;
                self.scene.insert_component(
                    self.instances[1],
                    ModelAnimation {
                        speed: 0.,
                        ..ModelAnimation::default()
                    },
                )?;
                self.commit_authoring()?;
                let authoring = self.authoring_document()?;
                self.toggle_play()?;
                let smoke = self.animation_smoke.as_mut().unwrap();
                smoke.authoring = Some(authoring);
                smoke.phase = 1;
                smoke.since = self.frames;
            }
            1 => {
                if self.play.simulation_ticks < 12 {
                    return Ok(false);
                }
                let graphics = self
                    .graphics
                    .as_mut()
                    .ok_or("missing native animation graphics")?;
                let first = *self.instances.first().ok_or("missing moving owner")?;
                let second = *self.instances.get(1).ok_or("missing paused owner")?;
                let Some(moving) = graphics.animated_models.pose_signature(first)? else {
                    return Ok(false);
                };
                let Some(paused) = graphics.animated_models.pose_signature(second)? else {
                    return Ok(false);
                };
                if moving == paused {
                    return Err("native owners failed to independently animate/pause".into());
                }
                let (owners, gpu_primitives, sources) = graphics.animated_models.counts();
                if owners != 2 {
                    return Err("native animation retained the wrong owner count".into());
                }
                if gpu_primitives > 0 && sources * 2 != gpu_primitives {
                    return Err("native animation sources were not shared".into());
                }
                println!(
                    "ANIMATION NATIVE PLAY PASS frames={} ticks={} owners={owners} gpu_primitives={gpu_primitives} shared_sources={sources} bytes={}",
                    self.frames,
                    self.play.simulation_ticks,
                    graphics.animated_models.allocation_bytes()
                );
                self.toggle_play()?;
                let smoke = self.animation_smoke.as_mut().unwrap();
                smoke.phase = 2;
                smoke.since = self.frames;
            }
            2 => {
                let graphics = self
                    .graphics
                    .as_ref()
                    .ok_or("missing native graphics after Stop")?;
                if graphics.animated_models.allocation_bytes() != 0 {
                    return Err("Stop retained animated GPU allocations".into());
                }
                if self.authoring_document()?
                    != *self
                        .animation_smoke
                        .as_ref()
                        .unwrap()
                        .authoring
                        .as_ref()
                        .unwrap()
                {
                    return Err("Stop changed authored animation settings or transforms".into());
                }
                println!(
                    "ANIMATION NATIVE STOP PASS frames={} animated_bytes=0 authoring_restored=true",
                    self.frames
                );
                return Ok(true);
            }
            _ => unreachable!(),
        }
        Ok(false)
    }
}
