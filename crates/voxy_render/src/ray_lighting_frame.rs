//! Ordered ray lighting from rasterized primary-surface guides.
use crate::{
    PrimarySurfaceJob, RadianceComposition, RayScene, RaySceneError, SurfaceLightingJob,
    SurfacePointLight, SurfaceReflectionJob, SurfaceReflectionOptions,
};

/// Guides must describe the same rasterized frame, dimensions and GPU device.
/// Diffuse reflectance excludes energy assigned to reflection; mirror F0 is a
/// linear material map, not reflected radiance. Alpha zero marks invalid F0.
#[derive(Clone, Copy, Debug)]
pub struct RayLightingInputs<'a> {
    pub depth: &'a wgpu::Texture,
    pub normal_roughness: &'a wgpu::Texture,
    pub diffuse: &'a wgpu::Texture,
    pub mirror_f0: &'a wgpu::Texture,
    pub view_projection: glam::Mat4,
    pub clear_depth: f32,
}

/// Owns reconstruction, direct opaque shadows, material-weighted emissive mirror
/// or seeded GGX reflections and HDR composition for immutable frame inputs.
/// `new` supports zero roughness; `with_ggx` samples rough reflections. Callers must recreate this object when geometry replacement
/// changes the TLAS binding, or when guides/camera/light/material inputs change.
#[derive(Debug)]
pub struct RayLightingFrame {
    primary: PrimarySurfaceJob,
    direct: SurfaceLightingJob,
    reflected: SurfaceReflectionJob,
    combined: RadianceComposition,
}
/// Device-owned compiled direct/reflection GGX and wide HDR pipelines.
/// Retain across scene updates and resize; recreate after device replacement.
/// Each prepared frame still owns independent uniforms, bindings and outputs.
#[derive(Debug)]
pub struct GgxRayLightingPipeline {
    device: wgpu::Device,
    primary: crate::PrimarySurfacePipeline,
    direct: crate::GgxLightingPipeline,
    reflected: crate::GgxReflectionPipeline,
    combined: crate::HdrCompositionPipeline,
}
/// Per-frame random state and optional reconstruction storage transfer.
/// Order all previous storage consumers before encoding the new frame.
#[derive(Debug)]
pub struct GgxRayLightingState {
    pub seed: u32,
    pub previous_primary: Option<PrimarySurfaceJob>,
    pub previous_combined: Option<RadianceComposition>,
    pub previous_direct: Option<SurfaceLightingJob>,
    pub previous_reflected: Option<SurfaceReflectionJob>,
}
impl GgxRayLightingPipeline {
    /// # Errors
    /// Rejects missing ray-query features and insufficient resource limits.
    pub fn new(device: &wgpu::Device) -> Result<Self, RaySceneError> {
        Ok(Self {
            device: device.clone(),
            primary: crate::PrimarySurfacePipeline::new(device)?,
            direct: crate::GgxLightingPipeline::new(device)?,
            reflected: crate::GgxReflectionPipeline::new(device)?,
            combined: crate::HdrCompositionPipeline::new(device)?,
        })
    }
    /// Prepare immutable frame bindings without recompiling GGX/HDR pipelines.
    /// Reconstruction is cached too; raster pipelines remain outside this cache.
    /// # Errors
    /// Preserves scene ownership, guide, material and lighting validation errors.
    pub fn prepare(
        &self,
        scene: &RayScene,
        inputs: RayLightingInputs<'_>,
        light: SurfacePointLight,
        emission: &[[f32; 4]],
        reflection: SurfaceReflectionOptions,
        seed: u32,
    ) -> Result<RayLightingFrame, RaySceneError> {
        self.prepare_with_state(
            scene,
            inputs,
            light,
            emission,
            reflection,
            GgxRayLightingState {
                seed,
                previous_primary: None,
                previous_combined: None,
                previous_direct: None,
                previous_reflected: None,
            },
        )
    }
    /// Prepare a frame, optionally transferring a previous surface buffer.
    /// The transfer follows `PrimarySurfacePipeline::prepare_reusing` ordering rules.
    /// # Errors
    /// Preserves device ownership and all normal frame validation errors.
    pub fn prepare_with_state(
        &self,
        scene: &RayScene,
        inputs: RayLightingInputs<'_>,
        light: SurfacePointLight,
        emission: &[[f32; 4]],
        reflection: SurfaceReflectionOptions,
        state: GgxRayLightingState,
    ) -> Result<RayLightingFrame, RaySceneError> {
        scene.validate_device(&self.device)?;
        let primary = if let Some(previous) = state.previous_primary {
            self.primary.prepare_reusing(
                previous,
                inputs.depth,
                inputs.normal_roughness,
                inputs.view_projection,
                inputs.clear_depth,
            )?
        } else {
            self.primary.prepare(
                inputs.depth,
                inputs.normal_roughness,
                inputs.view_projection,
                inputs.clear_depth,
            )?
        };
        let direct = if let Some(previous) = state.previous_direct {
            self.direct.create_job_reusing(
                scene,
                crate::GgxLightingInputs {
                    primary: &primary,
                    diffuse: inputs.diffuse,
                    f0: inputs.mirror_f0,
                    camera: reflection.camera,
                    light,
                },
                previous,
            )?
        } else {
            self.direct.create_job(
                scene,
                &primary,
                inputs.diffuse,
                inputs.mirror_f0,
                reflection.camera,
                light,
            )?
        };
        let reflected = if let Some(previous) = state.previous_reflected {
            self.reflected.create_job_reusing(
                scene,
                crate::GgxReflectionInputs {
                    primary: &primary,
                    emission,
                    options: reflection,
                    material_f0: inputs.mirror_f0,
                    seed: state.seed,
                },
                previous,
            )?
        } else {
            self.reflected.create_job(
                scene,
                &primary,
                emission,
                reflection,
                inputs.mirror_f0,
                state.seed,
            )?
        };
        let combined = if let Some(previous) = state.previous_combined {
            self.combined
                .create_job_reusing(previous, direct.output(), reflected.radiance())?
        } else {
            self.combined
                .create_job(direct.output(), reflected.radiance())?
        };
        Ok(RayLightingFrame {
            primary,
            direct,
            reflected,
            combined,
        })
    }
}
/// Resources transferred from a completed/ordered ray-lighting frame.
#[derive(Debug)]
pub struct RayLightingResources {
    pub primary: PrimarySurfaceJob,
    pub direct: SurfaceLightingJob,
    pub reflected: SurfaceReflectionJob,
    pub combined: RadianceComposition,
}
impl RayLightingFrame {
    /// Transfer all mutable outputs; old consumers must precede new writes.
    #[must_use]
    pub fn into_all_resources(self) -> RayLightingResources {
        RayLightingResources {
            primary: self.primary,
            direct: self.direct,
            reflected: self.reflected,
            combined: self.combined,
        }
    }

