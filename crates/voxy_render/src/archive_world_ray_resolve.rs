//! Prepared original-source query region, encoded before opaque lighting.
use crate::{ArchiveWorldRayProducer, ExactWorldNearest, RaySceneError, WorldRayKind};
use std::sync::Arc;
#[path = "archive_world_consumer_profile.rs"]
mod consumer_profile;
#[path = "archive_world_ray_profile.rs"]
mod profile;
pub(crate) use consumer_profile::ConsumerWorkspace;
pub(crate) fn consumer_profile_workspace(
    device: &wgpu::Device,
    capacity: u32,
) -> Result<ConsumerWorkspace, RaySceneError> {
    consumer_profile::ConsumerStages::buffers(device, capacity)
}

pub(crate) fn consumer_sorted_workspace(
    device: &wgpu::Device,
    capacity: u32,
) -> Result<ConsumerWorkspace, RaySceneError> {
    consumer_profile::ConsumerStages::sorted_buffers(device, capacity)
}

/// Exact rectangular partition; tile dimensions bound memory, never ray coverage.
pub(crate) fn frame_tiles(
    size: [u32; 2],
    tile: [u32; 2],
    capacity: u32,
) -> Result<Vec<[u32; 4]>, RaySceneError> {
    if size.contains(&0) || tile.contains(&0) {
        return Err(RaySceneError::Capacity);
    }
    let extent = [tile[0].min(size[0]), tile[1].min(size[1])];
    if extent[0]
        .checked_mul(extent[1])
        .is_none_or(|count| count > capacity)
    {
        return Err(RaySceneError::Capacity);
    }
    let mut tiles = Vec::new();
    for y in (0..size[1]).step_by(extent[1] as usize) {
        for x in (0..size[0]).step_by(extent[0] as usize) {
            tiles.push([x, y, extent[0].min(size[0] - x), extent[1].min(size[1] - y)]);
        }
    }
    Ok(tiles)
}

#[derive(Debug)]
pub(crate) struct ArchiveWorldRayFrame {
    kinds: Vec<Vec<ArchiveWorldRayRegion>>,
    tiles: Vec<[u32; 4]>,
    audit_enabled: bool,
    encoded_frames: std::sync::atomic::AtomicU64,
    profile: Option<profile::WorldRayProfile>,
}
impl ArchiveWorldRayFrame {
    pub(crate) fn new(
        device: &wgpu::Device,
        templates: Vec<ArchiveWorldRayRegion>,
        tiles: Vec<[u32; 4]>,
    ) -> Result<Self, RaySceneError> {
        if tiles.is_empty() || !(1..=2).contains(&templates.len()) {
            return Err(RaySceneError::InvalidGeometry);
        }
        let mode = templates[0].consumer_stages.as_ref().map(|s| s.is_sorted());
        if templates
            .iter()
            .any(|r| r.consumer_stages.as_ref().map(|s| s.is_sorted()) != mode)
        {
            return Err(RaySceneError::Unsupported);
        }
        let profile = if std::env::var("VOXY_STREET_WORLD_RAY_STAGE_PROFILE").as_deref() == Ok("1")
        {
            if templates.iter().any(|r| !r.job.can_profile_nearest()) {
                return Err(RaySceneError::Unsupported);
            }
            Some(profile::WorldRayProfile::new(
                device,
                templates.len() * tiles.len(),
                templates[0].job.nearest_profile_stage_count(),
                templates[0].job.nearest_profile_has_native_hint(),
                templates[0].job.nearest_profile_has_native_witness(),
                templates.iter().all(|r| r.cache.is_some()),
                templates[0]
                    .consumer_stages
                    .as_ref()
                    .is_some_and(|s| !s.is_sorted()),
                templates[0]
                    .consumer_stages
                    .as_ref()
                    .is_some_and(|s| s.is_sorted()),
                tiles.len(),
            )?)
        } else {
            None
        };
        let mut kinds = Vec::new();
        for template in templates {
            if template.extent != [tiles[0][2], tiles[0][3]] {
                return Err(RaySceneError::InvalidGeometry);
            }
            let mut regions = vec![template];
            for &[x, y, w, h] in &tiles[1..] {
                let region = regions[0].for_region(device, [x, y], [w, h])?;
                regions.push(region);
            }
            kinds.push(regions);
        }
        let audit_enabled = kinds[0][0].audit_enabled;
        Ok(Self {
            kinds,
            tiles,
            audit_enabled,
            encoded_frames: std::sync::atomic::AtomicU64::new(0),
            profile,
        })
    }
    pub(super) fn needs_queries(
        &self,
        history_available: bool,
        all_guides_use_history: bool,
    ) -> bool {
        history_available || !all_guides_use_history || self.audit_enabled
    }
    pub(super) fn consumer_sorted(&self) -> bool {
        self.kinds
            .iter()
            .flatten()
            .all(|r| r.consumer_stages.as_ref().is_some_and(|s| s.is_sorted()))
    }
    pub(super) fn encoded_frames(&self) -> u64 {
        self.encoded_frames
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timing: Option<&wgpu::QuerySet>,
    ) {
        self.encoded_frames
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        for (kind, regions) in self.kinds.iter().enumerate() {
            let base = 8 + kind as u32 * 2;
            for (index, region) in regions.iter().enumerate() {
                region.encode_profiled(
                    encoder,
                    timing,
                    (index == 0).then_some(base),
                    (index + 1 == regions.len()).then_some(base + 1),
                    self.profile.as_ref().map(|p| {
                        (
                            &p.set,
                            (kind * self.tiles.len() + index) as u32 * p.stage_count() * 2,
                            p.consumer_stage_count(),
                            p.resolve_query(kind * self.tiles.len() + index),
                        )
                    }),
                );
            }
        }
    }
    pub(super) fn profile_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
        self.profile
            .as_ref()
            .map(|p| {
                p.snapshot(
                    device,
                    queue,
                    self.tiles.len(),
                    self.kinds.len(),
                    self.encoded_frames(),
                )
            })
            .transpose()
    }
    pub(super) fn work_stats_buffer(&self) -> Option<&wgpu::Buffer> {
        let first = &self.kinds.first()?.first()?.job;
        if self
            .kinds
            .iter()
            .flatten()
            .any(|r| !Arc::ptr_eq(&r.job, first))
        {
            return None;
        }
        first.traversal_work_stats_buffer()
    }
    pub(super) fn count(&self) -> usize {
        self.kinds.len() * self.tiles.len()
    }
    pub(super) fn audit_buffer(&self, index: usize) -> Option<&wgpu::Buffer> {
        self.kinds
            .get(index / self.tiles.len())?
            .get(index % self.tiles.len())
            .map(ArchiveWorldRayRegion::audit_buffer)
    }
    pub(super) fn rectangles(&self) -> &[[u32; 4]] {
        &self.tiles
    }
}

