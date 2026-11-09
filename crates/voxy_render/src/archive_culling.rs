//! Stable GPU frustum compaction of immutable, full precision instance payloads.
use crate::{ComputeError, ComputeMemoryBudget, ComputeStorage};
use glam::{Vec3, Vec4};
#[derive(Debug)]
pub struct ArchiveInstanceCulling {
    bind: wgpu::BindGroup,
    mark: wgpu::ComputePipeline,
    scan: wgpu::ComputePipeline,
    scatter: wgpu::ComputePipeline,
    planes: ComputeStorage,
    arguments: ComputeStorage,
    _mesh: ComputeStorage,
    _spheres: ComputeStorage,
    _flags: ComputeStorage,
    _blocks: ComputeStorage,
    groups: u32,
}
impl ArchiveInstanceCulling {
    /// Input/output buffers must retain STORAGE and VERTEX usage and outlive this
    /// producer. No geometry, material data or instance payload is approximated.
    pub fn new(
        device: &wgpu::Device,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
        spheres: &[(Vec3, f32)],
        index_count: u32,
    ) -> Result<Self, ComputeError> {
        let count = u32::try_from(spheres.len()).map_err(|_| ComputeError::InvalidBuffer)?;
        if input.size() < u64::from(count) * 112
            || output.size() < u64::from(count) * 112
            || !input.usage().contains(wgpu::BufferUsages::STORAGE)
            || !output.usage().contains(wgpu::BufferUsages::STORAGE)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let groups = count.div_ceil(256);
        if count == 0
            || groups > device.limits().max_compute_workgroups_per_dimension
            || device.limits().max_compute_invocations_per_workgroup < 256
        {
            return Err(ComputeError::Unsupported);
        }
        let budget = ComputeMemoryBudget::for_device(device);
        let make = |label, bytes: &[u8], usage| {
            budget
                .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                    label,
                    size: bytes.len() as u64,
                    contents: Some(bytes),
                    usage,
                }])
                .map(|[b]| b)
        };
        let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let storage = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        let planes = make("instance frustum planes", &[0; 128], uniform)?;
        let mesh = make(
            "instance culling counts",
            bytemuck::cast_slice(&[count, groups, index_count, 0]),
            uniform,
        )?;
        let sphere_data: Vec<[f32; 4]> = spheres.iter().map(|(c, r)| [c.x, c.y, c.z, *r]).collect();
        let sphere_buffer = make(
            "original conservative instance spheres",
            bytemuck::cast_slice(&sphere_data),
            storage,
        )?;
        let flags = make(
            "stable visibility prefixes",
            &vec![0; count as usize * 8],
            storage,
        )?;
        let blocks = make(
            "visibility block prefixes",
            &vec![0; groups as usize * 8],
            storage,
        )?;
        let arguments = make(
            "full geometry indirect arguments",
            &[0; 20],
            storage | wgpu::BufferUsages::INDIRECT,
        )?;
        let entries: Vec<_> = (0..8)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding < 2 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: binding == 2 || binding == 3,
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stable instance culling"),
            entries: &entries,
        });
        let buffers: [&wgpu::Buffer; 8] = [
            &planes,
            &mesh,
            input,
            &sphere_buffer,
            &flags,
            &blocks,
            output,
            &arguments,
        ];
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("full precision stable frustum compaction"),
            source: wgpu::ShaderSource::Wgsl(
                if device.features().contains(wgpu::Features::SUBGROUP)
                    && std::env::var_os("VOXY_STREET_SUBGROUP_CULLING").is_some()
                {
                    concat!(
                        include_str!("archive_culling.wgsl"),
                        include_str!("archive_culling_subgroup.wgsl")
                    )
                    .into()
                } else {
                    concat!(
                        include_str!("archive_culling.wgsl"),
                        include_str!("archive_culling_mark.wgsl")
                    )
                    .into()
                },
            ),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |name| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(name),
                layout: Some(&pl),
                module: &shader,
                entry_point: Some(name),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Ok(Self {
            bind,
            mark: pipeline("mark"),
            scan: pipeline("scan"),
            scatter: pipeline("scatter"),
            planes,
            arguments,
            _mesh: mesh,
            _spheres: sphere_buffer,
            _flags: flags,
            _blocks: blocks,
            groups,
        })
    }
    pub fn prepare(&self, queue: &wgpu::Queue, planes: [Vec4; 6]) {
        let mut values = [[0_f32; 4]; 8];
        for (i, p) in planes.into_iter().enumerate() {
            values[i] = p.to_array();
            values[6 + i / 4][i % 4] = p.truncate().length();
        }
        queue.write_buffer(&self.planes, 0, bytemuck::cast_slice(&values));
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (pipeline, groups) in [
            (&self.mark, self.groups),
            (&self.scan, 1),
            (&self.scatter, self.groups),
        ] {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("stable full geometry instance culling"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.dispatch_workgroups(groups, 1, 1);
        }
    }
    pub fn indirect(&self) -> &wgpu::Buffer {
        &self.arguments
    }
}

