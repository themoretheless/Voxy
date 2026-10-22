//! One derived immutable geometry bounds table per retained source-query owner.
use crate::{ComputeMemoryBudget, ComputeStorage, RayScene, RaySceneError};
#[path = "world_geometry_bounds_layout.rs"]
mod layout;
#[derive(Debug)]
pub(super) struct WorldGeometryBounds {
    pub buffer: ComputeStorage,
    pipelines: Vec<wgpu::ComputePipeline>,
    groups: Vec<wgpu::BindGroup>,
    triangles: u32,
    check: bool,
}
impl WorldGeometryBounds {
    pub fn new(
        device: &wgpu::Device,
        scene: &RayScene,
        check: bool,
    ) -> Result<Self, RaySceneError> {
        let counts = scene
            .archive_source_primitive_counts()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let limits = device.limits();
        let layout = layout::Layout::new(
            &counts,
            limits
                .max_storage_buffer_binding_size
                .min(limits.max_buffer_size),
            limits.max_compute_workgroups_per_dimension,
        )
        .ok_or(RaySceneError::Capacity)?;
        if check && layout.triangles.div_ceil(64) > limits.max_compute_workgroups_per_dimension {
            return Err(RaySceneError::Capacity);
        }
        let mut words = vec![0u32; layout.words as usize];
        words[1] = layout.slots;
        words[2] = layout.triangles;
        words[3] = layout.records;
        words[4] = 19;
        words[7] = u32::from(check);
        let revision = scene.geometry_revision();
        words[8] = revision as u32;
        words[9] = (revision >> 32) as u32;
        words[16..16 + layout.prefix.len()].copy_from_slice(&layout.prefix);
        let [buffer] = ComputeMemoryBudget::for_device(device)
            .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                label: "same-owner exact source geometry intervals and normals",
                size: u64::from(layout.words) * 4,
                contents: Some(bytemuck::cast_slice(&words)),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            }])
            .map_err(|_| RaySceneError::Capacity)?;
        let source = [
            include_str!("world_ray_deferred_input.wgsl"),
            include_str!("world_ray_source_bindings.wgsl"),
            include_str!("archive_projection_pair.wgsl"),
            include_str!("world_triangle_pair.wgsl"),
            include_str!("world_ray_bvh_interval.wgsl"),
            include_str!("world_geometry_bounds.wgsl"),
            include_str!("world_geometry_bounds_prepare.wgsl"),
        ]
        .join("\n");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("original source geometry bounds preparation"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let sources = scene
            .archive_candidate_source_bindings()
            .ok_or(RaySceneError::InvalidGeometry)?;
        let mut pipelines = Vec::new();
        let mut groups = Vec::new();
        for entry in ["cs_bounds_prepare", "cs_bounds_check", "cs_bounds_ready"] {
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            });
            let entries = if entry == "cs_bounds_ready" {
                vec![wgpu::BindGroupEntry {
                    binding: 11,
                    resource: buffer.as_entire_binding(),
                }]
            } else {
                vec![
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: sources[0].clone(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: sources[1].clone(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: sources[2].clone(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: scene
                            .archive_candidate_instance_layout()
                            .ok_or(RaySceneError::InvalidGeometry)?,
                    },
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: buffer.as_entire_binding(),
                    },
                ]
            };
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(entry),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            }));
            pipelines.push(pipeline);
        }
        Ok(Self {
            buffer,
            pipelines,
            groups,
            triangles: layout.triangles,
            check,
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        // Repeated scene builds must reset checked totals and readiness before reuse.
        encoder.clear_buffer(&self.buffer, 0, Some(4));
        encoder.clear_buffer(&self.buffer, 20, Some(8));
        for index in [0usize, 1, 2] {
            if index == 1 && !self.check {
                continue;
            }
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("prepare exact source geometry bounds cache"),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipelines[index]);
            pass.set_bind_group(0, &self.groups[index], &[]);
            pass.dispatch_workgroups(
                match index {
                    0 => self.triangles.div_ceil(64),
                    1 => self.triangles.div_ceil(64),
                    _ => 1,
                },
                1,
                1,
            );
        }
    }
    pub fn source(&self) -> String {
        // Keep the current original-source certificate/material policy. Only replace
        // geometry-only arithmetic in certificate kernels, never the exact predicate.
        let original = include_str!("world_source_certificate.wgsl");
        let begin = original
            .find(" let info=exact_layout[slot];")
            .expect("source certificate geometry start");
        let end = original
            .find(" let direction=WorldInterval3(")
            .expect("source certificate geometry end");
        let relative = r" let info=exact_layout[slot];let va=info.y+exact_indices[first];
 let geometry=source_cached_geometry_bounds(slot,first);let e1=geometry.e1;let e2=geometry.e2;
 var delta:array<WorldInterval,3>;
 for(var axis=0u;axis<3u;axis++) {
  var z=wi_add(wi_sub(wi_source_word(exact_ray_word(ray+axis)),wi_source_word(exact_instances[slot*28u+12u+axis])),wi_source_word(exact_ray_word(ray+4u+axis)));
  for(var k=0u;k<3u;k++) {
   let matrix=wi_source_word(exact_instances[slot*28u+k*4u+axis]);let a=wi_source_word(exact_vertices[va*12u+k]);
   z=wi_sub(z,wi_mul(matrix,a));
  }
  delta[axis]=z;
 }
";
        let source = format!("{}{}{}", &original[..begin], relative, &original[end..]);
        let marker = "let normal=wi3_cross(edge1,edge2);";
        assert_eq!(source.matches(marker).count(), 1);
        [
            include_str!("world_geometry_bounds.wgsl").to_owned(),
            include_str!("world_geometry_bounds_read.wgsl").to_owned(),
            source.replace(marker, "let normal=geometry.normal;"),
        ]
        .join("\n")
    }
}
