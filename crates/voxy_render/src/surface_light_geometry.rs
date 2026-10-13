//! Resident assembly of the existing positive-weight surface light operator.
//! Topology is compiled once. Per-pose positions may come directly from skinning.
use crate::{SceneError, SurfaceLightEdge};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Parameters {
    vertices: u32,
    triangles: u32,
    edges: u32,
    stride: u32,
    edge_rows: u32,
    padding: [u32; 3],
}

#[derive(Debug)]
pub struct SurfaceLightGeometry {
    pipelines: Vec<wgpu::ComputePipeline>,
    group: wgpu::BindGroup,
    pub(super) mass: wgpu::Buffer,
    pub(super) edges: wgpu::Buffer,
    pub(super) rows: Vec<u32>,
    pub(super) neighbors: Vec<SurfaceLightEdge>,
    status: wgpu::Buffer,
    count: usize,
    triangles: usize,
}
impl SurfaceLightGeometry {
    /// Positions occupy the first three f32 words of each `stride`-word vertex.
    /// The same storage stream can be written by a preceding skinning dispatch.
    /// Geometry status and an independent residual/parity gate must pass before
    /// publishing a solve. A zero status is necessary, not sufficient admission.
    /// # Errors
    /// Rejects malformed topology, incompatible buffers and device limits.
    pub fn new(
        device: &wgpu::Device,
        positions: &wgpu::Buffer,
        count: usize,
        stride: u32,
        triangles: &[[u32; 3]],
    ) -> Result<Self, SceneError> {
        let limits = device.limits();
        if count == 0 || count > u32::MAX as usize || stride < 3 {
            return Err(SceneError::InvalidGeometry);
        }
        let words = (count as u64 - 1) * u64::from(stride) + 3;
        if count == 0
            || count > u32::MAX as usize
            || stride < 3
            || words > u64::from(u32::MAX)
            || triangles.is_empty()
            || triangles.len() > u32::MAX as usize / 9
            || words * 4 > positions.size()
            || !positions.usage().contains(wgpu::BufferUsages::STORAGE)
            || limits.max_storage_buffers_per_shader_stage < 8
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
            || triangles.iter().any(|t| {
                t.iter().any(|&i| i as usize >= count)
                    || t[0] == t[1]
                    || t[1] == t[2]
                    || t[2] == t[0]
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        let mut vertex_incidence = vec![Vec::new(); count];
        let mut edge_incidence = std::collections::BTreeMap::<(u32, u32), Vec<u32>>::new();
        for (index, t) in triangles.iter().enumerate() {
            for &v in t {
                vertex_incidence[v as usize].push(index as u32 * 3);
            }
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                edge_incidence
                    .entry((a.min(b), a.max(b)))
                    .or_default()
                    .push(index as u32 * 3 + k as u32);
            }
        }
        let mut adjacency = vec![std::collections::BTreeMap::new(); count];
        for ((a, b), codes) in edge_incidence {
            adjacency[a as usize].insert(b, codes.clone());
            adjacency[b as usize].insert(a, codes);
        }
        let mut rows = vec![0u32];
        let mut neighbors = Vec::new();
        let mut edge_codes = Vec::new();
        for row in adjacency {
            for (neighbor, codes) in row {
                neighbors.push(SurfaceLightEdge {
                    neighbor,
                    weight: 0.,
                });
                edge_codes.push(codes);
            }
            rows.push(u32::try_from(neighbors.len()).map_err(|_| SceneError::InvalidGeometry)?);
        }
        let mut incidence = Vec::new();
        let mut offsets = vec![0u32];
        for row in vertex_incidence {
            incidence.extend(row);
            offsets.push(incidence.len() as u32);
        }
        // Separate row ranges share one offset array; each entry is an absolute
        // incidence index, including the first edge row after the mass sentinel.
        offsets.push(incidence.len() as u32);
        for row in edge_codes {
            incidence.extend(row);
            offsets.push(incidence.len() as u32);
        }
        let counts = [triangles.len(), count, neighbors.len()];
        if counts
            .iter()
            .any(|n| n.div_ceil(64) > limits.max_compute_workgroups_per_dimension as usize)
        {
            return Err(SceneError::InvalidGeometry);
        }
        let sizes = [
            32,
            words * 4,
            triangles.len() as u64 * 16,
            offsets.len() as u64 * 4,
            incidence.len() as u64 * 4,
            triangles.len() as u64 * 16,
            count as u64 * 4,
            neighbors.len() as u64 * 8,
            4,
        ];
        if sizes
            .iter()
            .any(|s| *s > limits.max_buffer_size || *s > limits.max_storage_buffer_binding_size)
        {
            return Err(SceneError::InvalidGeometry);
        }
        let input = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let storage = wgpu::BufferUsages::STORAGE;
        let params = Parameters {
            vertices: count as u32,
            triangles: triangles.len() as u32,
            edges: neighbors.len() as u32,
            stride,
            edge_rows: count as u32 + 1,
            padding: [0; 3],
        };
        let uniform = input(
            "surface assembly parameters",
            bytemuck::bytes_of(&params),
            wgpu::BufferUsages::UNIFORM,
        );
        let triangles_gpu: Vec<_> = triangles.iter().map(|t| [t[0], t[1], t[2], 0]).collect();
        let topology = input(
            "surface assembly triangles",
            bytemuck::cast_slice(&triangles_gpu),
            storage,
        );
        let offsets_gpu = input(
            "surface assembly incidence rows",
            bytemuck::cast_slice(&offsets),
            storage,
        );
        let incidence_gpu = input(
            "surface assembly incidence",
            bytemuck::cast_slice(&incidence),
            storage,
        );
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let coefficients = buffer("surface triangle coefficients", sizes[5], storage);
        let mass = buffer(
            "surface assembled mass",
            sizes[6],
            storage | wgpu::BufferUsages::COPY_SRC,
        );
        let edges = input(
            "surface assembled edges",
            bytemuck::cast_slice(&neighbors),
            storage | wgpu::BufferUsages::COPY_SRC,
        );
        let status = buffer(
            "surface assembly status",
            4,
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
                            read_only: binding < 5,
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("surface assembly layout"),
            entries: &entries,
        });
        let buffers = [
            &uniform,
            positions,
            &topology,
            &offsets_gpu,
            &incidence_gpu,
            &coefficients,
            &mass,
            &edges,
            &status,
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
            label: Some("surface assembly inputs"),
            layout: &layout,
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface assembly"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_light_geometry.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("surface assembly pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = ["triangle_coefficients", "gather_mass", "gather_edges"]
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
            pipelines,
            group,
            mass,
            edges,
            rows,
            neighbors,
            status,
            count,
            triangles: triangles.len(),
        })
    }
    /// Rebuild coefficients from the current resident geometry, without readback.
    /// Status is reset per invocation: bit 0 invalid triangles, bit 1 sum overflow.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_timed(encoder, None);
    }
    /// Optional timestamps measure coefficient assembly, excluding CPU setup.
    pub fn encode_timed(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: Option<(&wgpu::QuerySet, u32, u32)>,
    ) {
        encoder.clear_buffer(&self.status, 0, Some(4));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("surface operator assembly"),
            timestamp_writes: timestamps.map(|(query_set, begin, end)| {
                wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(begin),
                    end_of_pass_write_index: Some(end),
                }
            }),
        });
        pass.set_bind_group(0, &self.group, &[]);
        for (i, count) in [self.triangles, self.count, self.neighbors.len()]
            .into_iter()
            .enumerate()
        {
            pass.set_pipeline(&self.pipelines[i]);
            pass.dispatch_workgroups((count as u32).div_ceil(64), 1, 1);
        }
    }
    #[must_use]
    pub fn mass_buffer(&self) -> &wgpu::Buffer {
        &self.mass
    }
    #[must_use]
    pub fn edge_buffer(&self) -> &wgpu::Buffer {
        &self.edges
    }
    #[must_use]
    pub fn status_buffer(&self) -> &wgpu::Buffer {
        &self.status
    }
}
