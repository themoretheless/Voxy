//! Resident Jacobi-preconditioned CG for the same screened surface equation.
//! Iteration budgets are not convergence claims: admit results against an
//! independent equation residual and CPU reference before presentation.
use crate::{SceneError, SurfaceLightEdge};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Parameters {
    lambda: [f32; 4],
    count: u32,
    groups: u32,
    padding: [u32; 2],
}

#[derive(Debug)]
pub struct SurfaceLightPcg {
    pipelines: Vec<wgpu::ComputePipeline>,
    group: wgpu::BindGroup,
    source: wgpu::Buffer,
    output: wgpu::Buffer,
    count: usize,
    initialized: bool,
    prepared: bool,
    completed: u32,
    bytes: u64,
}
impl SurfaceLightPcg {
    /// Uses the same symmetric CSR admission as SurfaceLightDiffusion.
    /// Requires eight storage bindings and 64-lane workgroups.
    /// # Errors
    /// Rejects invalid operators and insufficient device limits before allocation.
    pub fn new(
        device: &wgpu::Device,
        mass: &[f32],
        offsets: &[u32],
        edges: &[SurfaceLightEdge],
        radii: [f32; 3],
    ) -> Result<Self, SceneError> {
        Self::new_internal(device, mass, offsets, edges, radii, None)
    }
    /// Share the resident operator assembled by SurfaceLightGeometry. The caller
    /// must encode assembly before each new solve, then admit geometry status and
    /// equation residual before publishing its output. No coefficient readback.
    /// # Errors
    /// Rejects unsupported limits or malformed immutable topology/radii.
    pub fn from_geometry(
        device: &wgpu::Device,
        geometry: &crate::SurfaceLightGeometry,
        radii: [f32; 3],
    ) -> Result<Self, SceneError> {
        let mass = vec![1.; geometry.rows.len() - 1];
        Self::new_internal(
            device,
            &mass,
            &geometry.rows,
            &geometry.neighbors,
            radii,
            Some((&geometry.mass, &geometry.edges)),
        )
    }
    fn new_internal(
        device: &wgpu::Device,
        mass: &[f32],
        offsets: &[u32],
        edges: &[SurfaceLightEdge],
        radii: [f32; 3],
        resident: Option<(&wgpu::Buffer, &wgpu::Buffer)>,
    ) -> Result<Self, SceneError> {
        super::surface_light_diffusion::validate_surface(device, mass, offsets, edges, radii)?;
        let count = mass.len();
        let groups = count.div_ceil(64);
        let limits = device.limits();
        let sizes = [
            32,
            count as u64 * 4,
            offsets.len() as u64 * 4,
            (edges.len() as u64 * 8).max(8),
            count as u64 * 16,
            count as u64 * 48,
            groups as u64 * 16,
            48,
            count as u64 * 16,
        ];
        if limits.max_storage_buffers_per_shader_stage < 8
            || sizes.iter().any(|s| {
                *s > limits.max_buffer_size
                    || *s > u64::from(limits.max_storage_buffer_binding_size)
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        let storage = wgpu::BufferUsages::STORAGE;
        let input = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let params = Parameters {
            lambda: [radii[0].powi(2), radii[1].powi(2), radii[2].powi(2), 0.],
            count: count as u32,
            groups: groups as u32,
            padding: [0; 2],
        };
        let uniform = input(
            "surface PCG parameters",
            bytemuck::bytes_of(&params),
            wgpu::BufferUsages::UNIFORM,
        );
        let masses = resident.map_or_else(
            || input("surface PCG mass", bytemuck::cast_slice(mass), storage),
            |(mass, _)| mass.clone(),
        );
        let rows = input("surface PCG rows", bytemuck::cast_slice(offsets), storage);
        let padding = [SurfaceLightEdge {
            neighbor: 0,
            weight: 0.,
        }];
        let edge_buffer = resident.map_or_else(
            || {
                input(
                    "surface PCG edges",
                    bytemuck::cast_slice(if edges.is_empty() { &padding } else { edges }),
                    storage,
                )
            },
            |(_, edges)| edges.clone(),
        );
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let source = buffer(
            "surface PCG incident",
            sizes[4],
            storage | wgpu::BufferUsages::COPY_DST,
        );
        let state = buffer("surface PCG vectors", sizes[5], storage);
        let partial = buffer("surface PCG reductions", sizes[6], storage);
        let scalars = buffer("surface PCG recurrence", sizes[7], storage);
        let output = buffer(
            "surface PCG solution",
            sizes[8],
            storage | wgpu::BufferUsages::COPY_SRC,
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
                            read_only: binding < 4,
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("surface PCG layout"),
            entries: &entries,
        });
        let buffers = [
            &uniform,
            &masses,
            &rows,
            &edge_buffer,
            &source,
            &state,
            &partial,
            &scalars,
            &output,
        ];
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface PCG inputs"),
            layout: &layout,
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface PCG"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_light_pcg.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("surface PCG pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipelines = [
            "initialize",
            "initial_rho",
            "multiply",
            "alpha",
            "update",
            "beta",
            "direction",
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
            pipelines,
            group,
            source,
            output,
            count,
            initialized: false,
            prepared: false,
            completed: 0,
            bytes: sizes.iter().sum(),
        })
    }
    /// Start a new solve at the incident source; all RGB data must be finite/nonnegative.
    /// # Errors
    /// Rejects dimensions and invalid values before writing to the queue.
    pub fn update_source(
        &mut self,
        queue: &wgpu::Queue,
        source: &[[f32; 4]],
    ) -> Result<(), SceneError> {
        if source.len() != self.count || source.iter().flatten().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(SceneError::InvalidGeometry);
        }
        queue.write_buffer(&self.source, 0, bytemuck::cast_slice(source));
        self.initialized = true;
        self.prepared = false;
        self.completed = 0;
        Ok(())
    }
    /// Initialize from an ordered resident incident-light stream. Values are
    /// intentionally not read back here; GPU publication admission is required.
    /// # Errors
    /// Rejects insufficient dimensions or missing copy usage before encoding.
    pub fn reset_from_resident_source(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        incident: &wgpu::Buffer,
    ) -> Result<(), SceneError> {
        let bytes = self.count as u64 * 16;
        if incident.size() < bytes || !incident.usage().contains(wgpu::BufferUsages::COPY_SRC) {
            return Err(SceneError::InvalidGeometry);
        }
        encoder.copy_buffer_to_buffer(incident, 0, &self.source, 0, bytes);
        self.initialized = true;
        self.prepared = false;
        self.completed = 0;
        Ok(())
    }
    /// Ordered GPU-only recurrence. Optional timestamps span initialization and all iterations.
    /// # Errors
    /// Rejects missing source or cumulative budgets beyond 8192 iterations.
    pub fn encode_timed(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        iterations: u32,
        timestamps: Option<(&wgpu::QuerySet, u32, u32)>,
    ) -> Result<(), SceneError> {
        if !self.initialized
            || !(1..=8192).contains(&iterations)
            || self.completed + iterations > 8192
        {
            return Err(SceneError::InvalidGeometry);
        }
        let mut dispatches = Vec::with_capacity(iterations as usize * 5 + 2);
        if !self.prepared {
            dispatches.extend([0usize, 1, 6]);
        }
        for _ in 0..iterations {
            dispatches.extend([2usize, 3, 4, 5, 6]);
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("surface PCG solve"),
            timestamp_writes: timestamps.map(|(query_set, begin, end)| {
                wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(begin),
                    end_of_pass_write_index: Some(end),
                }
            }),
        });
        pass.set_bind_group(0, &self.group, &[]);
        for &pipeline in &dispatches {
            pass.set_pipeline(&self.pipelines[pipeline]);
            pass.dispatch_workgroups(
                if matches!(pipeline, 1 | 3 | 5) {
                    1
                } else {
                    (self.count as u32).div_ceil(64)
                },
                1,
                1,
            );
        }
        drop(pass);
        self.prepared = true;
        self.completed += iterations;
        Ok(())
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Buffer {
        &self.output
    }
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.bytes
    }
}
