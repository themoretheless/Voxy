//! Optional Windows Streamline runtime ownership. Renderer integration is pending.
pub mod neural_rendering;
#[cfg(feature = "render-policy")]
pub mod render_policy;
use std::{marker::PhantomData, path::Path, rc::Rc};
#[cfg(all(windows, feature = "wgpu-dx12"))]
pub mod dx12;
#[cfg(all(windows, feature = "wgpu-dx12"))]
pub mod rr_dx12;
#[cfg(all(windows, feature = "scene-dx12"))]
pub mod scene_dx12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamlineError {
    Unsupported,
    InvalidPath,
    InvalidOptions,
    WrongBackend,
    Dxgi {
        hresult: i32,
    },
    Native {
        domain: u32,
        code: u32,
        windows_error: u32,
    },
}

#[derive(Clone, Copy, Debug, Default)]
// Independent SDK plugin selections, rather than lifecycle state flags.
#[allow(clippy::struct_excessive_bools)]
pub struct StreamlineFeatures {
    pub super_resolution: bool,
    pub frame_generation: bool,
    pub reflex: bool,
    pub ray_reconstruction: bool,
    /// Request the SDK's NR plugin; inference/evaluation integration is pending.
    pub neural_rendering: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StreamlineFeature {
    SuperResolution = 1,
    FrameGeneration = 2,
    Reflex = 4,
    RayReconstruction = 8,
    NeuralRendering = 16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DlssQuality {
    Performance = 1,
    Balanced = 2,
    Quality = 3,
    UltraPerformance = 4,
    UltraQuality = 5,
    Dlaa = 6,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct DlssRenderSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum ReflexMode {
    #[default]
    Off = 0,
    LowLatency = 1,
    Boost = 2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FrameGeneration {
    #[default]
    Off,
    Fixed {
        generated_frames: u32,
    },
    Dynamic {
        target_fps: Option<f32>,
    },
}

impl FrameGeneration {
    /// Validate this policy against a previously queried SDK capability snapshot.
    /// Counts exclude the rendered frame. SDK configuration remains authoritative;
    /// a snapshot can become stale after adapter/device or runtime changes.
    /// # Errors
    /// Rejects zero/excessive fixed counts, unavailable dynamic MFG and invalid
    /// explicit dynamic targets. `Off` is always valid, allowing recovery.
    pub fn validate(self, capabilities: &FrameGenerationState) -> Result<(), StreamlineError> {
        match self {
            Self::Off => Ok(()),
            Self::Fixed { generated_frames }
                if generated_frames > 0
                    && generated_frames <= capabilities.maximum_generated_frames =>
            {
                Ok(())
            }
            Self::Dynamic { target_fps }
                if capabilities.maximum_generated_frames > 0
                    && capabilities.dynamic_supported == 1
                    && target_fps.is_none_or(|fps| fps.is_finite() && fps > 0.0) =>
            {
                Ok(())
            }
            _ => Err(StreamlineError::InvalidOptions),
        }
    }
}

/// Thread-bound runtime. Initialize before creating graphics devices.
#[derive(Debug)]
pub struct StreamlineRuntime {
    #[cfg(all(windows, feature = "native"))]
    handle: std::ptr::NonNull<std::ffi::c_void>,
    #[cfg(all(windows, feature = "wgpu-dx12"))]
    device_owner: Option<windows::Win32::Graphics::Direct3D12::ID3D12Device>,
    thread_bound: PhantomData<Rc<()>>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReflexMarker {
    SimulationStart = 0,
    SimulationEnd = 1,
    SubmitStart = 2,
    SubmitEnd = 3,
    PresentStart = 4,
    PresentEnd = 5,
}
#[derive(Debug)]
pub struct StreamlineFrame<'a> {
    #[cfg(all(windows, feature = "native"))]
    handle: std::ptr::NonNull<std::ffi::c_void>,
    runtime: PhantomData<&'a mut StreamlineRuntime>,
}
/// SDK row-major, unjittered camera matrices for perspective normal-depth inputs.
/// Backward 2D motion must include camera motion. Scales/jitter must match textures.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct CameraConstants {
    pub projection: [f32; 16],
    pub inverse_projection: [f32; 16],
    pub clip_to_previous: [f32; 16],
    pub previous_to_clip: [f32; 16],
    pub position: [f32; 3],
    pub up: [f32; 3],
    pub right: [f32; 3],
    pub forward: [f32; 3],
    pub jitter: [f32; 2],
    pub motion_scale: [f32; 2],
    pub near_plane: f32,
    pub far_plane: f32,
    pub fov: f32,
    pub aspect: f32,
    pub reset: u32,
}
/// Perspective camera inputs in glam's column-vector convention.
/// View must be a rigid, right-handed camera transform.
#[derive(Clone, Copy, Debug)]
pub struct PerspectiveFrame {
    pub projection: glam::Mat4,
    pub view: glam::Mat4,
    pub previous_view_projection: Option<glam::Mat4>,
    pub near_plane: f32,
    pub far_plane: f32,
    pub fov: f32,
    pub aspect: f32,
    pub jitter_pixels: [f32; 2],
    pub motion_scale: [f32; 2],
    pub reset: bool,
}
fn camera_basis(
    inverse_view: glam::Mat4,
) -> Result<(glam::Vec3, glam::Vec3, glam::Vec3), StreamlineError> {
    let up = inverse_view
        .y_axis
        .truncate()
        .try_normalize()
        .ok_or(StreamlineError::InvalidOptions)?;
    let right = inverse_view
        .x_axis
        .truncate()
        .try_normalize()
        .ok_or(StreamlineError::InvalidOptions)?;
    let forward = (-inverse_view.z_axis.truncate())
        .try_normalize()
        .ok_or(StreamlineError::InvalidOptions)?;
    let basis = [
        inverse_view.x_axis.truncate(),
        inverse_view.y_axis.truncate(),
        inverse_view.z_axis.truncate(),
    ];
    if basis
        .iter()
        .any(|axis| (axis.length_squared() - 1.0).abs() > 0.0001)
        || right.dot(up).abs() > 0.0001
        || right.dot(forward).abs() > 0.0001
        || up.dot(forward).abs() > 0.0001
        || right.cross(up).dot(-forward) < 0.9999
    {
        return Err(StreamlineError::InvalidOptions);
    }
    Ok((up, right, forward))
}
impl CameraConstants {
    /// Convert unjittered glam camera transforms to SDK row-vector matrices.
    /// # Errors
    /// Rejects nonfinite/singular transforms and invalid perspective parameters.
    // Affine view matrices require an exact homogeneous bottom row.
    #[allow(clippy::float_cmp)]
    pub fn from_perspective(frame: &PerspectiveFrame) -> Result<Self, StreamlineError> {
        if frame.view.x_axis.w != 0.0
            || frame.view.y_axis.w != 0.0
            || frame.view.z_axis.w != 0.0
            || frame.view.w_axis.w != 1.0
        {
            return Err(StreamlineError::InvalidOptions);
        }
        if frame
            .previous_view_projection
            .is_some_and(|matrix| !matrix.is_finite())
        {
            return Err(StreamlineError::InvalidOptions);
        }
        let projection = frame.projection;
        let view = frame.view;
        let current = projection * view;
        let reset = frame.reset || frame.previous_view_projection.is_none();
        let previous = if reset {
            current
        } else {
            frame.previous_view_projection.unwrap_or(current)
        };
        let inverse_projection = projection.inverse();
        let inverse_view = view.inverse();
        let inverse_current = current.inverse();
        let inverse_previous = previous.inverse();
        if [
            projection,
            view,
            current,
            previous,
            inverse_projection,
            inverse_view,
            inverse_current,
            inverse_previous,
        ]
        .iter()
        .any(|matrix| !matrix.is_finite())
            || !frame.near_plane.is_finite()
            || !frame.far_plane.is_finite()
            || frame.near_plane <= 0.0
            || frame.far_plane <= frame.near_plane
            || !frame.fov.is_finite()
            || frame.fov <= 0.0
            || frame.fov >= std::f32::consts::PI
            || !frame.aspect.is_finite()
            || frame.aspect <= 0.0
            || frame
                .jitter_pixels
                .iter()
                .chain(frame.motion_scale.iter())
                .any(|value| !value.is_finite())
        {
            return Err(StreamlineError::InvalidOptions);
        }
        // SDK uses row vectors: transpose the mathematical column-vector matrix.
        // Its row-major storage is therefore glam's original column-major array.
        let to_previous = if reset {
            glam::Mat4::IDENTITY
        } else {
            previous * inverse_current
        };
        let from_previous = if reset {
            glam::Mat4::IDENTITY
        } else {
            current * inverse_previous
        };
        if !to_previous.is_finite() || !from_previous.is_finite() {
            return Err(StreamlineError::InvalidOptions);
        }
        let (up, right, forward) = camera_basis(inverse_view)?;
        Ok(Self {
            projection: projection.to_cols_array(),
            inverse_projection: inverse_projection.to_cols_array(),
            clip_to_previous: to_previous.to_cols_array(),
            previous_to_clip: from_previous.to_cols_array(),
            position: inverse_view.w_axis.truncate().to_array(),
            up: up.to_array(),
            right: right.to_array(),
            forward: forward.to_array(),
            jitter: frame.jitter_pixels,
            motion_scale: frame.motion_scale,
            near_plane: frame.near_plane,
            far_plane: frame.far_plane,
            fov: frame.fov,
            aspect: frame.aspect,
            reset: u32::from(reset),
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RayReconstructionOptions {
    pub quality: DlssQuality,
    pub output: DlssRenderSize,
    /// 0 for LDR, 1 for HDR.
    pub hdr: u32,
    /// 0 for separate normal/roughness textures, 1 for packed roughness.
    pub packed_normal_roughness: u32,
    /// SDK row-major row-vector matrices; use glam's column arrays.
    pub world_to_view: [f32; 16],
    pub view_to_world: [f32; 16],
}
impl RayReconstructionOptions {
    /// Convert the engine's rigid right-handed view matrix to SDK convention.
    /// # Errors
    /// Rejects SDR, empty output, nonfinite, singular, scaled, mirrored or projective views.
    #[allow(clippy::float_cmp)] // Exact affine bottom row is required by this camera contract.
    pub fn from_view(
        quality: DlssQuality,
        output: DlssRenderSize,
        hdr: bool,
        packed_normal_roughness: bool,
        view: glam::Mat4,
    ) -> Result<Self, StreamlineError> {
        if !hdr
            || output.width == 0
            || output.height == 0
            || output.width == u32::MAX
            || output.height == u32::MAX
            || !view.is_finite()
            || view.x_axis.w != 0.0
            || view.y_axis.w != 0.0
            || view.z_axis.w != 0.0
            || view.w_axis.w != 1.0
        {
            return Err(StreamlineError::InvalidOptions);
        }
        let inverse = view.inverse();
        if !inverse.is_finite() {
            return Err(StreamlineError::InvalidOptions);
        }
        camera_basis(inverse)?;
        Ok(Self {
            quality,
            output,
            hdr: u32::from(hdr),
            packed_normal_roughness: u32::from(packed_normal_roughness),
            world_to_view: view.to_cols_array(),
            view_to_world: inverse.to_cols_array(),
        })
    }
}

/// Official Streamline buffer roles exposed by the DX12 bridge.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureRole {
    Depth = 0,
    MotionVectors = 1,
    HudLessColor = 2,
    ScalingInputColor = 3,
    ScalingOutputColor = 4,
    Normals = 5,
    Roughness = 6,
    Albedo = 7,
    SpecularAlbedo = 8,
    NormalRoughness = 14,
    SpecularHitDistance = 42,
    DiffuseHitDistance = 45,
}

/// Native DX12 texture description for Streamline's frame resource tags.
/// Lifecycle selectors: valid now=0, through present=1, through evaluation=2.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Dx12TextureTag {
    pub resource: std::ptr::NonNull<std::ffi::c_void>,
    pub state: u32,
    pub resource_type: TextureRole,
    pub lifecycle: u32,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

/// Retains a wgpu texture and its DX12 COM resource for native SDK access.
/// Keep this lease through the declared SDK lifecycle and GPU completion.
#[cfg(all(windows, feature = "wgpu-dx12"))]
#[derive(Debug)]
pub struct Dx12TextureLease {
    texture: wgpu::Texture,
    resource: windows::Win32::Graphics::Direct3D12::ID3D12Resource,
}
#[cfg(all(windows, feature = "wgpu-dx12"))]
impl Dx12TextureLease {
    /// Retain imported resources until all previously submitted queue work completes.
    /// Register this after submitting every command buffer that uses these resources.
    /// The application must continue polling the device to deliver queue callbacks.
    /// # Safety
    /// SDK use (including generated-frame presentation) must have ended before
    /// registering this callback. All GPU accesses must be submitted on this queue;
    /// later or external-queue work needs its own lifetime protection. Explicit
    /// texture/device destruction remains forbidden while GPU work is pending.
    #[allow(unsafe_code)]
    pub unsafe fn retain_until_queue_done(queue: &wgpu::Queue, resources: Vec<Self>) {
        queue.on_submitted_work_done(move || drop(resources));
    }
    /// # Safety
    /// Uphold wgpu-hal requirements. Do not explicitly destroy this texture or
    /// its device while the lease or related SDK/GPU work is live. Caller must
    /// synchronize native SDK access with wgpu's resource-state tracking.
    /// # Errors
    /// Rejects non-DX12 textures, multisampling, arrays and non-2D textures.
    #[allow(unsafe_code)]
    pub unsafe fn from_wgpu(texture: &wgpu::Texture) -> Result<Self, StreamlineError> {
        if texture.dimension() != wgpu::TextureDimension::D2
            || texture.sample_count() != 1
            || texture.depth_or_array_layers() != 1
        {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Caller guarantees HAL ownership/synchronization requirements.
        let hal = unsafe { texture.as_hal::<wgpu::hal::api::Dx12>() }
            .ok_or(StreamlineError::WrongBackend)?;
        // SAFETY: Borrowed guard protects the resource while cloning its COM owner.
        let resource = unsafe { hal.raw_resource() }.clone();
        Ok(Self {
            texture: texture.clone(),
            resource,
        })
    }

    /// Describe the full texture for a native frame tag. No barrier is inserted.
    /// Actual SDK usage must match `state` and the specified lifecycle.
    /// # Errors
    /// Rejects unspecified state, invalid lifecycle or a missing native pointer.
    pub fn tag(
        &self,
        role: TextureRole,
        state: u32,
        lifecycle: u32,
    ) -> Result<Dx12TextureTag, StreamlineError> {
        use windows::core::Interface;
        if state == u32::MAX || lifecycle > 2 {
            return Err(StreamlineError::InvalidOptions);
        }
        Ok(Dx12TextureTag {
            resource: std::ptr::NonNull::new(self.resource.as_raw())
                .ok_or(StreamlineError::InvalidOptions)?,
            state,
            resource_type: role,
            lifecycle,
            left: 0,
            top: 0,
            width: self.texture.width(),
            height: self.texture.height(),
        })
    }
}

impl StreamlineFrame<'_> {
    /// Tag native textures using this frame's SDK token.
    /// # Safety
    /// Resources must be live `ID3D12Resource` textures on the registered device.
    /// Extents must fit each texture. States must match actual SDK usage; caller
    /// provides barriers and synchronization and keeps resources valid for their
    /// declared lifecycle and all GPU work. A supplied command list must be live
    /// and recording on the same device. Volatile tags require a command list.
    /// # Errors
    /// Rejects unsupported builds, malformed tags and SDK failures.
    #[allow(unsafe_code)]
    pub unsafe fn tag_dx12(
        &mut self,
        viewport: u32,
        tags: &[Dx12TextureTag],
        command_list: Option<std::ptr::NonNull<std::ffi::c_void>>,
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::frame_tags(self.handle, viewport, tags, command_list)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (viewport, tags, command_list);
            Err(StreamlineError::Unsupported)
        }
    }
    /// Record SR/DLAA or Ray Reconstruction on a native DX12 command list.
    /// # Safety
    /// `command_list` must point to a live, recording `ID3D12GraphicsCommandList`
    /// on the registered device. Required SDK resource tags and camera constants
    /// must match this frame and viewport. Caller must provide correct resource
    /// states, barriers and synchronization, and retain all GPU resources until
    /// SDK and submitted GPU work finish. This does not submit or present work.
    /// # Errors
    /// Rejects FG/Reflex selectors, unsupported builds and SDK evaluation errors.
    #[allow(unsafe_code)]
    pub unsafe fn evaluate_dx12(
        &mut self,
        viewport: u32,
        feature: StreamlineFeature,
        command_list: std::ptr::NonNull<std::ffi::c_void>,
    ) -> Result<(), StreamlineError> {
        if !matches!(
            feature,
            StreamlineFeature::SuperResolution | StreamlineFeature::RayReconstruction
        ) {
            return Err(StreamlineError::InvalidOptions);
        }
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::frame_evaluate(self.handle, viewport, feature as u32, command_list)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (viewport, command_list);
            Err(StreamlineError::Unsupported)
        }
    }
    /// Submit camera constants using this frame's SDK token.
    /// # Errors
    /// Rejects nonfinite/invalid perspective parameters or SDK errors.
    pub fn set_camera_constants(
        &mut self,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::frame_constants(self.handle, viewport, constants)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (viewport, constants);
            Err(StreamlineError::Unsupported)
        }
    }

    /// # Errors
    /// Returns SDK sleep errors or invalid marker order.
    pub fn sleep(&mut self) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::frame_sleep(self.handle)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            Err(StreamlineError::Unsupported)
        }
    }
    /// # Errors
    /// Returns SDK marker errors or invalid sequence. Surround actual engine work.
    pub fn mark(&mut self, marker: ReflexMarker) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::frame_marker(self.handle, marker as u32)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = marker;
            Err(StreamlineError::Unsupported)
        }
    }
}
#[cfg(all(windows, feature = "native"))]
impl Drop for StreamlineFrame<'_> {
    fn drop(&mut self) {
        ffi::frame_destroy(self.handle);
    }
}

