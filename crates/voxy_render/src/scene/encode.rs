use super::{
    SceneDepthMode, SceneError, SceneGeometry, SceneRenderer, SceneTexture, SceneTransform,
};

#[derive(Debug)]
pub struct SceneDraw<'a> {
    pub geometry: &'a SceneGeometry,
    pub texture: &'a SceneTexture,
    pub transform: &'a SceneTransform,
    /// Draw after world geometry in painter order; ignores and does not write depth.
    pub overlay: bool,
}

/// A disjoint physical-pixel viewport in a shared render target. Draw transforms
/// contain this view's projection; model geometry and textures may be shared.
#[derive(Debug)]
pub struct SceneView<'a> {
    /// [left, top, width, height], matching the view's LOD selection dimensions.
    pub viewport: [u32; 4],
    pub draws: &'a [SceneDraw<'a>],
}

/// Borrowed attachments for one composed view frame. Four-sample color/depth
/// require a single-sample resolve output and overlay depth. X-ray depth, when
/// supplied, must be distinct from world depth and match its sample count/size.
#[derive(Debug)]
pub struct SceneViewTargets<'a> {
    pub color: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub xray_depth: Option<&'a wgpu::TextureView>,
    pub resolve: Option<&'a wgpu::TextureView>,
    pub overlay_depth: &'a wgpu::TextureView,
}

type ScenePipelines<'a> = (
    &'a wgpu::RenderPipeline,
    &'a wgpu::RenderPipeline,
    &'a wgpu::RenderPipeline,
    &'a wgpu::RenderPipeline,
);

struct ViewPass<'a> {
    resolve: Option<&'a wgpu::TextureView>,
    color_load: wgpu::LoadOp<wgpu::Color>,
    depth_load: wgpu::LoadOp<f32>,
    pipelines: ScenePipelines<'a>,
    stages: std::ops::Range<u8>,
    isolated_xray: bool,
}

impl SceneRenderer {
    #[inline]
    fn bind_and_draw_geometry(&self, pass: &mut wgpu::RenderPass<'_>, draw: &SceneDraw<'_>) {
        if let Some(shadow) = &self.shadow {
            pass.set_bind_group(2, &shadow.group, &[]);
        }
        if let Some(environment) = &self.environment {
            pass.set_bind_group(3, &environment.group, &[]);
        }
        pass.set_bind_group(0, &draw.transform.bind_group, &[]);
        pass.set_bind_group(1, &draw.texture.bind_group, &[]);
        pass.set_vertex_buffer(0, draw.geometry.vertices.slice(..));
        pass.set_vertex_buffer(1, draw.geometry.normals.slice(..));
        pass.set_vertex_buffer(2, draw.geometry.material_parameters.slice(..));
        pass.set_vertex_buffer(3, draw.geometry.material_coordinates.slice(..));
        if let Some(tangents) = &draw.geometry.tangents {
            pass.set_vertex_buffer(4, tangents.slice(..));
        }
        pass.set_index_buffer(draw.geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..draw.geometry.index_count, 0, 0..1);
    }

