//! GPU-only bit transport; coefficient ownership stays in `BandedSolveInput`.
use crate::{BandedSolveInput, ComputeError};
#[derive(Debug)]
pub struct BandedTransferProgram {
    device: wgpu::Device,
    layout: wgpu::BindGroupLayout,
    upload: wgpu::ComputePipeline,
    gather: wgpu::ComputePipeline,
}
impl BandedTransferProgram {
    pub async fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage < 2
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
        {
            return Err(ComputeError::Unsupported);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("banded RHS transport"),
            source: wgpu::ShaderSource::Wgsl(include_str!("banded_transfer.wgsl").into()),
        });
        let layout = crate::BindGroupLayoutBuilder::with_capacity(Some("banded RHS transport"), 2)
            .storage_buffer(0, wgpu::ShaderStages::COMPUTE, false)
            .storage_buffer(1, wgpu::ShaderStages::COMPUTE, false)
            .build(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("banded RHS transport"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("banded RHS transport"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let upload = pipeline("upload");
        let gather = pipeline("gather");
        if let Some(error) = scope.pop().await {
            return Err(ComputeError::Validation(error.to_string()));
        }
        Ok(Self {
            device: device.clone(),
            layout,
            upload,
            gather,
        })
    }
    /// Transfer only RHS/status words. Upload ignores the compact header;
    /// gathering publishes the resident header for the codec's admission gate.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        resident: &wgpu::Buffer,
        compact: &wgpu::Buffer,
        input: &BandedSolveInput,
        upload: bool,
    ) -> Result<(), ComputeError> {
        let limits = self.device.limits();
        if resident.size() != input.bytes().len() as u64
            || compact.size() != input.compact_output_size()
            || !resident.usage().contains(wgpu::BufferUsages::STORAGE)
            || !compact.usage().contains(wgpu::BufferUsages::STORAGE)
            || resident.size() > limits.max_storage_buffer_binding_size
            || compact.size() > limits.max_storage_buffer_binding_size
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let groups = (compact.size() / 4).div_ceil(64);
        if groups == 0 || groups > u64::from(limits.max_compute_workgroups_per_dimension) {
            return Err(ComputeError::InvalidDispatch);
        }
        let binding = crate::BindGroupBuilder::with_capacity(Some("banded RHS transport"), 2)
            .buffer(0, resident)
            .buffer(1, compact)
            .build(&self.device, &self.layout);
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(if upload { &self.upload } else { &self.gather });
        pass.set_bind_group(0, &binding, &[]);
        pass.dispatch_workgroups(groups as u32, 1, 1);
        Ok(())
    }
}
