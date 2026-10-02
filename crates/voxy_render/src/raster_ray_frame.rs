//! Raster primary guides and ordered ray lighting on a shared device.
use crate::{
    RayLightingFrame, RayLightingInputs, RayReconstructionGuides, RayScene, RaySceneError,
    ReconstructionGuideInputs, ReconstructionGuideMesh, ReconstructionGuidePass,
    ReconstructionMaterialError, SceneTexture, SurfacePointLight, SurfaceReflectionOptions,
};

#[derive(Clone, Copy, Debug)]
pub struct RasterRayOptions {
    pub dimensions: [u32; 2],
    pub view_projection: glam::Mat4,
    pub clear_depth: f32,
    pub light: SurfacePointLight,
    pub reflection: SurfaceReflectionOptions,
}
/// Owns G-buffer attachments and their ray-lighting consumers. Recreate on resize,
/// camera/light changes or ray geometry replacement; old submitted frames retain
/// their resources. Opaque mirror/GGX reflections; temporal history is caller-owned.
#[derive(Debug)]
pub struct RasterRayFrame {
    device: wgpu::Device,
    options: RasterRayOptions,
    guides: RayReconstructionGuides,
    depth: wgpu::Texture,
    placeholder: wgpu::Texture,
    pass: ReconstructionGuidePass,
    lighting: RayLightingFrame,
}
/// Device-owned raster attachments and compiled guide pass for ordered reuse.
/// Transfer after all previous consumers; recreate when dimensions change.
#[derive(Debug)]
pub struct RasterRayAttachments {
    device: wgpu::Device,
    guides: RayReconstructionGuides,
    depth: wgpu::Texture,
    placeholder: wgpu::Texture,
    pass: ReconstructionGuidePass,
}
impl RasterRayAttachments {
    #[must_use]
    pub fn guides(&self) -> &RayReconstructionGuides {
        &self.guides
    }
    #[must_use]
    pub fn depth(&self) -> &wgpu::Texture {
        &self.depth
    }
    fn new(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        dimensions: [u32; 2],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let guides = RayReconstructionGuides::new(device, adapter, dimensions[0], dimensions[1])?;
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raster ray depth"),
            size: guides.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let placeholder = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raster ray distance placeholder"),
            size: guides.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let pass = ReconstructionGuidePass::for_guides(device, &guides);
        Ok(Self {
            device: device.clone(),
            guides,
            depth,
            placeholder,
            pass,
        })
    }
}
/// Cached shading pipeline and optional previous surface storage for one frame.
#[derive(Debug)]
pub struct GgxRasterRayResources<'a> {
    pub pipeline: &'a crate::GgxRayLightingPipeline,
    pub state: crate::GgxRayLightingState,
    pub previous_attachments: Option<RasterRayAttachments>,
}
impl RasterRayFrame {
    /// Transfer raster and all ray-lighting outputs to an ordered later frame.
    #[must_use]
    pub fn into_all_resources(self) -> (RasterRayAttachments, crate::RayLightingResources) {
        (
            RasterRayAttachments {
                device: self.device,
                guides: self.guides,
                depth: self.depth,
                placeholder: self.placeholder,
                pass: self.pass,
            },
            self.lighting.into_all_resources(),
        )
    }
    #[must_use]
    pub fn reflection_distance(&self) -> &wgpu::Texture {
        self.lighting.reflection_distance()
    }

