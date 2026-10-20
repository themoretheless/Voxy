//! Interactive ripples around the viewer: splashes and wakes propagate, reflect off
//! basin walls and fade. Renderers sample [`RippleField::view`] with a repeating
//! sampler at `world_xz / RippleField::extent()` (toroidal addressing).

use super::{bind_group, compute_pipeline};
use glam::Vec2;

/// Grid resolution of the ripple window.
pub const RIPPLE_RES: u32 = 256;
const MAX_IMPULSES: usize = 64;
const SUBSTEPS: usize = 2;
pub const SHADER: &str = include_str!("../shaders/ripples.wgsl");

#[derive(Debug)]
pub struct RippleField {
    pipeline: wgpu::ComputePipeline,
    /// Substep groups: A -> B then B -> A, so the result always lands in A.
    groups: [wgpu::BindGroup; SUBSTEPS],
    uniforms: [wgpu::Buffer; SUBSTEPS],
    impulse_buffer: wgpu::Buffer,
    view: wgpu::TextureView,
    impulses: Vec<[f32; 4]>,
    cell: f32,
    speed: f32,
    damping: f32,
    mask: Option<(Vec2, Vec2)>,
    snap: [i32; 2],
    first: bool,
}

impl RippleField {
    /// A window `extent` metres wide; `speed` is the ripple celerity (m/s).
    pub fn new(device: &wgpu::Device, extent: f32, speed: f32) -> Self {
        let pipeline = compute_pipeline(device, "water ripples", SHADER);
        let texture = |label| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: RIPPLE_RES, height: RIPPLE_RES, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            })
        };
        let (a, b) = (texture("ripples a"), texture("ripples b"));
        let (va, vb) = (a.create_view(&Default::default()), b.create_view(&Default::default()));
        let uniforms = [0, 1].map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ripple step"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let impulse_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ripple impulses"),
            size: (MAX_IMPULSES * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = pipeline.get_bind_group_layout(0);
        use wgpu::BindingResource as R;
        let groups = [(&va, &vb, 0), (&vb, &va, 1)].map(|(src, dst, step)| {
            bind_group(
                device,
                &layout,
                "ripple step",
                &[uniforms[step].as_entire_binding(), R::TextureView(src), R::TextureView(dst), impulse_buffer.as_entire_binding()],
            )
        });
        Self {
            pipeline,
            groups,
            uniforms,
            impulse_buffer,
            view: va,
            impulses: Vec::new(),
            cell: extent / RIPPLE_RES as f32,
            speed,
            damping: 0.996,
            mask: None,
            snap: [0, 0],
            first: true,
        }
    }

    /// Ripple heights: r = current, g = previous step.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn extent(&self) -> f32 {
        self.cell * RIPPLE_RES as f32
    }

    /// World xz of the window centre after the last encode.
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.snap[0] as f32 + 0.5, self.snap[1] as f32 + 0.5) * self.cell
    }

    /// Confines ripples to a rectangle (e.g. a pool) whose edges reflect them.
    pub fn set_mask(&mut self, mask: Option<(Vec2, Vec2)>) {
        self.mask = mask;
    }

    /// Raises (positive) or depresses the surface by `height` metres around `at`.
    pub fn splash(&mut self, at: Vec2, radius: f32, height: f32) {
        if self.impulses.len() < MAX_IMPULSES {
            self.impulses.push([at.x, at.y, radius.max(self.cell), height]);
        }
    }

    /// A body moving from `from` to `to` this frame pushes a bow wave along the path.
    pub fn wake(&mut self, from: Vec2, to: Vec2, radius: f32, height: f32) {
        let distance = from.distance(to);
        let steps = ((distance / radius.max(1e-3)).ceil() as usize).clamp(1, 8);
        for i in 0..steps {
            let t = (i as f32 + 0.5) / steps as f32;
            self.splash(from.lerp(to, t), radius, height / steps as f32);
        }
    }

    /// Steps the field `dt` seconds with the window centred on `center`.
    pub fn encode(&mut self, queue: &wgpu::Queue, pass: &mut wgpu::ComputePass, center: Vec2, dt: f32) {
        let snap = [(center.x / self.cell).floor() as i32, (center.y / self.cell).floor() as i32];
        let previous = if self.first { [snap[0] + 100_000, snap[1] + 100_000] } else { self.snap };
        self.first = false;
        let h = (dt / SUBSTEPS as f32).min(0.5 * self.cell / self.speed.max(1e-3));
        let courant = (self.speed * h / self.cell).powi(2);
        let (mask_min, mask_max, masked) = self.mask.map_or((Vec2::ZERO, Vec2::ZERO, 0.0), |(a, b)| (a, b, 1.0));
        for step in 0..SUBSTEPS {
            let old = if step == 0 { previous } else { snap };
            let impulses = if step == 0 { self.impulses.len() as f32 } else { 0.0 };
            let mut u = [0u32; 16];
            u[..4].copy_from_slice(&[snap[0], snap[1], old[0], old[1]].map(|v| v as u32));
            u[4..8].copy_from_slice(&[self.cell, courant, self.damping, impulses].map(f32::to_bits));
            u[8..12].copy_from_slice(&[mask_min.x, mask_min.y, masked, 0.0].map(f32::to_bits));
            u[12..16].copy_from_slice(&[mask_max.x, mask_max.y, 0.0, 0.0].map(f32::to_bits));
            queue.write_buffer(&self.uniforms[step], 0, bytemuck::cast_slice(&u));
        }
        if !self.impulses.is_empty() {
            queue.write_buffer(&self.impulse_buffer, 0, bytemuck::cast_slice(&self.impulses));
            self.impulses.clear();
        }
        self.snap = snap;
        pass.set_pipeline(&self.pipeline);
        for group in &self.groups {
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(RIPPLE_RES / 8, RIPPLE_RES / 8, 1);
        }
    }
}
