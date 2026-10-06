//! General scene presentation without voxel/skinning resource requirements.
use crate::{
    GraphicsOptions, RenderOutcome, RendererError, SceneDraw, SceneRenderer, SurfaceState,
};

/// Explicit presentation encoding. HDR capability does not prove current display headroom.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SurfaceOutput {
    #[default]
    Sdr,
    /// Linear BT.709 extended range, requiring RGBA16Float/scRGB surface support.
    /// Shaders write linear light; do not apply SDR tone mapping or sRGB encoding.
    HdrLinear,
    /// BT.2020/PQ encoded HDR10, requiring RGB10A2/PQ surface support.
    /// Encode linear scene color with `TextureBlit::hdr10` before presentation.
    Hdr10,
}

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
struct MotionTarget {
    renderer: SceneRenderer,
    color_renderer: Option<SceneRenderer>,
    texture: wgpu::Texture,
    color: wgpu::Texture,
    frame_id: Option<u64>,
    reset_pending: bool,
    frame_reset: bool,
}

impl MotionTarget {
    fn inputs<'a>(&'a self, presentation_id: u64, depth: &'a wgpu::Texture) -> TemporalFrame<'a> {
        TemporalFrame {
            presentation_id,
            reset_history: self.reset_pending,
            motion: &self.texture,
            color: &self.color,
            depth,
        }
    }
}

#[derive(Debug)]
struct MsaaTarget {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    xray_depth: Option<wgpu::TextureView>,
}