    /// Transfer primary, direct-light and composed HDR resources for ordered reuse.
    #[must_use]
    pub fn into_lighting_resources(
        self,
    ) -> (PrimarySurfaceJob, SurfaceLightingJob, RadianceComposition) {
        (self.primary, self.direct, self.combined)
    }

    /// Transfer reconstruction and HDR storage to a later ordered frame.
    #[must_use]
    pub fn into_resources(self) -> (PrimarySurfaceJob, RadianceComposition) {
        (self.primary, self.combined)
    }

    /// Transfer primary storage for an explicitly ordered later frame.
    /// All current-frame consumers must run before the next reconstruction encode.
    #[must_use]
    pub fn into_primary(self) -> PrimarySurfaceJob {
        self.primary
    }

    /// Prepare the ordered passes. Does not submit commands or build the scene.
    /// Emission contains one linear RGB entry per opaque BLAS triangle.
    /// # Errors
    /// Preserves guide, material, ray feature, light and capacity validation errors.
    pub fn new(
        device: &wgpu::Device,
        scene: &RayScene,
        inputs: RayLightingInputs<'_>,
        light: SurfacePointLight,
        emission: &[[f32; 4]],
        reflection: SurfaceReflectionOptions,
    ) -> Result<Self, RaySceneError> {
        Self::create(device, scene, inputs, light, emission, reflection, None)
    }
    /// Prepare seeded GGX reflections with the same ordered raster lighting path.
    /// Direct light includes GGX specular; reflections sample opaque emissive geometry.
    /// # Errors
    /// Preserves guide, material, ray feature and capacity validation errors.
    pub fn with_ggx(
        device: &wgpu::Device,
        scene: &RayScene,
        inputs: RayLightingInputs<'_>,
        light: SurfacePointLight,
        emission: &[[f32; 4]],
        reflection: SurfaceReflectionOptions,
        seed: u32,
    ) -> Result<Self, RaySceneError> {
        Self::create(
            device,
            scene,
            inputs,
            light,
            emission,
            reflection,
            Some(seed),
        )
    }
    fn create(
        device: &wgpu::Device,
        scene: &RayScene,
        inputs: RayLightingInputs<'_>,
        light: SurfacePointLight,
        emission: &[[f32; 4]],
        reflection: SurfaceReflectionOptions,
        seed: Option<u32>,
    ) -> Result<Self, RaySceneError> {
        scene.validate_device(device)?;
        let primary = PrimarySurfaceJob::new(
            device,
            inputs.depth,
            inputs.normal_roughness,
            inputs.view_projection,
            inputs.clear_depth,
        )?;
        let direct = if seed.is_some() {
            SurfaceLightingJob::with_ggx(
                device,
                scene,
                &primary,
                inputs.diffuse,
                inputs.mirror_f0,
                reflection.camera,
                light,
            )?
        } else {
            SurfaceLightingJob::new(device, scene, &primary, inputs.diffuse, light)?
        };
        let reflected = if let Some(seed) = seed {
            SurfaceReflectionJob::with_ggx_material_map(
                device,
                scene,
                &primary,
                emission,
                reflection,
                inputs.mirror_f0,
                seed,
            )?
        } else {
            SurfaceReflectionJob::with_material_map(
                device,
                scene,
                &primary,
                emission,
                reflection,
                inputs.mirror_f0,
            )?
        };
        let combined = if seed.is_some() {
            RadianceComposition::new_hdr(device, direct.output(), reflected.radiance())?
        } else {
            RadianceComposition::new(device, direct.output(), reflected.radiance())?
        };
        Ok(Self {
            primary,
            direct,
            reflected,
            combined,
        })
    }
    /// Encode after rasterization of the input guides and BLAS/TLAS construction.
    /// Pass boundaries order storage writes before their consumers. No CPU readback
    /// or presentation is performed; the caller submits and handles frame history.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        self.encode_with_composition_timestamps(encoder, None);
    }
    /// Time the actual HDR composition pass; other lighting passes remain untimed.
    pub fn encode_with_composition_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        self.encode_with_lighting_timestamps(encoder, [None, None, timestamp_writes]);
    }
    /// Optional actual-pass timestamps in direct-light, reflection, composition
    /// order. Query ownership and index lifetime follow the individual jobs.
    pub fn encode_with_lighting_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: [Option<wgpu::ComputePassTimestampWrites<'_>>; 3],
    ) {
        let [direct, reflected, combined] = timestamps;
        self.primary.encode(encoder);
        self.direct.encode_with_timestamps(encoder, direct);
        self.reflected.encode_with_timestamps(encoder, reflected);
        self.combined.encode_with_timestamps(encoder, combined);
    }
    /// Linear HDR suitable for tone mapping or reconstruction input.
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        self.combined.output()
    }
    #[must_use]
    pub fn primary(&self) -> &PrimarySurfaceJob {
        &self.primary
    }
    #[must_use]
    pub fn direct_radiance(&self) -> &wgpu::Texture {
        self.direct.output()
    }
    #[must_use]
    pub fn reflected_radiance(&self) -> &wgpu::Texture {
        self.reflected.radiance()
    }
    /// Exact reflection producer, including GPU hit correspondence records.
    #[must_use]
    pub fn reflection_job(&self) -> &crate::SurfaceReflectionJob {
        &self.reflected
    }
    #[must_use]
    pub fn reflection_distance(&self) -> &wgpu::Texture {
        self.reflected.distance()
    }
}
