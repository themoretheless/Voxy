//! GPU-resident iteration for the existing screened surface irradiance equation.
//! Encoding an iteration budget does not assert convergence. Keep the CPU solver
//! as a reference until full-body residual and presentation gates pass.
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SurfaceLightEdge {
    pub neighbor: u32,
    pub weight: f32,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Parameters {
    lambda: [f32; 4],
    count: u32,
    padding: [u32; 3],
    alpha: [f32; 4],
    beta: [f32; 4],
}
#[derive(Debug)]
pub struct SurfaceLightDiffusion {
    pipeline: wgpu::ComputePipeline,
    groups: [wgpu::BindGroup; 2],
    source: wgpu::Buffer,
    values: [wgpu::Buffer; 2],
    count: usize,
    current: usize,
    initialized: bool,
    bytes: u64,
    accelerated: bool,
    completed: u32,
    uniform_stride: u32,
}
impl SurfaceLightDiffusion {
    /// Topology is a symmetric CSR matrix: each undirected edge appears in both
    /// endpoint rows. Mass and weights must come from the same surface assembly.
    /// # Errors
    /// Rejects malformed CSR, negative/nonfinite data and unsupported limits.
    pub fn new(
        device: &wgpu::Device,
        mass: &[f32],
        offsets: &[u32],
        edges: &[SurfaceLightEdge],
        radii: [f32; 3],
    ) -> Result<Self, crate::SceneError> {
        Self::new_internal(device, mass, offsets, edges, radii, false)
    }
    /// Chebyshev iteration using a conservative Gershgorin spectral bound.
    /// Admission still requires an independent equation and CPU parity audit.
    pub fn new_accelerated(
        device: &wgpu::Device,
        mass: &[f32],
        offsets: &[u32],
        edges: &[SurfaceLightEdge],
        radii: [f32; 3],
    ) -> Result<Self, crate::SceneError> {
        Self::new_internal(device, mass, offsets, edges, radii, true)
    }
    fn new_internal(
        device: &wgpu::Device,
        mass: &[f32],
        offsets: &[u32],
        edges: &[SurfaceLightEdge],
        radii: [f32; 3],
        accelerated: bool,
    ) -> Result<Self, crate::SceneError> {
        let n = mass.len();
        let limits = device.limits();
        if n == 0
            || n > u32::MAX as usize
            || n as u64 > u64::from(limits.max_storage_buffer_binding_size) / 16
            || offsets.len() != n + 1
            || offsets[0] != 0
            || offsets[n] as usize != edges.len()
            || offsets.windows(2).any(|p| p[0] > p[1])
            || mass.iter().any(|v| !v.is_finite() || *v < 0.)
            || radii
                .iter()
                .any(|v| !v.is_finite() || *v < 0. || !v.powi(2).is_finite())
            || edges
                .iter()
                .any(|e| e.neighbor as usize >= n || !e.weight.is_finite() || e.weight < 0.)
            || mass
                .iter()
                .enumerate()
                .any(|(i, m)| *m == 0. && offsets[i] != offsets[i + 1])
            || limits.max_storage_buffers_per_shader_stage < 7
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
            || n.div_ceil(64) > limits.max_compute_workgroups_per_dimension as usize
        {
            return Err(crate::SceneError::InvalidGeometry);
        }
        // Check symmetry before GPU admission; immutable topology is checked once.
        let mut entries = std::collections::BTreeMap::new();
        for i in 0..n {
            for e in &edges[offsets[i] as usize..offsets[i + 1] as usize] {
                if e.neighbor as usize == i
                    || entries.insert((i, e.neighbor as usize), e.weight).is_some()
                {
                    return Err(crate::SceneError::InvalidGeometry);
                }
            }
        }
        if entries
            .iter()
            .any(|(&(a, b), &w)| entries.get(&(b, a)) != Some(&w))
        {
            return Err(crate::SceneError::InvalidGeometry);
        }
        let mut delta = [0_f64; 3];
        for i in 0..n {
            let degree: f64 = edges[offsets[i] as usize..offsets[i + 1] as usize]
                .iter()
                .map(|e| f64::from(e.weight))
                .sum();
            if degree > f64::from(f32::MAX)
                || radii.iter().any(|r| {
                    f64::from(mass[i]) + f64::from(r.powi(2)) * degree > f64::from(f32::MAX)
                })
            {
                return Err(crate::SceneError::InvalidGeometry);
            }
            for c in 0..3 {
                let laplace = f64::from(radii[c].powi(2)) * degree;
                if mass[i] > 0. {
                    delta[c] = delta[c].max(laplace / (f64::from(mass[i]) + laplace));
                }
            }
        }
        let sizes = [
            (n * 4) as u64,
            (offsets.len() * 4) as u64,
            (edges.len() * 8).max(8) as u64,
            (n * 16) as u64,
        ];
        if sizes.iter().any(|s| {
            *s > limits.max_storage_buffer_binding_size as u64 || *s > limits.max_buffer_size
        }) {
            return Err(crate::SceneError::InvalidGeometry);
        }
        let params = Parameters {
            lambda: [radii[0].powi(2), radii[1].powi(2), radii[2].powi(2), 0.],
            count: n as u32,
            padding: [0; 3],
            alpha: [1.; 4],
            beta: [0.; 4],
        };
        let input = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let uniform_stride = 64_u32.next_multiple_of(limits.min_uniform_buffer_offset_alignment);
        let slots = if accelerated { 8192 } else { 1 };
        let mut uniform_data = vec![0_u8; slots * uniform_stride as usize];
        let mut rho = delta;
        for iteration in 0..slots {
            let mut p = params;
            if accelerated && iteration > 0 {
                for c in 0..3 {
                    if delta[c] > 0. {
                        let next_rho = 1. / (2. / delta[c] - rho[c]);
                        p.alpha[c] = (2. * next_rho / delta[c]) as f32;
                        p.beta[c] = (next_rho * rho[c]) as f32;
                        rho[c] = next_rho;
                    }
                }
            }
            let start = iteration * uniform_stride as usize;
            uniform_data[start..start + 64].copy_from_slice(bytemuck::bytes_of(&p));
        }
        let uniform = input(
            "surface light parameters",
            &uniform_data,
            wgpu::BufferUsages::UNIFORM,
        );
        let masses = input(
            "surface light mass",
            bytemuck::cast_slice(mass),
            wgpu::BufferUsages::STORAGE,
        );
        let rows = input(
            "surface light rows",
            bytemuck::cast_slice(offsets),
            wgpu::BufferUsages::STORAGE,
        );
        let padding = [SurfaceLightEdge {
            neighbor: 0,
            weight: 0.,
        }];
        let edge_buffer = input(
            "surface light edges",
            bytemuck::cast_slice(if edges.is_empty() { &padding } else { edges }),
            wgpu::BufferUsages::STORAGE,
        );
        let buffer = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: sizes[3],
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };
        let source = buffer("surface light source");
        let values = [buffer("surface light ping"), buffer("surface light pong")];
        let velocity = buffer("surface light Chebyshev velocity");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface light diffusion"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_light_diffusion.wgsl").into()),
        });
        let layout_entries: Vec<_> = (0..8)
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
                    has_dynamic_offset: binding == 0,
                    min_binding_size: if binding == 0 {
                        std::num::NonZeroU64::new(64)
                    } else {
                        None
                    },
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("surface light layout"),
            entries: &layout_entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("surface light pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("surface light iteration"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("diffuse"),
            compilation_options: Default::default(),
            cache: None,
        });
        let groups = std::array::from_fn(|i| {
            let buffers = [
                &uniform,
                &masses,
                &rows,
                &edge_buffer,
                &source,
                &values[i],
                &values[1 - i],
                &velocity,
            ];
            let entries: Vec<_> = buffers
                .iter()
                .enumerate()
                .map(|(binding, b)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: if binding == 0 {
                        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: b,
                            offset: 0,
                            size: std::num::NonZeroU64::new(64),
                        })
                    } else {
                        b.as_entire_binding()
                    },
                })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("surface light iteration inputs"),
                layout: &layout,
                entries: &entries,
            })
        });
        Ok(Self {
            pipeline,
            groups,
            source,
            values,
            count: n,
            current: 0,
            initialized: false,
            bytes: uniform_data.len() as u64 + sizes[0] + sizes[1] + sizes[2] + 4 * sizes[3],
            accelerated,
            completed: 0,
            uniform_stride,
        })
    }
    /// Upload incident irradiance and reset the initial iterate to that source.
    /// # Errors
    /// Rejects changed dimensions or nonfinite/negative values before any upload.
    pub fn update_source(
        &mut self,
        queue: &wgpu::Queue,
        source: &[[f32; 4]],
    ) -> Result<(), crate::SceneError> {
        if source.len() != self.count || source.iter().flatten().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(crate::SceneError::InvalidGeometry);
        }
        queue.write_buffer(&self.source, 0, bytemuck::cast_slice(source));
        queue.write_buffer(&self.values[0], 0, bytemuck::cast_slice(source));
        self.current = 0;
        self.completed = 0;
        self.initialized = true;
        Ok(())
    }
    /// Update incident light while retaining the resident solution as a warm start.
    /// Starts a fresh Chebyshev recurrence; old momentum is ignored on its first pass.
    /// This does not admit a fixed iteration count without a residual/parity gate.
    pub fn update_incident(
        &mut self,
        queue: &wgpu::Queue,
        source: &[[f32; 4]],
    ) -> Result<(), crate::SceneError> {
        if !self.initialized {
            return self.update_source(queue, source);
        }
        if source.len() != self.count || source.iter().flatten().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(crate::SceneError::InvalidGeometry);
        }
        queue.write_buffer(&self.source, 0, bytemuck::cast_slice(source));
        self.completed = 0;
        Ok(())
    }
    /// Add ordered GPU iterations without CPU readback or waiting.
    /// # Errors
    /// Rejects missing source and invalid iteration budgets.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        iterations: u32,
    ) -> Result<(), crate::SceneError> {
        self.encode_timed(encoder, iterations, None)
    }
    /// Optionally records first-pass start and last-pass end into an enabled timestamp query.
    /// The caller owns query resolution; no readback is added to the render path.
    pub fn encode_timed(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        iterations: u32,
        timestamps: Option<(&wgpu::QuerySet, u32, u32)>,
    ) -> Result<(), crate::SceneError> {
        if !self.initialized
            || !(1..=8192).contains(&iterations)
            || (self.accelerated && self.completed + iterations > 8192)
        {
            return Err(crate::SceneError::InvalidGeometry);
        }
        for step in 0..iterations {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("surface light iteration"),
                timestamp_writes: timestamps.and_then(|(query_set, begin, end)| {
                    if step == 0 || step + 1 == iterations {
                        Some(wgpu::ComputePassTimestampWrites {
                            query_set,
                            beginning_of_pass_write_index: (step == 0).then_some(begin),
                            end_of_pass_write_index: (step + 1 == iterations).then_some(end),
                        })
                    } else {
                        None
                    }
                }),
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(
                0,
                &self.groups[self.current],
                &[if self.accelerated {
                    self.completed * self.uniform_stride
                } else {
                    0
                }],
            );
            pass.dispatch_workgroups((self.count as u32).div_ceil(64), 1, 1);
            drop(pass);
            self.current = 1 - self.current;
            self.completed += 1;
        }
        Ok(())
    }
    /// Storage buffer for subsequent shading after the encoded iterations.
    #[must_use]
    pub fn output(&self) -> &wgpu::Buffer {
        &self.values[self.current]
    }
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.bytes
    }
}
