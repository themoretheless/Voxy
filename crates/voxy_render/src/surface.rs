//! General scene presentation without voxel/skinning resource requirements.
#[path = "command_buffer.rs"]
pub mod command_buffer;
mod temporal;

pub use temporal::TemporalFrame;
use temporal::{MotionTarget, create_motion_texture, create_temporal_color};

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
    pixel_probe: Option<(crate::HdrPixelProbe, [u32; 2], Option<u64>)>,
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
        // Shared transient shader inputs are enabled only for native Apple
        // Silicon Metal. Unsupported devices keep the portable upload path.
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let required_features = required_features
            | if adapter.get_info().backend == wgpu::Backend::Metal {
                adapter.features() & wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
            } else {
                wgpu::Features::empty()
            };
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
            pixel_probe: None,
        })
    }
    /// Request one asynchronous pixel from the next successfully submitted frame.
    /// Opt-in diagnostics only; coordinates address physical surface pixels.
    /// Resize cancels pending probes. sRGB results are normalized storage values.
    /// # Errors
    /// Rejects unsupported copy usage, coordinates, formats or another pending probe.
    pub fn request_pixel_probe(&mut self, point: [u32; 2]) -> Result<(), RendererError> {
        if self.pixel_probe.is_some()
            || point[0] >= self.config.width
            || point[1] >= self.config.height
            || !self
                .surface
                .get_capabilities(&self.adapter)
                .usages
                .contains(wgpu::TextureUsages::COPY_SRC)
            || !matches!(
                self.config.format,
                wgpu::TextureFormat::Rgba8Unorm
                    | wgpu::TextureFormat::Rgba8UnormSrgb
                    | wgpu::TextureFormat::Bgra8Unorm
                    | wgpu::TextureFormat::Bgra8UnormSrgb
                    | wgpu::TextureFormat::Rgba16Float
                    | wgpu::TextureFormat::Rgba32Float
            )
        {
            return Err(RendererError::Scene(crate::SceneError::InvalidTexture));
        }
        self.check_device()?;
        if !self.config.usage.contains(wgpu::TextureUsages::COPY_SRC) {
            self.config.usage |= wgpu::TextureUsages::COPY_SRC;
            if self.state != SurfaceState::Suspended {
                self.surface.configure(&self.device, &self.config);
            }
        }
        self.pixel_probe = Some((crate::HdrPixelProbe::new(&self.device), point, None));
        Ok(())
    }
    #[must_use]
    pub fn pixel_probe_pending(&self) -> bool {
        self.pixel_probe.is_some()
    }
    /// Poll without waiting; returns the source presentation ID and mapping result once.
    pub fn take_pixel_probe(&mut self) -> Option<(u64, Result<[f32; 4], String>)> {
        let _ = self.device.poll(wgpu::PollType::Poll);
        let (probe, _, frame) = self.pixel_probe.as_ref()?;
        let frame = (*frame)?;
        let result = probe.take_result()?;
        self.pixel_probe = None;
        Some((frame, result))
    }
    fn encode_pixel_probe(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) -> Result<(), RendererError> {
        if let Some((probe, point, None)) = &mut self.pixel_probe {
            probe
                .encode(encoder, texture, point[0], point[1])
                .map_err(RendererError::Scene)?;
        }
        Ok(())
    }
    fn submit_pixel_probe(&mut self, frame_id: u64) {
        if let Some((probe, _, frame @ None)) = &mut self.pixel_probe {
            probe.begin_read();
            *frame = Some(frame_id);
        }
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
    /// Explicit benchmark/low-latency presentation. Returns the selected backend
    /// mode; an unavailable non-vsync mode is reported instead of claiming it.
    pub fn enable_unthrottled_presentation(&mut self) -> Result<wgpu::PresentMode, RendererError> {
        let capabilities = self.surface.get_capabilities(&self.adapter);
        let mode = [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox]
            .into_iter()
            .find(|mode| capabilities.present_modes.contains(mode))
            .ok_or(RendererError::UnsupportedSurface)?;
        self.config.present_mode = mode;
        if self.state == SurfaceState::Active {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(mode)
    }
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RendererError> {
        self.check_device()?;
        validate_size(&self.device, width, height)?;
        self.pixel_probe = None;
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
        self.render_custom(|encoder, output| {
            scene
                .encode_view_frame_with_fluids(
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
                    fluids,
                    overlays,
                )
                .map_err(RendererError::Scene)
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
        // A presented frame flushes pending uploads with its own submission;
        // skipped frames flush explicitly so write_buffer staging does not accumulate.
        if self.state == SurfaceState::Suspended {
            self.queue.submit([]);
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
                self.queue.submit([]);
                self.invalidate_temporal_history();
                return Err(RendererError::SurfaceLost.into());
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
        self.invalidate_temporal_history();
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.color_format()),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encode(&mut encoder, &view)?;
        self.encode_pixel_probe(&mut encoder, &frame.texture)
            .map_err(E::from)?;
        self.queue.submit([encoder.finish()]);
        self.submit_pixel_probe(frame_id);
        self.queue.present(frame);
        self.presented_frames = frame_id;
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(RenderOutcome::Presented)
    }

    /// Partitions a slice of [`SceneDraw`] items into balanced contiguous slices for parallel
    /// command buffer recording (Optimization #76-80).
    #[must_use]
    pub fn partition_draw_slices<'a>(
        draws: &'a [SceneDraw<'a>],
        max_partitions: usize,
    ) -> Vec<&'a [SceneDraw<'a>]> {
        command_buffer::partition_work_slices(draws, max_partitions)
    }

    /// Records multiple independent GPU passes on thread-local command encoders and submits
    /// them in deterministic order via a single `queue.submit` call (Optimization #76-80).
    ///
    /// # Errors
    /// Propagates the first pass recording error before submitting any work to the queue.
    pub fn record_parallel_passes<F, E>(&self, passes: Vec<(&str, F)>) -> Result<usize, E>
    where
        F: FnOnce(&mut wgpu::CommandEncoder) -> Result<(), E> + Send,
        E: Send,
    {
        let recorder = command_buffer::ParallelCommandRecorder::new();
        self.record_parallel_passes_with_threads(recorder.max_threads(), passes)
    }

    /// Records multiple independent GPU passes capped at `max_threads` worker threads and
    /// submits them in deterministic order via a single `queue.submit` call.
    ///
    /// # Errors
    /// Propagates the first pass recording error before submitting any work to the queue.
    pub fn record_parallel_passes_with_threads<F, E>(
        &self,
        max_threads: usize,
        passes: Vec<(&str, F)>,
    ) -> Result<usize, E>
    where
        F: FnOnce(&mut wgpu::CommandEncoder) -> Result<(), E> + Send,
        E: Send,
    {
        command_buffer::ParallelCommandRecorder::with_max_threads(max_threads).record_and_submit(
            &self.device,
            &self.queue,
            passes,
        )
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