#[derive(Debug)]
pub(crate) struct ArchiveWorldRayRegion {
    producer: ArchiveWorldRayProducer,
    job: Arc<ExactWorldNearest>,
    pipeline: wgpu::ComputePipeline,
    groups: [wgpu::BindGroup; 3],
    extent: [u32; 2],
    audit: crate::ComputeStorage,
    views: [wgpu::TextureView; 3],
    audit_enabled: bool,
    consumer_audit: Option<(wgpu::ComputePipeline, [wgpu::BindGroup; 3])>,
    cache: Option<Arc<crate::ComputeStorage>>,
    cache_check: bool,
    consumer_stages: Option<consumer_profile::ConsumerStages>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_frame_partition_covers_each_pixel_once_and_preserves_partial_edges() {
        for (size, tile, capacity) in [
            ([32, 32], [13, 7], 91),
            ([65, 17], [16, 8], 128),
            ([1, 1], [16, 16], 1),
            ([3840, 2160], [512, 256], 131072),
            ([3840, 2160], [1024, 1024], 1048576),
            ([3840, 2160], [1280, 1280], 1638400),
            ([3840, 2160], [768, 2160], 1658880),
            ([3840, 2160], [3840, 432], 1658880),
            ([3840, 2160], [1024, 512], 524288),
        ] {
            let tiles = frame_tiles(size, tile, capacity).unwrap();
            let mut seen = vec![false; size[0] as usize * size[1] as usize];
            for [x, y, w, h] in tiles {
                assert!(w * h <= capacity);
                for row in y..y + h {
                    for col in x..x + w {
                        let index = (row * size[0] + col) as usize;
                        assert!(!seen[index]);
                        seen[index] = true;
                    }
                }
            }
            assert!(seen.into_iter().all(|v| v));
        }
        assert!(frame_tiles([32, 32], [13, 7], 90).is_err());
        assert!(frame_tiles([0, 32], [13, 7], 91).is_err());
        assert!(frame_tiles([32, 32], [0, 7], 91).is_err());
    }
}
impl ArchiveWorldRayRegion {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        device: &wgpu::Device,
        camera: &crate::ComputeStorage,
        guides: &[Arc<voxy_gpu_core::ManagedTexture>; 2],
        producer: ArchiveWorldRayProducer,
        job: Arc<ExactWorldNearest>,
        kind: WorldRayKind,
        extent: [u32; 2],
        background: &wgpu::Texture,
        depth: &wgpu::Texture,
        history: &wgpu::Texture,
        bounds: &wgpu::Texture,
        output: &wgpu::Texture,
        base_constants: &[(&str, f64)],
        cache_requested: bool,
        consumer_workspace: Option<ConsumerWorkspace>,
    ) -> Result<Self, RaySceneError> {
        if !job.query_kind().supports_surface_nearest() {
            return Err(RaySceneError::Unsupported);
        }
        let [audit] = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                label: "source radiance optional branch audit",
                size: 48,
                contents: None,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            }])
            .map_err(|_| RaySceneError::Capacity)?;
        let cache_mode = cache_requested && job.history_any_hit_enabled();
        if consumer_workspace.is_some() && !cache_mode {
            return Err(RaySceneError::Unsupported);
        }
        let cache_check = cache_mode
            && std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref() == Ok("1");
        let cache = if cache_mode {
            let [buffer] = crate::ComputeMemoryBudget::for_device(device)
                .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                    label: "shared exact Float32 consumer colours",
                    size: u64::from(job.capacity()) * 16,
                    contents: None,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                }])
                .map_err(|_| RaySceneError::Capacity)?;
            Some(Arc::new(buffer))
        } else {
            None
        };
        let cache_source = if cache_mode {
            include_str!("archive_world_consumer_cache.wgsl")
        } else {
            "fn world_consumer_pruned(index:u32,admission:u32,ray:ArchiveRayInput)->bool {return false;} fn world_consumer_cached(index:u32,ray:ArchiveRayInput)->vec4<f32> {return archive_world_query_fallback(ray);}"
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("original-source GI/reflection radiance consumer"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}\n{}\n{}",
                    crate::archive_scene::archive_shader_source(),
                    crate::WORLD_RAY_PRODUCER_WGSL,
                    include_str!("archive_world_ray_resolve.wgsl"),
                    cache_source,
                    consumer_workspace
                        .as_ref()
                        .map_or("", ConsumerWorkspace::source)
                )
                .into(),
            ),
        });
        let audit_enabled = std::env::var_os("VOXY_STREET_WORLD_RAY_RESOLVE_AUDIT").is_some();
        let mut constants = base_constants.to_vec();
        constants.push((
            "ARCHIVE_WORLD_POSITION_BASE",
            job.world_position_offset_bytes()
                .map_or(0., |offset| (offset / 4) as f64),
        ));
        constants.push((
            "ARCHIVE_WORLD_MATERIAL_SIDES",
            if job.query_kind() == crate::WorldRayQueryKind::MaterialSurface {
                1.
            } else {
                0.
            },
        ));
        constants.extend([
            (
                "ARCHIVE_WORLD_RESOLVE_AUDIT",
                if audit_enabled { 1. } else { 0. },
            ),
            (
                "ARCHIVE_WORLD_HIT_WORDS",
                f64::from(job.output_record_words()),
            ),
            (
                "ARCHIVE_TRACE_SSR",
                if matches!(kind, WorldRayKind::Reflection) {
                    1.
                } else {
                    0.
                },
            ),
        ]);
        if cache_mode {
            constants.push((
                "WORLD_CONSUMER_CACHE_CHECK",
                if cache_check { 1. } else { 0. },
            ));
        }
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("source hits to actual GI/reflection texture"),
            layout: None,
            module: &shader,
            entry_point: Some("cs_world_ray_resolve"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants,
                ..Default::default()
            },
            cache: None,
        });
        // Views are kept locally while the immutable bindings are created.
        let world = guides[0].create_view(&Default::default());
        let normal = guides[1].create_view(&Default::default());
        let out = output.create_view(&Default::default());
        let back = background.create_view(&Default::default());
        let z = depth.create_view(&Default::default());
        let old_z = history.create_view(&Default::default());
        let ranges = bounds.create_view(&Default::default());
        let image = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let mut resolve_entries = vec![
            image(6, &world),
            image(7, &normal),
            image(8, &out),
            wgpu::BindGroupEntry {
                binding: 12,
                resource: producer.region_uniform().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 18,
                resource: job.input().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 19,
                resource: job.output().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 20,
                resource: audit.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 21,
                resource: job.source_material_words().as_entire_binding(),
            },
        ];
        if let Some(buffer) = &cache {
            resolve_entries.push(wgpu::BindGroupEntry {
                binding: 22,
                resource: buffer.as_entire_binding(),
            });
        }
        let groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                }],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(1),
                entries: &resolve_entries,
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(2),
                entries: &[
                    image(0, &back),
                    image(3, &z),
                    image(4, &old_z),
                    image(7, &ranges),
                ],
            }),
        ];
        let consumer_audit = if cache_mode
            || (job.history_any_hit_enabled()
                && std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_ZERO_AUDIT").as_deref() == Ok("1"))
        {
            let audit_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("diagnostic complete fallback zero contribution"),
                layout: None,
                module: &shader,
                entry_point: Some(if cache_mode {
                    "cs_world_consumer_cache"
                } else {
                    "cs_world_consumer_zero_audit"
                }),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                cache: None,
            });
            let mut audit_entries = vec![
                image(6, &world),
                image(7, &normal),
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: producer.region_uniform().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 20,
                    resource: audit.as_entire_binding(),
                },
            ];
            if let Some(buffer) = &cache {
                audit_entries.push(wgpu::BindGroupEntry {
                    binding: 17,
                    resource: job.input().as_entire_binding(),
                });
                audit_entries.push(wgpu::BindGroupEntry {
                    binding: 22,
                    resource: buffer.as_entire_binding(),
                });
            }
            let audit_groups = [
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &audit_pipeline.get_bind_group_layout(0),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera.as_entire_binding(),
                    }],
                }),
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &audit_pipeline.get_bind_group_layout(1),
                    entries: &audit_entries,
                }),
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &audit_pipeline.get_bind_group_layout(2),
                    entries: &[
                        image(0, &back),
                        image(3, &z),
                        image(4, &old_z),
                        image(7, &ranges),
                    ],
                }),
            ];
            Some((audit_pipeline, audit_groups))
        } else {
            None
        };
        let consumer_stages = consumer_workspace.map(|buffers| {
            consumer_profile::ConsumerStages::new(
                device,
                &shader,
                &constants,
                buffers,
                camera,
                [&world, &normal],
                producer.region_uniform(),
                job.input(),
                &audit,
                cache.as_ref().unwrap(),
                [&back, &z, &old_z, &ranges],
            )
        });
        Ok(Self {
            producer,
            job,
            pipeline,
            groups,
            extent,
            audit,
            views: [world, normal, out],
            audit_enabled,
            consumer_audit,
            cache,
            cache_check,
            consumer_stages,
        })
    }
    pub(super) fn for_region(
        &self,
        device: &wgpu::Device,
        offset: [u32; 2],
        extent: [u32; 2],
    ) -> Result<Self, RaySceneError> {
        let producer = self
            .producer
            .for_region(device, &self.job, offset, extent)?;
        let [audit] = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffers([voxy_gpu_core::ManagedBufferDescriptor {
                label: "shared world resolve region audit",
                size: 48,
                contents: None,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            }])
            .map_err(|_| RaySceneError::Capacity)?;
        let image = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let mut resolve_entries = vec![
            image(6, &self.views[0]),
            image(7, &self.views[1]),
            image(8, &self.views[2]),
            wgpu::BindGroupEntry {
                binding: 12,
                resource: producer.region_uniform().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 18,
                resource: self.job.input().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 19,
                resource: self.job.output().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 20,
                resource: audit.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 21,
                resource: self.job.source_material_words().as_entire_binding(),
            },
        ];
        if let Some(buffer) = &self.cache {
            resolve_entries.push(wgpu::BindGroupEntry {
                binding: 22,
                resource: buffer.as_entire_binding(),
            });
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shared world resolve region"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &resolve_entries,
        });
        let consumer_audit = self.consumer_audit.as_ref().map(|(pipeline, groups)| {
            let mut entries = vec![
                image(6, &self.views[0]),
                image(7, &self.views[1]),
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: producer.region_uniform().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 20,
                    resource: audit.as_entire_binding(),
                },
            ];
            if let Some(buffer) = &self.cache {
                entries.push(wgpu::BindGroupEntry {
                    binding: 17,
                    resource: self.job.input().as_entire_binding(),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: 22,
                    resource: buffer.as_entire_binding(),
                });
            }
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shared consumer diagnostic/cache region"),
                layout: &pipeline.get_bind_group_layout(1),
                entries: &entries,
            });
            (
                pipeline.clone(),
                [groups[0].clone(), group, groups[2].clone()],
            )
        });
        let consumer_stages = self.consumer_stages.as_ref().map(|stages| {
            stages.for_region(
                device,
                [&self.views[0], &self.views[1]],
                producer.region_uniform(),
                self.job.input(),
                &audit,
                self.cache.as_ref().unwrap(),
            )
        });
        Ok(Self {
            producer,
            job: self.job.clone(),
            pipeline: self.pipeline.clone(),
            groups: [self.groups[0].clone(), group, self.groups[2].clone()],
            extent,
            audit,
            views: self.views.clone(),
            audit_enabled: self.audit_enabled,
            consumer_audit,
            cache: self.cache.clone(),
            cache_check: self.cache_check,
            consumer_stages,
        })
    }
    pub(super) fn audit_buffer(&self) -> &wgpu::Buffer {
        &self.audit
    }
    pub(super) fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_timed(encoder, None, None, None);
    }
    pub(super) fn encode_timed(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timing: Option<&wgpu::QuerySet>,
        begin: Option<u32>,
        end: Option<u32>,
    ) {
        self.encode_profiled(encoder, timing, begin, end, None);
    }
    fn encode_profiled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timing: Option<&wgpu::QuerySet>,
        begin: Option<u32>,
        end: Option<u32>,
        profile: Option<(&wgpu::QuerySet, u32, u32, Option<u32>)>,
    ) {
        if self.cache_check {
            // Error count is sticky for this owner, including earlier frames.
            // Fresh managed buffers are zero-initialized by the device contract.
            encoder.clear_buffer(&self.audit, 0, Some(40));
            encoder.clear_buffer(&self.audit, 44, Some(4));
        } else {
            encoder.clear_buffer(&self.audit, 0, None);
        }
        // A pass has only one timestamp pair. Keep the legacy whole-component
        // start in a separate boundary pass only when detailed profiling is on.
        if profile.is_some() {
            if let Some(set) = timing.filter(|_| begin.is_some()) {
                let _boundary = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("world producer profile boundary"),
                    timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: begin,
                        end_of_pass_write_index: None,
                    }),
                });
            }
        }
        self.producer.encode_timed(
            encoder,
            if let Some((set, base, _, _)) = profile {
                Some(wgpu::ComputePassTimestampWrites {
                    query_set: set,
                    beginning_of_pass_write_index: Some(base),
                    end_of_pass_write_index: Some(base + 1),
                })
            } else {
                timing
                    .filter(|_| begin.is_some())
                    .map(|set| wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: begin,
                        end_of_pass_write_index: None,
                    })
            },
        );
        if let Some(stages) = &self.consumer_stages {
            stages.encode(
                encoder,
                self.extent,
                profile.map(|(set, base, _, _)| (set, base + 2)),
            );
        } else if let Some((pipeline, groups)) = &self.consumer_audit {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("consumer contribution calculation"),
                timestamp_writes: profile.filter(|p| p.2 > 0).map(|(set, base, _, _)| {
                    wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: Some(base + 2),
                        end_of_pass_write_index: Some(base + 3),
                    }
                }),
            });
            pass.set_pipeline(pipeline);
            for (index, group) in groups.iter().enumerate() {
                pass.set_bind_group(index as u32, group, &[]);
            }
            pass.dispatch_workgroups(self.extent[0].div_ceil(8), self.extent[1].div_ceil(8), 1);
        }
        self.job
            .encode_profiled(
                encoder,
                self.producer.count(),
                profile.map(|(set, base, consumer, _)| (set, base + 2 + 2 * consumer)),
            )
            .expect("immutable prepared source region");
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("original-source GI/reflection resolve"),
            timestamp_writes: if let Some((set, _, _, Some(sample))) = profile {
                debug_assert!(end.is_none());
                Some(wgpu::ComputePassTimestampWrites {
                    query_set: set,
                    beginning_of_pass_write_index: Some(sample),
                    end_of_pass_write_index: Some(sample + 1),
                })
            } else {
                timing
                    .filter(|_| end.is_some())
                    .map(|set| wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: None,
                        end_of_pass_write_index: end,
                    })
            },
        });
        pass.set_pipeline(&self.pipeline);
        for (index, group) in self.groups.iter().enumerate() {
            pass.set_bind_group(index as u32, group, &[]);
        }
        pass.dispatch_workgroups(self.extent[0].div_ceil(8), self.extent[1].div_ceil(8), 1);
    }
}