impl StreamlineRuntime {
    /// Obtain the underlying native interface for third-party SDK interoperability.
    /// No COM reference is added by this wrapper.
    /// # Safety
    /// `proxy` must be a live supported COM proxy from this initialized SDK session.
    /// Follow SDK ownership rules for the result and retain proxy and runtime while
    /// borrowing it. Do not adopt it as an owned COM reference without an `AddRef`.
    /// # Errors
    /// Preserves SDK errors or unsupported-platform status.
    #[allow(unsafe_code)]
    pub unsafe fn native_interface(
        &mut self,
        proxy: *mut std::ffi::c_void,
    ) -> Result<*mut std::ffi::c_void, StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::native_interface(self.handle, proxy)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = proxy;
            Err(StreamlineError::Unsupported)
        }
    }
    /// Upgrade a newly created native D3D/DXGI interface for manual hooking.
    /// # Safety
    /// Runtime must be initialized; invoke immediately after interface creation.
    /// The slot must contain a live supported COM interface with ownership/refcounts
    /// managed according to Streamline's replacement contract. SDK may modify the
    /// slot even on failure. Keep SDK loaded while proxies are live; do not apply
    /// this to handles already owned/tracked by wgpu without a supported handoff.
    /// # Errors
    /// Preserves SDK errors or unsupported-platform status.
    #[allow(unsafe_code)]
    pub unsafe fn upgrade_interface(
        &mut self,
        base_interface: &mut *mut std::ffi::c_void,
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::upgrade_interface(self.handle, base_interface)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = base_interface;
            Err(StreamlineError::Unsupported)
        }
    }
    /// Acquire one SDK token. The returned frame prevents runtime shutdown/reconfiguration.
    /// # Errors
    /// Returns SDK acquisition/resolution errors or unsupported-platform status.
    pub fn begin_frame(
        &mut self,
        index: Option<u32>,
    ) -> Result<StreamlineFrame<'_>, StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            Ok(StreamlineFrame {
                handle: ffi::begin_frame(self.handle, index)?,
                runtime: PhantomData,
            })
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = index;
            Err(StreamlineError::Unsupported)
        }
    }

    /// Query SDK capabilities, runtime errors, presentation counters and input fence.
    /// Call on the present thread. This consumes the SDK presentation counter.
    /// # Errors
    /// Returns SDK query errors or unsupported-platform status.
    pub fn frame_generation_state(
        &mut self,
        viewport: u32,
    ) -> Result<FrameGenerationState, StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::fg_state(self.handle, viewport)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = viewport;
            Err(StreamlineError::Unsupported)
        }
    }
    /// Set SDK FG policy. Requires SDK-accepted Reflex options before enabling.
    /// Generated frame counts exclude the rendered frame. This does not install
    /// presentation hooks or generate frames by itself.
    ///
    /// # Errors
    /// Returns SDK capability/Reflex/option errors or unsupported-platform status.
    pub fn configure_frame_generation(
        &mut self,
        viewport: u32,
        mode: FrameGeneration,
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            let (mode, count, target) = match mode {
                FrameGeneration::Off => (0, 0, 0.0),
                FrameGeneration::Fixed { generated_frames } => (1, generated_frames, 0.0),
                FrameGeneration::Dynamic { target_fps } => {
                    if target_fps.is_some_and(|fps| !fps.is_finite() || fps <= 0.0) {
                        return Err(StreamlineError::InvalidOptions);
                    }
                    (2, 0, target_fps.unwrap_or(0.0))
                }
            };
            ffi::configure_fg(self.handle, viewport, mode, count, target)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (viewport, mode);
            Err(StreamlineError::Unsupported)
        }
    }

    /// Set Reflex options; successful activation still requires per-frame sleep/markers.
    /// Frame limiting uses microseconds; zero disables the limiter.
    ///
    /// # Errors
    /// Returns SDK availability/option errors or unsupported-platform status.
    pub fn configure_reflex(
        &mut self,
        mode: ReflexMode,
        frame_limit_us: u32,
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::configure_reflex(self.handle, mode as u32, frame_limit_us)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (mode, frame_limit_us);
            Err(StreamlineError::Unsupported)
        }
    }

    /// Configure SR/DLAA and return the SDK-recommended input size.
    /// This does not evaluate the feature or resize renderer resources.
    ///
    /// # Errors
    /// Returns SDK/option errors or unsupported-platform status.
    pub fn configure_dlss(
        &mut self,
        viewport: u32,
        quality: DlssQuality,
        output: DlssRenderSize,
        hdr: bool,
    ) -> Result<DlssRenderSize, StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::configure_dlss(self.handle, viewport, quality as u32, output, hdr)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (viewport, quality, output, hdr);
            Err(StreamlineError::Unsupported)
        }
    }

    /// Configure Ray Reconstruction and obtain its recommended rendering size.
    /// Matrices must describe the same camera used by frame constants and buffers.
    /// # Errors
    /// Returns SDK errors, invalid parameters, or unsupported-platform status.
    pub fn configure_ray_reconstruction(
        &mut self,
        viewport: u32,
        options: &RayReconstructionOptions,
    ) -> Result<DlssRenderSize, StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::configure_rr(self.handle, viewport, options)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (viewport, options);
            Err(StreamlineError::Unsupported)
        }
    }

    /// Check support on the exact adapter selected by wgpu, using its DXGI LUID.
    ///
    /// # Errors
    /// Rejects other backends, DXGI descriptor errors and SDK support failures.
    #[cfg(all(windows, feature = "wgpu-dx12"))]
    #[allow(unsafe_code)]
    pub fn wgpu_dx12_support(
        &self,
        feature: StreamlineFeature,
        adapter: &wgpu::Adapter,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Read-only HAL access; the guard keeps the adapter live.
        let hal = unsafe { adapter.as_hal::<wgpu::hal::api::Dx12>() }
            .ok_or(StreamlineError::WrongBackend)?;
        // SAFETY: Borrowed live DXGI interface, descriptor query does not mutate GPU state.
        let descriptor =
            unsafe { hal.raw_adapter().GetDesc1() }.map_err(|error| StreamlineError::Dxgi {
                hresult: error.code().0,
            })?;
        let mut luid = [0u8; 8];
        luid[..4].copy_from_slice(&descriptor.AdapterLuid.LowPart.to_le_bytes());
        luid[4..].copy_from_slice(&descriptor.AdapterLuid.HighPart.to_le_bytes());
        self.dx12_support(feature, &luid)
    }

    /// Check feature support for a native DXGI adapter using its actual LUID.
    /// # Errors
    /// Returns DXGI descriptor errors or precise SDK support failures.
    #[cfg(all(windows, feature = "wgpu-dx12"))]
    #[allow(unsafe_code)]
    pub fn dxgi_support(
        &self,
        feature: StreamlineFeature,
        adapter: &windows::Win32::Graphics::Dxgi::IDXGIAdapter1,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Borrowed live COM adapter; descriptor query is read-only.
        let descriptor = unsafe { adapter.GetDesc1() }.map_err(|error| StreamlineError::Dxgi {
            hresult: error.code().0,
        })?;
        let mut luid = [0u8; 8];
        luid[..4].copy_from_slice(&descriptor.AdapterLuid.LowPart.to_le_bytes());
        luid[4..].copy_from_slice(&descriptor.AdapterLuid.HighPart.to_le_bytes());
        self.dx12_support(feature, &luid)
    }

    /// Check SDK support for the selected adapter's DXGI LUID.
    ///
    /// # Errors
    /// Preserves unsupported-adapter/driver/OS reasons from the SDK.
    pub fn dx12_support(
        &self,
        feature: StreamlineFeature,
        luid: &[u8; 8],
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::support(self.handle, feature as u32, luid)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (feature, luid);
            Err(StreamlineError::Unsupported)
        }
    }

    /// Register and retain the native COM device underlying a wgpu DX12 device.
    ///
    /// # Safety
    /// Initialize SDK before creating this device. Uphold Streamline/wgpu native
    /// interop requirements and stop SDK/GPU work before shutdown. Do not explicitly
    /// destroy the wgpu device while Streamline is using it.
    ///
    /// # Errors
    /// Rejects other backends and SDK registration failures.
    #[cfg(all(windows, feature = "wgpu-dx12"))]
    #[allow(unsafe_code)]
    pub unsafe fn register_wgpu_dx12(
        &mut self,
        device: &wgpu::Device,
    ) -> Result<(), StreamlineError> {
        use windows::core::Interface;
        // SAFETY: Guard borrows the HAL device; only its COM handle is cloned.
        let hal = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }
            .ok_or(StreamlineError::WrongBackend)?;
        let owner = hal.raw_device().clone();
        ffi::register_device(self.handle, owner.as_raw())?;
        self.device_owner = Some(owner);
        Ok(())
    }

    /// # Errors
    /// Returns an SDK initialization error, or unsupported on other platforms.
    pub fn initialize_dx12(&mut self, features: StreamlineFeatures) -> Result<(), StreamlineError> {
        self.initialize_dx12_with_interposition(features, Dx12Interposition::Automatic)
    }

    /// Initialize before creating graphics interfaces. Manual modes require the
    /// host to upgrade newly created interfaces through `upgrade_interface`.
    ///
    /// # Errors
    /// Returns SDK initialization errors or unsupported on other platforms.
    pub fn initialize_dx12_with_interposition(
        &mut self,
        features: StreamlineFeatures,
        interposition: Dx12Interposition,
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            let flags = u32::from(features.super_resolution)
                | (u32::from(features.frame_generation) << 1)
                | (u32::from(features.reflex) << 2)
                | (u32::from(features.ray_reconstruction) << 3)
                | (u32::from(features.neural_rendering) << 4);
            ffi::initialize(self.handle, flags, interposition as u32)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = (features, interposition);
            Err(StreamlineError::Unsupported)
        }
    }
    /// Register the renderer's native DX12 device after SDK initialization.
    ///
    /// # Safety
    /// `device` must point to a live `ID3D12Device` on this runtime's thread. Retain
    /// its COM owner until SDK shutdown succeeds; on failed shutdown retain it
    /// until cleanup succeeds or the process exits. This function borrows it.
    ///
    /// # Errors
    /// Returns invalid-state/SDK registration errors or unsupported.
    #[allow(unsafe_code)]
    pub unsafe fn register_dx12_device(
        &mut self,
        device: *mut std::ffi::c_void,
    ) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::register_device(self.handle, device)
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = device;
            Err(StreamlineError::Unsupported)
        }
    }
    /// # Errors
    /// Preserves shutdown failure so callers can retry before dropping runtime.
    pub fn close(&mut self) -> Result<(), StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            ffi::close(self.handle)?;
            #[cfg(feature = "wgpu-dx12")]
            {
                self.device_owner = None;
            }
            Ok(())
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            Err(StreamlineError::Unsupported)
        }
    }

    /// # Errors
    /// Returns unsupported unless Windows native support was built, or a loader error.
    pub fn load(path: &Path) -> Result<Self, StreamlineError> {
        #[cfg(all(windows, feature = "native"))]
        {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
            if wide.is_empty() || wide.len() > 32767 || wide.contains(&0) {
                return Err(StreamlineError::InvalidPath);
            }
            let handle = ffi::load(&wide)?;
            Ok(Self {
                handle,
                #[cfg(feature = "wgpu-dx12")]
                device_owner: None,
                thread_bound: PhantomData,
            })
        }
        #[cfg(not(all(windows, feature = "native")))]
        {
            let _ = path;
            Err(StreamlineError::Unsupported)
        }
    }
}
#[cfg(all(windows, feature = "native"))]
impl Drop for StreamlineRuntime {
    fn drop(&mut self) {
        #[cfg(feature = "wgpu-dx12")]
        if ffi::close(self.handle).is_err() {
            // Failed SDK cleanup may retain COM references. Keep our reference
            // for process lifetime rather than releasing a live SDK dependency.
            if let Some(owner) = self.device_owner.take() {
                std::mem::forget(owner);
            }
        }
        ffi::destroy(self.handle);
    }
}

