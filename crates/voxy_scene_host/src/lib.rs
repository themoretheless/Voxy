//! Contract between the windowed scene shell (`voxy_shell::SceneApp`) and the
//! demo it presents. The shell owns the window, the surface, the frame loop and
//! the shared draw resources; a demo owns its simulation and its own GPU
//! geometry and answers the frame phases below. No winit here, so demos stay
//! free of the window and the event loop.
use glam::{Mat4, Vec3};
use std::time::Instant;
use voxy_render::{
    RendererError, SceneCamera, SceneDraw, SceneGeometry, SceneMesh, SceneProjection,
    SceneRenderer, SceneTexture, SceneTransform,
};

pub mod orbit;
pub use orbit::{OrbitController, OrbitLimits};

/// Result of a demo phase; the shell reports the error and stops.
pub type DemoResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Facts about the run that are known before the window exists.
#[derive(Clone, Copy, Debug)]
pub struct DemoEnv {
    /// The run is an automated smoke test with a fixed step and a frame budget.
    pub smoke: bool,
}

/// What a demo asks of the window and the surface before they are created.
#[derive(Clone, Copy, Debug)]
pub struct HostRequest {
    /// Initial window size in physical pixels; `None` keeps the shell default.
    pub physical_size: Option<[u32; 2]>,
    /// Render the scene with 4x MSAA.
    pub msaa4: bool,
    /// Present without waiting for vertical sync when the surface allows it.
    pub unthrottled_presentation: bool,
    /// Bring the window to the front once it is visible.
    pub focus: bool,
    /// Wall-clock budget of a smoke run before it is reported as timed out.
    pub smoke_timeout_seconds: u64,
    /// Print the shell's per-frame phase trace when the profiling variables are set.
    pub frame_trace: bool,
}

impl Default for HostRequest {
    fn default() -> Self {
        Self {
            physical_size: None,
            msaa4: false,
            unthrottled_presentation: false,
            focus: false,
            smoke_timeout_seconds: 20,
            frame_trace: false,
        }
    }
}

/// Device access for creating a demo's GPU resources.
#[derive(Debug)]
pub struct DemoGpu<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub scene: &'a mut SceneRenderer,
    /// Surface size in physical pixels.
    pub size: [u32; 2],
    pub paused: bool,
}

/// One unpaused simulation tick.
#[derive(Clone, Copy, Debug)]
pub struct DemoTick {
    /// Wall-clock seconds since the previous frame.
    pub elapsed: f32,
    /// `elapsed` clamped to the shell's largest accepted frame step.
    pub dt: f32,
    pub smoke: bool,
    /// Frames presented so far.
    pub frame: u32,
}

/// Inputs for the demo's camera.
#[derive(Clone, Copy, Debug)]
pub struct DemoView {
    pub aspect: f32,
    /// World matrix of the shell's primary scene object.
    pub object_world: Mat4,
}

/// Per-frame upload access, before the draw list is built.
#[derive(Debug)]
pub struct DemoFrame<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub adapter: &'a wgpu::Adapter,
    pub color_format: wgpu::TextureFormat,
    pub scene: &'a SceneRenderer,
    /// Texture shared by every world draw; a demo may replace it.
    pub texture: &'a mut SceneTexture,
    pub world_transform: &'a SceneTransform,
    pub camera: SceneCamera,
    pub object_world: Mat4,
    /// Surface size in physical pixels.
    pub size: [u32; 2],
    /// Frames presented so far.
    pub frame: u32,
    pub paused: bool,
    /// The shell renders the float velocity pass this run.
    pub motion_vectors: bool,
    /// Start of the frame when the shell prints its phase trace.
    pub trace_started: Option<Instant>,
}

/// Shell-owned bindings a demo's draws refer to.
#[derive(Clone, Copy, Debug)]
pub struct SharedDraw<'a> {
    pub texture: &'a SceneTexture,
    pub world_transform: &'a SceneTransform,
    pub overlay_transform: &'a SceneTransform,
}

