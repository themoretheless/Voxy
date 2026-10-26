//! Opt-in same-frame, same-PSO raw HDR output comparison. Never used for timings.
use crate::{ComputeMemoryBudget, ComputeStorage, RaySceneError};
#[derive(Debug)]
pub(super) struct ConsumerPair {
    original: ComputeStorage,
    candidate: ComputeStorage,
    stats: ComputeStorage,
    pipeline: wgpu::ComputePipeline,
    regions: Vec<(wgpu::BindGroup, ComputeStorage, [u32; 4])>,
}
impl ConsumerPair {
    pub fn new(
        device: &wgpu::Device,
        size: [u32; 2],
        tiles: &[[u32; 4]],
        kinds: usize,
    ) -> Result<Self, RaySceneError> {
        validate_partition(size, tiles)?;
        let max = tiles
            .iter()
            .map(|r| u64::from(pitch(r[2])) * u64::from(r[3]))
            .max()
            .ok_or(RaySceneError::Capacity)?;
        if max > device.limits().max_storage_buffer_binding_size
            || max > device.limits().max_buffer_size
        {
            return Err(RaySceneError::Capacity);
        }
        let descriptor = |label, size, usage| voxy_gpu_core::ManagedBufferDescriptor {
            label,
            size,
            contents: None,
            usage,
        };
        let [original, candidate, stats] = ComputeMemoryBudget::for_device(device)
            .allocate_buffers([
                descriptor(
                    "original native HDR tile",
                    max,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                descriptor(
                    "candidate native HDR tile",
                    max,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                descriptor(
                    "native HDR comparison counters",
                    64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                ),
            ])
            .map_err(|_| RaySceneError::Capacity)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native HDR comparison"),
            source: wgpu::ShaderSource::Wgsl(include_str!("world_consumer_pair.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("native HDR comparison"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let mut regions = Vec::new();
        for kind in 0..kinds {
            for &rect in tiles {
                let words = [
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    pitch(rect[2]) / 4,
                    kind as u32,
                    0,
                    0,
                ];
                let [uniform] = ComputeMemoryBudget::for_device(device)
                    .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                        label: "HDR comparison rectangle",
                        size: 32,
                        contents: Some(bytemuck::cast_slice(&words)),
                        usage: wgpu::BufferUsages::UNIFORM,
                    }])
                    .map_err(|_| RaySceneError::Capacity)?;
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("same actual HDR output"),
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: original.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: candidate.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: stats.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: uniform.as_entire_binding(),
                        },
                    ],
                });
                regions.push((group, uniform, rect));
            }
        }
        Ok(Self {
            original,
            candidate,
            stats,
            pipeline,
            regions,
        })
    }
    fn copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        index: usize,
        buffer: &wgpu::Buffer,
    ) {
        let [x, y, w, h] = self.regions[index].2;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch(w)),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
    pub fn original(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        index: usize,
    ) {
        self.copy(encoder, texture, index, &self.original);
    }
    pub fn compare(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        index: usize,
    ) {
        self.copy(encoder, texture, index, &self.candidate);
        let (group, _, rect) = &self.regions[index];
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("raw native HDR equality"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.dispatch_workgroups(rect[2].div_ceil(8), rect[3].div_ceil(8), 1);
    }
    pub fn proof(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("HDR proof readback"),
            size: 64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.stats, 0, &buffer, 0, 64);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let bytes = buffer.slice(..).get_mapped_range()?;
        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let pixels = u64::from(words[1]) | (u64::from(words[2]) << 32);
        let proof = serde_json::json!({"schema":1,"same_actual_texture_and_pipeline":true,"native_format":"RGBA32Float","comparison":"raw_u32_no_tolerance","compared_pixels":pixels,"failure_flags":words[0],"all_native_words_equal_and_finite":pixels>0&&words[0]==0,"first_failure":(words[3]!=0).then(||serde_json::json!({"xy":[words[4],words[5]],"channel":words[6],"kind":words[7],"original_rgba_u32":&words[8..12],"candidate_rgba_u32":&words[12..16]})),"diagnostic_only":true,"performance_qualified":false});
        drop(bytes);
        buffer.unmap();
        Ok(proof)
    }
}
fn pitch(width: u32) -> u32 {
    (width * 16).div_ceil(256) * 256
}
// Plane sweep: overlap-free complete horizontal coverage in every vertical slab.
fn validate_partition(size: [u32; 2], tiles: &[[u32; 4]]) -> Result<(), RaySceneError> {
    use std::collections::BTreeMap;
    if size.contains(&0) || size[0] > u32::MAX / 16 - 16 {
        return Err(RaySceneError::Capacity);
    }
    let mut events = Vec::new();
    for (i, &[x, y, w, h]) in tiles.iter().enumerate() {
        if w == 0
            || h == 0
            || x.checked_add(w).is_none_or(|v| v > size[0])
            || y.checked_add(h).is_none_or(|v| v > size[1])
        {
            return Err(RaySceneError::InvalidGeometry);
        }
        events.push((y, 1u8, x, w, i));
        events.push((y + h, 0u8, x, w, i));
    }
    events.sort_unstable();
    let mut active = BTreeMap::<u32, (u32, usize)>::new();
    let mut width = 0u64;
    let mut last = 0;
    for (y, add, x, w, i) in events {
        if y > last && width != u64::from(size[0]) {
            return Err(RaySceneError::InvalidGeometry);
        }
        last = y;
        if add == 0 {
            if active.remove(&x) != Some((x + w, i)) {
                return Err(RaySceneError::InvalidGeometry);
            }
            width -= u64::from(w);
        } else {
            if active.range(..=x).next_back().is_some_and(|(_, v)| v.0 > x)
                || active
                    .range(x..)
                    .next()
                    .is_some_and(|(&start, _)| start < x + w)
            {
                return Err(RaySceneError::InvalidGeometry);
            }
            active.insert(x, (x + w, i));
            width += u64::from(w);
        }
    }
    if last != size[1] || !active.is_empty() {
        return Err(RaySceneError::InvalidGeometry);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_partition_rejects_gaps_overlaps_and_bounds() {
        assert!(validate_partition([8, 4], &[[0, 0, 3, 2], [3, 0, 5, 2], [0, 2, 8, 2]]).is_ok());
        for tiles in [
            vec![],
            vec![[0, 0, 8, 3]],
            vec![[0, 0, 7, 4]],
            vec![[0, 0, 8, 4], [1, 1, 1, 1]],
            vec![[0, 0, 9, 4]],
            vec![[0, 0, 4, 4], [3, 0, 5, 4]],
            vec![[0, 1, 8, 3]],
        ] {
            assert!(validate_partition([8, 4], &tiles).is_err());
        }
    }
    #[test]
    fn comparison_shader_validates() {
        let m = naga::front::wgsl::parse_str(include_str!("world_consumer_pair.wgsl")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&m)
        .unwrap();
    }
}
#[cfg(all(test, target_os = "macos"))]
mod metal_tests {
    use super::*;
    #[test]
    #[ignore = "exclusive Metal GPU, fresh VOXY_CONSUMER_PAIR_PROOF required"]
    fn raw_hdr_comparison_detects_low_bits_and_nonfinite() -> Result<(), Box<dyn std::error::Error>>
    {
        use wgpu::util::DeviceExt;
        let root = std::path::PathBuf::from(
            std::env::var_os("VOXY_CONSUMER_PAIR_PROOF").ok_or("fresh directory required")?,
        );
        std::fs::create_dir(&root)?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raw HDR diagnostic synthetic source"),
            size: wgpu::Extent3d {
                width: 8,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut results = Vec::new();
        for case in [
            "equal",
            "last_pixel_low_bit",
            "signed_zero",
            "subnormal",
            "equal_nan",
            "equal_inf",
            "row_padding_ignored",
        ] {
            let pair = ConsumerPair::new(&device, [8, 4], &[[0, 0, 3, 4], [3, 0, 5, 4]], 1)?;
            let mut a = vec![0x3f800000u32; 64 * 4];
            let mut b = a.clone();
            let tested = 3 * 64 + 2 * 4 + 3;
            match case {
                "last_pixel_low_bit" => b[tested] ^= 1,
                "signed_zero" => {
                    a[tested] = 0;
                    b[tested] = 0x80000000;
                }
                "subnormal" => {
                    a[tested] = 1;
                    b[tested] = 2;
                }
                "equal_nan" => {
                    a[tested] = 0x7fc00001;
                    b[tested] = a[tested];
                }
                "equal_inf" => {
                    a[tested] = 0x7f800000;
                    b[tested] = a[tested];
                }
                "row_padding_ignored" => b[3 * 64 + 40] ^= 1,
                _ => {}
            }
            let upload = |words: &[u32]| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("synthetic HDR texture words"),
                    contents: bytemuck::cast_slice(words),
                    usage: wgpu::BufferUsages::COPY_SRC,
                })
            };
            let a = upload(&a);
            let b = upload(&b);
            let mut encoder = device.create_command_encoder(&Default::default());
            let copy = |encoder: &mut wgpu::CommandEncoder, buffer: &wgpu::Buffer| {
                encoder.copy_buffer_to_texture(
                    wgpu::TexelCopyBufferInfo {
                        buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(256),
                            rows_per_image: Some(4),
                        },
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    texture.size(),
                )
            };
            copy(&mut encoder, &a);
            pair.original(&mut encoder, &texture, 0);
            copy(&mut encoder, &b);
            pair.compare(&mut encoder, &texture, 0);
            queue.submit([encoder.finish()]);
            let proof = pair.proof(&device, &queue)?;
            assert_eq!(proof["compared_pixels"], 12);
            let expected = matches!(case, "equal" | "row_padding_ignored");
            assert_eq!(
                proof["all_native_words_equal_and_finite"], expected,
                "{case}: {proof}"
            );
            if !expected {
                assert_eq!(proof["first_failure"]["xy"], serde_json::json!([2, 3]));
                assert_eq!(proof["first_failure"]["channel"], 3);
            }
            results.push(serde_json::json!({"case":case,"proof":proof}));
        }
        std::fs::write(
            root.join("proof.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"scope":"synthetic raw HDR comparator only","all_cases_passed":true,"cases":results}),
            )?,
        )?;
        Ok(())
    }
}
