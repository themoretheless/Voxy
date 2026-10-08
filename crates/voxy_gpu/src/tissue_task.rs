//! Cross-platform GPU rest-material search backend using WGSL compute shaders.
//! Physical force/energy admission and nonlinear Newton steps remain native.

use physics::biomechanics::{TissueSearchBackend, TissueSearchOperation, TissueSearchSnapshot, Vec3};
use std::sync::Arc;
use voxy_render::ComputeError;
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub enum GpuTissueError {
    InvalidInput,
    Budget,
    NumericalOverflow,
    Compute(ComputeError),
    Wgpu(String),
}

impl std::fmt::Display for GpuTissueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compute(err) => write!(f, "GPU tissue compute error: {err}"),
            Self::Wgpu(msg) => write!(f, "GPU tissue wgpu error: {msg}"),
            other => write!(f, "GPU tissue error: {other:?}"),
        }
    }
}

impl std::error::Error for GpuTissueError {}

fn layout(n: usize, e: usize, limit: usize) -> Result<(usize, usize, usize), GpuTissueError> {
    if n == 0 || e == 0 || n > u32::MAX as usize || e > u32::MAX as usize {
        return Err(GpuTissueError::InvalidInput);
    }
    // offsets_base = 2 + 5*n + 19*e
    // refs_base = offsets_base + n + 1
    // force_base = refs_base + 4*e = 3 + 6*n + 23*e
    // output_base = force_base + 12*e = 3 + 6*n + 35*e
    // total_words = output_base + 3*n = 3 + 9*n + 35*e
    let n6 = n.checked_mul(6).ok_or(GpuTissueError::Budget)?;
    let e23 = e.checked_mul(23).ok_or(GpuTissueError::Budget)?;
    let force_base = n6
        .checked_add(e23)
        .and_then(|x| x.checked_add(3))
        .ok_or(GpuTissueError::Budget)?;

    let e12 = e.checked_mul(12).ok_or(GpuTissueError::Budget)?;
    let output_base = force_base
        .checked_add(e12)
        .ok_or(GpuTissueError::Budget)?;

    let n3 = n.checked_mul(3).ok_or(GpuTissueError::Budget)?;
    let total_words = output_base
        .checked_add(n3)
        .ok_or(GpuTissueError::Budget)?;

    let total_bytes = total_words
        .checked_mul(4)
        .ok_or(GpuTissueError::Budget)?;

    if total_bytes > limit {
        return Err(GpuTissueError::Budget);
    }
    Ok((force_base, output_base, total_words))
}

#[allow(clippy::cast_possible_truncation)]
fn pack(
    snapshot: &TissueSearchSnapshot,
    direction: &[[f64; 3]],
    limit: usize,
) -> Result<(Vec<f32>, usize), GpuTissueError> {
    let n = snapshot.pinned().len();
    let e = snapshot.elements().len();
    let (_, output_base, total_words) = layout(n, e, limit)?;

    if direction.len() != n || direction.iter().flatten().any(|x| !x.is_finite()) {
        return Err(GpuTissueError::InvalidInput);
    }

    let mut data = Vec::with_capacity(total_words);
    data.push(f32::from_bits(n as u32));
    data.push(f32::from_bits(e as u32));

    for &pin in snapshot.pinned() {
        data.push(if pin { 1.0 } else { 0.0 });
    }
    for &weight in snapshot.inertia_weights() {
        if !weight.is_finite() || weight < 0.0 {
            return Err(GpuTissueError::InvalidInput);
        }
        data.push(weight as f32);
    }
    for dir in direction {
        data.push(dir[0] as f32);
        data.push(dir[1] as f32);
        data.push(dir[2] as f32);
    }

    let mut refs = vec![Vec::new(); n];
    for (id, elem) in snapshot.elements().iter().enumerate() {
        for &node in &elem.nodes {
            data.push(f32::from_bits(node as u32));
        }
        for g in &elem.gradients_m_inverse {
            data.push(g[0] as f32);
            data.push(g[1] as f32);
            data.push(g[2] as f32);
        }
        data.push(elem.reference_volume_m3 as f32);
        data.push(elem.shear_pa as f32);
        data.push(elem.bulk_pa as f32);

        for (corner, &node) in elem.nodes.iter().enumerate() {
            refs[node].push((4 * id + corner) as u32);
        }
    }

    let mut offset = 0u32;
    data.push(f32::from_bits(0));
    for list in &refs {
        offset += list.len() as u32;
        data.push(f32::from_bits(offset));
    }
    for list in refs {
        for r in list {
            data.push(f32::from_bits(r));
        }
    }

    data.resize(total_words, 0.0);
    Ok((data, output_base))
}

