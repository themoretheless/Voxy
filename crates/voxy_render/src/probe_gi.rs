//! GPU projection of rendered cube faces into order-1 SH irradiance probes.

/// Probe capacity of the archive scene's preallocated SH buffer.
pub(crate) const MAX_PROBES: usize = 4096;
/// Bytes per probe: four RGB(A) SH coefficients.
pub(crate) const PROBE_BYTES: u64 = 4 * 16;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Bake {
    forward: [f32; 4],
    right: [f32; 4],
    up: [f32; 4],
    square: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ProbeGrid {
    pub origin: [f32; 4],
    pub spacing: [f32; 4],
    pub dims: [u32; 4],
}

#[derive(Debug)]
pub(crate) struct ProbeProjector {
    pipeline: wgpu::ComputePipeline,
    uniform: wgpu::Buffer,
}

impl ProbeProjector {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("irradiance probe projection"),
            source: wgpu::ShaderSource::Wgsl(include_str!("probe_gi.wgsl").into()),
        });
        Self {
            pipeline: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("irradiance probe projection"),
                layout: None,
                module: &module,
                entry_point: Some("cs_project"),
                compilation_options: Default::default(),
                cache: None,
            }),
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("irradiance probe face"),
                size: std::mem::size_of::<Bake>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    /// Accumulate one rendered face (90-degree vertical FOV, centred square of
    /// side `height`) into probe `index`. Face 0 resets the slot.
    #[allow(clippy::too_many_arguments)]
    pub fn project(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        face_view: &wgpu::TextureView,
        (width, height): (u32, u32),
        probes: &wgpu::Buffer,
        index: usize,
        face: usize,
        basis: [glam::Vec3; 3],
        inverse_exposure: f32,
    ) {
        let [forward, right, up] = basis;
        let size = height.min(width) as f32;
        let bake = Bake {
            forward: [forward.x, forward.y, forward.z, index as f32],
            right: [right.x, right.y, right.z, face as f32],
            up: [up.x, up.y, up.z, inverse_exposure],
            square: [((width as f32 - size) * 0.5).floor(), size, (size / 128.).max(1.).floor(), 0.],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&bake));
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("irradiance probe face"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(face_view) },
                wgpu::BindGroupEntry { binding: 1, resource: self.uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: probes.as_entire_binding() },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        queue.submit([encoder.finish()]);
    }
}

/// Grid of probe positions covering `min..=max` at roughly `spacing`.
/// Every axis has at least two probes so trilinear lookup is always defined.
pub(crate) fn grid(min: glam::Vec3, max: glam::Vec3, spacing: f32) -> Option<(ProbeGrid, Vec<glam::Vec3>)> {
    if !(min.is_finite() && max.is_finite() && spacing.is_finite() && spacing > 0.) || max.cmple(min).any() {
        return None;
    }
    let extent = max - min;
    let dims = (extent / spacing).ceil().as_uvec3().max(glam::UVec3::ONE) + 1;
    if (dims.x * dims.y * dims.z) as usize > MAX_PROBES {
        return None;
    }
    let step = extent / (dims - 1).as_vec3();
    let mut points = Vec::with_capacity((dims.x * dims.y * dims.z) as usize);
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                points.push(min + step * glam::UVec3::new(x, y, z).as_vec3());
            }
        }
    }
    Some((
        ProbeGrid {
            origin: [min.x, min.y, min.z, 1.],
            spacing: [step.x, step.y, step.z, 0.],
            dims: [dims.x, dims.y, dims.z, 0],
        },
        points,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn grid_covers_bounds_with_at_least_two_probes_per_axis() {
        let (g, points) = grid(Vec3::ZERO, Vec3::new(10., 0.5, 30.), 4.).unwrap();
        assert_eq!(g.dims, [4, 2, 9, 0]);
        assert_eq!(points.len(), 4 * 2 * 9);
        assert_eq!(points[0], Vec3::ZERO);
        assert!((*points.last().unwrap() - Vec3::new(10., 0.5, 30.)).length() < 1e-4);
    }

    #[test]
    fn grid_rejects_degenerate_or_oversized_requests() {
        assert!(grid(Vec3::ZERO, Vec3::ZERO, 1.).is_none());
        assert!(grid(Vec3::ZERO, Vec3::ONE, 0.).is_none());
        assert!(grid(Vec3::ZERO, Vec3::splat(1000.), 1.).is_none());
    }
}
