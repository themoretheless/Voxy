//! Simulated muscle ring inside a ghost shell, presented by the scene shell.
use super::pressed_key;
use crate::xray_demo::XrayDemo;
use glam::Vec3;
use voxy_render::{SceneCamera, SceneDraw, SceneGeometry, SceneTexture};
use voxy_scene_host::{
    DemoEnv, DemoFrame, DemoGpu, DemoInput, DemoKey, DemoReply, DemoResult, DemoTick, DemoView,
    FrameParts, HostRequest, MeshSlot, SceneDemo, SharedDraw, perspective_camera,
};

#[derive(Debug)]
pub struct XrayScene {
    demo: XrayDemo,
    shell: MeshSlot,
    internal: Option<SceneGeometry>,
}

impl XrayScene {
    fn with(demo: XrayDemo) -> Self {
        Self {
            demo,
            shell: MeshSlot::default(),
            internal: None,
        }
    }
    /// Abstract shell. X toggles reveal; S toggles 15% speed; arrows orbit.
    #[must_use]
    pub fn new() -> Self {
        Self::with(XrayDemo::new())
    }
    /// Uses the imported Blender body as the shell, with illustrative internals.
    /// # Errors
    /// Rejects invalid OBJ geometry or asset budgets.
    pub fn with_model() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::with(XrayDemo::with_model()?))
    }
}

impl Default for XrayScene {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneDemo for XrayScene {
    fn configure(&mut self, _env: DemoEnv) -> HostRequest {
        HostRequest {
            focus: true,
            smoke_timeout_seconds: 60,
            ..HostRequest::default()
        }
    }
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult<Option<SceneTexture>> {
        self.shell
            .upload(&gpu, &self.demo.shell(Vec3::new(0.0, 0.2, 3.0))?)?;
        self.internal = Some(gpu.scene.upload_mesh(gpu.device, &self.demo.internal()?)?);
        Ok(None)
    }
    fn release(&mut self) {
        self.shell.release();
        self.internal = None;
    }
    fn advance(&mut self, tick: DemoTick) -> DemoResult {
        if tick.smoke {
            self.demo.enabled = !(40..80).contains(&tick.frame);
            self.demo.slow = tick.frame >= 80;
        }
        self.demo.advance(f64::from(tick.dt))?;
        Ok(())
    }
    fn camera(&self, view: DemoView) -> SceneCamera {
        perspective_camera(
            self.demo.eye(view.object_world),
            view.object_world.transform_point3(self.demo.focus()),
            view.aspect,
            100.0,
        )
    }
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult {
        let eye = frame
            .object_world
            .inverse()
            .transform_point3(self.demo.eye(frame.object_world));
        let shell = self.shell.geometry_mut()?;
        shell.update(frame.queue, &self.demo.shell(eye)?)?;
        shell.set_depth_mode(self.demo.shell_depth_mode());
        if let Some(internal) = &mut self.internal {
            internal.update(frame.queue, &self.demo.internal_for_eye(eye)?)?;
            internal.set_depth_mode(self.demo.internal_depth_mode());
        }
        Ok(())
    }
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>> {
        let internal = self
            .internal
            .as_ref()
            .ok_or("X-ray internals are not created")?;
        Ok(FrameParts {
            draws: vec![
                self.shell.draw(shared)?,
                SceneDraw {
                    geometry: internal,
                    texture: shared.texture,
                    transform: shared.world_transform,
                    overlay: false,
                },
            ],
            ..FrameParts::default()
        })
    }
    fn input(&mut self, input: DemoInput) -> DemoResult<DemoReply> {
        let mut reply = DemoReply::default();
        match pressed_key(input) {
            Some(DemoKey::F) => self.demo.closeup = !self.demo.closeup,
            Some(DemoKey::L) => self.demo.local = !self.demo.local,
            Some(DemoKey::X) => self.demo.enabled = !self.demo.enabled,
            Some(DemoKey::S) => self.demo.slow = !self.demo.slow,
            Some(DemoKey::ArrowLeft) => reply.rotate_scene = -0.15,
            Some(DemoKey::ArrowRight) => reply.rotate_scene = 0.15,
            _ => {}
        }
        Ok(reply)
    }
    fn title(&self, _paused: bool) -> Option<String> {
        Some(self.demo.title())
    }
}
