//! `SceneSurface` temporal inputs imported into the native DX12 SDK path.
#![allow(unsafe_code)]
use crate::{
    CameraConstants, StreamlineError, StreamlineFrame,
    dx12::{
        FrameGenerationResources, Recorder, Submission, SuperResolutionResources,
        SuperResolutionTextures, TaggedFrameGenerationInputs, WgpuQueue,
    },
    rr_dx12::{RayReconstructionResources, RayReconstructionTextures},
};
use voxy_render::TemporalFrame;

/// Owned RR scene candidate with material guides and history reset requirements.
#[derive(Debug)]
pub struct SceneRayReconstruction {
    presentation_id: u64,
    reset_history: bool,
    resources: RayReconstructionResources,
    resolve: Option<voxy_render::HdrHalfResolveJob>,
}
impl SceneRayReconstruction {
    #[must_use]
    pub const fn presentation_id(&self) -> u64 {
        self.presentation_id
    }
    #[must_use]
    pub const fn reset_history(&self) -> bool {
        self.reset_history
    }
    /// Import HDR noisy scene color, two-channel motion and matching world guides.
    /// # Safety
    /// All textures must share the registered DX12 device. Color must contain
    /// noisy ray-traced radiance. Match guide depth, surface data, ray distances,
    /// camera matrices and packed/HDR RR options to this candidate. Preserve
    /// native queue/state/lifetime contracts through SDK and GPU completion.
    /// # Errors
    /// Rejects missing HDR/RG motion, sizes, usages, formats and resource aliasing.
    pub unsafe fn import(
        inputs: &TemporalFrame<'_>,
        guides: &voxy_render::RayReconstructionGuides,
        output: &wgpu::Texture,
    ) -> Result<Self, StreamlineError> {
        // SAFETY: Caller guarantees noisy ray-traced color in this candidate.
        unsafe { Self::import_radiance(inputs, inputs.color, guides, output) }
    }
    /// Import separately composed ray-traced HDR color for this temporal candidate.
    /// Use this when lighting composition writes a separate texture instead of the
    /// raster scene color. Presentation identity, depth, motion and reset are retained.
    /// # Safety
    /// Uphold `import`'s device/state/lifetime contracts. Radiance and guides must
    /// match this exact candidate's primary surfaces, camera, jitter and pixel grid.
    /// The lighting producers and composition must finish before preparation.
    /// # Errors
    /// Rejects incompatible HDR/RG motion, sizes, usages, formats and native aliases.
    pub unsafe fn import_radiance(
        inputs: &TemporalFrame<'_>,
        radiance: &wgpu::Texture,
        guides: &voxy_render::RayReconstructionGuides,
        output: &wgpu::Texture,
    ) -> Result<Self, StreamlineError> {
        let radiance_frame = TemporalFrame {
            presentation_id: inputs.presentation_id,
            reset_history: inputs.reset_history,
            motion: inputs.motion,
            depth: inputs.depth,
            color: radiance,
        };
        let inputs = &radiance_frame;
        if inputs.motion.format() != wgpu::TextureFormat::Rg16Float {
            return Err(StreamlineError::InvalidOptions);
        }
        validate_color(inputs, true)?;
        let textures = RayReconstructionTextures {
            color: inputs.color,
            depth: inputs.depth,
            motion: inputs.motion,
            normal_roughness: guides.normal_roughness(),
            diffuse_albedo: guides.diffuse_albedo(),
            specular_albedo: guides.specular_albedo(),
            specular_hit_distance: guides.specular_hit_distance(),
            output,
        };
        // SAFETY: Caller supplies same-device matching RR data and native lifetime contracts.
        let resources = unsafe { RayReconstructionResources::import(&textures, true) }?;
        Ok(Self {
            presentation_id: inputs.presentation_id,
            reset_history: inputs.reset_history,
            resources,
            resolve: None,
        })
    }
    /// Resolve wide linear radiance into an owned half-float input for this candidate.
    /// `prepare` encodes conversion before the native resource transitions, so the
    /// caller need not separately allocate or schedule the SDK color input.
    /// # Safety
    /// Uphold `import_radiance`'s contracts. The resolve pipeline must use the same
    /// registered device. Encode all radiance producers before calling `prepare`.
    /// # Errors
    /// Rejects incompatible resolve inputs or RR resources before SDK evaluation.
    pub unsafe fn import_wide_radiance(
        inputs: &TemporalFrame<'_>,
        radiance: &wgpu::Texture,
        pipeline: &voxy_render::HdrHalfResolvePipeline,
        guides: &voxy_render::RayReconstructionGuides,
        output: &wgpu::Texture,
    ) -> Result<Self, StreamlineError> {
        if radiance.size() != inputs.depth.size()
            || radiance.size() != inputs.motion.size()
            || inputs.motion.format() != wgpu::TextureFormat::Rg16Float
        {
            return Err(StreamlineError::InvalidOptions);
        }
        let resolve = pipeline
            .prepare(radiance)
            .map_err(|_| StreamlineError::InvalidOptions)?;
        // SAFETY: Caller guarantees matching candidate data and registered device.
        let mut candidate =
            unsafe { Self::import_radiance(inputs, resolve.output(), guides, output) }?;
        candidate.resolve = Some(resolve);
        Ok(candidate)
    }
    /// Prepare after scene, material guides, ray lighting and HDR composition writes.
    /// # Errors
    /// Preserves resource usage validation errors.
    pub fn prepare(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), StreamlineError> {
        if let Some(resolve) = &self.resolve {
            resolve.encode(encoder);
        }
        self.resources.prepare(encoder)
    }
    /// Record RR, reconcile exit states, then transfer all owners to a native fence.
    /// # Safety
    /// Submit preparation on this registered serialized queue first. Match the
    /// SDK token, viewport and constants to this candidate. Reconciliation must
    /// restore prepared states/order UAV writes using actual SDK exit states;
    /// never close/reset/submit the recorder inside the callback. Preserve SDK
    /// runtime lifetime until completion and retain later consumers separately.
    /// # Errors
    /// Rejects omitted required reset before SDK use. After evaluation starts,
    /// SDK/callback/closing failures conservatively retain texture owners.
    pub unsafe fn submit_with_reconciliation(
        self,
        queue: &WgpuQueue,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
        reconcile: impl FnOnce(
            &mut Recorder,
            &RayReconstructionResources,
        ) -> Result<(), StreamlineError>,
    ) -> Result<Submission, StreamlineError> {
        if self.reset_history && constants.reset == 0 {
            return Err(StreamlineError::InvalidOptions);
        }
        let mut recorder = queue.recorder()?;
        // SAFETY: Caller guarantees matching prepared resources, configured RR and queue ordering.
        if let Err(error) = unsafe {
            self.resources
                .evaluate(&mut recorder, frame, viewport, constants)
        } {
            std::mem::forget(recorder);
            std::mem::forget(self);
            return Err(error);
        }
        if let Err(error) = reconcile(&mut recorder, &self.resources) {
            std::mem::forget(recorder);
            std::mem::forget(self);
            return Err(error);
        }
        let commands = match recorder.finish() {
            Ok(commands) => commands,
            Err(error) => {
                std::mem::forget(self);
                return Err(error);
            }
        };
        // SAFETY: Caller upholds queue/state contracts; all eight RR leases transfer to the fence.
        unsafe { self.resources.submit(queue, commands) }
    }
}

