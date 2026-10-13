//! GPU-only numerical admission and retained accepted surface illumination.
//! Explicit equation checks are independent of the solver's recursive residual.
//! The floating-point max-error estimate is not an interval certificate and must
//! remain qualified against CPU references for the admitted model domain.
use crate::{SceneError, SurfaceLightGeometry};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Parameters {
    lambda: [f32; 4],
    count: u32,
    groups: u32,
    absolute_error: f32,
    relative_squared: f32,
}

#[derive(Debug)]
pub struct SurfaceLightAdmission {
    device: wgpu::Device,
    pipelines: Vec<wgpu::ComputePipeline>,
    group: wgpu::BindGroup,
    accepted: wgpu::Buffer,
    header: wgpu::Buffer,
    geometry_status: wgpu::Buffer,
    count: usize,
}
impl SurfaceLightAdmission {
    /// Bind one immutable operator topology, candidate and incident stream.
    /// Each attempt must follow ordered assembly and solve for the same pose.
    /// Consumers must check header.has_accepted before using the retained stream.
    /// # Errors
    /// Rejects invalid tolerances, buffer contracts and unsupported device limits.
    pub fn new(
        device: &wgpu::Device,
        geometry: &SurfaceLightGeometry,
        candidate: &wgpu::Buffer,
        incident: &wgpu::Buffer,
        radii: [f32; 3],
        absolute_error: f32,
        relative_squared: f32,
    ) -> Result<Self, SceneError> {
        let count = geometry.rows.len() - 1;
        let bytes = count as u64 * 16;
        let groups = count.div_ceil(64);
        let limits = device.limits();
        if !absolute_error.is_finite()
            || absolute_error <= 0.
            || !relative_squared.is_finite()
            || relative_squared <= 0.
            || radii
                .iter()
                .any(|r| !r.is_finite() || *r < 0. || !r.powi(2).is_finite())
            || [candidate, incident]
                .iter()
                .any(|b| b.size() < bytes || !b.usage().contains(wgpu::BufferUsages::STORAGE))
            || limits.max_storage_buffers_per_shader_stage < 8
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
            || groups > limits.max_compute_workgroups_per_dimension as usize
            || [
                bytes * 2,
                geometry.edges.size(),
                geometry.mass.size(),
                geometry.rows.len() as u64 * 4,
                count as u64 * 64,
            ]
            .iter()
            .any(|s| {
                *s > limits.max_buffer_size
                    || *s > u64::from(limits.max_storage_buffer_binding_size)
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        let params = Parameters {
            lambda: [radii[0].powi(2), radii[1].powi(2), radii[2].powi(2), 0.],
            count: count as u32,
            groups: groups as u32,
            absolute_error,
            relative_squared,
        };
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface light admission parameters"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let rows = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface light admission rows"),
            contents: bytemuck::cast_slice(&geometry.rows),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let storage = wgpu::BufferUsages::STORAGE;
        let partial = buffer(
            "surface light admission reductions",
            count as u64 * 64,
            storage,
        );
        let accepted = buffer(
            "accepted surface light and incident snapshot",
            bytes * 2,
            storage | wgpu::BufferUsages::COPY_SRC,
        );
        let header = buffer(
            "surface light publication header",
            32,
            storage | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let entries: Vec<_> = (0..9)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 0 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: binding < 6,
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("surface light admission layout"),
            entries: &entries,
        });
        let buffers = [
            &uniform,
            &geometry.mass,
            &rows,
            &geometry.edges,
            candidate,
            incident,
            &partial,
            &accepted,
            &header,
        ];
        let sizes = [
            32,
            geometry.mass.size(),
            geometry.rows.len() as u64 * 4,
            geometry.edges.size(),
            bytes,
            bytes,
            count as u64 * 64,
            bytes * 2,
            32,
        ];
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: b,
                    offset: 0,
                    size: std::num::NonZeroU64::new(sizes[i]),
                }),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface light admission inputs"),
            layout: &layout,
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface light admission"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_light_admission.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("surface light admission pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = [
            "initialize",
            "correct_even",
            "correct_odd",
            "audit",
            "decide",
            "publish",
        ]
        .iter()
        .map(|entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        })
        .collect();
        Ok(Self {
            device: device.clone(),
            pipelines,
            group,
            accepted,
            header,
            geometry_status: geometry.status_buffer().clone(),
            count,
        })
    }
    /// Audit and conditionally publish with no CPU wait/readback. Rejection leaves
    /// the complete accepted RGBA buffer and its generation unchanged.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_timed(encoder, None);
    }
    /// Optional timestamps measure audit, decision and conditional publication.
    pub fn encode_timed(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: Option<(&wgpu::QuerySet, u32, u32)>,
    ) {
        encoder.copy_buffer_to_buffer(&self.geometry_status, 0, &self.header, 0, 4);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("surface light admission"),
            timestamp_writes: timestamps.map(|(query_set, begin, end)| {
                wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(begin),
                    end_of_pass_write_index: Some(end),
                }
            }),
        });
        pass.set_bind_group(0, &self.group, &[]);
        for i in [0usize, 1, 2, 1, 2, 3, 4, 5] {
            pass.set_pipeline(&self.pipelines[i]);
            pass.dispatch_workgroups(
                if i == 4 {
                    1
                } else {
                    (self.count as u32).div_ceil(64)
                },
                1,
                1,
            );
        }
    }
    #[must_use]
    /// First count RGBA values are accepted diffuse light; the following count
    /// values retain the incident light from the same accepted attempt.
    pub fn accepted_buffer(&self) -> &wgpu::Buffer {
        &self.accepted
    }
    #[must_use]
    pub(crate) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.count
    }
    /// Eight 32-bit words: geometry status, current admission, has accepted,
    /// publication generation, then f32 max-error estimate/relative residual²/r²/b².
    #[must_use]
    pub fn header_buffer(&self) -> &wgpu::Buffer {
        &self.header
    }
}