#[derive(Debug)]
pub struct SceneSurface {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    device_failure: std::sync::Arc<std::sync::OnceLock<String>>,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    xray_depth_view: Option<wgpu::TextureView>,
    msaa: Option<MsaaTarget>,
    optical_background: Option<crate::SceneTexture>,
    temporal_optical_background: Option<crate::SceneTexture>,
    state: SurfaceState,
    motion: Option<MotionTarget>,
    presented_frames: u64,
}
impl SceneSurface {
    /// # Errors
    /// Returns typed surface, adapter, device or size initialization errors.
    pub async fn new<T>(target: T, width: u32, height: u32) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        Self::new_with_options(target, width, height, GraphicsOptions::default()).await
    }

    /// Requests compute limits on capable adapters, WebGL2 limits otherwise.
    /// # Errors
    /// Returns an initialization error if the selected API or surface is unavailable.
    pub async fn new_with_options<T>(
        target: T,
        width: u32,
        height: u32,
        options: GraphicsOptions,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        let instance = options.create_instance();
        Self::new_with_instance(target, width, height, options, instance).await
    }

    /// Uses an instance connected to the platform display, including GLES/Wayland.
    /// # Errors
    /// Returns surface, adapter, device or size initialization errors.
    pub async fn new_with_instance<T>(
        target: T,
        width: u32,
        height: u32,
        options: GraphicsOptions,
        instance: wgpu::Instance,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        Self::new_with_instance_and_output(
            target,
            width,
            height,
            options,
            instance,
            SurfaceOutput::Sdr,
        )
        .await
    }

    /// Initializes an explicit output encoding on a caller-owned display instance.
    /// Existing constructors retain SDR behavior. Final composition pipelines use
    /// `color_format`. HDR10 world color must be rendered into a linear offscreen
    /// target, then encoded with `TextureBlit::hdr10` into this surface.
    /// # Errors
    /// Rejects unsupported HDR format/color-space pairs without an SDR fallback,
    /// and reports normal surface/device initialization errors.
    pub async fn new_with_instance_and_output<T>(
        target: T,
        width: u32,
        height: u32,
        options: GraphicsOptions,
        instance: wgpu::Instance,
        output: SurfaceOutput,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        let surface = instance
            .create_surface(target)
            .map_err(RendererError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                force_fallback_adapter: options.force_fallback_adapter,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .map_err(RendererError::RequestAdapter)?;
        Self::from_surface(
            surface,
            adapter,
            width,
            height,
            wgpu::Features::empty(),
            output,
            wgpu::ExperimentalFeatures::default(),
        )
        .await
    }
    /// Uses a caller-selected adapter and explicit device features, e.g. a
    /// UUID-matched Vulkan adapter for external compute. No adapter fallback.
    /// The adapter must belong to the supplied instance.
    /// # Errors
    /// Reports surface incompatibility, unsupported device features or size errors.
    pub async fn new_with_adapter<T>(
        target: T,
        width: u32,
        height: u32,
        instance: &wgpu::Instance,
        adapter: wgpu::Adapter,
        required_features: wgpu::Features,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        Self::new_with_adapter_and_output(
            target,
            width,
            height,
            instance,
            adapter,
            required_features,
            SurfaceOutput::Sdr,
        )
        .await
    }

    /// Explicit encoding with a caller-selected adapter, including native interop.
    /// The adapter must belong to `instance`; HDR requests never select another adapter.
    /// # Errors
    /// Reports unsupported output encoding, surface, features or size.
    pub async fn new_with_adapter_and_output<T>(
        target: T,
        width: u32,
        height: u32,
        instance: &wgpu::Instance,
        adapter: wgpu::Adapter,
        required_features: wgpu::Features,
        output: SurfaceOutput,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        Self::new_with_adapter_and_output_experimental(
            target,
            width,
            height,
            instance,
            adapter,
            required_features,
            output,
            wgpu::ExperimentalFeatures::default(),
        )
        .await
    }

    /// Create a surface device with explicit experimental API acknowledgement.
    /// The caller supplies wgpu's acknowledgement token when requesting ray query;
    /// ordinary constructors retain the default experimental-feature policy.
    /// # Errors
    /// Reports unsupported surface/output/features or device creation failures.
    #[allow(clippy::too_many_arguments)]
    pub async fn new_with_adapter_and_output_experimental<T>(
        target: T,
        width: u32,
        height: u32,
        instance: &wgpu::Instance,
        adapter: wgpu::Adapter,
        required_features: wgpu::Features,
        output: SurfaceOutput,
        experimental_features: wgpu::ExperimentalFeatures,
    ) -> Result<Self, RendererError>
    where
        T: Into<wgpu::SurfaceTarget<'static>>,
    {
        let surface = instance
            .create_surface(target)
            .map_err(RendererError::CreateSurface)?;
        if !adapter.is_surface_supported(&surface) {
            return Err(RendererError::UnsupportedSurface);
        }
        Self::from_surface(
            surface,
            adapter,
            width,
            height,
            required_features,
            output,
            experimental_features,
        )
        .await
    }
    async fn from_surface(
        surface: wgpu::Surface<'static>,
        adapter: wgpu::Adapter,
        width: u32,
        height: u32,
        required_features: wgpu::Features,
        output: SurfaceOutput,
        experimental_features: wgpu::ExperimentalFeatures,
    ) -> Result<Self, RendererError> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("general scene device"),
                required_features,
                experimental_features,
                required_limits: if required_features
                    .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
                {
                    adapter.limits()
                } else if adapter
                    .get_downlevel_capabilities()
                    .flags
                    .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
                {
                    wgpu::Limits::default()
                } else {
                    wgpu::Limits::downlevel_webgl2_defaults()
                }
                .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(RendererError::RequestDevice)?;
        let device_failure = std::sync::Arc::new(std::sync::OnceLock::new());
        let failure_callback = std::sync::Arc::clone(&device_failure);
        device.set_device_lost_callback(move |reason, message| {
            let _ = failure_callback.set(format!("{reason:?}: {message}"));
        });
        validate_size(&device, width, height)?;
        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or(RendererError::UnsupportedSurface)?;
        configure_output(&mut config, &surface.get_capabilities(&adapter), output)?;
        let format = config.format.add_srgb_suffix();
        if config.format != format {
            config.view_formats = vec![format];
        }
        let depth = create_depth(&device, config.width, config.height);
        let xray_depth_view = None;
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let state = if width == 0 || height == 0 {
            SurfaceState::Suspended
        } else {
            surface.configure(&device, &config);
            SurfaceState::Active
        };
        Ok(Self {
            surface,
            adapter,
            device,
            device_failure,
            queue,
            config,
            depth,
            depth_view,
            xray_depth_view,
            msaa: None,
            optical_background: None,
            temporal_optical_background: None,
            state,
            motion: None,
            presented_frames: 0,
        })
    }
    /// First reported device failure. Recreate this surface and its GPU resources.
    #[must_use]
    pub fn device_failure(&self) -> Option<&str> {
        self.device_failure.get().map(String::as_str)
    }
    fn check_device(&mut self) -> Result<(), RendererError> {
        if let Some(message) = self.device_failure().map(str::to_owned) {
            self.invalidate_temporal_history();
            return Err(RendererError::DeviceLost(message));
        }
        Ok(())
    }
    /// # Errors
    /// Rejects reported device loss and unsupported surface dimensions.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RendererError> {
        self.check_device()?;
        validate_size(&self.device, width, height)?;
        self.invalidate_temporal_history();
        if width == 0 || height == 0 {
            self.state = SurfaceState::Suspended;
            return Ok(());
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.xray_depth_view = None;
        self.optical_background = None;
        self.temporal_optical_background = None;
        if self.msaa.is_some() {
            self.msaa = Some(create_msaa_target(
                &self.device,
                width,
                height,
                self.color_format(),
            ));
        }
        self.depth = create_depth(&self.device, width, height);
        self.depth_view = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        if let Some(motion) = &mut self.motion {
            motion.texture =
                create_motion_texture(&self.device, width, height, motion.texture.format());
            motion.color =
                create_temporal_color(&self.device, width, height, motion.color.format());
        }
        self.state = SurfaceState::Active;
        Ok(())
    }
    #[must_use]
    pub fn create_scene_renderer(&self) -> SceneRenderer {
        if self.msaa.is_some() {
            SceneRenderer::new_msaa4(&self.device, self.color_format())
        } else {
            SceneRenderer::new(&self.device, self.color_format())
        }
    }
    /// Replace scene geometry and invalidate temporal consumers only after success.
    /// The renderer must belong to this surface's device.
    /// # Errors
    /// Returns mesh validation or device mismatch errors without changing geometry
    /// or the published temporal history.
    pub fn replace_scene_mesh(
        &mut self,
        renderer: &SceneRenderer,
        target: &mut crate::SceneGeometry,
        mesh: &crate::SceneMesh,
    ) -> Result<(), crate::SceneError> {
        renderer.replace_mesh(&self.device, target, mesh)?;
        self.invalidate_temporal_history();
        Ok(())
    }

    /// Replace an sRGB material texture on the presentation device.
    /// A successful replacement resets temporal consumers; rejected input preserves
    /// both the active material and published history.
    /// # Errors
    /// Returns texture validation or renderer device mismatch errors.
    pub fn replace_scene_texture(
        &mut self,
        renderer: &SceneRenderer,
        target: &mut crate::SceneTexture,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: crate::TextureSampling,
    ) -> Result<(), crate::SceneError> {
        let replacement = renderer.upload_texture_with_sampling(
            &self.device,
            &self.queue,
            width,
            height,
            rgba,
            sampling,
        )?;
        *target = replacement;
        self.invalidate_temporal_history();
        Ok(())
    }

    /// Replace a material's complete authored or generated mip chain.
    /// Uploads all levels before committing the material and resetting temporal
    /// history. Prepare generated chains on an asset worker for large images.
    /// # Errors
    /// Rejects incomplete levels, unsupported dimensions, invalid sampling or
    /// a foreign renderer without changing the material or presented history.
    pub fn replace_scene_image_mips(
        &mut self,
        renderer: &SceneRenderer,
        target: &mut crate::SceneTexture,
        images: &[crate::ImageAsset],
        sampling: crate::TextureSampling,
    ) -> Result<(), crate::SceneError> {
        let replacement =
            renderer.upload_image_mips(&self.device, &self.queue, images, sampling)?;
        *target = replacement;
        self.invalidate_temporal_history();
        Ok(())
    }

    /// Compile a compute program on the same device as scene presentation.
    /// Encode its jobs through the frame hooks or `render_custom` before consumers.
    /// # Errors
    /// Returns unsupported compute limits or WGSL/ABI validation diagnostics.
    pub async fn create_compute_program(
        &self,
        source: &str,
        entry_point: &str,
    ) -> Result<crate::ComputeProgram, crate::ComputeError> {
        crate::ComputeProgram::with_entry_point(&self.device, source, entry_point).await
    }

    /// Create reusable linear half-float conversion on the presentation device.
    /// Jobs can share the frame encoder with ray lighting and scene composition.
    /// This does not change the surface's HDR mode or apply display encoding.
    /// # Errors
    /// Rejects insufficient GPU limits or unavailable resolve configuration.
    pub fn create_hdr_resolve_pipeline(
        &self,
    ) -> Result<crate::HdrHalfResolvePipeline, crate::RaySceneError> {
        crate::HdrHalfResolvePipeline::new(&self.device)
    }
    /// Enables four-sample coverage for ordinary scene presentation.
    /// Call before creating the scene renderer. Temporal inputs and isolated
    /// X-ray passes retain their single-sample targets.
    /// # Errors
    /// Rejects unsupported color/depth formats before allocating targets.
    pub fn enable_msaa4(&mut self) -> Result<(), crate::SceneShaderError> {
        for format in [self.color_format(), wgpu::TextureFormat::Depth32Float] {
            if !self
                .adapter
                .get_texture_format_features(format)
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
            {
                return Err(crate::SceneShaderError(format!(
                    "four-sample rendering unsupported for {format:?}"
                )));
            }
        }
        if self.msaa.is_none() {
            self.msaa = Some(create_msaa_target(
                &self.device,
                self.config.width,
                self.config.height,
                self.color_format(),
            ));
            self.invalidate_temporal_history();
        }
        Ok(())
    }

    /// Returns to single-sample presentation, retaining renderer pipeline caches.
    /// Quality changes invalidate temporal accumulation on the next frame.
    pub fn disable_msaa4(&mut self) {
        if self.msaa.take().is_some() {
            self.invalidate_temporal_history();
        }
    }

    /// Color format of the presentation view used by scene and composition passes.
    #[must_use]
    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.config.format.add_srgb_suffix()
    }
    /// Configured compositor encoding; this is distinct from live HDR display state.
    #[must_use]
    pub fn color_space(&self) -> wgpu::SurfaceColorSpace {
        self.config.color_space
    }

    /// Allocate ray reconstruction guides on this presentation device.
    /// # Errors
    /// Reports unsupported formats, dimensions and device limits.
    pub fn create_ray_guides(
        &self,
        width: u32,
        height: u32,
    ) -> Result<crate::RayReconstructionGuides, crate::ReconstructionGuideError> {
        crate::RayReconstructionGuides::new(&self.device, &self.adapter, width, height)
    }

    /// Queries current platform HDR state/headroom. Unknown fields remain `None`.
    /// Query again after moving the window or changing display brightness/HDR mode.
    #[must_use]
    pub fn display_hdr_info(&self) -> wgpu::DisplayHdrInfo {
        self.surface.display_hdr_info(&self.adapter)
    }

    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    /// Process available device callbacks without waiting for GPU completion.
    /// # Errors
    /// Returns the backend's polling error.
    pub fn poll_device(&self) -> Result<wgpu::PollStatus, wgpu::PollError> {
        self.device.poll(wgpu::PollType::Poll)
    }
    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    /// Adapter retained by this presentation context; additional devices share its instance.
    #[must_use]
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }
    #[must_use]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }
    #[must_use]
    pub const fn surface_state(&self) -> SurfaceState {
        self.state
    }

    /// Dimensions of the last nonzero surface configuration.
    #[must_use]
    pub const fn configured_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

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

    /// Presents disjoint scene views and full-window overlays in one acquisition.
    /// Geometry/textures may be shared; each view owns its camera transforms.
    /// Uses the custom-frame lifecycle: skipped acquisition encodes nothing and
    /// successful presentation invalidates the single-camera temporal history.
    /// # Errors
    /// Reports invalid regions/attachments/global overlays, missing MSAA
    /// pipelines and surface/device failures. X-ray depth is allocated lazily.
    pub fn render_scene_views(
        &mut self,
        scene: &SceneRenderer,
        views: &[crate::SceneView<'_>],
        overlays: &[SceneDraw<'_>],
    ) -> Result<RenderOutcome, RendererError> {
        let has_xray = views
            .iter()
            .flat_map(|view| view.draws)
            .any(|draw| !draw.overlay && draw.geometry.depth_mode() == crate::SceneDepthMode::Xray);
        let depth = self.depth_view.clone();
        let (color_depth, resolve_msaa, xray) = if let Some(msaa) = &mut self.msaa {
            if has_xray && msaa.xray_depth.is_none() {
                msaa.xray_depth = Some(
                    self.device
                        .create_texture(&wgpu::TextureDescriptor {
                            label: Some("four-sample isolated X-ray depth"),
                            size: wgpu::Extent3d {
                                width: self.config.width,
                                height: self.config.height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 4,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Depth32Float,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &[],
                        })
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                );
            }
            (
                Some((msaa.color.clone(), msaa.depth.clone())),
                true,
                msaa.xray_depth.clone(),
            )
        } else {
            if has_xray && self.xray_depth_view.is_none() {
                self.xray_depth_view = Some(
                    create_depth(&self.device, self.config.width, self.config.height)
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                );
            }
            (None, false, self.xray_depth_view.clone())
        };
        self.render_custom(|encoder, output| {
            let (color, world_depth) = color_depth
                .as_ref()
                .map_or((output, &depth), |(color, depth)| (color, depth));
            scene
                .encode_view_frame_targets(
                    encoder,
                    crate::SceneViewTargets {
                        color,
                        depth: world_depth,
                        xray_depth: xray.as_ref(),
                        resolve: resolve_msaa.then_some(output),
                        overlay_depth: &depth,
                    },
                    wgpu::Color::BLACK,
                    views,
                    overlays,
                )
                .map_err(RendererError::Scene)
        })
    }

    /// Compose disjoint optical fluid views after the ordinary scene and before UI.
    /// # Errors
    /// MSAA, duplicate indices, mismatched view sizes and normal scene/surface failures.
    pub fn render_scene_views_with_fluids(
        &mut self,
        scene: &SceneRenderer,
        views: &[crate::SceneView<'_>],
        fluids: &[(usize, &crate::ScreenSpaceFluidRenderer)],
        overlays: &[SceneDraw<'_>],
    ) -> Result<RenderOutcome, RendererError> {
        if self.msaa.is_some()
            || fluids.iter().enumerate().any(|(i, (index, fluid))| {
                views
                    .get(*index)
                    .is_none_or(|v| v.viewport[2..] != fluid.size())
                    || fluids[..i].iter().any(|(other, _)| other == index)
            })
        {
            return Err(RendererError::Scene(crate::SceneError::InvalidGeometry));
        }
        if overlays.iter().any(|draw| !draw.overlay) {
            return Err(RendererError::Scene(crate::SceneError::InvalidGeometry));
        }
        let has_xray = views
            .iter()
            .flat_map(|view| view.draws)
            .any(|draw| !draw.overlay && draw.geometry.depth_mode() == crate::SceneDepthMode::Xray);
        if has_xray && self.xray_depth_view.is_none() {
            self.xray_depth_view = Some(
                create_depth(&self.device, self.config.width, self.config.height)
                    .create_view(&Default::default()),
            );
        }
        let xray = self.xray_depth_view.clone();
        let depth = self.depth_view.clone();
        let size = [self.config.width, self.config.height];
        self.render_custom(|encoder, output| {
            scene
                .encode_view_frame_targets(
                    encoder,
                    crate::SceneViewTargets {
                        color: output,
                        depth: &depth,
                        xray_depth: xray.as_ref(),
                        resolve: None,
                        overlay_depth: &depth,
                    },
                    wgpu::Color::BLACK,
                    views,
                    &[],
                )
                .map_err(RendererError::Scene)?;
            for (index, fluid) in fluids {
                let view = &views[*index];
                fluid
                    .encode_viewport(
                        scene,
                        encoder,
                        output,
                        &depth,
                        size,
                        view.viewport,
                        wgpu::Color::BLACK,
                        view.draws,
                    )
                    .map_err(|_| RendererError::Scene(crate::SceneError::InvalidGeometry))?;
            }
            scene.encode_overlays(encoder, output, &depth, overlays);
            Ok(())
        })
    }

    /// Presents caller-encoded GPU work on this surface's device and queue.
    /// Acquisition happens before the callback, so skipped frames encode no physics.
    /// Custom frames invalidate scene temporal history; they do not supply motion inputs.
    /// # Errors
    /// Reports surface loss, exhausted frame IDs or a callback error. Failed work
    /// is not submitted or presented.
    pub fn render_custom<F, E>(&mut self, encode: F) -> Result<RenderOutcome, E>
    where
        F: FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView) -> Result<(), E>,
        E: From<RendererError>,
    {
        self.check_device()?;
        // Consume pending uploads even when suspension or acquisition skips
        // presentation; otherwise write_buffer staging allocations accumulate.
        self.queue.submit([]);
        if self.state == SurfaceState::Suspended {
            return Ok(RenderOutcome::Suspended);
        }
        let frame_id = self
            .presented_frames
            .checked_add(1)
            .ok_or_else(|| E::from(RendererError::FrameIdExhausted))?;
        let (frame, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Lost => {
                self.invalidate_temporal_history();
                return Err(RendererError::SurfaceLost.into());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                self.invalidate_temporal_history();
                return Ok(RenderOutcome::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Timeout => return Ok(RenderOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Validation => {
                self.invalidate_temporal_history();
                return Ok(RenderOutcome::SkippedValidation);
            }
        };
        self.invalidate_temporal_history();
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.color_format()),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encode(&mut encoder, &view)?;
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        self.presented_frames = frame_id;
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(RenderOutcome::Presented)
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

    fn present_overlays(
        &self,
        scene: &SceneRenderer,
        draws: &[SceneDraw<'_>],
        view: &wgpu::TextureView,
    ) {
        if self.motion.is_some() && draws.iter().any(|draw| draw.overlay) {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            scene.encode_overlays(&mut encoder, view, &self.depth_view, draws);
            self.queue.submit([encoder.finish()]);
        }
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
        self.check_device()?;
        // Consume pending uploads even when suspension or acquisition skips
        // presentation; otherwise write_buffer staging allocations accumulate.
        self.queue.submit([]);
        if self.state == SurfaceState::Suspended {
            return Ok(RenderOutcome::Suspended);
        }
        let frame_id = self
            .presented_frames
            .checked_add(1)
            .ok_or(RendererError::FrameIdExhausted)?;
        let (frame, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Lost => {
                self.invalidate_temporal_history();
                return Err(RendererError::SurfaceLost);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                self.invalidate_temporal_history();
                return Ok(RenderOutcome::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Timeout => return Ok(RenderOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Validation => {
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
        self.present_overlays(scene, draws, &view);
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
fn validate_size(device: &wgpu::Device, width: u32, height: u32) -> Result<(), RendererError> {
    let max_dimension = device.limits().max_texture_dimension_2d;
    if width > max_dimension || height > max_dimension {
        return Err(RendererError::InvalidSurfaceSize {
            width,
            height,
            max_dimension,
        });
    }
    Ok(())
}
fn create_msaa_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> MsaaTarget {
    let attachment = |format| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("four-sample scene attachment"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    MsaaTarget {
        color: attachment(format),
        depth: attachment(wgpu::TextureFormat::Depth32Float),
        xray_depth: None,
    }
}

fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("general scene depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn create_motion_texture(
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

fn create_temporal_color(
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

fn configure_output(
    config: &mut wgpu::SurfaceConfiguration,
    capabilities: &wgpu::SurfaceCapabilities,
    output: SurfaceOutput,
) -> Result<(), RendererError> {
    let (format, color_space, required) = match output {
        SurfaceOutput::Sdr => return Ok(()),
        SurfaceOutput::HdrLinear => (
            wgpu::TextureFormat::Rgba16Float,
            wgpu::SurfaceColorSpace::ExtendedSrgbLinear,
            wgpu::SurfaceColorSpaces::EXTENDED_SRGB_LINEAR,
        ),
        SurfaceOutput::Hdr10 => (
            wgpu::TextureFormat::Rgb10a2Unorm,
            wgpu::SurfaceColorSpace::Bt2100Pq,
            wgpu::SurfaceColorSpaces::BT2100_PQ,
        ),
    };
    if !capabilities.color_spaces(format).contains(required) {
        return Err(RendererError::UnsupportedSurface);
    }
    config.format = format;
    config.color_space = color_space;
    config.view_formats.clear();
    Ok(())
}

#[cfg(test)]
mod output_tests {
    use super::*;

    #[test]
    fn hdr_requires_matching_format_and_color_space_including_opt_in_formats() {
        let mut config = wgpu::SurfaceConfiguration {
            format: wgpu::TextureFormat::Bgra8Unorm,
            color_space: wgpu::SurfaceColorSpace::Auto,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            width: 4,
            height: 4,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };
        let original = config.clone();
        let mut capabilities = wgpu::SurfaceCapabilities::default();
        assert!(configure_output(&mut config, &capabilities, SurfaceOutput::HdrLinear).is_err());
        assert_eq!(config, original);
        capabilities
            .format_capabilities
            .push(wgpu::SurfaceFormatCapabilities {
                format: wgpu::TextureFormat::Rgba16Float,
                color_spaces: wgpu::SurfaceColorSpaces::SRGB,
            });
        assert!(configure_output(&mut config, &capabilities, SurfaceOutput::HdrLinear).is_err());
        capabilities.format_capabilities[0].color_spaces =
            wgpu::SurfaceColorSpaces::EXTENDED_SRGB_LINEAR;
        // HDR-only formats need not appear in the legacy default formats list.
        assert!(capabilities.formats.is_empty());
        configure_output(&mut config, &capabilities, SurfaceOutput::HdrLinear).unwrap();
        assert_eq!(config.format, wgpu::TextureFormat::Rgba16Float);
        assert_eq!(
            config.color_space,
            wgpu::SurfaceColorSpace::ExtendedSrgbLinear
        );
        assert!(config.view_formats.is_empty());
        assert!(configure_output(&mut config, &capabilities, SurfaceOutput::Hdr10).is_err());
        capabilities
            .format_capabilities
            .push(wgpu::SurfaceFormatCapabilities {
                format: wgpu::TextureFormat::Rgb10a2Unorm,
                color_spaces: wgpu::SurfaceColorSpaces::BT2100_PQ,
            });
        configure_output(&mut config, &capabilities, SurfaceOutput::Hdr10).unwrap();
        assert_eq!(config.format, wgpu::TextureFormat::Rgb10a2Unorm);
        assert_eq!(config.color_space, wgpu::SurfaceColorSpace::Bt2100Pq);
        let mut sdr = original.clone();
        configure_output(&mut sdr, &capabilities, SurfaceOutput::Sdr).unwrap();
        assert_eq!(sdr, original);
    }
}
