//! GPU Compute Voxel Mesher with Direct Hardware Indirect Draw Buffers.
//! Transforms 34x34x34 padded voxel chunks into compacted triangle vertices and
//! generates `DrawIndirectArgs` on GPU with zero host readback overhead.

use voxy_render::ComputeError;
use wgpu::util::DeviceExt;

/// Size of a padded chunk volume: 32 + 2 boundary slices along each axis = 34^3.
pub const PADDED_CHUNK_VOLUME: usize = 34 * 34 * 34;

/// Compact GPU vertex representation (32 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVoxelVertex {
    pub position: [f32; 3],
    pub face_dir: u32,
    pub material: u32,
    pub ao: u32,
    pub uv: [f32; 2],
}

/// Standard hardware indirect draw parameters (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuDrawIndirectArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VoxelMeshConfig {
    max_quads: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

#[derive(Debug)]
pub enum GpuVoxelMeshError {
    InvalidInput,
    Compute(ComputeError),
    Wgpu(String),
}

impl std::fmt::Display for GpuVoxelMeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compute(err) => write!(f, "GPU voxel mesh compute error: {err}"),
            Self::Wgpu(msg) => write!(f, "GPU voxel mesh wgpu error: {msg}"),
            Self::InvalidInput => write!(f, "GPU voxel mesh invalid input dimensions"),
        }
    }
}

impl std::error::Error for GpuVoxelMeshError {}

/// GPU Resident result of voxel chunk meshing.
#[derive(Debug)]
pub struct GpuMeshResult {
    pub vertex_buffer: wgpu::Buffer,
    pub indirect_buffer: wgpu::Buffer,
    pub max_quads: u32,
}

impl From<GpuMeshResult> for voxy_render::GpuIndirectMesh {
    fn from(res: GpuMeshResult) -> Self {
        Self {
            vertex_buffer: res.vertex_buffer,
            indirect_buffer: res.indirect_buffer,
        }
    }
}

impl GpuMeshResult {
    /// Converts directly into renderer indirect mesh with zero host readback.
    #[must_use]
    pub fn into_indirect_mesh(self) -> voxy_render::GpuIndirectMesh {
        self.into()
    }
    /// Read back vertex and indirect parameters to CPU memory for test or fallback processing.
    /// # Errors
    /// Returns an error if device polling or buffer mapping fails.
    pub fn readback(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(GpuDrawIndirectArgs, Vec<GpuVoxelVertex>), GpuVoxelMeshError> {
        let indirect_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect staging"),
            size: std::mem::size_of::<GpuDrawIndirectArgs>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vertex_bytes_len = u64::from(self.max_quads) * 6 * std::mem::size_of::<GpuVoxelVertex>() as u64;
        let vertex_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex staging"),
            size: vertex_bytes_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("voxel mesh readback encoder"),
        });
        encoder.copy_buffer_to_buffer(
            &self.indirect_buffer,
            0,
            &indirect_staging,
            0,
            std::mem::size_of::<GpuDrawIndirectArgs>() as u64,
        );
        encoder.copy_buffer_to_buffer(&self.vertex_buffer, 0, &vertex_staging, 0, vertex_bytes_len);

        let submission = queue.submit([encoder.finish()]);

        let (tx1, rx1) = std::sync::mpsc::channel();
        indirect_staging.slice(..).map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx1.send(res);
        });

        let (tx2, rx2) = std::sync::mpsc::channel();
        vertex_staging.slice(..).map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx2.send(res);
        });

        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| GpuVoxelMeshError::Wgpu(e.to_string()))?;

        rx1.recv()
            .map_err(|_| GpuVoxelMeshError::Wgpu("indirect channel dropped".into()))?
            .map_err(|e| GpuVoxelMeshError::Wgpu(e.to_string()))?;

        rx2.recv()
            .map_err(|_| GpuVoxelMeshError::Wgpu("vertex channel dropped".into()))?
            .map_err(|e| GpuVoxelMeshError::Wgpu(e.to_string()))?;

        let indirect_mapped = indirect_staging
            .slice(..)
            .get_mapped_range()
            .map_err(|e| GpuVoxelMeshError::Wgpu(e.to_string()))?;
        let indirect_args: GpuDrawIndirectArgs = *bytemuck::from_bytes(&indirect_mapped);
        drop(indirect_mapped);
        indirect_staging.unmap();

        let count = (indirect_args.vertex_count as usize).min(self.max_quads as usize * 6);
        let vertex_mapped = vertex_staging
            .slice(..)
            .get_mapped_range()
            .map_err(|e| GpuVoxelMeshError::Wgpu(e.to_string()))?;
        let all_vertices: &[GpuVoxelVertex] = bytemuck::cast_slice(&vertex_mapped);
        let result_vertices = all_vertices[..count].to_vec();
        drop(vertex_mapped);
        vertex_staging.unmap();

        Ok((indirect_args, result_vertices))
    }
}

