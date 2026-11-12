use super::{SceneSurface, create_depth};
use crate::{RenderOutcome, RendererError, SceneDraw, SceneRenderer, SurfaceState};

/// Aligned GPU inputs from the last presented frame of this surface.
/// Borrowing prevents surface resize/render until the consumer releases the inputs.
#[derive(Debug)]
pub struct TemporalFrame<'a> {
    pub presentation_id: u64,
    /// First frame after enabling inputs or explicitly invalidating history.
    pub reset_history: bool,
    pub motion: &'a wgpu::Texture,
    /// World color before overlay: sRGB by default, linear `RGBA16Float` in HDR mode.
    pub color: &'a wgpu::Texture,
    pub depth: &'a wgpu::Texture,
}

#[derive(Debug)]
pub(super) struct MotionTarget {
    pub(super) renderer: SceneRenderer,
    pub(super) color_renderer: Option<SceneRenderer>,
    pub(super) texture: wgpu::Texture,
    pub(super) color: wgpu::Texture,
    pub(super) frame_id: Option<u64>,
    pub(super) reset_pending: bool,
    pub(super) frame_reset: bool,
}

impl MotionTarget {
    pub(super) fn inputs<'a>(
        &'a self,
        presentation_id: u64,
        depth: &'a wgpu::Texture,
    ) -> TemporalFrame<'a> {
        TemporalFrame {
            presentation_id,
            reset_history: self.reset_pending,
            motion: &self.texture,
            color: &self.color,
            depth,
        }
    }
}

pub(super) fn create_motion_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene motion vectors"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

