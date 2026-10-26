//! One source owner; an extra original pipeline and GPU result snapshot only.
use crate::{ComputeMemoryBudget, ComputeStorage, RaySceneError};
#[derive(Debug)]
pub(super) struct WorldTraversalPair {
    pub original_fast: (wgpu::ComputePipeline, wgpu::BindGroup),
    snapshot: ComputeStorage,
    rays_snapshot: ComputeStorage,
    pub stats: ComputeStorage,
    failure_capture: ComputeStorage,
    compare: wgpu::ComputePipeline,
}
impl WorldTraversalPair {
    pub fn new(
        device: &wgpu::Device,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
        words: u32,
        position: u64,
        original_fast: (wgpu::ComputePipeline, wgpu::BindGroup),
    ) -> Result<Self, RaySceneError> {
        if !matches!(words, 20 | 28)
            || position > u64::from(u32::MAX)
            || output.size() / 4 > u64::from(u32::MAX)
        {
            return Err(RaySceneError::Capacity);
        }
        let [snapshot, rays_snapshot, stats, failure_capture] =
            ComputeMemoryBudget::for_device(device)
                .allocate_buffers([
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "original GPU traversal result snapshot",
                        size: output.size(),
                        contents: None,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "original GPU ray coefficient snapshot",
                        size: input.size(),
                        contents: None,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "same-producer traversal native-word proof",
                        size: 64,
                        contents: None,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                    voxy_gpu_core::ManagedBufferDescriptor {
                        label: "first mismatched actual GPU ray and both query results",
                        size: 512,
                        contents: None,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    },
                ])
                .map_err(|_| RaySceneError::Capacity)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("same-producer traversal native-word proof"),
            source: wgpu::ShaderSource::Wgsl(include_str!("world_traversal_pair.wgsl").into()),
        });
        let compare = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("same-producer traversal native-word proof"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[
                    ("PAIR_RECORD_WORDS", f64::from(words)),
                    ("PAIR_POSITION_WORD", position as f64),
                ],
                ..Default::default()
            },
            cache: None,
        });
        Ok(Self {
            original_fast,
            snapshot,
            rays_snapshot,
            stats,
            failure_capture,
            compare,
        })
    }
    pub fn original_buffers(&self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        (&self.rays_snapshot, &self.snapshot)
    }
    pub fn read_proof(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("same-producer traversal proof readback"),
            size: 576,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.stats, 0, &read, 0, 64);
        encoder.copy_buffer_to_buffer(&self.failure_capture, 0, &read, 64, 512);
        let submission = queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = send.send(r);
        });
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        receive.recv()??;
        let bytes = read.slice(..).get_mapped_range()?;
        let words: Vec<u32> = bytes[..64]
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        let records = u64::from(words[1]) | (u64::from(words[2]) << 32);
        let first_failure = (words[3] != 0).then(|| serde_json::json!({"record":words[4],"field":words[5],"original_u32":words[6],"candidate_u32":words[7],"source_ray_id":[words[8],words[9]]}));
        let first_capture = (words[3] != 0).then(|| {
            bytes[64..]
                .chunks_exact(4)
                .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                .collect::<Vec<_>>()
        });
        let pair_count = |base: usize| u64::from(words[base]) | (u64::from(words[base + 1]) << 32);
        let result = serde_json::json!({"first_failure_capture_u32":first_capture,"explicitly_admitted_records":pair_count(10),"matched_hit_records":pair_count(12),"matched_unresolved_records":pair_count(14),"first_observed_failure":first_failure,"schema":1,"mode":"original_then_wide_same_GPU_producer","raw_words":words,
            "compared_records":records,"failure_flags":words[0],"all_protected_words_equal":words[0]==0 && records>0,
            "protected_input_padding_excluded":[7,11,15],"output_metadata_excluded":[3,16],"output_word18_protected_mask":30,
            "comparison_reads_buffers_only":true,"diagnostic_only":true,"performance_qualified":false});
        drop(bytes);
        read.unmap();
        Ok(result)
    }
    pub fn snapshot(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
    ) {
        encoder.copy_buffer_to_buffer(output, 0, &self.snapshot, 0, output.size());
        encoder.copy_buffer_to_buffer(input, 0, &self.rays_snapshot, 0, input.size());
    }
    pub fn compare(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
        count: u32,
    ) {
        assert!(count > 0);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("same actual GPU ray batch"),
            layout: &self.compare.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.snapshot.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: input,
                        offset: 0,
                        size: std::num::NonZeroU64::new(u64::from(count) * 80),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.stats.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.rays_snapshot,
                        offset: 0,
                        size: std::num::NonZeroU64::new(u64::from(count) * 80),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.failure_capture.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compare original and wide protected query words"),
            ..Default::default()
        });
        pass.set_pipeline(&self.compare);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn comparator_shader_validates() {
        let module =
            naga::front::wgsl::parse_str(include_str!("world_traversal_pair.wgsl")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}

#[cfg(test)]
#[path = "world_traversal_pair_tests.rs"]
mod metal_tests;