/// Encodes a demo's compute work into the frame's command encoder.
pub type PrepareFn<'a> =
    Box<dyn FnOnce(&wgpu::Queue, &mut wgpu::CommandEncoder) -> Result<(), RendererError> + 'a>;

/// Replaces the shell's scene pass; receives the complete draw list of the frame.
pub type CustomPassFn<'a> = Box<
    dyn FnOnce(&SceneRenderer, &mut wgpu::CommandEncoder, &wgpu::TextureView, &[SceneDraw<'_>])
        + 'a,
>;

/// Everything a demo contributes to one frame. Draws and the preparation
/// closure come from one `&mut` borrow because the closure mutates compute
/// state next to the geometry the draws refer to.
#[derive(Default)]
pub struct FrameParts<'a> {
    pub draws: Vec<SceneDraw<'a>>,
    /// Refractive layers drawn after the opaque scene.
    pub refractive: Vec<SceneDraw<'a>>,
    pub prepare: Option<PrepareFn<'a>>,
    pub custom_pass: Option<CustomPassFn<'a>>,
}

impl std::fmt::Debug for FrameParts<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameParts")
            .field("draws", &self.draws.len())
            .field("refractive", &self.refractive.len())
            .field("prepare", &self.prepare.is_some())
            .field("custom_pass", &self.custom_pass.is_some())
            .finish()
    }
}

/// A frame that reached the screen.
#[derive(Clone, Copy, Debug)]
pub struct Presented {
    /// Index of the presented frame (frames presented before it).
    pub frame: u32,
    pub size: [u32; 2],
    /// The shell's primary object is active, so world draws were submitted.
    pub object_active: bool,
    pub smoke: bool,
}

/// Keys demos react to, independent of the windowing library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoKey {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    BracketLeft,
    BracketRight,
    B,
    C,
    F,
    G,
    J,
    K,
    L,
    M,
    P,
    R,
    S,
    X,
}

/// Scroll wheel movement along the vertical axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DemoScroll {
    Lines(f32),
    Pixels(f64),
}

/// Window input forwarded to the demo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DemoInput {
    Key {
        key: DemoKey,
        pressed: bool,
        repeat: bool,
    },
    PrimaryButton {
        pressed: bool,
    },
    PointerMoved {
        x: f64,
        y: f64,
    },
    Scroll(DemoScroll),
    FocusLost,
}

/// What the shell should do in response to an input.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DemoReply {
    /// Radians added to the shell's scene rotation.
    pub rotate_scene: f32,
}

/// Shell-side facts for the final smoke verification.
#[derive(Clone, Copy, Debug)]
pub struct SmokeReport {
    pub frames: u32,
    pub resize_stages: u8,
    pub motion_vectors: bool,
}

/// A scene presented by the shell. Methods are the phases of a frame.
pub trait SceneDemo: std::fmt::Debug {
    /// Called before the window and the surface are created, and again before
    /// they are recreated after a device loss.
    fn configure(&mut self, _env: DemoEnv) -> HostRequest {
        HostRequest::default()
    }
    /// Creates the demo's GPU resources; repeated after a device loss. A
    /// returned texture replaces the shell's shared world texture.
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult<Option<SceneTexture>>;
    /// Drops GPU resources when the surface goes away.
    fn release(&mut self) {}
    /// Runs every frame, paused or not, before `advance`.
    fn begin_frame(&mut self, _dt: f32) {}
    /// Advances the simulation; skipped while paused.
    fn advance(&mut self, tick: DemoTick) -> DemoResult;
    fn camera(&self, view: DemoView) -> SceneCamera;
    /// Uploads this frame's geometry.
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult;
    /// Draw list and compute preparation for this frame.
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>>;
    fn presented(&mut self, _info: Presented) -> DemoResult {
        Ok(())
    }
    fn input(&mut self, _input: DemoInput) -> DemoResult<DemoReply> {
        Ok(DemoReply::default())
    }
    /// Window title for the current state.
    fn title(&self, _paused: bool) -> Option<String> {
        None
    }
    /// The smoke run may finish once the frame budget is reached.
    fn smoke_ready(&self) -> bool {
        true
    }
    /// Checks the demo's own smoke invariants and prints its pass lines.
    fn verify_smoke(&self, _report: SmokeReport) -> DemoResult {
        Ok(())
    }
}