    /// Reveals internals with their own depth buffer, preserving self-occlusion.
    /// `xray_depth` must be a distinct `Depth32Float` target matching the color size.
    /// Opaque X-ray surfaces are order independent. Transparent internals still
    /// require sorting; UI is composed last. World depth remains untouched.
    pub fn encode_with_xray_depth(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        xray_depth: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
    ) {
        let world: Vec<_> = draws
            .iter()
            .filter(|d| !d.overlay && d.geometry.depth_mode != SceneDepthMode::Xray)
            .map(|d| SceneDraw {
                geometry: d.geometry,
                texture: d.texture,
                transform: d.transform,
                overlay: false,
            })
            .collect();
        self.encode(encoder, color, depth, clear, &world);
        for overlay in [false, true] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if overlay {
                    "X-ray UI"
                } else {
                    "isolated X-ray internals"
                }),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: if overlay { depth } else { xray_depth },
                    depth_ops: Some(wgpu::Operations {
                        load: if overlay {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(self.depth_clear_value())
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            self.encode_draw_stages(
                &mut pass,
                draws,
                (
                    &self.solid_opaque_pipeline,
                    &self.world_pipeline,
                    &self.overlay_pipeline,
                    &self.transparent_pipeline,
                ),
                if overlay { 3..4 } else { 2..3 },
                !overlay,
            );
        }
    }

    /// Draw UI/overlay layers over existing composed color without clearing it.
    /// Depth is retained; overlay pipeline ignores depth and does not write it.
    pub fn encode_overlays(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
    ) {
        self.encode_overlays_region(encoder, target, depth, draws, None);
    }

    /// Draw per-view overlays after composition into a full-sized depth/color target.
    pub fn encode_overlays_viewport(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
        viewport: [u32; 4],
    ) {
        self.encode_overlays_region(encoder, target, depth, draws, Some(viewport));
    }

    fn encode_overlays_region(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
        viewport: Option<[u32; 4]>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene overlays after composition"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        if let Some([x, y, w, h]) = viewport {
            pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0., 1.);
            pass.set_scissor_rect(x, y, w, h);
        }
        pass.set_pipeline(&self.overlay_pipeline);
        for draw in draws.iter().filter(|draw| draw.overlay) {
            self.bind_and_draw_geometry(&mut pass, draw);
        }
    }

    /// Draw depth-tested layers over existing colour/depth without clearing or depth writes.
    pub fn encode_transparent_over(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
    ) {
        self.encode_transparent_pass(
            encoder,
            target,
            depth,
            None,
            draws,
            &self.transparent_pipeline,
        );
    }

    /// Composite depth-tested four-sample layers and resolve to a single-sample target.
    /// # Errors
    /// Rejects renderers without four-sample pipelines before recording commands.
    pub fn encode_transparent_over_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        let pipelines = self
            .msaa_pipelines
            .as_ref()
            .ok_or(SceneError::MultisamplingNotEnabled)?;
        self.encode_transparent_pass(encoder, target, depth, Some(resolve), draws, &pipelines.3);
        Ok(())
    }

    fn encode_transparent_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        draws: &[SceneDraw<'_>],
        pipeline: &wgpu::RenderPipeline,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("transparent scene colour composition"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: resolve,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        for draw in draws.iter().filter(|draw| !draw.overlay) {
            self.bind_and_draw_geometry(&mut pass, draw);
        }
    }

    /// Records opaque, transparent, X-ray and overlay layers in that order.
    /// Preserves caller order within each layer; alpha geometry needs back-to-front sorting.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
    ) {
        self.encode_pass(
            encoder,
            color,
            depth,
            None,
            clear,
            draws,
            (
                &self.solid_opaque_pipeline,
                &self.world_pipeline,
                &self.overlay_pipeline,
                &self.transparent_pipeline,
            ),
            None,
        );
    }

    /// Records opaque, transparent, X-ray and overlay layers over existing color and depth
    /// attachments (such as a pre-rendered HDR voxel pass) without clearing either target, so
    /// foreground voxel geometry properly occludes scene draws when sharing the same depth convention.
    pub fn encode_over(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[SceneDraw<'_>],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene over existing HDR color and depth"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            ..Default::default()
        });
        self.encode_draws(
            &mut pass,
            draws,
            (
                &self.solid_opaque_pipeline,
                &self.world_pipeline,
                &self.overlay_pipeline,
                &self.transparent_pipeline,
            ),
        );
    }

    /// Timestamp the ordinary single-sample scene pass. Query indices/type and
    /// device must satisfy wgpu validation. Does not measure presentation or
    /// other shadow/composition passes. Rejects unavailable timing before encoding.
    pub fn encode_profiled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
        timestamps: wgpu::RenderPassTimestampWrites<'_>,
    ) -> Result<(), SceneError> {
        if !self
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            return Err(SceneError::TimestampQueriesNotEnabled);
        }
        self.encode_pass(
            encoder,
            color,
            depth,
            None,
            clear,
            draws,
            (
                &self.solid_opaque_pipeline,
                &self.world_pipeline,
                &self.overlay_pipeline,
                &self.transparent_pipeline,
            ),
            Some(timestamps),
        );
        Ok(())
    }

    /// Records disjoint views in one pass, clearing color/depth once. Each view
    /// has its own viewport and scissor; its overlays cannot escape that region.
    /// Views must reference base-mip, single-sample color/depth targets of equal
    /// dimensions. Separate transforms are required when cameras differ.
    /// # Errors
    /// Rejects empty, overflowing, out-of-target or overlapping regions before
    /// recording commands. Overlap requires separate depth targets/composition.
    pub fn encode_views(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
    ) -> Result<(), SceneError> {
        self.encode_view_pass(
            encoder,
            color,
            depth,
            views,
            ViewPass {
                resolve: None,
                color_load: wgpu::LoadOp::Clear(clear),
                depth_load: wgpu::LoadOp::Clear(self.depth_clear_value()),
                pipelines: (
                    &self.solid_opaque_pipeline,
                    &self.world_pipeline,
                    &self.overlay_pipeline,
                    &self.transparent_pipeline,
                ),
                stages: 0..4,
                isolated_xray: false,
            },
        )
    }

    /// Resolves all disjoint views together after one four-sample pass.
    /// # Errors
    /// Rejects missing MSAA pipelines, inconsistent sample counts/target sizes,
    /// or invalid view regions before recording commands.
    pub fn encode_views_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
    ) -> Result<(), SceneError> {
        let (solid, world, overlay, transparent) = self
            .msaa_pipelines
            .as_ref()
            .ok_or(SceneError::MultisamplingNotEnabled)?;
        self.encode_view_pass(
            encoder,
            color,
            depth,
            views,
            ViewPass {
                resolve: Some(resolve),
                color_load: wgpu::LoadOp::Clear(clear),
                depth_load: wgpu::LoadOp::Clear(self.depth_clear_value()),
                pipelines: (solid, world, overlay, transparent),
                stages: 0..4,
                isolated_xray: false,
            },
        )
    }

    fn encode_view_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        views: &[SceneView<'_>],
        setup: ViewPass<'_>,
    ) -> Result<(), SceneError> {
        let size = color.texture().size();
        if size != depth.texture().size()
            || color.texture().sample_count() != if setup.resolve.is_some() { 4 } else { 1 }
            || depth.texture().sample_count() != color.texture().sample_count()
        {
            return Err(SceneError::InvalidGeometry);
        }
        if setup.resolve.is_some_and(|target| {
            target.texture().size() != size || target.texture().sample_count() != 1
        }) {
            return Err(SceneError::InvalidGeometry);
        }
        validate_view_regions([size.width, size.height], views)?;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("disjoint scene views"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: setup.resolve,
                ops: wgpu::Operations {
                    load: setup.color_load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: setup.depth_load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        for view in views {
            let [x, y, width, height] = view.viewport;
            pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0., 1.);
            pass.set_scissor_rect(x, y, width, height);
            self.encode_draw_stages(
                &mut pass,
                view.draws,
                setup.pipelines,
                setup.stages.clone(),
                setup.isolated_xray,
            );
        }
        Ok(())
    }

    /// Compose native scene, disjoint optical fluid views and global UI using
    /// the same targets for surface and offscreen callers. No work is submitted.
    /// # Errors
    /// Rejects MSAA, duplicate/mismatched fluid views and invalid native targets.
    pub fn encode_view_frame_with_fluids(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: SceneViewTargets<'_>,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
        fluids: &[(usize, &crate::ScreenSpaceFluidRenderer)],
        overlays: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        if targets.resolve.is_some()
            || targets.color.texture().sample_count() != 1
            || overlays.iter().any(|draw| !draw.overlay)
            || fluids.iter().enumerate().any(|(i, (index, fluid))| {
                views
                    .get(*index)
                    .is_none_or(|v| v.viewport[2..] != fluid.size())
                    || fluids[..i].iter().any(|(other, _)| other == index)
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        let output = targets.color;
        let depth = targets.overlay_depth;
        let size = output.texture().size();
        self.encode_view_frame_targets(encoder, targets, clear, views, &[])?;
        for (index, fluid) in fluids {
            let view = &views[*index];
            fluid
                .encode_viewport(
                    self,
                    encoder,
                    output,
                    depth,
                    [size.width, size.height],
                    view.viewport,
                    clear,
                    view.draws,
                )
                .map_err(|_| SceneError::InvalidGeometry)?;
        }
        self.encode_overlays(encoder, output, depth, overlays);
        Ok(())
    }

    /// Composes viewport-local content followed by whole-target editor/game UI.
    /// The final overlay pass starts with the target's full viewport, so it does
    /// not inherit the last scene view's scissor or viewport.
    /// # Errors
    /// Rejects non-overlay global draws and invalid view regions before encoding.
    pub fn encode_view_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
        overlays: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        self.encode_view_frame_targets(
            encoder,
            SceneViewTargets {
                color,
                depth,
                xray_depth: None,
                resolve: None,
                overlay_depth: depth,
            },
            clear,
            views,
            overlays,
        )
    }

    /// Four-sample scene views followed by single-sample full-window UI. UI uses
    /// its own depth attachment and does not read or overwrite world depth.
    /// # Errors
    /// Rejects non-overlay global draws, invalid targets/regions and missing MSAA.
    pub fn encode_view_frame_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        msaa: (&wgpu::TextureView, &wgpu::TextureView),
        output: &wgpu::TextureView,
        overlay_depth: &wgpu::TextureView,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
        overlays: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        self.encode_view_frame_targets(
            encoder,
            SceneViewTargets {
                color: msaa.0,
                depth: msaa.1,
                xray_depth: None,
                resolve: Some(output),
                overlay_depth,
            },
            clear,
            views,
            overlays,
        )
    }

    /// Composes disjoint worlds, self-occluding X-ray internals, view-local UI,
    /// then global UI. World depth is never cleared by the X-ray/UI passes.
    /// # Errors
    /// Validates regions, sizes, sample counts and mandatory nonaliased X-ray
    /// depth before encoding. Views use matching base-mip attachment formats.
    pub fn encode_view_frame_targets(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: SceneViewTargets<'_>,
        clear: wgpu::Color,
        views: &[SceneView<'_>],
        overlays: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        let size = targets.color.texture().size();
        let samples = if targets.resolve.is_some() { 4 } else { 1 };
        let output = targets.resolve.unwrap_or(targets.color);
        let has_xray = views
            .iter()
            .flat_map(|view| view.draws)
            .any(|draw| !draw.overlay && draw.geometry.depth_mode == SceneDepthMode::Xray);
        if overlays.iter().any(|draw| !draw.overlay)
            || targets.color.texture().sample_count() != samples
            || targets.depth.texture().size() != size
            || targets.depth.texture().sample_count() != samples
            || targets.depth.texture().format() != wgpu::TextureFormat::Depth32Float
            || output.texture().size() != size
            || output.texture().sample_count() != 1
            || targets.overlay_depth.texture().size() != size
            || targets.overlay_depth.texture().sample_count() != 1
            || targets.overlay_depth.texture().format() != wgpu::TextureFormat::Depth32Float
            || (has_xray && targets.xray_depth.is_none())
            || targets.xray_depth.is_some_and(|depth| {
                depth.texture() == targets.depth.texture()
                    || depth.texture().size() != size
                    || depth.texture().sample_count() != samples
                    || depth.texture().format() != wgpu::TextureFormat::Depth32Float
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        validate_view_regions([size.width, size.height], views)?;
        let pipelines = if samples == 4 {
            let (solid, world, overlay, transparent) = self
                .msaa_pipelines
                .as_ref()
                .ok_or(SceneError::MultisamplingNotEnabled)?;
            (solid, world, overlay, transparent)
        } else {
            (
                &self.solid_opaque_pipeline,
                &self.world_pipeline,
                &self.overlay_pipeline,
                &self.transparent_pipeline,
            )
        };
        let depth_clear = self.depth_clear_value();
        self.encode_view_pass(
            encoder,
            targets.color,
            targets.depth,
            views,
            ViewPass {
                resolve: targets.resolve,
                color_load: wgpu::LoadOp::Clear(clear),
                depth_load: wgpu::LoadOp::Clear(depth_clear),
                pipelines,
                stages: if has_xray { 0..2 } else { 0..4 },
                isolated_xray: false,
            },
        )?;
        if has_xray {
            self.encode_view_pass(
                encoder,
                targets.color,
                targets.xray_depth.unwrap(),
                views,
                ViewPass {
                    resolve: targets.resolve,
                    color_load: wgpu::LoadOp::Load,
                    depth_load: wgpu::LoadOp::Clear(depth_clear),
                    pipelines,
                    stages: 2..3,
                    isolated_xray: true,
                },
            )?;
            if views
                .iter()
                .flat_map(|view| view.draws)
                .any(|draw| draw.overlay)
            {
                self.encode_view_pass(
                    encoder,
                    targets.color,
                    targets.depth,
                    views,
                    ViewPass {
                        resolve: targets.resolve,
                        color_load: wgpu::LoadOp::Load,
                        depth_load: wgpu::LoadOp::Load,
                        pipelines,
                        stages: 3..4,
                        isolated_xray: false,
                    },
                )?;
            }
        }
        if !overlays.is_empty() {
            self.encode_overlays(encoder, output, targets.overlay_depth, overlays);
        }
        Ok(())
    }

    /// Compose refractive layers against this frame's shaded world, then draw UI.
    /// The sampled background must be distinct from the output. Layer shaders
    /// receive it as their texture. Re-rendering the world into the output avoids
    /// requiring `COPY_DST` usage on acquired surface textures.
    /// # Errors
    /// Rejects a foreign background, unsupported MSAA, X-ray world geometry,
    /// or overlay layers before recording commands.
    pub fn encode_refractive_layers(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        msaa: Option<(&wgpu::TextureView, &wgpu::TextureView)>,
        background: &SceneTexture,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
        layers: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        if !background.belongs_to(&self.device) {
            return Err(SceneError::DeviceMismatch);
        }
        if msaa.is_some() && self.msaa_pipelines.is_none() {
            return Err(SceneError::MultisamplingNotEnabled);
        }
        if draws
            .iter()
            .any(|d| !d.overlay && d.geometry.depth_mode() == SceneDepthMode::Xray)
            || layers.iter().any(|d| d.overlay)
        {
            return Err(SceneError::InvalidGeometry);
        }
        let world: Vec<_> = draws
            .iter()
            .filter(|d| !d.overlay)
            .map(|d| SceneDraw {
                geometry: d.geometry,
                texture: d.texture,
                transform: d.transform,
                overlay: false,
            })
            .collect();
        let wet: Vec<_> = layers
            .iter()
            .map(|d| SceneDraw {
                geometry: d.geometry,
                texture: background,
                transform: d.transform,
                overlay: false,
            })
            .collect();
        let sampled = background
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if let Some((color, multisampled_depth)) = msaa {
            self.encode_msaa4(encoder, color, multisampled_depth, &sampled, clear, &world)?;
            self.encode_msaa4(encoder, color, multisampled_depth, output, clear, &world)?;
            self.encode_transparent_over_msaa4(encoder, color, multisampled_depth, output, &wet)?;
        } else {
            self.encode(encoder, &sampled, depth, clear, &world);
            self.encode(encoder, output, depth, clear, &world);
            self.encode_transparent_over(encoder, output, depth, &wet);
        }
        self.encode_overlays(encoder, output, depth, draws);
        Ok(())
    }

    /// Resolves four-sample geometry coverage into a single-sample color target.
    /// # Errors
    /// Rejects renderers created without four-sample pipelines.
    pub fn encode_msaa4(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: &wgpu::TextureView,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
    ) -> Result<(), SceneError> {
        let (solid, world, overlay, transparent) = self
            .msaa_pipelines
            .as_ref()
            .ok_or(SceneError::MultisamplingNotEnabled)?;
        self.encode_pass(
            encoder,
            color,
            depth,
            Some(resolve),
            clear,
            draws,
            (solid, world, overlay, transparent),
            None,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        clear: wgpu::Color,
        draws: &[SceneDraw<'_>],
        pipelines: ScenePipelines<'_>,
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("general 2D/3D scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.depth_clear_value()),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes,
            ..Default::default()
        });
        self.encode_draws(&mut pass, draws, pipelines);
    }

    fn encode_draws(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[SceneDraw<'_>],
        pipelines: ScenePipelines<'_>,
    ) {
        self.encode_draw_stages(pass, draws, pipelines, 0..4, false);
    }

    fn encode_draw_stages(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[SceneDraw<'_>],
        pipelines: ScenePipelines<'_>,
        stages: std::ops::Range<u8>,
        isolated_xray: bool,
    ) {
        // Opaque depth first, then ghost shells, revealed internals, finally UI.
        for stage in stages {
            pass.set_pipeline(match stage {
                0 => pipelines.1,
                1 => pipelines.3,
                2 if isolated_xray => pipelines.1,
                _ => pipelines.2,
            });
            for draw in draws.iter().filter(|draw| {
                let draw_stage = if draw.overlay {
                    3
                } else {
                    match draw.geometry.depth_mode {
                        SceneDepthMode::SolidOpaque | SceneDepthMode::Opaque => 0,
                        SceneDepthMode::Transparent => 1,
                        SceneDepthMode::Xray => 2,
                    }
                };
                draw_stage == stage
            }) {
                if stage == 0 {
                    let msaa = self
                        .msaa_pipelines
                        .as_ref()
                        .is_some_and(|p| std::ptr::eq(pipelines.1, &raw const p.1));
                    let default_opaque = if draw.geometry.depth_mode == SceneDepthMode::SolidOpaque
                    {
                        pipelines.0
                    } else {
                        pipelines.1
                    };
                    pass.set_pipeline(
                        draw.geometry
                            .opaque_shader
                            .as_ref()
                            .filter(|p| p.0 == self.shader_revision && p.1 == self.transform_layout)
                            .map_or(default_opaque, |p| if msaa { &p.3 } else { &p.2 }),
                    );
                }
                self.bind_and_draw_geometry(pass, draw);
            }
        }
    }
}

pub(super) fn validate_view_regions(
    size: [u32; 2],
    views: &[SceneView<'_>],
) -> Result<(), SceneError> {
    for (index, view) in views.iter().enumerate() {
        let [x, y, w, h] = view.viewport;
        let right = x.checked_add(w).ok_or(SceneError::InvalidGeometry)?;
        let bottom = y.checked_add(h).ok_or(SceneError::InvalidGeometry)?;
        if w == 0 || h == 0 || right > size[0] || bottom > size[1] {
            return Err(SceneError::InvalidGeometry);
        }
        for prior in &views[..index] {
            let [px, py, pw, ph] = prior.viewport;
            if x < px + pw && px < right && y < py + ph && py < bottom {
                return Err(SceneError::InvalidGeometry);
            }
        }
    }
    Ok(())
}