/// GPU Compute Pipeline for parallel voxel chunk meshing.
#[derive(Debug)]
pub struct GpuVoxelMesher {
    device: wgpu::Device,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_reset: wgpu::ComputePipeline,
    pipeline_mesh: wgpu::ComputePipeline,
}

impl GpuVoxelMesher {
    /// Creates a new GPU voxel mesher pipeline.
    /// # Errors
    /// Returns an error if WGSL compilation or pipeline creation fails.
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuVoxelMeshError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("voxel mesher shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("voxel_mesh.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("voxel mesher layout"),
            entries: &[
                // binding 0: config
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: padded voxels
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: indirect draw args
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: output vertices
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("voxel mesher pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline_reset = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("voxel mesher reset"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_reset"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_mesh = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("voxel mesher mesh"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_mesh"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        if let Some(err) = scope.pop().await {
            return Err(GpuVoxelMeshError::Wgpu(err.to_string()));
        }

        Ok(Self {
            device: device.clone(),
            bind_group_layout,
            pipeline_reset,
            pipeline_mesh,
        })
    }

    /// Meshes a 34x34x34 padded voxel chunk directly into GPU vertex and indirect draw buffers.
    /// # Errors
    /// Returns an error if the voxel slice has incorrect length or allocation fails.
    pub fn mesh_chunk(
        &self,
        queue: &wgpu::Queue,
        padded_voxels: &[u32],
        max_quads: u32,
    ) -> Result<GpuMeshResult, GpuVoxelMeshError> {
        if padded_voxels.len() != PADDED_CHUNK_VOLUME || max_quads == 0 {
            return Err(GpuVoxelMeshError::InvalidInput);
        }

        let config = VoxelMeshConfig {
            max_quads,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let config_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("voxel mesh config"),
            contents: bytemuck::bytes_of(&config),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let voxels_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("padded voxels buffer"),
            contents: bytemuck::cast_slice(padded_voxels),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let indirect_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voxel mesh indirect args"),
            size: std::mem::size_of::<GpuDrawIndirectArgs>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let vertex_bytes_len = u64::from(max_quads) * 6 * std::mem::size_of::<GpuVoxelVertex>() as u64;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voxel mesh vertices"),
            size: vertex_bytes_len,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("voxel mesh bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: voxels_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indirect_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: vertex_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("voxel mesher encoder"),
        });

        // Pass 1: Reset indirect arguments
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("voxel mesher reset pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_reset);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        // Pass 2: Extract visible quads in 32x32x32 chunk volume (4x4x4 workgroup size -> 8x8x8 dispatches)
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("voxel mesher mesh pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_mesh);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(8, 8, 8);
        }

        queue.submit([encoder.finish()]);

        Ok(GpuMeshResult {
            vertex_buffer,
            indirect_buffer,
            max_quads,
        })
    }
}

/// Unified GPU Voxel Pipeline combining Cellular Automata Lighting with Hardware Indirect Meshing.
#[derive(Debug)]
pub struct GpuVoxelPipeline {
    lighting: crate::lighting::GpuLightingProgram,
    mesher: GpuVoxelMesher,
}

#[derive(Debug)]
pub enum GpuVoxelPipelineError {
    Lighting(crate::lighting::GpuLightingError),
    Mesher(GpuVoxelMeshError),
}

impl std::fmt::Display for GpuVoxelPipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lighting(err) => write!(f, "GPU lighting error: {err}"),
            Self::Mesher(err) => write!(f, "GPU meshing error: {err}"),
        }
    }
}

impl std::error::Error for GpuVoxelPipelineError {}

impl From<crate::lighting::GpuLightingError> for GpuVoxelPipelineError {
    fn from(err: crate::lighting::GpuLightingError) -> Self {
        Self::Lighting(err)
    }
}

