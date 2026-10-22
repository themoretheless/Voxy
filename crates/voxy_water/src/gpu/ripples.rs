//! Interactive ripples around the viewer: splashes and wakes propagate, reflect off
//! basin walls and fade. Renderers sample [`RippleField::view`] with a repeating
//! sampler at `world_xz / RippleField::extent()` (toroidal addressing).

use super::{bind_group, compute_pipeline};
use glam::Vec2;
#[path = "ripple_clock.rs"]
mod clock;
use clock::RippleClock;

/// Grid resolution of the ripple window.
pub const RIPPLE_RES: u32 = 256;
const MAX_IMPULSES: usize = 64;
const GROUPS: usize = 4;
pub const SHADER: &str = include_str!("../shaders/ripples.wgsl");

#[derive(Debug)]
pub struct RippleField {
    pipeline: wgpu::ComputePipeline,
    /// First A -> B, B -> A, steady A -> B, and copy-only B -> A.
    groups: [wgpu::BindGroup; GROUPS],
    uniforms: [wgpu::Buffer; GROUPS],
    impulse_buffer: wgpu::Buffer,
    view: wgpu::TextureView,
    impulses: Vec<[f32; 4]>,
    cell: f32,
    speed: f32,
    damping: f32,
    mask: Option<(Vec2, Vec2)>,
    snap: [i32; 2],
    first: bool,
    clock: RippleClock,
}

