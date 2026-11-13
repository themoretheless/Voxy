//! Simulated grass and hair, presented by the scene shell.
use crate::strands_demo::StrandsDemo;
use glam::Vec3;
use voxy_render::{SceneCamera, SceneTexture};
use voxy_scene_host::{
    DemoEnv, DemoFrame, DemoGpu, DemoResult, DemoTick, DemoView, FrameParts, HostRequest, MeshSlot,
    SceneDemo, SharedDraw, perspective_camera,
};

#[derive(Debug)]
pub struct StrandsScene {
    demo: StrandsDemo,
    slot: MeshSlot,
}

impl StrandsScene {
    #[must_use]
    pub fn new() -> Self {
        Self {
            demo: StrandsDemo::new(),
            slot: MeshSlot::default(),
        }
    }
}

impl Default for StrandsScene {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneDemo for StrandsScene {
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
        self.demo.advance(f64::from(tick.dt))?;
        Ok(())
    }
    fn camera(&self, view: DemoView) -> SceneCamera {
        perspective_camera(Vec3::new(0.0, 0.2, 6.0), Vec3::ZERO, view.aspect, 100.0)
    }
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult {
        self.slot.update(frame.queue, &self.demo.mesh()?)
    }
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>> {
        self.slot.frame(shared)
    }
    fn title(&self, _paused: bool) -> Option<String> {
        Some("Voxy grass & hair | Space: pause | Esc: exit".to_owned())
    }
}