    /// Transfer raster attachments and reconstruction storage to a later frame.
    /// Order all current consumers before new writes on the same queue.
    /// Cloned texture/buffer handles obey the same ordering requirement.
    #[must_use]
    pub fn into_resources(self) -> (RasterRayAttachments, crate::PrimarySurfaceJob) {
        let (attachments, primary, _) = self.into_reusable_resources();
        (attachments, primary)
    }
    /// Transfer raster, reconstruction and unfiltered HDR output resources.
    /// Previous consumers must precede new writes on the same queue.
    #[must_use]
    pub fn into_reusable_resources(
        self,
    ) -> (
        RasterRayAttachments,
        crate::PrimarySurfaceJob,
        crate::RadianceComposition,
    ) {
        let (attachments, primary, _, combined) = self.into_lighting_resources();
        (attachments, primary, combined)
    }
    /// Transfer raster, primary, direct-light and composed HDR storage.
    /// Previous consumers must precede all new writes on the same queue.
    #[must_use]
    pub fn into_lighting_resources(
        self,
    ) -> (
        RasterRayAttachments,
        crate::PrimarySurfaceJob,
        crate::SurfaceLightingJob,
        crate::RadianceComposition,
    ) {
        let (primary, direct, combined) = self.lighting.into_lighting_resources();
        (
            RasterRayAttachments {
                device: self.device,
                guides: self.guides,
                depth: self.depth,
                placeholder: self.placeholder,
                pass: self.pass,
            },
            primary,
            direct,
            combined,
        )
    }
    #[must_use]
    pub fn depth(&self) -> &wgpu::Texture {
        &self.depth
    }

    /// Transfer reconstruction storage for a later ordered frame.
    #[must_use]
    pub fn into_primary(self) -> crate::PrimarySurfaceJob {
        self.lighting.into_primary()
    }
    #[must_use]
    pub fn primary(&self) -> &crate::PrimarySurfaceJob {
        self.lighting.primary()
    }