impl From<GpuVoxelMeshError> for GpuVoxelPipelineError {
    fn from(err: GpuVoxelMeshError) -> Self {
        Self::Mesher(err)
    }
}

impl GpuVoxelPipeline {
    /// Compiles both GPU lighting and GPU mesher pipelines.
    /// # Errors
    /// Returns an error if pipeline compilation fails.
    pub async fn new(device: &wgpu::Device) -> Result<Self, GpuVoxelPipelineError> {
        let lighting = crate::lighting::GpuLightingProgram::new(device).await?;
        let mesher = GpuVoxelMesher::new(device).await?;
        Ok(Self { lighting, mesher })
    }

    #[must_use]
    pub fn lighting(&self) -> &crate::lighting::GpuLightingProgram {
        &self.lighting
    }

    #[must_use]
    pub fn mesher(&self) -> &GpuVoxelMesher {
        &self.mesher
    }

    /// Meshes padded chunk voxels directly into a GPU-resident Indirect Mesh ready for drawing.
    /// # Errors
    /// Returns an error if meshing dispatch fails.
    pub fn mesh_chunk(
        &self,
        queue: &wgpu::Queue,
        padded_voxels: &[u32],
        max_quads: u32,
    ) -> Result<voxy_render::GpuIndirectMesh, GpuVoxelPipelineError> {
        let result = self.mesher.mesh_chunk(queue, padded_voxels, max_quads)?;
        Ok(result.into_indirect_mesh())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_voxel_mesh_single_voxel() {
        let instance = wgpu::Instance::default();
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return,
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let mesher = pollster::block_on(GpuVoxelMesher::new(&device)).unwrap();

        // 34x34x34 volume filled with air (0) except for one single voxel at local (0, 0, 0) -> padded (1, 1, 1)
        let mut voxels = vec![0u32; PADDED_CHUNK_VOLUME];
        let idx = 1 + 1 * 34 + 1 * 1156;
        voxels[idx] = 42; // solid block with material ID 42

        let result = mesher.mesh_chunk(&queue, &voxels, 1024).unwrap();
        let (indirect_args, vertices) = result.readback(&device, &queue).unwrap();

        // One isolated voxel has exactly 6 exposed faces = 6 quads = 36 vertices
        assert_eq!(indirect_args.vertex_count, 36);
        assert_eq!(indirect_args.instance_count, 1);
        assert_eq!(vertices.len(), 36);

        // Every vertex belongs to block 42
        for v in &vertices {
            assert_eq!(v.material, 42);
        }
    }

    #[test]
    fn gpu_voxel_mesh_solid_cube() {
        let instance = wgpu::Instance::default();
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return,
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let mesher = pollster::block_on(GpuVoxelMesher::new(&device)).unwrap();

        // 2x2x2 cube of solid voxels at local (0..2, 0..2, 0..2) -> padded (1..3, 1..3, 1..3)
        let mut voxels = vec![0u32; PADDED_CHUNK_VOLUME];
        for z in 1..=2 {
            for y in 1..=2 {
                for x in 1..=2 {
                    voxels[x + y * 34 + z * 1156] = 7;
                }
            }
        }

        let result = mesher.mesh_chunk(&queue, &voxels, 1024).unwrap();
        let (indirect_args, vertices) = result.readback(&device, &queue).unwrap();

        // A 2x2x2 cube has 6 faces of 2x2 = 24 quads = 144 vertices (interior faces culled!)
        assert_eq!(indirect_args.vertex_count, 144);
        assert_eq!(vertices.len(), 144);
        for v in &vertices {
            assert_eq!(v.material, 7);
        }
    }

    #[test]
    fn gpu_voxel_pipeline_end_to_end() {
        let instance = wgpu::Instance::default();
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
            Ok(adapter) => adapter,
            Err(_) => return,
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(_) => return,
        };

        let pipeline = pollster::block_on(GpuVoxelPipeline::new(&device)).unwrap();
        let mut voxels = vec![0u32; PADDED_CHUNK_VOLUME];
        voxels[1 + 34 + 1156] = 1;
        let indirect_mesh = pipeline.mesh_chunk(&queue, &voxels, 512).unwrap();
        assert!(indirect_mesh.vertex_buffer.size() > 0);
        assert!(indirect_mesh.indirect_buffer.size() > 0);
    }
}
