//! Prescribed voxel abrasion and the emitted suspension, presented by the scene shell.
use super::{fixed_smoke_step, pressed_key};
use crate::wear_demo::WearDemo;
use glam::Vec3;
use voxy_render::{SceneCamera, SceneTexture};
use voxy_scene_host::{
    DemoFrame, DemoGpu, DemoInput, DemoKey, DemoReply, DemoResult, DemoTick, DemoView, FrameParts,
    MeshSlot, SceneDemo, SharedDraw, SmokeReport, perspective_camera,
};

#[derive(Debug)]
pub struct WearScene {
    demo: WearDemo,
    slot: MeshSlot,
}

impl WearScene {
    /// # Errors
    /// Returns invalid simulation setup errors.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            demo: WearDemo::new()?,
            slot: MeshSlot::default(),
        })
    }
}

impl SceneDemo for WearScene {
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult<Option<SceneTexture>> {
        self.slot
            .reserve(&gpu, 60_024, 60_024, &self.demo.mesh()?)?;
        Ok(None)
    }
    fn release(&mut self) {
        self.slot.release();
    }
    fn advance(&mut self, tick: DemoTick) -> DemoResult {
        self.demo.advance(fixed_smoke_step(tick))
    }
    fn camera(&self, view: DemoView) -> SceneCamera {
        perspective_camera(Vec3::new(0.0, 1.4, 6.0), Vec3::ZERO, view.aspect, 100.0)
    }
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult {
        self.slot.update(frame.queue, &self.demo.mesh()?)
    }
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>> {
        self.slot.frame(shared)
    }
    fn input(&mut self, input: DemoInput) -> DemoResult<DemoReply> {
        if pressed_key(input) == Some(DemoKey::R) {
            self.demo = WearDemo::new()?;
        }
        Ok(DemoReply::default())
    }
    fn title(&self, _paused: bool) -> Option<String> {
        Some(
            "Voxy wear | receding solid and emitted grains | Space: pause | R: reset | Esc: exit"
                .to_owned(),
        )
    }
    fn verify_smoke(&self, _report: SmokeReport) -> DemoResult {
        self.demo.verify()?;
        println!(
            "WEAR SMOKE PASS: {} steps, voxel geometry/collision and grain mass verified",
            self.demo.steps
        );
        Ok(())
    }
}