fn decode(
    snapshot: &TissueSearchSnapshot,
    words: &[f32],
) -> Result<Vec<[f64; 3]>, GpuTissueError> {
    let n = snapshot.pinned().len();
    if words.len() != n * 3 || words.iter().any(|x| !x.is_finite()) {
        return Err(GpuTissueError::NumericalOverflow);
    }
    let mut result = Vec::with_capacity(n);
    for (i, chunk) in words.chunks_exact(3).enumerate() {
        let is_pinned = snapshot.pinned()[i];
        if is_pinned {
            result.push([0.0, 0.0, 0.0]);
        } else {
            result.push([f64::from(chunk[0]), f64::from(chunk[1]), f64::from(chunk[2])]);
        }
    }
    Ok(result)
}

/// GPU Compute program for executing the rest-material search metric.
#[derive(Debug)]
pub struct GpuTissueProgram {
    device: wgpu::Device,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_elements: wgpu::ComputePipeline,
    pipeline_nodes: wgpu::ComputePipeline,
    max_bytes: usize,
}

impl GpuTissueProgram {
    /// Creates a new GPU tissue search program on the given device.
    /// # Errors
    /// Returns an error if the compute shader fails to compile or the device lacks compute support.
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuTissueError> {
        Self::with_budget(device, 64 * 1024 * 1024).await
    }

    /// Creates a program with an explicit buffer allocation budget limit.
    /// # Errors
    /// Returns validation or compilation errors.
    pub async fn with_budget(device: &wgpu::Device, max_bytes: usize) -> Result<Self, GpuTissueError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tissue rest-material search"),
            source: wgpu::ShaderSource::Wgsl(include_str!("tissue.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tissue storage layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tissue pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline_elements = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("tissue cs_elements"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_elements"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_nodes = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("tissue cs_nodes"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_nodes"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        if let Some(err) = scope.pop().await {
            return Err(GpuTissueError::Wgpu(err.to_string()));
        }

        Ok(Self {
            device: device.clone(),
            bind_group_layout,
            pipeline_elements,
            pipeline_nodes,
            max_bytes,
        })
    }

    /// Evaluates the rest-material search action on the GPU.
    /// # Errors
    /// Returns an error if input validation fails, buffers exceed limits, or execution fails.
    #[allow(clippy::cast_possible_truncation)]
    pub fn tissue_search_action(
        &self,
        queue: &wgpu::Queue,
        snapshot: &TissueSearchSnapshot,
        direction: &[[f64; 3]],
    ) -> Result<Vec<[f64; 3]>, GpuTissueError> {
        let n = snapshot.pinned().len();
        let e = snapshot.elements().len();
        let (packed, output_base) = pack(snapshot, direction, self.max_bytes)?;
        let output_byte_offset = (output_base * 4) as u64;
        let output_bytes_len = (n * 3 * 4) as u64;

        let storage_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tissue compute storage"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tissue compute bind group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            }],
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tissue readback staging"),
            size: output_bytes_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tissue search encoder"),
            });