impl RippleField {
    /// A window `extent` metres wide; `speed` is the ripple celerity (m/s).
    pub fn new(device: &wgpu::Device, extent: f32, speed: f32) -> Self {
        let pipeline = compute_pipeline(device, "water ripples", SHADER);
        let texture = |label| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: RIPPLE_RES,
                    height: RIPPLE_RES,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let (a, b) = (texture("ripples a"), texture("ripples b"));
        let (va, vb) = (
            a.create_view(&Default::default()),
            b.create_view(&Default::default()),
        );
        let uniforms = [0, 1, 2, 3].map(|_| {
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
        let groups =
            [(&va, &vb, 0), (&vb, &va, 1), (&va, &vb, 2), (&vb, &va, 3)].map(|(src, dst, step)| {
                bind_group(
                    device,
                    &layout,
                    "ripple step",
                    &[
                        uniforms[step].as_entire_binding(),
                        R::TextureView(src),
                        R::TextureView(dst),
                        impulse_buffer.as_entire_binding(),
                    ],
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
            clock: RippleClock::new(extent / RIPPLE_RES as f32, speed)
                .expect("finite positive ripple extent and nonnegative speed"),
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
            self.impulses
                .push([at.x, at.y, radius.max(self.cell), height]);
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
    /// Submitted solver time and pending credit; neither is a GPU wall duration.
    pub fn encoded_seconds(&self) -> f64 {
        self.clock.encoded_seconds()
    }
    pub fn pending_seconds(&self) -> f64 {
        self.clock.pending_seconds()
    }
    pub fn requested_seconds(&self) -> f64 {
        self.clock.requested_seconds
    }
    pub fn encoded_ticks(&self) -> u64 {
        self.clock.encoded_ticks
    }
    pub fn tick_rate(&self) -> u32 {
        self.clock.rate
    }

    pub fn encode(
        &mut self,
        queue: &wgpu::Queue,
        pass: &mut wgpu::ComputePass,
        center: Vec2,
        dt: f32,
    ) {
        self.encode_elapsed(queue, pass, center, f64::from(dt));
    }
    /// Fixed ticks preserve all elapsed time. A bounded batch retains any debt for
    /// the next encode. Submit the encoder before calling this again, since its
    /// uniforms are per submission. Paused frames only remap the spatial window.
    pub fn encode_elapsed(
        &mut self,
        queue: &wgpu::Queue,
        pass: &mut wgpu::ComputePass,
        center: Vec2,
        dt: f64,
    ) {
        let steps = self
            .clock
            .plan(dt)
            .expect("finite supported nonnegative ripple timestep");
        let snap = [
            (center.x / self.cell).floor() as i32,
            (center.y / self.cell).floor() as i32,
        ];
        let previous = if self.first {
            [snap[0] + 100_000, snap[1] + 100_000]
        } else {
            self.snap
        };
        self.first = false;
        let h = self.clock.step_seconds();
        let courant = (self.speed * h / self.cell).powi(2);
        let damping = self.clock.damping(self.damping);
        let (mask_min, mask_max, masked) = self
            .mask
            .map_or((Vec2::ZERO, Vec2::ZERO, 0.0), |(a, b)| (a, b, 1.0));
        for group in 0..GROUPS {
            let old = if group == 0 { previous } else { snap };
            let impulses = if group == 0 && steps > 0 {
                self.impulses.len() as f32
            } else {
                0.
            };
            let mut u = [0u32; 16];
            u[..4].copy_from_slice(&[snap[0], snap[1], old[0], old[1]].map(|v| v as u32));
            u[4..8].copy_from_slice(&[self.cell, courant, damping, impulses].map(f32::to_bits));
            u[8..12].copy_from_slice(&[mask_min.x, mask_min.y, masked, 0.].map(f32::to_bits));
            u[12..16].copy_from_slice(
                &[
                    mask_max.x,
                    mask_max.y,
                    0.,
                    if steps == 0 || group == 3 { 1. } else { 0. },
                ]
                .map(f32::to_bits),
            );
            queue.write_buffer(&self.uniforms[group], 0, bytemuck::cast_slice(&u));
        }
        if steps > 0 && !self.impulses.is_empty() {
            queue.write_buffer(
                &self.impulse_buffer,
                0,
                bytemuck::cast_slice(&self.impulses),
            );
            self.impulses.clear();
        }
        self.snap = snap;
        pass.set_pipeline(&self.pipeline);
        for step in 0..if steps == 0 { 2 } else { steps } {
            let group = if step == 0 {
                0
            } else if step % 2 == 1 {
                1
            } else {
                2
            };
            pass.set_bind_group(0, &self.groups[group], &[]);
            pass.dispatch_workgroups(RIPPLE_RES / 8, RIPPLE_RES / 8, 1);
        }
        if steps % 2 == 1 {
            pass.set_bind_group(0, &self.groups[3], &[]);
            pass.dispatch_workgroups(RIPPLE_RES / 8, RIPPLE_RES / 8, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn step(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        field: &mut RippleField,
        dt: f64,
        center: Vec2,
    ) {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            field.encode_elapsed(queue, &mut pass, center, dt);
        }
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
    }
    fn state(device: &wgpu::Device, queue: &wgpu::Queue, field: &RippleField) -> Vec<u8> {
        let size = u64::from(RIPPLE_RES) * u64::from(RIPPLE_RES) * 8;
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ripple state test"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            field.view.texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(RIPPLE_RES * 8),
                    rows_per_image: Some(RIPPLE_RES),
                },
            },
            field.view.texture().size(),
        );
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = read.slice(..).get_mapped_range().unwrap().to_vec();
        read.unmap();
        bytes
    }
    #[test]
    #[ignore = "requires a native non-CPU GPU; run explicitly for clock qualification"]
    fn ripple_fixed_ticks_are_frame_partition_invariant_on_gpu() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        eprintln!("RIPPLE CLOCK GPU {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let create = || {
            let mut f = RippleField::new(&device, 24., 0.6);
            f.set_mask(Some((Vec2::splat(-2.), Vec2::splat(2.))));
            f.splash(Vec2::ZERO, 0.4, 0.02);
            f
        };
        let mut whole = create();
        let mut split = create();
        step(&device, &queue, &mut whole, 0.25, Vec2::ZERO);
        for _ in 0..32 {
            step(&device, &queue, &mut split, 1. / 128., Vec2::ZERO);
        }
        assert_eq!(whole.encoded_ticks(), 60);
        assert_eq!(split.encoded_ticks(), 60);
        assert_eq!(whole.pending_seconds(), 0.);
        assert_eq!(split.pending_seconds(), 0.);
        let expected = state(&device, &queue, &whole);
        let actual = state(&device, &queue, &split);
        assert_eq!(
            expected, actual,
            "one 0.25-second frame and 32 fractional frames must preserve every state byte"
        );
        assert!(
            actual
                .chunks_exact(8)
                .any(|p| p[..4].iter().any(|&b| b != 0)),
            "the impulse must actually evolve"
        );
        assert!(
            actual
                .chunks_exact(2)
                .all(|p| u16::from_le_bytes([p[0], p[1]]) & 0x7c00 != 0x7c00),
            "finite half state"
        );
        whole.splash(Vec2::ZERO, 0.3, 0.01);
        step(&device, &queue, &mut whole, 0., Vec2::ZERO);
        assert_eq!(
            state(&device, &queue, &whole),
            expected,
            "paused state must not advance or damp"
        );
        assert_eq!(
            whole.impulses.len(),
            1,
            "paused frames must retain pending impulses"
        );
        let center = Vec2::new(whole.cell, 0.);
        step(&device, &queue, &mut whole, 0., center);
        assert_eq!(
            state(&device, &queue, &whole),
            expected,
            "scroll preserves overlapping world cells without advancing time"
        );
        assert_eq!(whole.encoded_ticks(), 60);
        assert_eq!(whole.impulses.len(), 1);
        eprintln!(
            "RIPPLE FIXED CLOCK PASS state_bytes={} ticks=60 elapsed=0.25 paused_exact=true scroll_exact=true pending_impulse_preserved=true",
            actual.len()
        );
    }
}