fn validate_color(inputs: &TemporalFrame<'_>, hdr: bool) -> Result<(), StreamlineError> {
    let supported = if hdr {
        inputs.color.format() == wgpu::TextureFormat::Rgba16Float
    } else {
        matches!(
            inputs.color.format(),
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
        )
    };
    if !supported
        || inputs.depth.format() != wgpu::TextureFormat::Depth32Float
        || !matches!(
            inputs.motion.format(),
            wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rg16Float
        )
    {
        return Err(StreamlineError::InvalidOptions);
    }
    Ok(())
}
/// Owned scene SR inputs with the surface presentation identity and reset state.
#[derive(Debug)]
pub struct SceneSuperResolution {
    presentation_id: u64,
    reset_history: bool,
    resources: SuperResolutionResources,
}
impl SceneSuperResolution {
    #[must_use]
    pub const fn presentation_id(&self) -> u64 {
        self.presentation_id
    }
    #[must_use]
    pub const fn reset_history(&self) -> bool {
        self.reset_history
    }

    /// Import the candidate frame provided by the surface preparation hook.
    /// # Safety
    /// All inputs/output must use the registered DX12 device. Keep resources and
    /// device live through native GPU/SDK use; preserve queue/state contracts.
    /// SDK HDR options must match `hdr`; output must use the configured SR size.
    /// # Errors
    /// Rejects format/HDR mismatch, invalid textures, usage or input dimensions.
    pub unsafe fn import(
        inputs: &TemporalFrame<'_>,
        output: &wgpu::Texture,
        hdr: bool,
    ) -> Result<Self, StreamlineError> {
        validate_color(inputs, hdr)?;
        let expected_output = if hdr {
            wgpu::TextureFormat::Rgba16Float
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        if output.format() != expected_output {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Caller supplies same-device textures and native lifetime requirements.
        let resources = unsafe {
            SuperResolutionResources::import(inputs.color, inputs.depth, inputs.motion, output)
        }?;
        Ok(Self {
            presentation_id: inputs.presentation_id,
            reset_history: inputs.reset_history,
            resources,
        })
    }
    /// Encode transitions after scene writes in the supplied preparation encoder.
    /// # Errors
    /// Preserves resource usage validation errors.
    pub fn prepare(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), StreamlineError> {
        self.resources.prepare(encoder)
    }
    /// Evaluate with constants for this exact scene frame after preparation submission.
    /// # Safety
    /// Uphold native recording/queue contracts; match SDK token, viewport, matrices
    /// and motion conventions to this scene candidate. Retain resources through use.
    /// # Errors
    /// Rejects omitted required history reset and preserves SDK evaluation errors.
    pub unsafe fn evaluate(
        &self,
        recorder: &mut Recorder,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<(), StreamlineError> {
        if self.reset_history && constants.reset == 0 {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Caller binds matching frame/constants and already submitted preparation.
        unsafe {
            self.resources
                .evaluate(recorder, frame, viewport, constants)
        }
    }
    /// Transfer all imported leases to native submission ownership.
    #[must_use]
    pub fn into_resources(self) -> SuperResolutionResources {
        self.resources
    }
    /// Record SR and submit its resource owners on the imported wgpu queue.
    /// # Safety
    /// Preparation must already be submitted on this same registered device/queue.
    /// Serialize native and wgpu submissions, match the SDK frame/constants, and
    /// SDK evaluation must return resources in their prepared states; otherwise
    /// use `submit_with_reconciliation` to restore them before submission. Keep
    /// the SDK runtime live through completion; this fence covers SR work only.
    /// # Errors
    /// Preserves recording, SDK and submission errors. Once evaluation starts,
    /// failed evaluation/closing conservatively retains textures for process
    /// lifetime because SDK resource consumption may have started.
    pub unsafe fn submit(
        self,
        queue: &WgpuQueue,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<Submission, StreamlineError> {
        // SAFETY: Caller guarantees the SDK leaves prepared states restored.
        unsafe {
            self.submit_with_reconciliation(
                queue,
                frame,
                viewport,
                constants,
                |recorder, textures| {
                    // The caller guarantees evaluation restores output to its prepared UAV state.
                    recorder.uav_barrier(textures.output);
                    Ok(())
                },
            )
        }
    }
    /// Record SR, then reconcile native resource states before closing/submitting.
    /// The callback receives the exact retained textures to restore states or
    /// order UAV writes on the same command list, without another HAL import.
    /// # Safety
    /// Uphold `submit` requirements. The callback must record barriers using the
    /// actual SDK exit states and restore the states established by `prepare`.
    /// It must not submit, close or reset the recorder or retain its native pointer.
    /// # Errors
    /// Preserves preparation, SDK, callback and submission errors. Failed SDK or
    /// callback execution retains the recorder and textures conservatively.
    pub unsafe fn submit_with_reconciliation(
        self,
        queue: &WgpuQueue,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
        reconcile: impl FnOnce(
            &mut Recorder,
            &SuperResolutionTextures<'_>,
        ) -> Result<(), StreamlineError>,
    ) -> Result<Submission, StreamlineError> {
        if self.reset_history && constants.reset == 0 {
            return Err(StreamlineError::InvalidOptions);
        }
        let mut recorder = queue.recorder()?;
        // SAFETY: Caller guarantees prepared states, matching frame and queue ordering.
        if let Err(error) = unsafe { self.evaluate(&mut recorder, frame, viewport, constants) } {
            std::mem::forget(recorder);
            std::mem::forget(self);
            return Err(error);
        }
        if let Err(error) = reconcile(&mut recorder, &self.resources.textures()) {
            std::mem::forget(recorder);
            std::mem::forget(self);
            return Err(error);
        }
        let commands = match recorder.finish() {
            Ok(commands) => commands,
            Err(error) => {
                std::mem::forget(self);
                return Err(error);
            }
        };
        // SAFETY: All four imported SR resources transfer to this queue's fence owner.
        unsafe { self.resources.submit(queue, commands) }
    }
}
/// Owned FG scene candidate, preserving presentation identity and history reset.
#[derive(Debug)]
pub struct SceneFrameGeneration {
    presentation_id: u64,
    reset_history: bool,
    resources: FrameGenerationResources,
    resolve: Option<voxy_render::HdrHalfResolveJob>,
}
impl SceneFrameGeneration {
    /// Exact color retained for FG, including any owned wide-HDR conversion.
    /// Encode producers/preparation before sampling; keep native state ordering
    /// consistent and do not overwrite or destroy it during SDK/GPU consumption.
    #[must_use]
    pub fn hudless_color(&self) -> &wgpu::Texture {
        self.resources.hudless_color()
    }

    #[must_use]
    pub const fn presentation_id(&self) -> u64 {
        self.presentation_id
    }
    #[must_use]
    pub const fn reset_history(&self) -> bool {
        self.reset_history
    }
    /// Import HUD-less scene color and opaque depth/motion before presentation.
    /// # Safety
    /// Uphold `import_frame_generation` device, state and lifetime requirements.
    /// # Errors
    /// Preserves HDR/format, usage, dimensions and alias validation errors.
    pub unsafe fn import(inputs: &TemporalFrame<'_>, hdr: bool) -> Result<Self, StreamlineError> {
        // SAFETY: Caller guarantees the scene/SDK native resource contracts.
        let resources = unsafe { import_frame_generation(inputs, hdr) }?;
        Ok(Self {
            presentation_id: inputs.presentation_id,
            reset_history: inputs.reset_history,
            resources,
            resolve: None,
        })
    }
    /// Import separately composed HUD-less color with this candidate's guides.
    /// Color may use output resolution while depth/motion use render resolution.
    /// # Safety
    /// Uphold `import`'s native contracts. Color must correspond to this exact
    /// scene candidate, camera/exposure and SDK HDR mode, without UI overlays.
    /// # Errors
    /// Preserves color/guide format, usage, extent and alias validation errors.
    pub unsafe fn import_radiance(
        inputs: &TemporalFrame<'_>,
        color: &wgpu::Texture,
        hdr: bool,
    ) -> Result<Self, StreamlineError> {
        let composed = TemporalFrame {
            presentation_id: inputs.presentation_id,
            reset_history: inputs.reset_history,
            color,
            depth: inputs.depth,
            motion: inputs.motion,
        };
        // SAFETY: Caller guarantees same-candidate color and native contracts.
        unsafe { Self::import(&composed, hdr) }
    }
    /// Prepare wide HUD-less ray radiance as an owned half-float HDR FG input.
    /// Motion, opaque depth, presentation identity and reset belong to the same
    /// scene candidate; overlays must be composed separately after this color.
    /// # Safety
    /// Uphold `import`'s native contracts. Radiance must match this candidate's
    /// camera and exposure. Color may use presentation resolution while depth
    /// and motion share render resolution. The pipeline uses the registered device.
    /// Submit all producers and preparation before tagging on the same queue.
    /// # Errors
    /// Rejects mismatched extents, unsupported resolve inputs and FG resources.
    pub unsafe fn import_wide_radiance(
        inputs: &TemporalFrame<'_>,
        radiance: &wgpu::Texture,
        pipeline: &voxy_render::HdrHalfResolvePipeline,
    ) -> Result<Self, StreamlineError> {
        if inputs.depth.size() != inputs.motion.size() {
            return Err(StreamlineError::InvalidOptions);
        }
        let resolve = pipeline
            .prepare(radiance)
            .map_err(|_| StreamlineError::InvalidOptions)?;
        // SAFETY: Caller guarantees matching initialized scene inputs and native contracts.
        let mut candidate = unsafe { Self::import_radiance(inputs, resolve.output(), true) }?;
        candidate.resolve = Some(resolve);
        Ok(candidate)
    }
    /// Encode preparation after scene writes and the preceding FG completion wait.
    /// # Errors
    /// Preserves resource usage validation errors.
    pub fn prepare(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), StreamlineError> {
        if let Some(resolve) = &self.resolve {
            resolve.encode(encoder);
        }
        self.resources.prepare(encoder)
    }
    /// Tag this exact scene candidate and transfer owners to the FG lifetime guard.
    /// # Safety
    /// Preparation must be submitted on the registered serialized queue. Match
    /// the SDK token/viewport/camera to this candidate, then present and retain
    /// the returned guard until every SDK/GPU consumer completes.
    /// # Errors
    /// Rejects a missing required reset before SDK calls; preserves tagging errors.
    pub unsafe fn tag(
        self,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<TaggedFrameGenerationInputs, StreamlineError> {
        if self.reset_history && constants.reset == 0 {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Caller supplies the matching scene frame and submitted preparation.
        unsafe { self.resources.tag(frame, viewport, constants) }
    }
}
/// Import scene HUD-less color/depth/motion for presentation-driven FG.
/// # Safety
/// Same-device resources and SDK HDR options must match; preserve all native GPU,
/// resource-state, frame-tag and presentation lifetime requirements.
/// # Errors
/// Rejects HDR/format mismatch, aliasing, missing usages or dimensions.
pub unsafe fn import_frame_generation(
    inputs: &TemporalFrame<'_>,
    hdr: bool,
) -> Result<FrameGenerationResources, StreamlineError> {
    validate_color(inputs, hdr)?;
    // SAFETY: Caller guarantees native device and resource lifetime/state contracts.
    unsafe { FrameGenerationResources::import(inputs.depth, inputs.motion, inputs.color) }
}
