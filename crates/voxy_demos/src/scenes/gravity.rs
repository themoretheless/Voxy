//! Finite spheres under mutual Newtonian gravity, presented by the scene shell.
use super::pressed_key;
use crate::gravity_demo::GravityDemo;
use glam::Vec3;
use voxy_render::{SceneCamera, SceneTexture};
use voxy_scene_host::{
    DemoEnv, DemoFrame, DemoGpu, DemoInput, DemoKey, DemoReply, DemoResult, DemoTick, DemoView,
    FrameParts, HostRequest, MeshSlot, SceneDemo, SharedDraw, SmokeReport, perspective_camera,
};

#[derive(Debug)]
pub struct GravityScene {
    demo: GravityDemo,
    slot: MeshSlot,
    smoke_stage: u8,
}

impl GravityScene {
    #[must_use]
    pub fn new(collision: bool) -> Self {
        Self {
            demo: GravityDemo::new(collision),
            slot: MeshSlot::default(),
            smoke_stage: 0,
        }
    }
    /// Starts with the gravity-relative planet walking demonstration.
    #[must_use]
    pub fn planet() -> Self {
        let mut scene = Self::new(false);
        scene.demo.planet();
        scene
    }
}

impl SceneDemo for GravityScene {
    fn configure(&mut self, _env: DemoEnv) -> HostRequest {
        HostRequest {
            focus: true,
            smoke_timeout_seconds: 60,
            ..HostRequest::default()
        }
    }
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult<Option<SceneTexture>> {
        self.slot.upload(&gpu, &self.demo.mesh()?)?;
        Ok(None)
    }
    fn release(&mut self) {
        self.slot.release();
    }
    fn advance(&mut self, tick: DemoTick) -> DemoResult {
        if tick.smoke && tick.frame >= 30 && self.smoke_stage == 0 {
            self.demo.reset(true);
            self.smoke_stage = 1;
        }
        if tick.smoke && tick.frame >= 90 && self.smoke_stage == 1 && self.demo.contacts > 0 {
            println!("GRAVITY COLLISION PASS: {} contacts", self.demo.contacts);
            self.demo.planet();
            self.smoke_stage = 2;
        }
        self.demo.advance(f64::from(tick.dt))?;
        Ok(())
    }
    fn camera(&self, view: DemoView) -> SceneCamera {
        let distance = self.demo.camera_distance(view.aspect);
        perspective_camera(
            Vec3::new(0.0, 0.0, distance),
            Vec3::ZERO,
            view.aspect,
            distance * 3.0,
        )
    }
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult {
        self.slot.update(frame.queue, &self.demo.mesh()?)
    }
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>> {
        self.slot.frame(shared)
    }
    fn input(&mut self, input: DemoInput) -> DemoResult<DemoReply> {
        match pressed_key(input) {
            Some(DemoKey::Digit1) => self.demo.reset(false),
            Some(DemoKey::Digit2) => self.demo.reset(true),
            Some(DemoKey::Digit3) => self.demo.planet(),
            Some(DemoKey::J) => self.demo.jump(),
            Some(DemoKey::R) => self.demo.restart(),
            _ => {}
        }
        Ok(DemoReply::default())
    }
    fn title(&self, _paused: bool) -> Option<String> {
        Some(self.demo.title())
    }
    fn smoke_ready(&self) -> bool {
        self.smoke_stage == 2 && self.demo.planet_verified()
    }
    fn verify_smoke(&self, _report: SmokeReport) -> DemoResult {
        if self.demo.contacts == 0 {
            return Err("gravity smoke did not observe a collision".into());
        }
        println!(
            "GRAVITY SMOKE PASS: {} steps, {} contacts; inverted walking, jump and landing verified",
            self.demo.steps, self.demo.contacts
        );
        Ok(())
    }
}
