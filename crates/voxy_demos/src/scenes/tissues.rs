//! Soft-tissue specimens and the skeleton-driven body motion, presented by the scene shell.
use crate::tissue_demo::TissueDemo;
use glam::Vec3;
use voxy_render::{SceneCamera, SceneTexture};
use voxy_scene_host::{
    DemoFrame, DemoGpu, DemoResult, DemoTick, DemoView, FrameParts, MeshSlot, SceneDemo,
    SharedDraw, perspective_camera,
};

#[derive(Debug)]
pub struct TissueScene {
    demo: TissueDemo,
    slot: MeshSlot,
    title: String,
}

impl TissueScene {
    fn with(demo: TissueDemo) -> Self {
        let title = if demo.is_body() {
            "Voxy skeleton + soft tissues | front / rear | walk, jump, rest | Space: pause"
        } else {
            "Voxy tissues: skin | buttock | breast | lip | sphincter | penis — Space: pause"
        };
        Self {
            demo,
            slot: MeshSlot::default(),
            title: title.to_owned(),
        }
    }
    /// Six abstract soft-tissue specimens, left to right:
    /// skin, buttock, breast, lip, sphincter, penis.
    #[must_use]
    pub fn new() -> Self {
        Self::with(TissueDemo::new())
    }
    /// Load-controlled finite-element biomechanics specimens.
    /// # Errors
    /// Returns invalid material or mesh construction errors.
    pub fn biomechanics() -> Result<Self, &'static str> {
        Ok(Self::with(TissueDemo::biomechanics()?))
    }
    /// Skeleton with front and rear secondary tissue motion.
    #[must_use]
    pub fn body() -> Self {
        Self::with(TissueDemo::body())
    }
}

impl Default for TissueScene {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneDemo for TissueScene {
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult<Option<SceneTexture>> {
        self.slot.upload(&gpu, &self.demo.mesh()?)?;
        Ok(None)
    }
    fn release(&mut self) {
        self.slot.release();
    }
    fn advance(&mut self, tick: DemoTick) -> DemoResult {
        self.demo.advance(f64::from(tick.dt))?;
        if let Some(title) = self
            .demo
            .body_motion_title()
            .or_else(|| self.demo.biomechanics_title())
        {
            self.title = title;
        }
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
        Some(self.title.clone())
    }
}
