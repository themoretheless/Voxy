use super::{ArchiveInstance, ComputeError, ComputeMemoryBudget, ComputeStorage};

pub(super) struct ShadowWind {
    prepare: wgpu::ComputePipeline,
    prepare_layout: wgpu::BindGroupLayout,
    pub vertex_layout: wgpu::BindGroupLayout,
}

pub(super) struct WindBuffer {
    values: ComputeStorage,
    pub vertex_bind: wgpu::BindGroup,
}

impl ShadowWind {
    pub(super) fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
    ) -> Self {
        let storage = |binding, visibility, read_only| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let prepare_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("plant shadow wind compute inputs"),
            entries: &[
                storage(3, wgpu::ShaderStages::COMPUTE, true),
                storage(4, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let vertex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("plant shadow wind vertex input"),
            entries: &[storage(2, wgpu::ShaderStages::VERTEX, true)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("plant shadow wind compute layout"),
            bind_group_layouts: &[Some(camera_layout), None, Some(&prepare_layout)],
            immediate_size: 0,
        });
        let prepare = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("plant shadow instance wind"),
            layout: Some(&layout),
            module: shader,
            entry_point: Some("cs_shadow_wind"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            prepare,
            prepare_layout,
            vertex_layout,
        }
    }

    pub(super) fn allocate(
        &self,
        device: &wgpu::Device,
        capacity: usize,
    ) -> Result<WindBuffer, ComputeError> {
        let [values] = ComputeMemoryBudget::for_device(device).allocate_buffers([
            voxy_gpu_core::ManagedBufferDescriptor {
                label: "plant shadow wind terms",
                size: (capacity * 16) as u64,
                contents: None,
                usage: wgpu::BufferUsages::STORAGE,
            },
        ])?;
        let vertex_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.vertex_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 2,
                resource: values.as_entire_binding(),
            }],
        });
        Ok(WindBuffer {
            values,
            vertex_bind,
        })
    }

    pub(super) fn encode(
        &self,
        device: &wgpu::Device,
        pass: &mut wgpu::ComputePass<'_>,
        camera: &wgpu::BindGroup,
        instances: &wgpu::Buffer,
        output: &WindBuffer,
        count: u32,
    ) {
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.prepare_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: instances,
                        offset: 0,
                        size: std::num::NonZeroU64::new(
                            u64::from(count) * size_of::<ArchiveInstance>() as u64,
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output.values.as_entire_binding(),
                },
            ],
        });
        pass.set_pipeline(&self.prepare);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(2, &bind, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
    }
}