/// Perspective camera shared by the shell and the demos: 55 degree vertical
/// field of view, Y up, near plane at 0.1.
#[must_use]
pub fn perspective_camera(eye: Vec3, target: Vec3, aspect: f32, far: f32) -> SceneCamera {
    SceneCamera {
        eye,
        target,
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 55_f32.to_radians(),
            aspect,
            near: 0.1,
            far,
        },
    }
}

/// The single world geometry of a demo that re-uploads one mesh per frame.
#[derive(Debug, Default)]
pub struct MeshSlot {
    geometry: Option<SceneGeometry>,
}

impl MeshSlot {
    /// Uploads `mesh` as a fixed-size geometry.
    pub fn upload(&mut self, gpu: &DemoGpu<'_>, mesh: &SceneMesh) -> DemoResult {
        self.geometry = Some(gpu.scene.upload_mesh(gpu.device, mesh)?);
        Ok(())
    }
    /// Reserves room for a mesh whose vertex and index counts change, then fills it.
    pub fn reserve(
        &mut self,
        gpu: &DemoGpu<'_>,
        vertices: usize,
        indices: usize,
        mesh: &SceneMesh,
    ) -> DemoResult {
        let mut geometry = gpu.scene.reserve_geometry(gpu.device, vertices, indices)?;
        geometry.update(gpu.queue, mesh)?;
        self.geometry = Some(geometry);
        Ok(())
    }
    pub fn release(&mut self) {
        self.geometry = None;
    }
    pub fn geometry(&self) -> DemoResult<&SceneGeometry> {
        self.geometry
            .as_ref()
            .ok_or_else(|| "demo geometry is not created".into())
    }
    pub fn geometry_mut(&mut self) -> DemoResult<&mut SceneGeometry> {
        self.geometry
            .as_mut()
            .ok_or_else(|| "demo geometry is not created".into())
    }
    pub fn update(&mut self, queue: &wgpu::Queue, mesh: &SceneMesh) -> DemoResult {
        self.geometry_mut()?.update(queue, mesh)?;
        Ok(())
    }
    /// One opaque world draw of the slot.
    pub fn draw<'a>(&'a self, shared: SharedDraw<'a>) -> DemoResult<SceneDraw<'a>> {
        Ok(SceneDraw {
            geometry: self.geometry()?,
            texture: shared.texture,
            transform: shared.world_transform,
            overlay: false,
        })
    }
    /// Frame consisting of the slot's single draw.
    pub fn frame<'a>(&'a self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>> {
        Ok(FrameParts {
            draws: vec![self.draw(shared)?],
            ..FrameParts::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_request_defaults_match_the_plain_scene() {
        let request = HostRequest::default();
        assert_eq!(request.physical_size, None);
        assert!(!request.msaa4 && !request.unthrottled_presentation && !request.focus);
        assert_eq!(request.smoke_timeout_seconds, 20);
    }

    #[test]
    fn perspective_camera_uses_the_shared_lens() {
        let camera = perspective_camera(Vec3::new(3., 2., 4.), Vec3::ZERO, 1.5, 100.);
        assert_eq!(camera.up, Vec3::Y);
        assert_eq!(
            camera.projection,
            SceneProjection::Perspective {
                vertical_fov: 55_f32.to_radians(),
                aspect: 1.5,
                near: 0.1,
                far: 100.,
            }
        );
        assert!(camera.view_projection().is_ok());
    }

    #[test]
    fn empty_mesh_slot_reports_missing_geometry() {
        let slot = MeshSlot::default();
        assert!(slot.geometry().is_err());
    }
}