    /// # Errors
    /// Preserves guide allocation and lighting validation errors. Device and adapter
    /// must share a context; emission corresponds to the supplied scene's triangles.
    pub fn new(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        scene: &RayScene,
        options: RasterRayOptions,
        emission: &[[f32; 4]],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::create(device, adapter, scene, options, emission, None, None)
    }
    /// Prepare this frame with seeded GGX emissive reflections.
    /// Direct point light includes GGX specular. Roughness comes from raster guides.
    /// # Errors
    /// Preserves allocation, ray and lighting validation errors.
    pub fn with_ggx(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        scene: &RayScene,
        options: RasterRayOptions,
        emission: &[[f32; 4]],
        seed: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::create(device, adapter, scene, options, emission, Some(seed), None)
    }
    /// Prepare a GGX frame using device-owned cached shading pipelines.
    /// Frame attachments and raster/reconstruction resources remain independent.
    /// # Errors
    /// Rejects foreign scenes and preserves allocation and lighting errors.
    pub fn with_ggx_pipeline(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        scene: &RayScene,
        options: RasterRayOptions,
        emission: &[[f32; 4]],
        seed: u32,
        pipeline: &crate::GgxRayLightingPipeline,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::create(
            device,
            adapter,
            scene,
            options,
            emission,
            Some(seed),
            Some(GgxRasterRayResources {
                pipeline,
                previous_attachments: None,
                state: crate::GgxRayLightingState {
                    seed,
                    previous_primary: None,
                    previous_combined: None,
                    previous_direct: None,
                    previous_reflected: None,
                },
            }),
        )
    }
    /// Prepare a cached GGX frame with explicit reconstruction storage transfer.
    /// Previous consumers must run before this frame's encode on the same queue.
    /// # Errors
    /// Preserves ownership, allocation and lighting validation errors.
    pub fn with_ggx_resources(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        scene: &RayScene,
        options: RasterRayOptions,
        emission: &[[f32; 4]],
        resources: GgxRasterRayResources<'_>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::create(
            device,
            adapter,
            scene,
            options,
            emission,
            None,
            Some(resources),
        )
    }
    fn create(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        scene: &RayScene,
        options: RasterRayOptions,
        emission: &[[f32; 4]],
        seed: Option<u32>,
        mut pipeline: Option<GgxRasterRayResources<'_>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        scene.validate_device(device)?;
        if options.clear_depth.to_bits() != 1.0_f32.to_bits() {
            return Err(RaySceneError::InvalidGeometry.into());
        }
        let previous = pipeline
            .as_mut()
            .and_then(|p| p.previous_attachments.take());
        if previous.as_ref().is_some_and(|p| p.device != *device) {
            return Err(RaySceneError::DeviceMismatch.into());
        }
        let attachments = if let Some(previous) = previous.filter(|p| {
            let size = p.guides.size();
            [size.width, size.height] == options.dimensions
        }) {
            previous
        } else {
            RasterRayAttachments::new(device, adapter, options.dimensions)?
        };
        let RasterRayAttachments {
            guides,
            depth,
            placeholder,
            pass,
            ..
        } = attachments;
        let inputs = RayLightingInputs {
            depth: &depth,
            normal_roughness: guides.normal_roughness(),
            diffuse: guides.diffuse_albedo(),
            mirror_f0: guides.material_f0(),
            view_projection: options.view_projection,
            clear_depth: options.clear_depth,
        };
        let lighting = if let Some(pipeline) = pipeline {
            pipeline.pipeline.prepare_with_state(
                scene,
                inputs,
                options.light,
                emission,
                options.reflection,
                pipeline.state,
            )?
        } else if let Some(seed) = seed {
            RayLightingFrame::with_ggx(
                device,
                scene,
                inputs,
                options.light,
                emission,
                options.reflection,
                seed,
            )?
        } else {
            RayLightingFrame::new(
                device,
                scene,
                inputs,
                options.light,
                emission,
                options.reflection,
            )?
        };
        Ok(Self {
            device: device.clone(),
            options,
            guides,
            depth,
            placeholder,
            pass,
            lighting,
        })
    }
    /// Prepare per-object material bindings for this frame's camera.
    /// # Errors
    /// Rejects foreign materials and invalid camera inputs.
    pub fn material_inputs(
        &self,
        material: &SceneTexture,
    ) -> Result<ReconstructionGuideInputs, ReconstructionMaterialError> {
        self.pass.scene_material_inputs(
            &self.device,
            self.options.view_projection,
            self.options.reflection.camera,
            &self.placeholder,
            material,
        )
    }
    /// Rasterize primary guides and F0, then reconstruct and shade their surfaces.
    /// Build/update the matching ray scene first in this encoder. Does not submit,
    /// present, read back or commit temporal state. Fixed conventional depth clear 1.
    /// # Errors
    /// Rejects unsupported clear depth and per-draw input size mismatch.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        draws: &[(&ReconstructionGuideMesh, &ReconstructionGuideInputs)],
    ) -> Result<(), RaySceneError> {
        self.encode_with_composition_timestamps(encoder, draws, None)
    }
    /// Encode the frame with timestamps on its actual HDR composition pass.
    /// # Errors
    /// Rejects invalid raster inputs, as in `Self::encode`.
    pub fn encode_with_composition_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        draws: &[(&ReconstructionGuideMesh, &ReconstructionGuideInputs)],
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) -> Result<(), RaySceneError> {
        self.encode_with_lighting_timestamps(encoder, draws, [None, None, timestamp_writes])
    }
    /// Optional actual-pass timestamps in direct-light, reflection, composition order.
    /// # Errors
    /// Rejects invalid raster inputs, as in `Self::encode`.
    pub fn encode_with_lighting_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        draws: &[(&ReconstructionGuideMesh, &ReconstructionGuideInputs)],
        timestamps: [Option<wgpu::ComputePassTimestampWrites<'_>>; 3],
    ) -> Result<(), RaySceneError> {
        let depth = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.pass
            .encode_draws(encoder, &self.guides, &depth, draws)
            .map_err(|_| RaySceneError::InvalidGeometry)?;
        self.pass
            .encode_material_f0_draws(encoder, &self.guides, &depth, draws)
            .map_err(|_| RaySceneError::InvalidGeometry)?;
        self.lighting
            .encode_with_lighting_timestamps(encoder, timestamps);
        Ok(())
    }
    /// Create backward normalized UV motion for static world-space geometry.
    /// Encode after this frame's primary reconstruction. Moving objects require
    /// the object/correspondence variants of `PrimaryMotionPass` instead.
    /// History reset writes zero motion; presentation history stays caller-owned.
    /// # Errors
    /// Rejects invalid previous camera matrices and insufficient device limits.
    pub fn camera_motion(
        &self,
        previous: glam::Mat4,
        reset_history: bool,
    ) -> Result<crate::PrimaryMotionPass, RaySceneError> {
        crate::PrimaryMotionPass::new(
            &self.device,
            self.lighting.primary(),
            self.options.view_projection,
            previous,
            reset_history,
        )
    }
    /// Rasterize IDs at the primary depth after `encode`, with the same meshes.
    /// IDs index the model-pair table used by `object_motion`.
    /// # Errors
    /// Rejects reserved IDs and invalid camera inputs.
    pub fn encode_object_ids(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        meshes: &[(&ReconstructionGuideMesh, u32)],
    ) -> Result<(), ReconstructionMaterialError> {
        let inputs = self.pass.inputs(
            &self.device,
            self.options.view_projection,
            self.options.reflection.camera,
            &self.placeholder,
        )?;
        let depth = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.pass
            .encode_object_ids(&self.device, encoder, &self.guides, &depth, &inputs, meshes)
    }
    /// Motion for independently moving affine objects at their current world positions.
    /// Encode after primary reconstruction and this frame's object-ID pass. Model
    /// pairs are [current, previous]; deformation needs correspondence data instead.
    /// # Errors
    /// Rejects invalid model/camera matrices and unsupported resource limits.
    pub fn object_motion(
        &self,
        previous_camera: glam::Mat4,
        models: &[[glam::Mat4; 2]],
        reset_history: bool,
    ) -> Result<crate::PrimaryMotionPass, RaySceneError> {
        crate::PrimaryMotionPass::for_objects(
            &self.device,
            self.lighting.primary(),
            [self.options.view_projection, previous_camera],
            models,
            self.guides.object_ids(),
            reset_history,
        )
    }
    /// Raster motion for matching current/previous deformed world-space vertices.
    /// Encode after `encode` using identical current opaque coverage and topology.
    /// Cameras must be unjittered. Output is backward normalized UV; background
    /// and history reset are zero. Pose storage and history remain caller-owned.
    /// # Errors
    /// Rejects invalid paired geometry, cameras and insufficient device limits.
    pub fn deformation_motion(
        &self,
        previous_camera: glam::Mat4,
        vertices: &[crate::PreviousPositionVertex],
        reset_history: bool,
    ) -> Result<crate::RasterMotionPass, RaySceneError> {
        crate::RasterMotionPass::new(
            &self.device,
            &self.depth,
            [self.options.view_projection, previous_camera],
            vertices,
            reset_history,
        )
    }
    /// Expected previous-camera depth for deformation-aware reprojection.
    /// Encode after this frame's primary pass with identical current coverage.
    /// # Errors
    /// Rejects invalid paired geometry, previous camera or device capacity.
    pub fn previous_depth(
        &self,
        previous_camera: glam::Mat4,
        vertices: &[crate::PreviousPositionVertex],
    ) -> Result<crate::PreviousDepthPass, RaySceneError> {
        crate::PreviousDepthPass::new(
            &self.device,
            &self.depth,
            [self.options.view_projection, previous_camera],
            vertices,
        )
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        self.lighting.output()
    }
    /// Direct diffuse/specular point-light radiance with opaque ray shadows.
    #[must_use]
    pub fn direct_radiance(&self) -> &wgpu::Texture {
        self.lighting.direct_radiance()
    }
    /// Exact ray reflection producer for correspondence and temporal consumers.
    #[must_use]
    pub fn reflection_job(&self) -> &crate::SurfaceReflectionJob {
        self.lighting.reflection_job()
    }
    /// Unfiltered BSDF-weighted emissive reflection contribution.
    #[must_use]
    pub fn reflected_radiance(&self) -> &wgpu::Texture {
        self.lighting.reflected_radiance()
    }
    #[must_use]
    pub fn guides(&self) -> &RayReconstructionGuides {
        &self.guides
    }
}