#[cfg(all(windows, feature = "native"))]
#[allow(unsafe_code)]
mod ffi {
    use super::{
        CameraConstants, DlssRenderSize, Dx12TextureTag, FrameGenerationState,
        RayReconstructionOptions, StreamlineError,
    };
    use std::{ffi::c_void, ptr::NonNull};
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Status {
        domain: u32,
        code: u32,
        windows_error: u32,
    }
    unsafe extern "C" {
        fn voxy_streamline_load(path: *const u16, length: u32, output: *mut *mut c_void) -> Status;
        fn voxy_streamline_initialize_dx12_ex(
            handle: *mut c_void,
            features: u32,
            interposition: u32,
        ) -> Status;
        fn voxy_streamline_register_dx12(handle: *mut c_void, device: *mut c_void) -> Status;
        fn voxy_streamline_native_interface(
            handle: *mut c_void,
            proxy: *mut c_void,
            output: *mut *mut c_void,
        ) -> Status;
        fn voxy_streamline_upgrade_interface(
            handle: *mut c_void,
            base_interface: *mut *mut c_void,
        ) -> Status;
        fn voxy_streamline_dx12_support(
            handle: *mut c_void,
            feature: u32,
            luid: *const u8,
        ) -> Status;
        fn voxy_streamline_close(handle: *mut c_void) -> Status;
        fn voxy_streamline_configure_rr(
            handle: *mut c_void,
            viewport: u32,
            options: *const RayReconstructionOptions,
            output: *mut DlssRenderSize,
        ) -> Status;
        fn voxy_streamline_configure_dlss(
            handle: *mut c_void,
            viewport: u32,
            mode: u32,
            width: u32,
            height: u32,
            hdr: u32,
            output: *mut DlssRenderSize,
        ) -> Status;
        fn voxy_streamline_configure_reflex(
            handle: *mut c_void,
            mode: u32,
            frame_limit_us: u32,
        ) -> Status;
        fn voxy_streamline_fg_state(
            handle: *mut c_void,
            viewport: u32,
            output: *mut FrameGenerationState,
        ) -> Status;
        fn voxy_streamline_configure_fg(
            handle: *mut c_void,
            viewport: u32,
            mode: u32,
            count: u32,
            target: f32,
        ) -> Status;
        fn voxy_streamline_begin_frame(
            runtime: *mut c_void,
            index: *const u32,
            output: *mut *mut c_void,
        ) -> Status;
        fn voxy_streamline_frame_sleep(frame: *mut c_void) -> Status;
        fn voxy_streamline_frame_tag_dx12(
            frame: *mut c_void,
            viewport: u32,
            tags: *const Dx12TextureTag,
            count: u32,
            command_list: *mut c_void,
        ) -> Status;
        fn voxy_streamline_frame_evaluate_dx12(
            frame: *mut c_void,
            viewport: u32,
            feature: u32,
            command_list: *mut c_void,
        ) -> Status;
        fn voxy_streamline_frame_marker(frame: *mut c_void, marker: u32) -> Status;
        fn voxy_streamline_frame_constants(
            frame: *mut c_void,
            viewport: u32,
            constants: *const CameraConstants,
        ) -> Status;
        fn voxy_streamline_frame_destroy(frame: *mut c_void);
        fn voxy_streamline_destroy(handle: *mut c_void);
    }
    pub(super) fn load(path: &[u16]) -> Result<NonNull<c_void>, StreamlineError> {
        let length = u32::try_from(path.len()).map_err(|_| StreamlineError::InvalidPath)?;
        let mut handle = std::ptr::null_mut();
        // SAFETY: Live counted UTF-16 slice and writable output match the C ABI.
        let status = unsafe { voxy_streamline_load(path.as_ptr(), length, &raw mut handle) };
        if status.domain != 0 {
            return Err(StreamlineError::Native {
                domain: status.domain,
                code: status.code,
                windows_error: status.windows_error,
            });
        }
        NonNull::new(handle).ok_or(StreamlineError::Native {
            domain: 3,
            code: 0,
            windows_error: 0,
        })
    }
    fn check(status: Status) -> Result<(), StreamlineError> {
        if status.domain == 0 {
            return Ok(());
        }
        Err(StreamlineError::Native {
            domain: status.domain,
            code: status.code,
            windows_error: status.windows_error,
        })
    }
    pub(super) fn initialize(
        handle: NonNull<c_void>,
        features: u32,
        interposition: u32,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Runtime uniquely owns a live opaque native handle.
        check(unsafe {
            voxy_streamline_initialize_dx12_ex(handle.as_ptr(), features, interposition)
        })
    }
    pub(super) fn register_device(
        handle: NonNull<c_void>,
        device: *mut c_void,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Valid native COM device/lifetime is required by the public unsafe API.
        check(unsafe { voxy_streamline_register_dx12(handle.as_ptr(), device) })
    }
    pub(super) fn native_interface(
        handle: NonNull<c_void>,
        proxy: *mut c_void,
    ) -> Result<*mut c_void, StreamlineError> {
        let mut output = std::ptr::null_mut();
        // SAFETY: Public unsafe API requires live SDK proxy; output slot is valid.
        check(unsafe {
            voxy_streamline_native_interface(handle.as_ptr(), proxy, &raw mut output)
        })?;
        if output.is_null() {
            return Err(StreamlineError::InvalidOptions);
        }
        Ok(output)
    }
    pub(super) fn upgrade_interface(
        handle: NonNull<c_void>,
        base_interface: &mut *mut c_void,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Public unsafe API requires live COM ownership and SDK replacement contract;
        // mutable slot remains valid throughout the C call and preserves SDK mutations.
        check(unsafe { voxy_streamline_upgrade_interface(handle.as_ptr(), base_interface) })
    }
    pub(super) fn support(
        handle: NonNull<c_void>,
        feature: u32,
        luid: &[u8; 8],
    ) -> Result<(), StreamlineError> {
        // SAFETY: Runtime handle and eight-byte LUID remain live during the call.
        check(unsafe { voxy_streamline_dx12_support(handle.as_ptr(), feature, luid.as_ptr()) })
    }
    pub(super) fn close(handle: NonNull<c_void>) -> Result<(), StreamlineError> {
        // SAFETY: Runtime uniquely owns a live handle; close leaves it allocated.
        check(unsafe { voxy_streamline_close(handle.as_ptr()) })
    }
    pub(super) fn configure_dlss(
        handle: NonNull<c_void>,
        viewport: u32,
        mode: u32,
        output: DlssRenderSize,
        hdr: bool,
    ) -> Result<DlssRenderSize, StreamlineError> {
        let mut size = DlssRenderSize::default();
        // SAFETY: Live runtime and writable repr(C) output match the counted scalar ABI.
        check(unsafe {
            voxy_streamline_configure_dlss(
                handle.as_ptr(),
                viewport,
                mode,
                output.width,
                output.height,
                u32::from(hdr),
                &raw mut size,
            )
        })?;
        Ok(size)
    }
    pub(super) fn configure_rr(
        handle: NonNull<c_void>,
        viewport: u32,
        options: &RayReconstructionOptions,
    ) -> Result<DlssRenderSize, StreamlineError> {
        let mut output = DlssRenderSize::default();
        // SAFETY: Live runtime and repr(C) input/output match the C bridge layout.
        check(unsafe {
            voxy_streamline_configure_rr(
                handle.as_ptr(),
                viewport,
                std::ptr::from_ref(options),
                &raw mut output,
            )
        })?;
        Ok(output)
    }
    pub(super) fn configure_reflex(
        handle: NonNull<c_void>,
        mode: u32,
        frame_limit_us: u32,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Valid opaque runtime and scalar configuration match the C ABI.
        check(unsafe { voxy_streamline_configure_reflex(handle.as_ptr(), mode, frame_limit_us) })
    }
    pub(super) fn fg_state(
        handle: NonNull<c_void>,
        viewport: u32,
    ) -> Result<FrameGenerationState, StreamlineError> {
        let mut output = FrameGenerationState::default();
        // SAFETY: Live runtime handle and writable ABI-compatible output slot.
        check(unsafe { voxy_streamline_fg_state(handle.as_ptr(), viewport, &raw mut output) })?;
        Ok(output)
    }
    pub(super) fn configure_fg(
        handle: NonNull<c_void>,
        viewport: u32,
        mode: u32,
        count: u32,
        target: f32,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Owned runtime and scalar configuration match the C ABI.
        check(unsafe {
            voxy_streamline_configure_fg(handle.as_ptr(), viewport, mode, count, target)
        })
    }
    pub(super) fn begin_frame(
        runtime: NonNull<c_void>,
        index: Option<u32>,
    ) -> Result<NonNull<c_void>, StreamlineError> {
        let mut frame = std::ptr::null_mut();
        let index_ptr = index.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
        // SAFETY: Live runtime, optional scalar input, and writable output match C ABI.
        check(unsafe { voxy_streamline_begin_frame(runtime.as_ptr(), index_ptr, &raw mut frame) })?;
        NonNull::new(frame).ok_or(StreamlineError::Native {
            domain: 3,
            code: 0,
            windows_error: 0,
        })
    }
    pub(super) fn frame_sleep(frame: NonNull<c_void>) -> Result<(), StreamlineError> {
        // SAFETY: Frame owns live native handle and borrows the runtime.
        check(unsafe { voxy_streamline_frame_sleep(frame.as_ptr()) })
    }
    pub(super) fn frame_tags(
        frame: NonNull<c_void>,
        viewport: u32,
        tags: &[Dx12TextureTag],
        command_list: Option<NonNull<c_void>>,
    ) -> Result<(), StreamlineError> {
        let count = u32::try_from(tags.len()).map_err(|_| StreamlineError::InvalidOptions)?;
        // SAFETY: Public unsafe entrypoint guarantees native resource/list validity
        // and synchronization. repr(C) slice remains valid throughout the call.
        check(unsafe {
            voxy_streamline_frame_tag_dx12(
                frame.as_ptr(),
                viewport,
                tags.as_ptr(),
                count,
                command_list.map_or(std::ptr::null_mut(), NonNull::as_ptr),
            )
        })
    }
    pub(super) fn frame_evaluate(
        frame: NonNull<c_void>,
        viewport: u32,
        feature: u32,
        command_list: NonNull<c_void>,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Public unsafe entrypoint requires a live recording command list,
        // matching tags/constants and GPU lifetime protection; frame borrows runtime.
        check(unsafe {
            voxy_streamline_frame_evaluate_dx12(
                frame.as_ptr(),
                viewport,
                feature,
                command_list.as_ptr(),
            )
        })
    }
    pub(super) fn frame_marker(frame: NonNull<c_void>, marker: u32) -> Result<(), StreamlineError> {
        // SAFETY: Live frame handle and typed marker selector match the C ABI.
        check(unsafe { voxy_streamline_frame_marker(frame.as_ptr(), marker) })
    }
    pub(super) fn frame_constants(
        frame: NonNull<c_void>,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Live borrowed frame and repr(C) scalar/array payload match the C ABI.
        check(unsafe {
            voxy_streamline_frame_constants(frame.as_ptr(), viewport, std::ptr::from_ref(constants))
        })
    }
    pub(super) fn frame_destroy(frame: NonNull<c_void>) {
        // SAFETY: Unique handle released exactly once; SDK token itself remains SDK-owned.
        unsafe { voxy_streamline_frame_destroy(frame.as_ptr()) };
    }
    pub(super) fn destroy(handle: NonNull<c_void>) {
        // SAFETY: Handle is uniquely owned and released exactly once by Drop.
        unsafe { voxy_streamline_destroy(handle.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frame_generation_policy_obeys_adapter_capabilities() {
        use super::{FrameGeneration, FrameGenerationState};
        let mut state = FrameGenerationState {
            maximum_generated_frames: 3,
            dynamic_supported: 1,
            ..FrameGenerationState::default()
        };
        assert!(FrameGeneration::Off.validate(&state).is_ok());
        for count in [1, 2, 3] {
            assert!(
                FrameGeneration::Fixed {
                    generated_frames: count
                }
                .validate(&state)
                .is_ok()
            );
        }
        for count in [0, 4, u32::MAX] {
            assert!(
                FrameGeneration::Fixed {
                    generated_frames: count
                }
                .validate(&state)
                .is_err()
            );
        }
        for target in [None, Some(60.0), Some(240.0)] {
            assert!(
                FrameGeneration::Dynamic { target_fps: target }
                    .validate(&state)
                    .is_ok()
            );
        }
        for target in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                FrameGeneration::Dynamic {
                    target_fps: Some(target)
                }
                .validate(&state)
                .is_err()
            );
        }
        state.maximum_generated_frames = 1;
        state.dynamic_supported = 0;
        assert!(
            FrameGeneration::Fixed {
                generated_frames: 2
            }
            .validate(&state)
            .is_err()
        );
        assert!(
            FrameGeneration::Dynamic { target_fps: None }
                .validate(&state)
                .is_err()
        );
        state.maximum_generated_frames = 0;
        state.dynamic_supported = 1;
        assert!(
            FrameGeneration::Dynamic { target_fps: None }
                .validate(&state)
                .is_err()
        );
        state.dynamic_supported = 2;
        assert!(
            FrameGeneration::Fixed {
                generated_frames: 1
            }
            .validate(&state)
            .is_err()
        );
        assert!(
            FrameGeneration::Dynamic { target_fps: None }
                .validate(&state)
                .is_err()
        );
        assert!(FrameGeneration::Off.validate(&state).is_ok());
    }

    use super::CameraConstants;
    #[test]
    fn camera_payload_matches_native_c_layout() {
        assert_eq!(std::mem::size_of::<super::RayReconstructionOptions>(), 148);
        assert_eq!(
            std::mem::offset_of!(super::RayReconstructionOptions, world_to_view),
            20
        );
        assert_eq!(
            std::mem::offset_of!(super::RayReconstructionOptions, view_to_world),
            84
        );
        assert_eq!(std::mem::size_of::<CameraConstants>(), 340);
        assert_eq!(std::mem::align_of::<CameraConstants>(), 4);
        assert_eq!(
            std::mem::offset_of!(CameraConstants, inverse_projection),
            64
        );
        assert_eq!(std::mem::offset_of!(CameraConstants, clip_to_previous), 128);
        assert_eq!(std::mem::offset_of!(CameraConstants, previous_to_clip), 192);
        assert_eq!(std::mem::offset_of!(CameraConstants, position), 256);
        assert_eq!(std::mem::offset_of!(CameraConstants, reset), 336);
    }
}

#[cfg(test)]
mod camera_tests {
    use super::*;
    use glam::{Mat4, Vec3, Vec4};
    #[test]
    fn rr_camera_roundtrip_and_invalid_views() {
        let output = DlssRenderSize {
            width: 1920,
            height: 1080,
        };
        let view = Mat4::from_rotation_y(0.35) * Mat4::from_translation(Vec3::new(2.0, -1.0, 4.0));
        assert!(matches!(
            RayReconstructionOptions::from_view(DlssQuality::Quality, output, false, true, view),
            Err(StreamlineError::InvalidOptions)
        ));
        let options =
            RayReconstructionOptions::from_view(DlssQuality::Quality, output, true, false, view)
                .unwrap();
        let world = Vec4::new(3.0, 2.0, -5.0, 1.0);
        let camera = sdk_transform(world, &options.world_to_view);
        assert!((camera - view * world).length() < 0.00001);
        assert!((sdk_transform(camera, &options.view_to_world) - world).length() < 0.00001);
        for invalid in [
            Mat4::ZERO,
            Mat4::from_scale(Vec3::splat(2.0)),
            Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)),
            Mat4::from_cols_array(&[f32::NAN; 16]),
        ] {
            assert!(
                RayReconstructionOptions::from_view(
                    DlssQuality::Quality,
                    output,
                    true,
                    false,
                    invalid
                )
                .is_err()
            );
        }
    }
    fn sdk_transform(vector: Vec4, rows: &[f32; 16]) -> Vec4 {
        let mut output = [0.0; 4];
        for column in 0..4 {
            for row in 0..4 {
                output[column] += vector[row] * rows[row * 4 + column];
            }
        }
        Vec4::from_array(output)
    }
    #[test]
    fn sdk_row_vectors_reproject_to_previous_camera_and_back() {
        let projection = glam::camera::rh::proj::directx::perspective(1.0, 1.5, 0.1, 100.0);
        let previous_view =
            glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let view =
            glam::camera::rh::view::look_at_mat4(Vec3::new(2.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let mut frame = PerspectiveFrame {
            projection,
            view,
            previous_view_projection: Some(projection * previous_view),
            near_plane: 0.1,
            far_plane: 100.0,
            fov: 1.0,
            aspect: 1.5,
            jitter_pixels: [0.0; 2],
            motion_scale: [1.0; 2],
            reset: false,
        };
        let constants = CameraConstants::from_perspective(&frame).unwrap();
        let point = Vec4::new(0.25, -0.5, 0.0, 1.0);
        let current = projection * view * point;
        let previous = projection * previous_view * point;
        assert!(sdk_transform(current, &constants.clip_to_previous).abs_diff_eq(previous, 0.0001));
        assert!(sdk_transform(previous, &constants.previous_to_clip).abs_diff_eq(current, 0.0001));
        assert!(sdk_transform(view * point, &constants.projection).abs_diff_eq(current, 0.0001));
        assert!(Vec3::from_array(constants.position).abs_diff_eq(Vec3::new(2.0, 0.0, 5.0), 0.0001));
        frame.reset = true;
        let cut = CameraConstants::from_perspective(&frame).unwrap();
        assert_eq!(
            cut.clip_to_previous.map(f32::to_bits),
            Mat4::IDENTITY.to_cols_array().map(f32::to_bits)
        );
        assert_eq!(cut.reset, 1);
        frame.previous_view_projection = Some(Mat4::from_cols_array(&[f32::NAN; 16]));
        assert_eq!(
            CameraConstants::from_perspective(&frame),
            Err(StreamlineError::InvalidOptions)
        );
        frame.previous_view_projection = None;
        frame.view = Mat4::from_scale(Vec3::splat(2.0));
        assert_eq!(
            CameraConstants::from_perspective(&frame),
            Err(StreamlineError::InvalidOptions)
        );
        frame.view = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        assert_eq!(
            CameraConstants::from_perspective(&frame),
            Err(StreamlineError::InvalidOptions)
        );
        frame.view = Mat4::ZERO;
        assert_eq!(
            CameraConstants::from_perspective(&frame),
            Err(StreamlineError::InvalidOptions)
        );
    }
}

/// Streamline DX12 interface interception strategy, selected before graphics creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Dx12Interposition {
    /// SDK automatic interception.
    Automatic = 0,
    /// Host explicitly upgrades each relevant interface.
    Manual = 1,
    /// Manual interception using the SDK DXGI factory proxy.
    ManualFactoryProxy = 2,
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
impl StreamlineRuntime {
    /// Register an owned native DX12 device, retaining its COM reference until shutdown.
    /// # Safety
    /// Initialize SDK before device creation; device belongs to this runtime thread.
    /// Serialize all SDK and device operations and preserve device until GPU work ends.
    /// # Errors
    /// Returns SDK device-registration errors.
    #[allow(unsafe_code)]
    pub unsafe fn register_owned_dx12(
        &mut self,
        device: &windows::Win32::Graphics::Direct3D12::ID3D12Device,
    ) -> Result<(), StreamlineError> {
        use windows::core::Interface;
        let owner = device.clone();
        ffi::register_device(self.handle, owner.as_raw())?;
        self.device_owner = Some(owner);
        Ok(())
    }
}

/// SDK FG state snapshot. Query on the present thread; presentation counter is
/// since the previous SDK state query, including queries during configuration.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FrameGenerationState {
    /// Raw SDK status flags; zero means no reported runtime errors.
    pub status: u32,
    pub minimum_dimension: u32,
    /// Generated frames per rendered frame, excluding that rendered frame.
    pub maximum_generated_frames: u32,
    pub frames_presented: u32,
    pub dynamic_supported: u32,
    pub vsync_supported: u32,
    pub estimated_vram_bytes: u64,
    /// Borrowed SDK-owned native fence. Never release it as an owned reference.
    /// Keep runtime/device alive; obey SDK queue-parallelism wait requirements
    /// before modifying or destroying inputs from the previous presentation.
    pub inputs_completion_fence: *mut std::ffi::c_void,
    pub inputs_completion_value: u64,
}

#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(std::mem::size_of::<FrameGenerationState>() == 48);
    assert!(std::mem::offset_of!(FrameGenerationState, estimated_vram_bytes) == 24);
    assert!(std::mem::offset_of!(FrameGenerationState, inputs_completion_fence) == 32);
};
