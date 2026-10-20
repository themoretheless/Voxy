//! Exact projected segments and hits share one bounded chunk across all pixels.
#[path = "archive_ray_capture.rs"]
mod capture;
use super::{ComputeMemoryBudget, ComputeStorage};
use std::num::NonZeroU64;
const SEGMENT_STRIDE: u64 = 176;
const HIT_STRIDE: u64 = 32;
const TILE_BYTES: u64 = 16;
#[derive(Debug)]
pub(super) struct ArchiveRayStages {
    _buffers: [ComputeStorage; 7],
    capture: Option<capture::RayCapture>,
    work_cursor: Option<ComputeStorage>,
    queue: Option<(Vec<[wgpu::ComputePipeline; 3]>, bool)>,
    hierarchy_reciprocal: bool,
    hierarchy_near_first: bool,
    pipelines: Vec<[wgpu::ComputePipeline; 3]>,
    groups: Vec<[wgpu::BindGroup; 3]>,
    tiles: Vec<([u32; 2], u32)>,
}
fn shape(
    width: u32,
    height: u32,
    binding: u64,
    buffer: u64,
) -> Result<(u32, u32), crate::ComputeError> {
    let capacity = (binding.min(buffer) / SEGMENT_STRIDE).min(u64::from(u32::MAX)) as u32;
    if width == 0 || height == 0 || capacity == 0 {
        return Err(crate::ComputeError::InvalidBuffer);
    }
    let w = width.min(1024).min(capacity);
    let h = height.min(512).min(capacity / w);
    Ok((w, h))
}
impl ArchiveRayStages {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        constants: &[(&str, f64)],
        camera: &ComputeStorage,
        world: &wgpu::Texture,
        normal: &wgpu::Texture,
        background: &wgpu::Texture,
        depth: &wgpu::Texture,
        history: &wgpu::Texture,
        bounds: &wgpu::Texture,
        pixel_bounds: &wgpu::Texture,
        outputs: &[&wgpu::Texture],
        width: u32,
        height: u32,
    ) -> Result<Self, crate::ComputeError> {
        let limits = device.limits();
        let capture_enabled = std::env::var_os("VOXY_STREET_RAY_CAPTURE_DIR").is_some();
        let copy_usage = if capture_enabled {
            wgpu::BufferUsages::COPY_SRC
        } else {
            wgpu::BufferUsages::empty()
        };
        let (mut tw, mut th) = shape(
            width,
            height,
            u64::from(limits.max_storage_buffer_binding_size),
            limits.max_buffer_size,
        )?;
        if std::env::var_os("VOXY_STREET_RAY_CHUNK_TEST").is_some() {
            if std::env::var_os("VOXY_STREET_GI_PRECISION_PROOF").is_none() {
                return Err(crate::ComputeError::Validation(
                    "small ray chunks require precision qualification".into(),
                ));
            }
            tw = tw.min(31);
            th = th.min(17);
        }
        let stride = limits.min_uniform_buffer_offset_alignment.max(16) as usize;
        let mut bytes = Vec::new();
        let mut tiles = Vec::new();
        for y in (0..height).step_by(th as usize) {
            for x in (0..width).step_by(tw as usize) {
                let extent = [tw.min(width - x), th.min(height - y)];
                let offset =
                    u32::try_from(bytes.len()).map_err(|_| crate::ComputeError::InvalidBuffer)?;
                for value in [x, y, extent[0], extent[1]] {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                bytes.resize(bytes.len() + stride - 16, 0);
                tiles.push((extent, offset));
            }
        }
        let [segments, hits, uniforms] =
            ComputeMemoryBudget::for_device(device).allocate_buffers([
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "exact projected segment chunk",
                    size: u64::from(tw) * u64::from(th) * SEGMENT_STRIDE,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE | copy_usage,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "exact depth hit chunk",
                    size: u64::from(tw) * u64::from(th) * HIT_STRIDE,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE | copy_usage,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "exact ray tile descriptors",
                    size: bytes.len() as u64,
                    contents: Some(&bytes),
                    usage: wgpu::BufferUsages::UNIFORM | copy_usage,
                },
            ])?;
        let persistent = std::env::var_os("VOXY_STREET_RAY_PERSISTENT").is_some();
        let work_cursor = if persistent {
            let [cursor] = ComputeMemoryBudget::for_device(device).allocate_buffers([
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "persistent exact ray work cursor",
                    size: 4,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ])?;
            Some(cursor)
        } else {
            None
        };
        let queue_enabled = std::env::var_os("VOXY_STREET_RAY_QUEUE").is_some();
        let hierarchy_reciprocal =
            std::env::var_os("VOXY_STREET_RAY_HIERARCHY_RECIPROCAL").is_some();
        let hierarchy_near_first =
            std::env::var_os("VOXY_STREET_RAY_HIERARCHY_NEAR_FIRST").is_some();
        let queue_proof =
            queue_enabled && std::env::var_os("VOXY_STREET_GI_PRECISION_PROOF").is_some();
        let pixels = u64::from(tw) * u64::from(th);
        let [order, bins, seen, errors] = ComputeMemoryBudget::for_device(device)
            .allocate_buffers([
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "original ray ID queue",
                    size: if queue_enabled { pixels * 4 } else { 4 },
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE | copy_usage,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "ray integer bins and prefix",
                    size: 1056,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "ray queue coverage marks",
                    size: if queue_proof { pixels * 4 } else { 4 },
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "ray queue proof counters and epoch",
                    size: 40,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                },
            ])?;
        let uniform = |binding, dynamic| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        };
        let texture = |binding, is_depth| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: if is_depth {
                    wgpu::TextureSampleType::Depth
                } else {
                    wgpu::TextureSampleType::Float { filterable: false }
                },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let storage = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layouts = [
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("staged ray camera"),
                entries: &[uniform(0, false)],
            }),
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("exact ray chunk and guides"),
                entries: &{
                    let mut entries = vec![
                        texture(6, false),
                        texture(7, false),
                        wgpu::BindGroupLayoutEntry {
                            binding: 8,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba32Float,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        storage(10),
                        storage(11),
                        uniform(12, true),
                        storage(13),
                        storage(14),
                        storage(15),
                        storage(16),
                    ];
                    if persistent {
                        entries.push(storage(17));
                    }
                    entries
                },
            }),
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("exact ray source images"),
                entries: &[
                    texture(0, false),
                    texture(3, true),
                    texture(4, true),
                    texture(7, false),
                    texture(10, false),
                ],
            }),
        ];
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("exact ray stages"),
            bind_group_layouts: &[Some(&layouts[0]), Some(&layouts[1]), Some(&layouts[2])],
            immediate_size: 0,
        });
        let mut pipelines = Vec::new();
        let mut groups = Vec::new();
        let mut queue_pipelines = Vec::new();
        for (kind, output) in outputs.iter().enumerate() {
            let mut flags = constants.to_vec();
            flags.push(("ARCHIVE_TRACE_SSR", kind as f64));
            flags.push((
                "ARCHIVE_RAY_COHERENT",
                if std::env::var_os("VOXY_STREET_RAY_COHERENT").is_some() {
                    1.
                } else {
                    0.
                },
            ));
            flags.push(("ARCHIVE_RAY_QUEUE", if queue_enabled { 1. } else { 0. }));
            flags.push(("ARCHIVE_RAY_QUEUE_PROOF", if queue_proof { 1. } else { 0. }));
            let hierarchy = std::env::var_os("VOXY_STREET_RAY_HIERARCHY").is_some();
            flags.push(("ARCHIVE_STACK_TRAVERSAL", if hierarchy { 1. } else { 0. }));
            flags.push((
                "ARCHIVE_STACK_PIXEL_LEAVES",
                if hierarchy { 1. } else { 0. },
            ));
            flags.push((
                "ARCHIVE_STACK_RECIPROCAL_CACHE",
                if hierarchy_reciprocal { 1. } else { 0. },
            ));
            flags.push((
                "ARCHIVE_STACK_NEAR_FIRST",
                if hierarchy_near_first { 1. } else { 0. },
            ));
            flags.push((
                "ARCHIVE_RAY_SPATIAL_KEY",
                if std::env::var_os("VOXY_STREET_RAY_SPATIAL_KEY").is_some() {
                    1.
                } else {
                    0.
                },
            ));
            if queue_enabled {
                queue_pipelines.push(
                    [
                        "cs_ray_queue_prefix",
                        "cs_ray_queue_scatter",
                        "cs_ray_queue_check",
                    ]
                    .map(|entry| {
                        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some(entry),
                            layout: Some(&layout),
                            module: shader,
                            entry_point: Some(entry),
                            compilation_options: wgpu::PipelineCompilationOptions {
                                constants: &flags,
                                ..Default::default()
                            },
                            cache: None,
                        })
                    }),
                );
            }
            pipelines.push(
                [
                    "cs_ray_project",
                    if persistent {
                        "cs_ray_traverse_persistent"
                    } else if queue_enabled {
                        "cs_ray_traverse_queue"
                    } else {
                        "cs_ray_traverse"
                    },
                    "cs_ray_resolve",
                ]
                .map(|entry| {
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some(entry),
                        layout: Some(&layout),
                        module: shader,
                        entry_point: Some(entry),
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants: &flags,
                            ..Default::default()
                        },
                        cache: None,
                    })
                }),
            );
            let world_view = world.create_view(&Default::default());
            let normal_view = normal.create_view(&Default::default());
            let output_view = output.create_view(&Default::default());
            let mut chunk_entries = vec![
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&world_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: segments.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: hits.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: order.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 14,
                    resource: bins.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 15,
                    resource: seen.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 16,
                    resource: errors.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniforms,
                        offset: 0,
                        size: NonZeroU64::new(TILE_BYTES),
                    }),
                },
            ];
            if let Some(cursor) = &work_cursor {
                chunk_entries.push(wgpu::BindGroupEntry {
                    binding: 17,
                    resource: cursor.as_entire_binding(),
                });
            }
            groups.push([
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &layouts[0],
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera.as_entire_binding(),
                    }],
                }),
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &layouts[1],
                    entries: &chunk_entries,
                }),
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &layouts[2],
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &background.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(
                                &depth.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(
                                &history.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: wgpu::BindingResource::TextureView(
                                &bounds.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 10,
                            resource: wgpu::BindingResource::TextureView(
                                &pixel_bounds.create_view(&Default::default()),
                            ),
                        },
                    ],
                }),
            ]);
        }
        let capture = if capture_enabled {
            Some(capture::RayCapture::new(
                device,
                camera,
                depth,
                &tiles,
                outputs.len(),
                constants,
            )?)
        } else {
            None
        };
        Ok(Self {
            capture,
            work_cursor,
            _buffers: [segments, hits, uniforms, order, bins, seen, errors],
            queue: queue_enabled.then_some((queue_pipelines, queue_proof)),
            hierarchy_reciprocal,
            hierarchy_near_first,
            pipelines,
            groups,
            tiles,
        })
    }
    pub(super) fn persistent_enabled(&self) -> bool {
        self.work_cursor.is_some()
    }
    pub(super) fn capture_enabled(&self) -> bool {
        self.capture.is_some()
    }
    pub(super) fn export_capture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.capture
            .as_ref()
            .ok_or("ray capture is not enabled")?
            .export(device, queue, path)
    }
    pub(super) fn configuration(&self) -> [u64; 3] {
        [
            self.tiles.len() as u64,
            self._buffers[0].size(),
            self._buffers[1].size(),
        ]
    }
    pub(super) fn hierarchy_near_first_enabled(&self) -> bool {
        self.hierarchy_near_first
    }
    pub(super) fn hierarchy_reciprocal_enabled(&self) -> bool {
        self.hierarchy_reciprocal
    }
    pub(super) fn queue_enabled(&self) -> bool {
        self.queue.is_some()
    }
    pub(super) fn queue_proof_buffer(&self) -> Option<&wgpu::Buffer> {
        if self.queue.as_ref()?.1 {
            Some(&self._buffers[6])
        } else {
            None
        }
    }
    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timing: Option<&wgpu::QuerySet>,
    ) {
        let capture_frame = self.capture.as_ref().is_some_and(|c| c.next_frame());
        if self.queue.as_ref().is_some_and(|q| q.1) {
            encoder.clear_buffer(&self._buffers[6], 0, Some(28));
            encoder.clear_buffer(&self._buffers[6], 32, Some(8));
        }
        for (kind, pipelines) in self.pipelines.iter().enumerate() {
            for (tile, (extent, offset)) in self.tiles.iter().enumerate() {
                let index = |stage: u32, end: bool, set: &wgpu::QuerySet| {
                    let base = if self.pipelines.len() == 2 { 12 } else { 10 };
                    if crate::archive_gpu_timing::ray_stage_count(base, self.tiles.len() as u32)
                        == Some(set.count())
                    {
                        Some(crate::archive_gpu_timing::ray_stage_index(
                            base,
                            self.tiles.len() as u32,
                            kind as u32,
                            tile as u32,
                            stage,
                            end,
                        ))
                    } else if !end && tile == 0 && stage == 0 {
                        Some(8 + kind as u32 * 2)
                    } else if end && tile + 1 == self.tiles.len() && stage == 2 {
                        Some(9 + kind as u32 * 2)
                    } else {
                        None
                    }
                };
                let timestamps = |stage: u32, begin: bool, end: bool| {
                    timing.and_then(|set| {
                        let b = if begin {
                            index(stage, false, set)
                        } else {
                            None
                        };
                        let e = if end { index(stage, true, set) } else { None };
                        (b.is_some() || e.is_some()).then_some(wgpu::ComputePassTimestampWrites {
                            query_set: set,
                            beginning_of_pass_write_index: b,
                            end_of_pass_write_index: e,
                        })
                    })
                };
                if self.queue.is_some() {
                    encoder.clear_buffer(&self._buffers[4], 0, None);
                }
                for stage in 0..3 {
                    if stage == 1 {
                        if let Some(cursor) = &self.work_cursor {
                            encoder.clear_buffer(cursor, 0, None);
                        }
                    }
                    {
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some(
                                [
                                    "exact projection and optional scheduling",
                                    "exact depth traversal",
                                    "exact radiance resolve",
                                ][stage],
                            ),
                            timestamp_writes: timestamps(
                                stage as u32,
                                true,
                                stage != 0 || self.queue.is_none(),
                            ),
                            ..Default::default()
                        });
                        pass.set_pipeline(&pipelines[stage]);
                        pass.set_bind_group(0, &self.groups[kind][0], &[]);
                        pass.set_bind_group(1, &self.groups[kind][1], &[*offset]);
                        pass.set_bind_group(2, &self.groups[kind][2], &[]);
                        if stage == 1 && self.queue.is_some() {
                            pass.dispatch_workgroups(
                                if self.work_cursor.is_some() {
                                    (extent[0] * extent[1]).div_ceil(64).min(256)
                                } else {
                                    (extent[0] * extent[1]).div_ceil(64)
                                },
                                1,
                                1,
                            );
                        } else {
                            pass.dispatch_workgroups(
                                extent[0].div_ceil(8),
                                extent[1].div_ceil(8),
                                1,
                            );
                        }
                    }
                    if let Some((extra, proof)) = &self.queue {
                        let run = |encoder: &mut wgpu::CommandEncoder, which: usize, writes| {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some(
                                        [
                                            "ray bin prefix",
                                            "ray ID scatter",
                                            "ray queue coverage proof",
                                        ][which],
                                    ),
                                    timestamp_writes: writes,
                                    ..Default::default()
                                });
                            pass.set_pipeline(&extra[kind][which]);
                            pass.set_bind_group(0, &self.groups[kind][0], &[]);
                            pass.set_bind_group(1, &self.groups[kind][1], &[*offset]);
                            pass.set_bind_group(2, &self.groups[kind][2], &[]);
                            pass.dispatch_workgroups(
                                if which == 0 {
                                    1
                                } else {
                                    (extent[0] * extent[1]).div_ceil(64)
                                },
                                1,
                                1,
                            );
                        };
                        if stage == 0 {
                            run(encoder, 0, None);
                            run(encoder, 1, timestamps(0, false, true));
                        }
                        if stage == 1 && *proof {
                            run(encoder, 2, None);
                        }
                    }
                }
                if capture_frame {
                    if let Some(capture) = &self.capture {
                        if tile == capture.tile {
                            capture.encode_kind(encoder, kind, &self._buffers);
                        }
                    }
                }
            }
        }
        if capture_frame {
            if let Some(capture) = &self.capture {
                capture.finish_encode(encoder, &self._buffers, self.tiles[capture.tile].1);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::shape;
    #[test]
    fn resource_bindings_remain_unique_across_ray_and_material_entries() {
        let module =
            naga::front::wgsl::parse_str(&crate::archive_scene::archive_shader_source()).unwrap();
        let mut bindings = std::collections::BTreeMap::new();
        for (_, global) in module.global_variables.iter() {
            if let Some(binding) = &global.binding {
                assert!(
                    bindings
                        .insert((binding.group, binding.binding), global.name.as_deref())
                        .is_none(),
                    "resource binding reused by {:?}",
                    global.name
                );
            }
        }
    }
    #[test]
    fn queue_bin_buffer_matches_actual_shader_layout() {
        let module =
            naga::front::wgsl::parse_str(&crate::archive_scene::archive_shader_source()).unwrap();
        let ty = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("ArchiveRayBins"))
            .unwrap()
            .1;
        match &ty.inner {
            naga::TypeInner::Struct { span, .. } => assert_eq!(*span, 1056),
            _ => panic!("bins must be a typed atomic block"),
        }
    }
    #[test]
    fn storage_strides_match_actual_wgsl_types() {
        let module =
            naga::front::wgsl::parse_str(&crate::archive_scene::archive_shader_source()).unwrap();
        for (name, expected) in [
            ("staged_segments", super::SEGMENT_STRIDE),
            ("staged_hits", super::HIT_STRIDE),
        ] {
            let variable = module
                .global_variables
                .iter()
                .find(|(_, v)| v.name.as_deref() == Some(name))
                .unwrap()
                .1;
            match module.types[variable.ty].inner {
                naga::TypeInner::Array { stride, .. } => assert_eq!(u64::from(stride), expected),
                _ => panic!("ray transport must be a typed array"),
            }
        }
        let tile = module
            .global_variables
            .iter()
            .find(|(_, v)| v.name.as_deref() == Some("ray_tile"))
            .unwrap()
            .1;
        match module.types[tile.ty].inner {
            naga::TypeInner::Struct { span, .. } => assert_eq!(u64::from(span), super::TILE_BYTES),
            _ => panic!("ray tile ABI changed"),
        }
    }
    #[test]
    fn full_precision_chunks_fit_default_binding() {
        let (w, h) = shape(3840, 2160, 128 * 1024 * 1024, u64::MAX).unwrap();
        assert_eq!((w, h), (1024, 512));
        assert!(u64::from(w) * u64::from(h) * 176 <= 128 * 1024 * 1024);
    }
    #[test]
    fn small_limits_preserve_pixels_by_smaller_chunks() {
        assert_eq!(shape(3841, 2161, 176 * 7, 176 * 7).unwrap(), (7, 1));
        assert!(shape(1, 1, 175, u64::MAX).is_err());
        assert!(shape(0, 1, 176, 176).is_err());
    }
}