/// Projects a view-space bounding sphere (`center_view` with forward `-Z` or `+Z` distance `|z| > radius + z_near`)
/// into conservative screen-space UV bounds `[min_u, min_v, max_u, max_v]` in `[0, 1]^4` using Mara & McGuire (2013)
/// exact perspective sphere-horizon projection for Hi-Z occlusion culling.
#[must_use]
pub fn project_sphere_screen_aabb(
    center_view: [f32; 3],
    radius: f32,
    z_near: f32,
    proj_00: f32,
    proj_11: f32,
) -> Option<[f32; 4]> {
    if center_view.iter().any(|c| !c.is_finite())
        || !radius.is_finite()
        || !z_near.is_finite()
        || !proj_00.is_finite()
        || !proj_11.is_finite()
        || radius <= 0.0
        || z_near <= 0.0
        || proj_00 <= 0.0
        || proj_11 <= 0.0
    {
        return None;
    }
    let cz = center_view[2].abs();
    if cz <= radius + z_near {
        return None;
    }
    let r2 = radius * radius;
    let axis_bounds = |c: f32, p_scale: f32| -> Option<(f32, f32)> {
        let d2 = c * c + cz * cz;
        if d2 <= r2 {
            return None;
        }
        let a = (d2 - r2).sqrt();
        let denom_min = cz * a + c * radius;
        let denom_max = cz * a - c * radius;
        if denom_min <= 1e-6 || denom_max <= 1e-6 {
            return None;
        }
        let ndc_min = ((c * a - cz * radius) / denom_min) * p_scale;
        let ndc_max = ((c * a + cz * radius) / denom_max) * p_scale;
        Some((ndc_min.min(ndc_max), ndc_min.max(ndc_max)))
    };
    let (min_x, max_x) = axis_bounds(center_view[0], proj_00)?;
    let (min_y, max_y) = axis_bounds(center_view[1], proj_11)?;
    let min_u = (0.5 * min_x + 0.5).clamp(0.0, 1.0);
    let max_u = (0.5 * max_x + 0.5).clamp(0.0, 1.0);
    let min_v = (0.5 - 0.5 * max_y).clamp(0.0, 1.0);
    let max_v = (0.5 - 0.5 * min_y).clamp(0.0, 1.0);
    Some([min_u, min_v, max_u, max_v])
}

#[cfg(test)]
mod sphere_aabb_tests {
    use super::*;

    #[test]
    fn mara_mcguire_sphere_projection_brackets_screen_center_and_rejects_near_clip() {
        let aabb = project_sphere_screen_aabb([0.0, 0.0, -10.0], 1.0, 0.1, 1.0, 1.0).unwrap();
        assert!(aabb[0] < 0.5 && aabb[2] > 0.5);
        assert!(aabb[1] < 0.5 && aabb[3] > 0.5);
        assert!(project_sphere_screen_aabb([0.0, 0.0, -0.5], 0.6, 0.1, 1.0, 1.0).is_none());
    }
}
