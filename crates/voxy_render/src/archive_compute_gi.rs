//! Owned opaque GI and reflection scheduling; transparent layers retain their own ray queries.
#[path = "archive_ray_stages.rs"]
mod stages;
#[path = "archive_world_ray_resolve.rs"]
mod world_resolve;
use super::{ComputeMemoryBudget, ComputeStorage, Material, Mesh};
use std::sync::Arc;
pub(crate) use world_resolve::{
    ArchiveWorldRayFrame, ArchiveWorldRayRegion, consumer_profile_workspace,
    consumer_sorted_workspace, frame_tiles,
};
pub(super) fn output_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> Result<Arc<voxy_gpu_core::ManagedTexture>, crate::ComputeError> {
    ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
        label: Some("exact opaque GI result"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
#[derive(Debug)]
pub(super) struct ArchiveComputeGi {
    world: Arc<voxy_gpu_core::ManagedTexture>,
    normal: Arc<voxy_gpu_core::ManagedTexture>,
    reference: Option<(
        Arc<voxy_gpu_core::ManagedTexture>,
        [wgpu::RenderPipeline; 2],
    )>,
    ssr_reference: Option<(
        Arc<voxy_gpu_core::ManagedTexture>,
        [wgpu::RenderPipeline; 2],
    )>,
    stages: Option<stages::ArchiveRayStages>,
    world_regions: Vec<ArchiveWorldRayRegion>,
    world_frame: Option<ArchiveWorldRayFrame>,
    ray_constants: Vec<(&'static str, f64)>,
    guides: [wgpu::RenderPipeline; 2],
    ssr: Option<(wgpu::ComputePipeline, [wgpu::BindGroup; 3])>,
    trace: wgpu::ComputePipeline,
    groups: [wgpu::BindGroup; 3],
    width: u32,
    height: u32,
}
impl ArchiveComputeGi {
    pub(super) fn world_ray_guides(&self) -> [Arc<voxy_gpu_core::ManagedTexture>; 2] {
        [self.world.clone(), self.normal.clone()]
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        guide_layout: &wgpu::PipelineLayout,
        reference_layout: &wgpu::PipelineLayout,
        vertices: &[Option<wgpu::VertexBufferLayout<'_>>],
        camera: &ComputeStorage,
        background: &wgpu::Texture,
        depth: &wgpu::Texture,
        history: &wgpu::Texture,
        bounds: &wgpu::Texture,
        pixel_bounds: &wgpu::Texture,
        output: &wgpu::Texture,
        ssr_output: &wgpu::Texture,
        ssr_enabled: bool,
        width: u32,
        height: u32,
    ) -> Result<Self, crate::ComputeError> {
        let allocate = |label| {
            ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let world = allocate("full float32 GI world input")?;
        let normal = allocate("full float32 GI final normal input")?;
        let flags = [
            (
                "ARCHIVE_DDA_INTERVAL_CACHE",
                "VOXY_STREET_DDA_INTERVAL_CACHE",
            ),
            (
                "ARCHIVE_RAY_FORCE_FALLBACK",
                "VOXY_STREET_RAY_FORCE_FALLBACK",
            ),
            ("ARCHIVE_DDA_PLANE_CACHE", "VOXY_STREET_DDA_PLANE_CACHE"),
            ("ARCHIVE_DDA_POINT_CACHE", "VOXY_STREET_DDA_POINT_CACHE"),
            ("ARCHIVE_PAIRED_DDA", "VOXY_STREET_PAIRED_DDA"),
            ("ARCHIVE_DDA_TILES", "VOXY_STREET_PAIRED_DDA_TILES"),
            ("ARCHIVE_DDA_CELL_BOUNDS", "VOXY_STREET_DDA_CELL_BOUNDS"),
            ("ARCHIVE_DDA_FAR_BOUND", "VOXY_STREET_DDA_FAR_BOUND"),
            ("ARCHIVE_DDA_HIERARCHY", "VOXY_STREET_DDA_HIERARCHY"),
            ("ARCHIVE_DDA_EDGE_CACHE", "VOXY_STREET_DDA_EDGE_CACHE"),
            ("ARCHIVE_REFINE_CACHE", "VOXY_STREET_REFINE_CACHE"),
        ];
        let mut constants: Vec<_> = flags
            .iter()
            .map(|&(name, env)| {
                (
                    name,
                    if std::env::var_os(env).is_some() {
                        1.
                    } else {
                        0.
                    },
                )
            })
            .collect();
        constants.push((
            "ARCHIVE_DDA_TILE_EDGE_CACHE",
            if std::env::var("VOXY_STREET_DDA_TILE_EDGE_CACHE").as_deref() == Ok("1") {
                1.
            } else {
                0.
            },
        ));
        constants.push((
            "ARCHIVE_RAY_BOUNDS",
            if std::env::var_os("VOXY_STREET_RAY_REFERENCE").is_some() {
                0.
            } else {
                1.
            },
        ));
        constants.push(("ARCHIVE_COMPUTE_SSR", if ssr_enabled { 1. } else { 0. }));
        let raster = |entry: &str,
                      pipeline_layout: &wgpu::PipelineLayout,
                      targets: &[Option<wgpu::ColorTargetState>]| {
            [false, true].map(|double| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("exact opaque ray inputs"),
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: vertices,
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: if double { None } else { Some(wgpu::Face::Back) },
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(wgpu::CompareFunction::Equal),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: shader,
                        entry_point: Some(entry),
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants: &constants,
                            ..Default::default()
                        },
                        targets,
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
        };
        let colour = || {
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba32Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        };
        let guides = raster("fs_gi_guides", guide_layout, &[colour(), colour()]);
        let reference = if std::env::var_os("VOXY_STREET_GI_PRECISION_PROOF").is_some() {
            Some((
                output_texture(device, width, height)?,
                raster("fs_gi_reference", reference_layout, &[colour()]),
            ))
        } else {
            None
        };

        let ssr_reference = if reference.is_some() && ssr_enabled {
            Some((
                output_texture(device, width, height)?,
                raster("fs_ssr_reference", reference_layout, &[colour()]),
            ))
        } else {
            None
        };
        let trace = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("full resolution exact opaque GI"),
            layout: None,
            module: shader,
            entry_point: Some("cs_opaque_gi"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants,
                ..Default::default()
            },
            cache: None,
        });
        let groups_for = |trace: &wgpu::ComputePipeline, output: &wgpu::Texture| {
            [
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &trace.get_bind_group_layout(0),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera.as_entire_binding(),
                    }],
                }),
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &trace.get_bind_group_layout(1),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: wgpu::BindingResource::TextureView(
                                &world.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: wgpu::BindingResource::TextureView(
                                &normal.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 8,
                            resource: wgpu::BindingResource::TextureView(
                                &output.create_view(&Default::default()),
                            ),
                        },
                    ],
                }),
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &trace.get_bind_group_layout(2),
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
                    ],
                }),
            ]
        };
        let groups = groups_for(&trace, output);
        let ssr = if ssr_enabled {
            let mut ssr_constants = constants.clone();
            ssr_constants.push(("ARCHIVE_TRACE_SSR", 1.));
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("full resolution exact opaque reflections"),
                layout: None,
                module: shader,
                entry_point: Some("cs_opaque_gi"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &ssr_constants,
                    ..Default::default()
                },
                cache: None,
            });
            let bindings = groups_for(&pipeline, ssr_output);
            Some((pipeline, bindings))
        } else {
            None
        };
        let stages = if std::env::var_os("VOXY_STREET_RAY_STAGES").is_some() {
            if std::env::var_os("VOXY_STREET_PAIRED_DDA").is_none() {
                return Err(crate::ComputeError::Validation(
                    "staged rays require paired DDA".into(),
                ));
            }
            let outputs = if ssr_enabled {
                vec![output, ssr_output]
            } else {
                vec![output]
            };
            Some(stages::ArchiveRayStages::new(
                device,
                shader,
                &constants,
                camera,
                &world,
                &normal,
                background,
                depth,
                history,
                bounds,
                pixel_bounds,
                &outputs,
                width,
                height,
            )?)
        } else {
            None
        };
        Ok(Self {
            world,
            normal,
            reference,
            stages,
            world_regions: Vec::new(),
            world_frame: None,
            ray_constants: constants,
            ssr_reference,
            guides,
            trace,
            ssr,
            groups,
            width,
            height,
        })
    }
    pub(super) fn world_query_encoded_frames(&self) -> u64 {
        self.world_frame.as_ref().map_or(0, |f| f.encoded_frames())
    }
    pub(super) fn world_work_stats_buffer(&self) -> Option<&wgpu::Buffer> {
        self.world_frame.as_ref()?.work_stats_buffer()
    }
    pub(super) fn world_profile_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
        if let Some(frame) = &self.world_frame {
            frame.profile_snapshot(device, queue)
        } else {
            Ok(None)
        }
    }
    pub(super) fn world_frame_enabled(&self) -> bool {
        self.world_frame.is_some()
    }
    pub(super) fn world_frame_tiles(&self) -> Option<&[[u32; 4]]> {
        Some(self.world_frame.as_ref()?.rectangles())
    }
    pub(super) fn tile_edge_cache_enabled(&self) -> bool {
        self.ray_constants
            .iter()
            .any(|&(name, value)| name == "ARCHIVE_DDA_TILE_EDGE_CACHE" && value == 1.)
    }
    pub(super) fn world_consumer_sorted(&self) -> bool {
        self.world_frame
            .as_ref()
            .is_some_and(|f| f.consumer_sorted())
    }
    pub(super) fn set_world_frame(&mut self, frame: ArchiveWorldRayFrame) {
        self.world_regions.clear();
        self.world_frame = Some(frame);
    }
    pub(super) fn world_resolve_constants(&self) -> &[(&str, f64)] {
        &self.ray_constants
    }
    pub(super) fn supports_world_kind(&self, kind: crate::WorldRayKind) -> bool {
        self.ray_constants
            .iter()
            .any(|&(name, value)| name == "ARCHIVE_PAIRED_DDA" && value == 1.)
            && (!matches!(kind, crate::WorldRayKind::Reflection) || self.ssr.is_some())
    }
    pub(super) fn clear_world_regions(&mut self) {
        self.world_regions.clear();
        self.world_frame = None;
    }
    pub(super) fn add_world_region(&mut self, region: ArchiveWorldRayRegion) {
        self.world_regions.push(region);
    }
    pub(super) fn world_region_audit_buffer(&self, index: usize) -> Option<&wgpu::Buffer> {
        if let Some(frame) = &self.world_frame {
            frame.audit_buffer(index)
        } else {
            Some(self.world_regions.get(index)?.audit_buffer())
        }
    }
    pub(super) fn world_region_count(&self) -> usize {
        self.world_frame
            .as_ref()
            .map_or(self.world_regions.len(), |f| f.count())
    }
    pub(super) fn reference_texture(&self) -> Option<&wgpu::Texture> {
        self.reference.as_ref().map(|(image, _)| &***image)
    }
    pub(super) fn ssr_reference_texture(&self) -> Option<&wgpu::Texture> {
        self.ssr_reference.as_ref().map(|(image, _)| &***image)
    }
    pub(super) fn ray_persistent_enabled(&self) -> bool {
        self.world_frame.is_none() && self.stages.as_ref().is_some_and(|s| s.persistent_enabled())
    }
    pub(super) fn capture_enabled(&self) -> bool {
        self.stages.as_ref().is_some_and(|s| s.capture_enabled())
    }
    pub(super) fn export_capture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.stages
            .as_ref()
            .ok_or("ray stages are not active")?
            .export_capture(device, queue, path)
    }
    pub(super) fn hierarchy_near_first_enabled(&self) -> bool {
        self.stages
            .as_ref()
            .is_some_and(|s| s.hierarchy_near_first_enabled())
    }
    pub(super) fn hierarchy_reciprocal_enabled(&self) -> bool {
        self.stages
            .as_ref()
            .is_some_and(|s| s.hierarchy_reciprocal_enabled())
    }
    pub(super) fn ray_queue_enabled(&self) -> bool {
        self.world_frame.is_none() && self.stages.as_ref().is_some_and(|s| s.queue_enabled())
    }
    pub(super) fn ray_queue_proof_buffer(&self) -> Option<&wgpu::Buffer> {
        if self.world_frame.is_some() {
            None
        } else {
            self.stages.as_ref()?.queue_proof_buffer()
        }
    }
    pub(super) fn stages_configuration(&self) -> Option<[u64; 3]> {
        if self.world_frame.is_some() {
            None
        } else {
            self.stages.as_ref().map(|s| s.configuration())
        }
    }
    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        camera: &wgpu::BindGroup,
        frame: &wgpu::BindGroup,
        environment: &wgpu::BindGroup,
        materials: &[Material],
        meshes: &[Mesh],
        timing: Option<&wgpu::QuerySet>,
        history_available: bool,
    ) {
        let world = self.world.create_view(&Default::default());
        let normal = self.normal.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("full precision opaque GI inputs"),
                timestamp_writes: timing.map(|set| wgpu::RenderPassTimestampWrites {
                    query_set: set,
                    beginning_of_pass_write_index: Some(6),
                    end_of_pass_write_index: Some(7),
                }),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &world,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &normal,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, camera, &[]);
            for mesh in meshes {
                let material = &materials[mesh.material];
                if material.transparent || mesh.visible_count == 0 {
                    continue;
                }
                pass.set_pipeline(&self.guides[usize::from(material.double_sided)]);
                pass.set_bind_group(1, &material.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.visible_instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(culling) = &mesh.culling {
                    pass.draw_indexed_indirect(culling.indirect(), 0);
                } else {
                    pass.draw_indexed(0..mesh.count, 0, 0..mesh.visible_count);
                }
            }
        }
        // Probe faces and history resets use the identical monolithic query.
        // Avoid streaming large intermediate records for guaranteed GI misses.
        let all_guides_use_history = materials.iter().all(|m| m.transparent || m.history_guides);
        if let Some(frame) = self
            .world_frame
            .as_ref()
            .filter(|frame| frame.needs_queries(history_available, all_guides_use_history))
        {
            frame.encode(encoder, timing);
        } else if let Some(stages) = self
            .stages
            .as_ref()
            .filter(|_| history_available && self.world_frame.is_none())
        {
            stages.encode(encoder, timing);
        } else {
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("separate exact opaque GI traversal"),
                    timestamp_writes: timing.map(|set| wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: Some(8),
                        end_of_pass_write_index: Some(9),
                    }),
                });
                pass.set_pipeline(&self.trace);
                for (i, group) in self.groups.iter().enumerate() {
                    pass.set_bind_group(i as u32, group, &[]);
                }
                pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
            }
            if let Some((pipeline, groups)) = &self.ssr {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("separate exact opaque reflections"),
                    timestamp_writes: timing.map(|set| wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: Some(10),
                        end_of_pass_write_index: Some(11),
                    }),
                });
                pass.set_pipeline(pipeline);
                for (i, group) in groups.iter().enumerate() {
                    pass.set_bind_group(i as u32, group, &[]);
                }
                pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
            }
        }
        for region in &self.world_regions {
            region.encode(encoder);
        }
        for (image, pipelines) in [self.reference.as_ref(), self.ssr_reference.as_ref()]
            .into_iter()
            .flatten()
        {
            let view = image.create_view(&Default::default());

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("raw float32 GI precision reference"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, camera, &[]);
            pass.set_bind_group(2, frame, &[]);
            pass.set_bind_group(3, environment, &[]);
            for mesh in meshes {
                let material = &materials[mesh.material];
                if material.transparent || mesh.visible_count == 0 {
                    continue;
                }
                pass.set_pipeline(&pipelines[usize::from(material.double_sided)]);
                pass.set_bind_group(1, &material.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, mesh.visible_instances.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(culling) = &mesh.culling {
                    pass.draw_indexed_indirect(culling.indirect(), 0);
                } else {
                    pass.draw_indexed(0..mesh.count, 0, 0..mesh.visible_count);
                }
            }
        }
    }
}