pub(super) fn create_temporal_color(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("temporal world color without overlay"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

impl SceneSurface {
    /// Enables a separate opaque-world velocity pass sharing frame submission.
    /// HDR presentation automatically captures linear RGBA16 world color, never PQ.
    /// # Errors
    /// Rejects unsupported float attachment formats or invalid motion shader pipelines.
    pub async fn enable_motion_vectors(&mut self) -> Result<(), crate::SceneShaderError> {
        if self.motion.is_some() {
            return Ok(());
        }
        let required = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        let features = self
            .adapter
            .get_texture_format_features(wgpu::TextureFormat::Rgba16Float);
        if !features.allowed_usages.contains(required)
            || !features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::BLENDABLE)
        {
            return Err(crate::SceneShaderError(
                "motion vectors require blendable RGBA16Float attachments".into(),
            ));
        }
        let hdr_capture = self.config.color_space.is_hdr();
        let color_format = if hdr_capture {
            wgpu::TextureFormat::Rgba16Float
        } else {
            self.config.format.add_srgb_suffix()
        };
        if !self
            .adapter
            .get_texture_format_features(color_format)
            .allowed_usages
            .contains(required)
        {
            return Err(crate::SceneShaderError(
                "temporal color format is not sampleable/readable".into(),
            ));
        }
        let mut renderer = SceneRenderer::new(&self.device, wgpu::TextureFormat::Rgba16Float);
        renderer
            .reload_shader(&self.device, crate::MOTION_SCENE_SHADER)
            .await?;
        let texture = create_motion_texture(
            &self.device,
            self.config.width,
            self.config.height,
            wgpu::TextureFormat::Rgba16Float,
        );
        let color = create_temporal_color(
            &self.device,
            self.config.width,
            self.config.height,
            color_format,
        );
        self.motion = Some(MotionTarget {
            renderer,
            color_renderer: hdr_capture.then(|| SceneRenderer::new(&self.device, color_format)),
            texture,
            color,
            frame_id: None,
            reset_pending: true,
            frame_reset: true,
        });
        Ok(())
    }

    /// Use two-channel `RG16Float` backward motion for Ray Reconstruction inputs.
    /// Preserves the temporal color mode and invalidates history when switching.
    /// # Errors
    /// Reports unsupported attachments or motion shader pipeline errors.
    pub async fn enable_two_channel_motion_vectors(
        &mut self,
    ) -> Result<(), crate::SceneShaderError> {
        self.enable_motion_vectors().await?;
        if self
            .motion
            .as_ref()
            .is_some_and(|motion| motion.texture.format() == wgpu::TextureFormat::Rg16Float)
        {
            return Ok(());
        }
        let features = self
            .adapter
            .get_texture_format_features(wgpu::TextureFormat::Rg16Float);
        let required = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        if !features.allowed_usages.contains(required)
            || !features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::BLENDABLE)
        {
            return Err(crate::SceneShaderError(
                "motion vectors require blendable RG16Float attachments".into(),
            ));
        }
        let mut renderer = SceneRenderer::new(&self.device, wgpu::TextureFormat::Rg16Float);
        renderer
            .reload_shader(&self.device, crate::MOTION_SCENE_SHADER)
            .await?;
        let texture = create_motion_texture(
            &self.device,
            self.config.width,
            self.config.height,
            wgpu::TextureFormat::Rg16Float,
        );
        if let Some(motion) = &mut self.motion {
            motion.renderer = renderer;
            motion.texture = texture;
        }
        self.invalidate_temporal_history();
        Ok(())
    }

    /// Capture temporal world color in linear `RGBA16Float` for HDR processing.
    /// Uses the standard scene shader; custom color shaders can be reloaded through
    /// `hdr_temporal_renderer_mut`. Presentation retains the configured surface format;
    /// Compose through tone mapping for SDR, direct linear blit for scRGB or PQ
    /// encoding for HDR10 in the submitted hook.
    /// # Errors
    /// Reports unsupported float attachments or motion shader compilation errors.
    pub async fn enable_hdr_temporal_color(&mut self) -> Result<(), crate::SceneShaderError> {
        if self
            .motion
            .as_ref()
            .is_some_and(|motion| motion.color_renderer.is_some())
        {
            return Ok(());
        }
        self.enable_motion_vectors().await?;
        if let Some(motion) = &mut self.motion {
            motion.color_renderer = Some(SceneRenderer::new(
                &self.device,
                wgpu::TextureFormat::Rgba16Float,
            ));
            motion.color = create_temporal_color(
                &self.device,
                self.config.width,
                self.config.height,
                wgpu::TextureFormat::Rgba16Float,
            );
        }
        self.invalidate_temporal_history();
        Ok(())
    }

    /// HDR capture renderer for custom shader reload; does not change presentation.
    pub fn hdr_temporal_renderer_mut(&mut self) -> Option<&mut SceneRenderer> {
        self.motion.as_mut()?.color_renderer.as_mut()
    }

    /// Invalidate temporal inputs after a camera cut, scene replacement or recovery.
    /// The next successfully presented frame carries `reset_history = true`.
    /// This does not update object motion matrices; callers reset their own histories.
    pub fn invalidate_temporal_history(&mut self) {
        if let Some(motion) = &mut self.motion {
            motion.frame_id = None;
            motion.reset_pending = true;
        }
    }

    /// Velocity texture and presentation ID of its last successfully presented frame.
    /// IDs start at one and are local to this surface. Skipped frames retain their
    /// old ID; resize/suspension invalidate access until another frame is presented.
    /// Presentation is asynchronous: this does not prove GPU completion.
    #[must_use]
    pub fn motion_frame(&self) -> Option<(&wgpu::Texture, u64)> {
        let motion = self.motion.as_ref()?;
        Some((&motion.texture, motion.frame_id?))
    }

    /// Returns depth and velocity from one successfully presented frame.
    /// Depth is device-space [0,1], cleared to 1, with opaque world writes only.
    /// This is not a GPU completion fence or native SDK resource-state transition.
    #[must_use]
    pub fn temporal_frame(&self) -> Option<TemporalFrame<'_>> {
        let (motion, presentation_id) = self.motion_frame()?;
        Some(TemporalFrame {
            presentation_id,
            reset_history: self.motion.as_ref()?.frame_reset,
            motion,
            color: &self.motion.as_ref()?.color,
            depth: &self.depth,
        })
    }

    #[must_use]
    pub fn motion_texture(&self) -> Option<&wgpu::Texture> {
        self.motion_frame().map(|(texture, _)| texture)
    }

    /// # Errors
    /// Reports surface loss requiring recreation by the platform shell.
    pub fn render_scene(
        &mut self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
    ) -> Result<RenderOutcome, RendererError> {
        self.render_scene_with_temporal(scene, draws, |_, _| {})
    }

    /// Encode a temporal consumer after scene passes and before submission/present.
    /// The callback runs only when motion inputs are enabled and acquisition succeeds.
    /// Its input ID is a candidate presentation ID; `temporal_frame` publishes it
    /// after presentation. Resources have pending GPU writes on the supplied encoder.
    /// This hook provides neither native SDK resource transitions nor completion fences.
    ///
    /// # Errors
    /// Reports acquisition failure or exhausted presentation IDs.
    pub fn render_scene_with_temporal<F>(
        &mut self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
        consume: F,
    ) -> Result<RenderOutcome, RendererError>
    where
        F: FnOnce(&TemporalFrame<'_>, &mut wgpu::CommandEncoder),
    {
        self.render_scene_with_temporal_hooks(scene, draws, consume, |_, _, _, _| Ok(()))
    }

    fn encode_presentation(
        &mut self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
        layers: &[SceneDraw<'_>],
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), RendererError> {
        let world;
        let draws = if self.motion.is_some() {
            world = draws
                .iter()
                .filter(|draw| !draw.overlay)
                .map(|draw| SceneDraw {
                    geometry: draw.geometry,
                    texture: draw.texture,
                    transform: draw.transform,
                    overlay: false,
                })
                .collect::<Vec<_>>();
            &world
        } else {
            draws
        };
        if !layers.is_empty() {
            if self.optical_background.is_none() {
                self.optical_background = Some(
                    scene
                        .create_sampled_color(&self.device, self.config.width, self.config.height)
                        .map_err(RendererError::Scene)?,
                );
            }
            scene
                .encode_refractive_layers(
                    encoder,
                    view,
                    &self.depth_view,
                    self.msaa.as_ref().map(|m| (&m.color, &m.depth)),
                    self.optical_background.as_ref().unwrap(),
                    wgpu::Color::BLACK,
                    draws,
                    layers,
                )
                .map_err(RendererError::Scene)?;
            return Ok(());
        }
        if draws
            .iter()
            .any(|draw| !draw.overlay && draw.geometry.depth_mode() == crate::SceneDepthMode::Xray)
        {
            let xray_depth = self.xray_depth_view.get_or_insert_with(|| {
                create_depth(&self.device, self.config.width, self.config.height)
                    .create_view(&wgpu::TextureViewDescriptor::default())
            });
            scene.encode_with_xray_depth(
                encoder,
                view,
                &self.depth_view,
                xray_depth,
                wgpu::Color::BLACK,
                draws,
            );
        } else if let Some(msaa) = &self.msaa {
            scene
                .encode_msaa4(
                    encoder,
                    &msaa.color,
                    &msaa.depth,
                    view,
                    wgpu::Color::BLACK,
                    draws,
                )
                .map_err(RendererError::Scene)?;
        } else {
            scene.encode(encoder, view, &self.depth_view, wgpu::Color::BLACK, draws);
        }
        Ok(())
    }

    fn encode_temporal_inputs(
        &mut self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
        layers: &[SceneDraw<'_>],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RendererError> {
        let Some(motion) = &self.motion else {
            return Ok(());
        };
        let world: Vec<_> = draws
            .iter()
            .filter(|draw| !draw.overlay)
            .map(|draw| SceneDraw {
                geometry: draw.geometry,
                texture: draw.texture,
                transform: draw.transform,
                overlay: false,
            })
            .collect();
        let opaque: Vec<_> = world
            .iter()
            .filter(|draw| draw.geometry.depth_mode() == crate::SceneDepthMode::Opaque)
            .map(|draw| SceneDraw {
                geometry: draw.geometry,
                texture: draw.texture,
                transform: draw.transform,
                overlay: false,
            })
            .collect();
        let color = motion
            .color
            .create_view(&wgpu::TextureViewDescriptor::default());
        let renderer = motion.color_renderer.as_ref().unwrap_or(scene);
        if !layers.is_empty() {
            if self
                .temporal_optical_background
                .as_ref()
                .is_none_or(|t| t.texture().format() != motion.color.format())
            {
                self.temporal_optical_background = Some(
                    renderer
                        .create_sampled_color(&self.device, self.config.width, self.config.height)
                        .map_err(RendererError::Scene)?,
                );
            }
            renderer
                .encode_refractive_layers(
                    encoder,
                    &color,
                    &self.depth_view,
                    None,
                    self.temporal_optical_background.as_ref().unwrap(),
                    wgpu::Color::BLACK,
                    &world,
                    layers,
                )
                .map_err(RendererError::Scene)?;
        } else if world
            .iter()
            .any(|draw| draw.geometry.depth_mode() == crate::SceneDepthMode::Xray)
        {
            let xray_depth = self.xray_depth_view.get_or_insert_with(|| {
                create_depth(&self.device, self.config.width, self.config.height)
                    .create_view(&wgpu::TextureViewDescriptor::default())
            });
            renderer.encode_with_xray_depth(
                encoder,
                &color,
                &self.depth_view,
                xray_depth,
                wgpu::Color::BLACK,
                &world,
            );
        } else {
            renderer.encode(
                encoder,
                &color,
                &self.depth_view,
                wgpu::Color::BLACK,
                &world,
            );
        }
        motion.renderer.encode(
            encoder,
            &motion
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
            &self.depth_view,
            wgpu::Color::TRANSPARENT,
            &opaque,
        );
        Ok(())
    }

    /// Encode temporal preparation, then run native work after submission and
    /// before presentation. Both callbacks use the same candidate frame inputs.
    /// The submitted callback receives device, queue and presentation target view;
    /// it can queue composition of a processed output before presentation.
    /// GPU writes are queued, not CPU-complete. Native users must preserve queue
    /// ordering, restore wgpu-tracked states and retain inputs until GPU/SDK use ends.
    /// Callbacks run only with enabled inputs and successful surface acquisition.
    /// # Errors
    /// Reports acquisition failure, exhausted presentation IDs or consumer failure.
    /// Consumer failure skips present and invalidates temporal history. Submitted
    /// scene/native work is not cancelled; callers must still retain GPU resources.
    pub fn render_scene_with_temporal_hooks<F, G>(
        &mut self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
        consume: F,
        submitted: G,
    ) -> Result<RenderOutcome, RendererError>
    where
        F: FnOnce(&TemporalFrame<'_>, &mut wgpu::CommandEncoder),
        G: FnOnce(
            &TemporalFrame<'_>,
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::TextureView,
        ) -> Result<(), RendererError>,
    {
        self.render_scene_with_refractive_layers_and_temporal_hooks(
            scene,
            draws,
            &[],
            consume,
            submitted,
        )
    }

    /// Render this frame with a separate refractive layer in presentation and temporal color.
    /// # Errors
    /// Reports the same surface/consumer failures as the ordinary temporal path, plus layer validation errors.
    pub fn render_scene_with_refractive_layers_and_temporal_hooks<F, G>(
        &mut self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
        layers: &[SceneDraw<'_>],
        consume: F,
        submitted: G,
    ) -> Result<RenderOutcome, RendererError>
    where
        F: FnOnce(&TemporalFrame<'_>, &mut wgpu::CommandEncoder),
        G: FnOnce(
            &TemporalFrame<'_>,
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::TextureView,
        ) -> Result<(), RendererError>,
    {
        self.render_scene_with_preparation_and_temporal_hooks(
            scene,
            draws,
            layers,
            |_, _| Ok(()),
            consume,
            submitted,
        )
    }

    /// Acquire a presentation frame before encoding resident geometry updates.
    /// Skipped/hidden surfaces do not run the preparation hook or its GPU work.
    pub fn render_scene_with_preparation_and_temporal_hooks<P, F, G>(
        &mut self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
        layers: &[SceneDraw<'_>],
        prepare: P,
        consume: F,
        submitted: G,
    ) -> Result<RenderOutcome, RendererError>
    where
        P: FnOnce(&wgpu::Queue, &mut wgpu::CommandEncoder) -> Result<(), RendererError>,
        F: FnOnce(&TemporalFrame<'_>, &mut wgpu::CommandEncoder),
        G: FnOnce(
            &TemporalFrame<'_>,
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::TextureView,
        ) -> Result<(), RendererError>,
    {
        self.check_device()?;
        // A presented frame flushes pending uploads with its own submission;
        // skipped frames flush explicitly so write_buffer staging does not accumulate.
        if self.state == SurfaceState::Suspended {
            self.queue.submit([]);
            return Ok(RenderOutcome::Suspended);
        }
        let frame_id = self
            .presented_frames
            .checked_add(1)
            .ok_or(RendererError::FrameIdExhausted)?;
        let (frame, suboptimal) = match self.acquire_surface_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Lost => {
                self.queue.submit([]);
                self.invalidate_temporal_history();
                return Err(RendererError::SurfaceLost);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.queue.submit([]);
                self.surface.configure(&self.device, &self.config);
                self.invalidate_temporal_history();
                return Ok(RenderOutcome::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.queue.submit([]);
                return Ok(RenderOutcome::SkippedTimeout);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.queue.submit([]);
                return Ok(RenderOutcome::SkippedOccluded);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                self.queue.submit([]);
                self.invalidate_temporal_history();
                return Ok(RenderOutcome::SkippedValidation);
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepare(&self.queue, &mut encoder)?;
        self.encode_temporal_inputs(scene, draws, layers, &mut encoder)?;
        self.encode_presentation(scene, draws, layers, &mut encoder, &view)?;

        if let Some(motion) = &self.motion {
            consume(&motion.inputs(frame_id, &self.depth), &mut encoder);
        }
        self.queue.submit([encoder.finish()]);
        if let Some(motion) = &self.motion
            && let Err(error) = submitted(
                &motion.inputs(frame_id, &self.depth),
                &self.device,
                &self.queue,
                &view,
            )
        {
            self.invalidate_temporal_history();
            return Err(error);
        }
        let has_overlays = self.motion.is_some() && draws.iter().any(|draw| draw.overlay);
        let has_probe = self
            .pixel_probe
            .as_ref()
            .is_some_and(|(_, _, id)| id.is_none());
        if has_overlays || has_probe {
            let mut post_encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            if has_overlays {
                scene.encode_overlays(&mut post_encoder, &view, &self.depth_view, draws);
            }
            if has_probe {
                self.encode_pixel_probe(&mut post_encoder, &frame.texture)?;
            }
            self.queue.submit([post_encoder.finish()]);
            if has_probe {
                self.submit_pixel_probe(frame_id);
            }
        }
        self.queue.present(frame);
        self.presented_frames = frame_id;
        if let Some(motion) = &mut self.motion {
            motion.frame_id = Some(frame_id);
            motion.frame_reset = motion.reset_pending;
            motion.reset_pending = false;
        }
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(RenderOutcome::Presented)
    }
}