        // Pass 1: cs_elements
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tissue cs_elements pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_elements);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (e as u32).div_ceil(64);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Pass 2: cs_nodes
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tissue cs_nodes pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_nodes);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (n as u32).div_ceil(64);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &storage_buffer,
            output_byte_offset,
            &staging_buffer,
            0,
            output_bytes_len,
        );

        let submission = queue.submit([encoder.finish()]);

        let slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| GpuTissueError::Wgpu(e.to_string()))?;

        rx.recv()
            .map_err(|_| GpuTissueError::Wgpu("readback channel dropped".into()))?
            .map_err(|e| GpuTissueError::Wgpu(e.to_string()))?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|e| GpuTissueError::Wgpu(e.to_string()))?;
        let words: &[f32] = bytemuck::cast_slice(&mapped);
        let decoded = decode(snapshot, words)?;
        drop(mapped);
        staging_buffer.unmap();

        Ok(decoded)
    }

    /// Adapts this GPU program into an explicit [`TissueSearchBackend`].
    #[must_use]
    pub fn into_backend(
        self: Arc<Self>,
        queue: Arc<wgpu::Queue>,
    ) -> Arc<dyn TissueSearchBackend> {
        Arc::new(GpuTissueSearchBackend {
            program: self,
            queue,
        })
    }
}

#[derive(Debug)]
struct GpuTissueSearchBackend {
    program: Arc<GpuTissueProgram>,
    queue: Arc<wgpu::Queue>,
}

#[derive(Debug)]
struct GpuTissueSearchOperation {
    program: Arc<GpuTissueProgram>,
    queue: Arc<wgpu::Queue>,
    snapshot: TissueSearchSnapshot,
}

impl TissueSearchBackend for GpuTissueSearchBackend {
    fn prepare(
        &self,
        snapshot: TissueSearchSnapshot,
    ) -> Result<Box<dyn TissueSearchOperation>, &'static str> {
        let (force_base, output_base, total_words) = layout(
            snapshot.pinned().len(),
            snapshot.elements().len(),
            self.program.max_bytes,
        )
        .map_err(|_| "GPU tissue search allocation budget exceeded")?;
        let _ = (force_base, output_base, total_words);

        Ok(Box::new(GpuTissueSearchOperation {
            program: Arc::clone(&self.program),
            queue: Arc::clone(&self.queue),
            snapshot,
        }))
    }
}

impl TissueSearchOperation for GpuTissueSearchOperation {
    fn apply(&self, direction: &[Vec3]) -> Result<Vec<Vec3>, &'static str> {
        self.program
            .tissue_search_action(&self.queue, &self.snapshot, direction)
            .map_err(|e| backend_error_message(&e))
    }
}

fn backend_error_message(err: &GpuTissueError) -> &'static str {
    match err {
        GpuTissueError::InvalidInput => "invalid GPU tissue search input or output",
        GpuTissueError::Budget => "GPU tissue search allocation budget exceeded",
        GpuTissueError::NumericalOverflow => "GPU tissue search numerical overflow",
        GpuTissueError::Compute(_) | GpuTissueError::Wgpu(_) => "GPU tissue search backend failure",
    }
}

