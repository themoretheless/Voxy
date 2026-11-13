//! Water and oil SPH reservoirs, jets and deposited films, presented by the scene shell.
use super::{fixed_smoke_step, pressed_key};
use crate::liquid_demo::LiquidDemo;
use glam::Vec3;
use voxy_render::{SceneCamera, SceneTexture, ScreenSpaceFluidRenderer};
use voxy_scene_host::{
    DemoFrame, DemoGpu, DemoInput, DemoKey, DemoReply, DemoResult, DemoTick, DemoView, FrameParts,
    MeshSlot, Presented, SceneDemo, SharedDraw, SmokeReport, perspective_camera,
};

#[derive(Debug)]
pub struct LiquidScene {
    demo: LiquidDemo,
    /// Screen-space droplets instead of the particle mesh.
    optics: bool,
    slot: MeshSlot,
    renderer: Option<ScreenSpaceFluidRenderer>,
    optical_frames: u64,
}

impl LiquidScene {
    fn with(demo: LiquidDemo, optics: bool) -> Self {
        Self {
            demo,
            optics,
            slot: MeshSlot::default(),
            renderer: None,
            optical_frames: 0,
        }
    }
    /// Water and oil particle reservoirs with fixed-step SPH simulation.
    /// # Errors
    /// Returns invalid simulation setup errors.
    pub fn new() -> Result<Self, physics::liquid::Error> {
        Ok(Self::with(LiquidDemo::new()?, false))
    }
    /// Finite point sources with mass depletion and energy-accounted recoil.
    pub fn finite_sources() -> Result<Self, physics::liquid::Error> {
        Ok(Self::with(LiquidDemo::new_finite_sources()?, false))
    }
    /// Jets, impact spray and surface-film deposition for water and oil.
    pub fn impacts() -> Result<Self, physics::liquid::Error> {
        Ok(Self::with(LiquidDemo::new_impacts()?, true))
    }
    /// Finite energy/mass sources with recoil, impact spray and deposited film.
    pub fn finite_impacts() -> Result<Self, physics::liquid::Error> {
        Ok(Self::with(LiquidDemo::new_finite_impacts()?, true))
    }
    /// Chooses between screen-space droplets and the older particle mesh.
    #[must_use]
    pub fn with_optics(mut self, enabled: bool) -> Self {
        self.optics = enabled;
        self
    }

    fn prepare_optics(&mut self, frame: &DemoFrame<'_>) -> DemoResult {
        if frame.motion_vectors {
            return Err("screen-space droplets do not yet provide temporal motion vectors".into());
        }
        if self
            .renderer
            .as_ref()
            .is_none_or(|fluid| fluid.size() != frame.size)
        {
            self.renderer = Some(ScreenSpaceFluidRenderer::new_with_adapter(
                frame.device,
                frame.adapter,
                frame.color_format,
                frame.size[0],
                frame.size[1],
                8192,
            )?);
        }
        let mesh = self.demo.optical_background()?;
        let (particle_count, particles) = self.demo.optical_particles()?;
        self.slot.update(frame.queue, &mesh)?;
        let films = self.demo.optical_film_triangles()?;
        self.renderer
            .as_mut()
            .ok_or("missing liquid renderer")?
            .update_generated_with_film(
                frame.queue,
                frame.camera,
                particle_count,
                particles,
                &films,
                1.1,
                voxy_render::FluidDepthFilter::Bilateral,
            )?;
        Ok(())
    }
}

impl SceneDemo for LiquidScene {
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult<Option<SceneTexture>> {
        self.slot
            .reserve(&gpu, 60_024, 60_024, &self.demo.mesh()?)?;
        self.renderer = None;
        self.optical_frames = 0;
        Ok(None)
    }
    fn release(&mut self) {
        self.slot.release();
        self.renderer = None;
    }
    fn advance(&mut self, tick: DemoTick) -> DemoResult {
        self.demo.advance(fixed_smoke_step(tick))?;
        Ok(())
    }
    fn camera(&self, view: DemoView) -> SceneCamera {
        let (eye, target) = if self.optics {
            (Vec3::new(0.0, 0.1, 3.5), Vec3::new(0.0, -0.35, 0.0))
        } else {
            (Vec3::new(0.0, 1.4, 6.0), Vec3::ZERO)
        };
        perspective_camera(eye, target, view.aspect, 100.0)
    }
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult {
        if self.optics {
            self.prepare_optics(frame)
        } else {
            self.slot.update(frame.queue, &self.demo.mesh()?)
        }
    }
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>> {
        let mut parts = self.slot.frame(shared)?;
        if self.optics {
            let fluid = self.renderer.as_ref().ok_or("missing liquid renderer")?;
            parts.custom_pass = Some(Box::new(move |scene, encoder, target, draws| {
                fluid.encode(scene, encoder, target, Default::default(), draws);
            }));
        }
        Ok(parts)
    }
    fn presented(&mut self, _info: Presented) -> DemoResult {
        if self.optics {
            self.optical_frames += 1;
        }
        Ok(())
    }
    fn input(&mut self, input: DemoInput) -> DemoResult<DemoReply> {
        if pressed_key(input) == Some(DemoKey::R) {
            self.demo = self.demo.restart()?;
        }
        Ok(DemoReply::default())
    }
    fn title(&self, _paused: bool) -> Option<String> {
        Some(
            if self.optics {
                "Voxy liquid optics | water (left), oil (right) | Space: pause | R: reset | Esc: exit"
            } else {
                "Voxy liquids | water (blue), oil (gold) | Space: pause | R: reset | Esc: exit"
            }
            .to_owned(),
        )
    }
    fn verify_smoke(&self, report: SmokeReport) -> DemoResult {
        self.demo.verify()?;
        if self.optics {
            if self.optical_frames != u64::from(report.frames) {
                return Err("liquid optical passes missing from presented frame".into());
            }
            println!(
                "LIQUID OPTICAL SMOKE PASS: {} presented frames",
                self.optical_frames
            );
        }
        println!(
            "LIQUID SMOKE PASS: {} fixed steps, both reservoirs conserve mass and spread",
            self.demo.steps
        );
        Ok(())
    }
}
