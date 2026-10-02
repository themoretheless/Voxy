//! Static-camera Monte Carlo accumulation with caller-owned presented history.
use crate::ComputeError;
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub struct RadianceAccumulator {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
}
#[derive(Debug)]
pub struct RadianceAccumulationFrame {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    bindings: wgpu::BindGroup,
    output: wgpu::Texture,
    samples: u32,
}
impl RadianceAccumulator {
    /// # Errors
    /// Rejects insufficient compute/storage/texture limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        let l = device.limits();
        if l.max_compute_invocations_per_workgroup < 64
            || l.max_compute_workgroup_size_x < 8
            || l.max_compute_workgroup_size_y < 8
            || l.max_storage_textures_per_shader_stage < 1
            || l.max_sampled_textures_per_shader_stage < 2
            || l.max_uniform_buffers_per_shader_stage < 1
        {
            return Err(ComputeError::Unsupported);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiance accumulation"),
            source: wgpu::ShaderSource::Wgsl(include_str!("accumulation.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("radiance accumulation"),
            layout: None,
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            device: device.clone(),
            pipeline,
        })
    }
    /// Average one linear radiance sample into the last presented candidate.
    /// Null ray samples are valid black contributions. Negative/nonfinite RGB
    /// channels become zero; alpha is ignored and output alpha is one.
    /// Reset with None on camera, scene, lighting, material or resolution changes.
    /// This does not reproject moving scenes. Commit only after presentation.
    /// # Errors
    /// Rejects incompatible textures/history, device mismatch or count overflow.
    pub fn prepare(
        &self,
        source: &wgpu::Texture,
        previous: Option<&RadianceAccumulationFrame>,
    ) -> Result<RadianceAccumulationFrame, ComputeError> {
        if source.dimension() != wgpu::TextureDimension::D2
            || source.depth_or_array_layers() != 1
            || source.sample_count() != 1
            || !source
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            || !matches!(
                source.format(),
                wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
            )
        {
            return Err(ComputeError::InvalidBuffer);
        }
        if previous.is_some_and(|p| p.device != self.device) {
            return Err(ComputeError::DeviceMismatch);
        }
        if previous.is_some_and(|p| p.output.size() != source.size()) {
            return Err(ComputeError::InvalidBuffer);
        }
        let samples = previous.map_or(1, |p| p.samples + 1);
        if samples > 16_777_216 {
            return Err(ComputeError::InvalidDispatch);
        }
        let l = self.device.limits();
        if source.width().div_ceil(8) > l.max_compute_workgroups_per_dimension
            || source.height().div_ceil(8) > l.max_compute_workgroups_per_dimension
        {
            return Err(ComputeError::InvalidDispatch);
        }
        let output = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mean linear radiance"),
            size: source.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let settings = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("accumulation count"),
                contents: bytemuck::cast_slice(&[samples, u32::from(previous.is_some()), 0, 0]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let current = source.create_view(&wgpu::TextureViewDescriptor::default());
        let old = previous
            .map_or(source, |p| &p.output)
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target = output.create_view(&wgpu::TextureViewDescriptor::default());
        let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("accumulation inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&current),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&old),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&target),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: settings.as_entire_binding(),
                },
            ],
        });
        Ok(RadianceAccumulationFrame {
            device: self.device.clone(),
            pipeline: self.pipeline.clone(),
            bindings,
            output,
            samples,
        })
    }
}
impl RadianceAccumulationFrame {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.dispatch_workgroups(
            self.output.width().div_ceil(8),
            self.output.height().div_ceil(8),
            1,
        );
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
    #[must_use]
    pub fn samples(&self) -> u32 {
        self.samples
    }
}
