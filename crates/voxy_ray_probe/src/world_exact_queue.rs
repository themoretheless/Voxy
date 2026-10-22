//! GPU compaction -> exact decision pass in the same submission. No CPU selector.
pub(super) struct QueuedExact {
    pub parameters: bool,
    words: usize,
    pub queue: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    scratch: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    queue_readback: wgpu::Buffer,
}
impl QueuedExact {
    pub fn new(device: &wgpu::Device, capacity: usize) -> Self {
        let parameters = std::env::var("VOXY_RAY_WORLD_EXACT_PARAMETERS").as_deref() == Ok("1");
        let words = if parameters { 16 } else { 8 };
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let queue = buffer(
            "GPU compact unresolved IDs",
            4 + capacity as u64 * 4,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        );
        let scratch = buffer(
            "bounded exact GPU workspace",
            16 * 512 * 128 * 4,
            wgpu::BufferUsages::STORAGE,
        );
        let output = buffer(
            "compact exact decisions",
            capacity as u64 * words as u64 * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let readback = buffer(
            "exact decisions proof",
            capacity as u64 * words as u64 * 4,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let queue_readback = buffer(
            "queue proof",
            4 + capacity as u64 * 4,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let source = format!(
            "{}\n{}\n{}\n{}\n{}",
            include_str!("../../voxy_render/src/world_ray_input.wgsl"),
            include_str!("../../voxy_render/src/world_ray_source_bindings.wgsl"),
            include_str!("../../voxy_render/src/world_exact_integer.wgsl"),
            include_str!("../../voxy_render/src/world_exact_ratio.wgsl"),
            include_str!("../../voxy_render/src/world_exact.wgsl")
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("queued exact closed decision"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some(if parameters {
                "main_queued_parameters"
            } else {
                "main_queued"
            }),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            parameters,
            words,
            queue,
            pipeline,
            scratch,
            output,
            readback,
            queue_readback,
        }
    }
    pub fn encode(
        &self,
        device: &wgpu::Device,
        scene: &voxy_render::RayScene,
        input: &wgpu::Buffer,
        count: usize,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let [vertices, indices, instances] = scene
            .archive_candidate_source_bindings()
            .ok_or("missing exact sources")?;
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: input,
                        offset: 0,
                        size: std::num::NonZeroU64::new(count as u64 * 80),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertices,
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indices,
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: instances,
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: scene
                        .archive_candidate_instance_layout()
                        .ok_or("missing exact layout")?,
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.scratch.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: self.queue.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(16, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &self.output,
            0,
            &self.readback,
            0,
            count as u64 * self.words as u64 * 4,
        );
        encoder.copy_buffer_to_buffer(
            &self.queue,
            0,
            &self.queue_readback,
            0,
            4 + count as u64 * 4,
        );
        Ok(())
    }
    pub fn read(
        &self,
        device: &wgpu::Device,
        count: usize,
    ) -> Result<Vec<(u32, [u32; 16])>, Box<dyn std::error::Error>> {
        let map = |buffer: &wgpu::Buffer, size: u64| -> Result<(), Box<dyn std::error::Error>> {
            let (tx, rx) = std::sync::mpsc::channel();
            buffer
                .slice(..size)
                .map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
            device.poll(wgpu::PollType::wait_indefinitely())?;
            rx.recv()??;
            Ok(())
        };
        let queue_bytes = 4 + count as u64 * 4;
        map(&self.queue_readback, queue_bytes)?;
        let mut ids = Vec::new();
        {
            let data = self
                .queue_readback
                .slice(..queue_bytes)
                .get_mapped_range()?;
            let words = bytemuck::cast_slice::<u8, u32>(&data);
            let pending = words[0] as usize;
            if pending > count {
                return Err("GPU exact work queue overflow".into());
            }
            ids.extend_from_slice(&words[1..=pending]);
        }
        self.queue_readback.unmap();
        let unique: std::collections::BTreeSet<_> = ids.iter().copied().collect();
        if unique.len() != ids.len() || ids.iter().any(|&id| id as usize >= count) {
            return Err("invalid compact GPU IDs".into());
        }
        let bytes = count as u64 * self.words as u64 * 4;
        map(&self.readback, bytes)?;
        let mut results = Vec::with_capacity(ids.len());
        {
            let data = self.readback.slice(..bytes).get_mapped_range()?;
            let records = bytemuck::cast_slice::<u8, u32>(&data);
            for (&id, values) in ids.iter().zip(records.chunks_exact(self.words)) {
                let mut record = [0u32; 16];
                record[..self.words].copy_from_slice(values);
                if record[0] > 1
                    || record[1] != 0
                    || record[2] != 2048
                    || record[3] == 0
                    || record[3] > 512
                {
                    return Err(
                        format!("unresolved/incomplete queued exact decision {record:?}").into(),
                    );
                }
                results.push((id, record));
            }
        }
        self.readback.unmap();
        Ok(results)
    }
}