/// Identifies whether a string error originated from GPU tissue search infrastructure.
#[must_use]
pub fn is_gpu_tissue_search_failure(error: &str) -> bool {
    matches!(
        error,
        "GPU tissue search disabled"
            | "GPU tissue search allocation budget exceeded"
            | "GPU tissue search numerical overflow"
            | "invalid GPU tissue search input or output"
            | "GPU tissue search backend failure"
            | "invalid tissue search backend output"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use physics::biomechanics::{Body, Material};

    fn specimen(count: usize) -> Body {
        let mut points = Vec::new();
        let mut cells = Vec::new();
        let mut pins = Vec::new();
        for i in 0..count {
            let first = points.len();
            let x = i as f64 * 2.;
            points.extend([
                [x, 0., 0.],
                [x + 1., 0., 0.],
                [x, 1., 0.],
                [x, 0., 1.],
                [x, 0., -1.],
            ]);
            pins.extend([i % 2 == 0, false, false, i % 3 == 0, false]);
            cells.push((
                [first, first + 1, first + 2, first + 3],
                Material::from_young_poisson(300. + i as f64, 0.4).unwrap(),
            ));
            cells.push((
                [first, first + 2, first + 1, first + 4],
                Material::from_young_poisson(500. + i as f64, 0.3).unwrap(),
            ));
        }
        Body::new(points, pins, cells).unwrap()
    }

    #[test]
    fn layout_and_packing_validation() {
        let body = specimen(2);
        let snapshot = body.tissue_search_snapshot(&[2.; 10]).unwrap();
        let (data, output_base) = pack(&snapshot, &[[1.; 3]; 10], usize::MAX).unwrap();
        assert_eq!(output_base, 3 + 6 * 10 + 35 * 4);
        assert_eq!(data.len(), 3 + 9 * 10 + 35 * 4);

        assert!(pack(&snapshot, &[[1.; 3]; 9], usize::MAX).is_err());
        assert!(pack(&snapshot, &[[f64::NAN; 3]; 10], usize::MAX).is_err());
    }

    #[test]
    fn gpu_tissue_search_on_device() {
        let instance = wgpu::Instance::default();
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return, // Skip test if no GPU adapter is present in test environment
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let program = Arc::new(pollster::block_on(GpuTissueProgram::new(&device)).unwrap());
        let queue = Arc::new(queue);

        for count in [1, 4, 16] {
            let body = specimen(count);
            let n = count * 5;
            let weights: Vec<_> = (0..n).map(|i| 2. + (i % 7) as f64 / 8.).collect();
            let snapshot = body.tissue_search_snapshot(&weights).unwrap();
            let direction: Vec<_> = (0..n).map(|i| [i as f64 * 0.001, 0.2, -0.1]).collect();

            let expected = body.tissue_search_action(&weights, &direction).unwrap();
            let actual = program.tissue_search_action(&queue, &snapshot, &direction).unwrap();

            assert_eq!(actual.len(), expected.len());
            for (act, exp) in actual.iter().zip(&expected) {
                for axis in 0..3 {
                    let diff = (act[axis] - exp[axis]).abs();
                    // Float32 GPU arithmetic vs Float64 CPU reference: relative error check
                    let scale = exp[axis].abs().max(1.0);
                    assert!(
                        diff / scale < 1e-4,
                        "Mismatch at count {count}: actual {act:?}, expected {exp:?}, diff {diff}"
                    );
                }
            }
        }
    }

    #[test]
    fn gpu_tissue_backend_implicit_stepping() {
        let instance = wgpu::Instance::default();
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return,
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let program = Arc::new(pollster::block_on(GpuTissueProgram::new(&device)).unwrap());
        let backend = program.into_backend(Arc::new(queue));

        let body = Body::new(
            vec![
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [0., 0., -1.],
            ],
            vec![true, false, false, true, false],
            vec![
                (
                    [0, 1, 2, 3],
                    Material::from_young_poisson(300., 0.4).unwrap(),
                ),
                (
                    [0, 2, 1, 4],
                    Material::from_young_poisson(500., 0.3).unwrap(),
                ),
            ],
        )
        .unwrap();

        let pins = body.tissue_search_snapshot(&[2.; 5]).unwrap().pinned().to_vec();
        let velocity = pins
            .iter()
            .map(|&p| if p { [0.; 3] } else { [0.1, -0.2, 0.3] })
            .collect();

        let mut gpu_body = physics::biomechanics::InertialBody::new_with_fixed_supports(body, &[6.; 2], velocity).unwrap();
        gpu_body.set_uniform_acceleration([0., -2., 0.]).unwrap();
        gpu_body.set_tissue_search_backend(Some(backend));

        let result = gpu_body.step_implicit_with_supports(None, 0.001, 1e-6);
        assert!(result.is_ok(), "GPU implicit step failed: {:?}", result.err());

        let positions = gpu_body.body().positions();
        // Fixed pins remain fixed
        assert_eq!(positions[0], [0., 0., 0.]);
        assert_eq!(positions[3], [0., 0., 1.]);
        // Free nodes moved
        assert_ne!(positions[1], [1., 0., 0.]);
    }
}
