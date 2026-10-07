//! Angle-weighted authored-normal transport, matching the native body reference.
use crate::{SceneError, SceneGeometry, SceneMesh};
use glam::Vec3;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Triangle {
    ids: [u32; 4],
    u: [f32; 4],
    v: [f32; 4],
    n: [f32; 4],
    angles: [f32; 4],
}
#[derive(Debug)]
pub(crate) struct SurfaceNormalTransport {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    _inputs: Vec<crate::ComputeStorage>,
    count: u32,
}
impl SurfaceNormalTransport {
    pub(crate) fn new(
        device: &wgpu::Device,
        mesh: &SceneMesh,
        geometry: &SceneGeometry,
    ) -> Result<Self, SceneError> {
        use crate::compute_memory::ManagedBufferDescriptor;
        let mut triangles = Vec::new();
        let mut adjacency = vec![Vec::new(); mesh.vertices().len()];
        for ids in mesh.indices().chunks_exact(3) {
            let p: [Vec3; 3] = std::array::from_fn(|k| {
                Vec3::from_array(mesh.vertices()[ids[k] as usize].position)
            });
            let u = p[1] - p[0];
            let v = p[2] - p[0];
            let Some(n) = u.cross(v).try_normalize() else {
                continue;
            };
            let angles: [f32; 3] = std::array::from_fn(|k| {
                let x = p[(k + 1) % 3] - p[k];
                let y = p[(k + 2) % 3] - p[k];
                x.cross(y).length().atan2(x.dot(y))
            });
            let index = triangles.len() as u32;
            triangles.push(Triangle {
                ids: [ids[0], ids[1], ids[2], 0],
                u: u.extend(0.).to_array(),
                v: v.extend(0.).to_array(),
                n: n.extend(0.).to_array(),
                angles: [angles[0], angles[1], angles[2], 0.],
            });
            for k in 0..3 {
                adjacency[ids[k] as usize].push([index, k as u32]);
            }
        }
        let mut rows = vec![0_u32];
        let mut corners = Vec::new();
        for row in adjacency {
            corners.extend(row);
            rows.push(u32::try_from(corners.len()).map_err(|_| SceneError::InvalidGeometry)?);
        }
        if triangles.is_empty() {
            triangles.push(Triangle::zeroed());
        }
        if corners.is_empty() {
            corners.push([0_u32; 2]);
        }
        use bytemuck::Zeroable;
        let generated;
        let authored = if let Some(normals) = mesh.authored_normals() {
            normals
        } else {
            generated = crate::scene::smooth_normals(mesh);
            &generated
        };
        let data = [
            bytemuck::cast_slice(authored),
            bytemuck::cast_slice(&rows),
            bytemuck::cast_slice(&corners),
            bytemuck::cast_slice(&triangles),
        ];
        if device.limits().max_storage_buffers_per_shader_stage < 6
            || data.iter().any(|d| {
                d.len() as u64 > u64::from(device.limits().max_storage_buffer_binding_size)
            })
        {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        let descriptors: Vec<_> = data
            .iter()
            .map(|data| ManagedBufferDescriptor {
                label: "surface normal immutable input",
                size: data.len() as u64,
                contents: Some(data),
                usage: wgpu::BufferUsages::STORAGE,
            })
            .collect();
        let inputs = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffer_batch(&descriptors)
            .map_err(|_| SceneError::MemoryBudget)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface authored normal transport"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_normal_transport.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("surface normal transport"),
            layout: None,
            module: &shader,
            entry_point: Some("transport"),
            compilation_options: Default::default(),
            cache: None,
        });
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: geometry.deformation_vertices().as_entire_binding(),
        }];
        entries.extend(
            inputs
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupEntry {
                    binding: i as u32 + 1,
                    resource: b.as_entire_binding(),
                }),
        );
        entries.push(wgpu::BindGroupEntry {
            binding: 5,
            resource: geometry.deformation_normals().as_entire_binding(),
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface normal inputs"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        Ok(Self {
            pipeline,
            group,
            _inputs: inputs,
            count: mesh.vertices().len() as u32,
        })
    }
    pub(crate) fn set_prefix(&mut self, count: u32) {
        self.count = count;
    }
    pub(crate) fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("surface normal transport"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(self.count.div_ceil(64), 1, 1);
    }
}
