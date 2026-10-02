//! Packed normal/roughness, specular-hit-distance DX12 Ray Reconstruction inputs.
#![allow(unsafe_code)]
use crate::{
    CameraConstants, Dx12TextureLease, StreamlineError, StreamlineFeature, StreamlineFrame,
    TextureRole,
    dx12::{RecordedCommands, Recorder, Submission, TextureAccess, WgpuQueue, prepare_texture},
};
use windows::core::Interface;

/// Renderer buffers for RR with packed world-space normals/roughness and hit distance.
#[derive(Debug)]
pub struct RayReconstructionTextures<'a> {
    pub color: &'a wgpu::Texture,
    pub depth: &'a wgpu::Texture,
    pub motion: &'a wgpu::Texture,
    pub normal_roughness: &'a wgpu::Texture,
    pub diffuse_albedo: &'a wgpu::Texture,
    pub specular_albedo: &'a wgpu::Texture,
    pub specular_hit_distance: &'a wgpu::Texture,
    pub output: &'a wgpu::Texture,
}
/// Owns the complete RR input set until native submission completion.
#[derive(Debug)]
pub struct RayReconstructionResources {
    inputs: Vec<(TextureRole, Dx12TextureLease)>,
    output: Dx12TextureLease,
}
impl RayReconstructionResources {
    /// Borrow tagged input roles and their retained leases for native barriers.
    pub fn inputs(&self) -> impl Iterator<Item = (TextureRole, &Dx12TextureLease)> {
        self.inputs.iter().map(|(role, lease)| (*role, lease))
    }
    /// Borrow the retained output for native transitions/UAV ordering.
    #[must_use]
    pub fn output(&self) -> &Dx12TextureLease {
        &self.output
    }
    /// Import linear material buffers, HW depth and reflection hit distance.
    /// # Safety
    /// All textures must share the registered DX12 device. Preserve native/SDK
    /// lifetimes, queue serialization and state tracking. Match packed RR options,
    /// HDR, camera matrices and buffer contents to the configured viewport.
    /// # Errors
    /// Rejects formats, dimensions, usages, aliasing and invalid HAL resources.
    pub unsafe fn import(
        textures: &RayReconstructionTextures<'_>,
        hdr: bool,
    ) -> Result<Self, StreamlineError> {
        use wgpu::TextureFormat as F;
        if !hdr {
            return Err(StreamlineError::InvalidOptions);
        }
        let color_format = if hdr { F::Rgba16Float } else { F::Rgba8Unorm };
        let float4 = |format| matches!(format, F::Rgba16Float | F::Rgba32Float);
        let albedo = |format| matches!(format, F::Rgba8Unorm | F::Rgba16Float | F::Rgba32Float);
        if textures.color.format() != color_format
            || textures.output.format() != color_format
            || textures.depth.format() != F::Depth32Float
            || !matches!(textures.motion.format(), F::Rg16Float | F::Rg32Float)
            || !float4(textures.normal_roughness.format())
            || !albedo(textures.diffuse_albedo.format())
            || !albedo(textures.specular_albedo.format())
            || !matches!(
                textures.specular_hit_distance.format(),
                F::R16Float | F::R32Float
            )
            || !textures.output.usage().contains(
                wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            )
        {
            return Err(StreamlineError::InvalidOptions);
        }
        let descriptions = [
            (TextureRole::ScalingInputColor, textures.color),
            (TextureRole::Depth, textures.depth),
            (TextureRole::MotionVectors, textures.motion),
            (TextureRole::NormalRoughness, textures.normal_roughness),
            (TextureRole::Albedo, textures.diffuse_albedo),
            (TextureRole::SpecularAlbedo, textures.specular_albedo),
            (
                TextureRole::SpecularHitDistance,
                textures.specular_hit_distance,
            ),
        ];
        let mut inputs = Vec::with_capacity(descriptions.len());
        for (role, texture) in descriptions {
            if texture.size() != textures.color.size()
                || !texture
                    .usage()
                    .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            {
                return Err(StreamlineError::InvalidOptions);
            }
            // SAFETY: Caller guarantees shared device, lifetime and native state contracts.
            inputs.push((role, unsafe { Dx12TextureLease::from_wgpu(texture) }?));
        }
        // SAFETY: Same caller guarantees cover the separately sized output.
        let output = unsafe { Dx12TextureLease::from_wgpu(textures.output) }?;
        let resources = Self { inputs, output };
        let leases: Vec<_> = resources
            .inputs
            .iter()
            .map(|(_, lease)| lease)
            .chain(std::iter::once(&resources.output))
            .collect();
        for (index, lease) in leases.iter().enumerate() {
            if leases[..index]
                .iter()
                .any(|other| other.resource.as_raw() == lease.resource.as_raw())
            {
                return Err(StreamlineError::InvalidOptions);
            }
        }
        Ok(resources)
    }
    /// Encode states after renderer writes; submit before native RR evaluation.
    /// # Errors
    /// Preserves usage validation errors.
    pub fn prepare(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), StreamlineError> {
        for (_, lease) in &self.inputs {
            prepare_texture(encoder, lease, TextureAccess::ShaderRead)?;
        }
        prepare_texture(encoder, &self.output, TextureAccess::StorageReadWrite)
    }
    /// Tag packed inputs and record RR on the same viewport/token.
    /// # Safety
    /// Preparation must be submitted first on the registered queue. Packed options
    /// and camera matrices must match the buffers. Retain all owners even on SDK
    /// errors, restore native states and preserve SDK lifetime through completion.
    /// # Errors
    /// Preserves constants, tagging and evaluation errors.
    pub unsafe fn evaluate(
        &self,
        recorder: &mut Recorder,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<(), StreamlineError> {
        let input_state = TextureAccess::ShaderRead.native_state().0.cast_unsigned();
        let output_state = TextureAccess::StorageReadWrite
            .native_state()
            .0
            .cast_unsigned();
        let mut tags = Vec::with_capacity(self.inputs.len() + 1);
        for (role, lease) in &self.inputs {
            tags.push(lease.tag(*role, input_state, 2)?);
        }
        tags.push(
            self.output
                .tag(TextureRole::ScalingOutputColor, output_state, 2)?,
        );
        frame.set_camera_constants(viewport, constants)?;
        // SAFETY: Recorder owns its open list; caller guarantees same-device resources/states.
        let pointer = unsafe { recorder.recording_pointer() }?;
        // SAFETY: Caller retains the full input set and matches native state/lifetime contracts.
        unsafe { frame.tag_dx12(viewport, &tags, Some(pointer)) }?;
        // SAFETY: Same configured packed RR viewport and token with prepared tags/constants.
        unsafe { recorder.evaluate(frame, viewport, StreamlineFeature::RayReconstruction) }
    }
    /// Transfer all eight leases into native completion-fence ownership.
    /// # Safety
    /// Commands/queue must share the registered device; preserve SDK lifetime and
    /// reconcile states before later wgpu access. Include no unretained extra inputs.
    /// # Errors
    /// Preserves native fence/submission errors.
    pub unsafe fn submit(
        self,
        queue: &WgpuQueue,
        commands: RecordedCommands,
    ) -> Result<Submission, StreamlineError> {
        let leases = self
            .inputs
            .into_iter()
            .map(|(_, lease)| lease)
            .chain(std::iter::once(self.output))
            .collect();
        // SAFETY: Caller guarantees queue/device/state contracts; every RR resource transfers.
        unsafe { queue.submit(commands, leases) }
    }
}
