//! GPU ownership, surface lifecycle, mesh upload formats, and material textures.

mod accumulation;
pub use accumulation::{RadianceAccumulationFrame, RadianceAccumulator};
pub use voxy_gpu_core::{
    BANDED_SOLVE_SHADER, BandedPackedReference, BandedSolveError, BandedSolveInput, BandedSystem,
    BandedTransferProgram, JOINT_CONTACT_DOT_SHADER, JOINT_CONTACT_EQUALITY_SHADER,
    JOINT_CONTACT_NORMALIZE_SERIAL_SHADER, JOINT_CONTACT_NORMALIZE_SHADER, JOINT_CONTACT_QR_SHADER,
    JointContactDotInput, JointDotError, ResidentContactEqualityInput,
    ResidentContactEqualityOutput, ResidentContactQrInput, ResidentContactQrOutput,
    ResidentContactQrPrefix, ValidatedResidentContactEquality,
};
mod blit;
mod bodycam;
pub use bodycam::{
    BodycamPostProcess, BodycamPostUniforms, BodycamRig, PhotorealLensProfile, SpringDamper3D,
    brown_conrady_barrel_undistort, circle_of_confusion_px, natural_vignette_cos4_gain,
};
mod camera;
mod planar_capture;
pub use planar_capture::{
    PLANAR_REFLECTION_CLIP_SHADER, PLANAR_REFLECTION_FRESNEL_SHADER,
    PLANAR_REFLECTION_ROUGH_SHADER, PLANAR_REFLECTION_SURFACE_SHADER, PlanarReflectionCapture,
    planar_reflection_pbr_clip_shader,
};
mod camera_lod;
pub use camera_lod::{SceneLodBounds, screen_space_geometric_error_px};
mod certified_lod;
pub use certified_lod::{
    CertifiedLodCheckpoint, CertifiedLodCheckpointError, CertifiedLodError, CertifiedLodIndexSet,
    CertifiedLodSubdivisionVariant, CertifiedLodVariant,
};
pub use voxy_gpu_core::{
    ComputeMemoryBudget, ComputeMemoryStats, ComputeStorage, GpuIndirectMesh,
    ManagedBufferDescriptor, PendingComputeRetirement,
};
pub use voxy_gpu_core::{ComputeReadbackLimits, ComputeReadbackPool, ComputeReadbackStats};
mod depth_motion;
mod depth_sample;
mod direct_lighting;
mod exposure;
mod frame_generation;
pub use exposure::{AutoExposure, AutoExposureFrame, ExposureSettings, ev100_from_camera_settings};
mod hdr_mips;
mod hdr_resolve;
pub use hdr_mips::HdrMipPyramid;
mod dfg;
pub use dfg::{
    EncodedGgxEnergyBatch, GGX_ENERGY_GPU_BATCH_LIMIT, GgxEnergyBatchKey, GgxEnergyBatchOutput,
    GgxEnergyBatchStage, GgxEnergyGpuPrecompute, PendingGgxEnergyBatch,
};
pub use dfg::{
    GgxDfgLut, GgxEnergyPatchData, GgxEnergyPatchError, GgxEnergyRefinementAssessment,
    GgxEnergyRefinementDecision, GgxEnergyRefinementError, GgxEnergyRefinementObservation,
    GgxEnergyRefinementPlan, GgxEnergyRefinementPolicy, GgxEnergyRefinementProbe,
    kulla_conty_multiscatter_compensation,
};
pub use dfg::{
    GgxEnergyAtlasBuild, GgxEnergyAtlasSlot, GgxEnergyBuildBudget, GgxEnergyBuildError,
    GgxEnergyBuildPhase, GgxEnergyBuildProgress, GgxEnergyOracleSample, ReadyGgxEnergyAtlas,
    approx_ggx_directional_albedo,
};
mod environment_diffuse;
mod environment_lighting;
pub use environment_diffuse::{DiffuseEnvironmentConvolution, sh2_non_negative_irradiance};
pub use environment_lighting::jimenez_gtc_specular_ao;
mod environment_ggx;
pub use environment_ggx::{
    GgxEnvironmentPrefilter, anisotropic_ggx_ndf, sample_ggx_vndf_isotropic,
    smith_height_correlated_aniso_g2,
};
mod imported_environment;
pub use imported_environment::{ImportedEnvironment, ImportedEnvironmentError};
mod hdr_probe;
pub use hdr_probe::HdrPixelProbe;
mod hdr_asset;
pub use hdr_asset::{HdrImageAsset, HdrImageError, linear_to_pq_st2084, pq_st2084_to_linear};
mod image_asset;
mod lod;
pub use lod::{LodError, LodIndexSet, LodLevel, LodPolicy};
mod lod_certificate;
pub use lod_certificate::{
    LOD_BARYCENTRIC_DENOMINATOR, LodCertificateError, LodSurface, LodTriangleWitness,
    certify_lod_error,
};
mod lod_witness;
mod material;
pub use lod_witness::{LodWitnessError, LodWitnesses, generate_lod_witnesses};
mod lod_archive;
mod lod_search;
pub use lod_archive::{
    LodArchiveError, LodArchiveLimits, SkinnedLodArchiveError, decode_lod_archive,
    decode_skinned_lod_archive, encode_lod_archive,
};
mod lod_subdivision;
pub use lod_search::{LodSearchBudget, LodSearchWork, generate_indexed_lod_witnesses};
pub use lod_subdivision::{
    LodSubdivisionWitness, LodSubdivisionWitnesses, certify_subdivided_lod_error,
    generate_subdivided_lod_witnesses,
};
mod motion;
mod obj;
mod ocean;
pub use ocean::{
    OceanCamera, OceanRenderer, OceanSettings, cdlod_morph_factor, jacobian_folding_foam_intensity,
    water_spectral_transmittance,
};
pub use voxy_water;
pub use voxy_water::WaveSpectrum;
mod archive_water;
mod previous_position;
mod primary_motion;
mod primary_surface;
mod radiance;
pub use hdr_resolve::{
    HdrHalfResolveJob, HdrHalfResolvePipeline, bilateral_upsample_weight, pbr_neutral_tonemap_rgb,
};
mod fluid_screen;
mod ray;
mod world_nearest_bvh;
mod world_nearest_job;
mod world_ray_producer;
pub use world_nearest_job::{
    ExactWorldNearest, ExactWorldOptions, WorldRayQueryJob, WorldRayQueryKind,
};
pub use world_ray_producer::{ArchiveWorldRayProducer, WORLD_RAY_PRODUCER_WGSL, WorldRayKind};
mod reconstruction;
mod reconstruction_distance;
mod reconstruction_material;
mod reconstruction_pass;
mod reflection;
mod render_graph;
mod renderer;
mod scene;
pub use fluid_screen::{
    FluidDepthFilter, FluidDiagnostic, FluidRenderFilmTriangle, FluidRenderParticle,
    ScreenSpaceFluidRenderer,
};
pub use render_graph::{RenderGraph, RenderGraphNode};
mod geometry_checkpoint;
mod skinned;
pub use geometry_checkpoint::{
    GeometryArchiveLimits, GeometryCheckpointError, ImageCheckpoint, SceneMeshCheckpoint,
    SkinnedMeshCheckpoint, SkinnedMeshSetCheckpoint, SkinnedMeshSetEntry, SkinnedMeshSetLimits,
    SkinnedVertexCheckpoint,
};
mod skinned_motion;
mod sprite;
mod surface;
mod surface_lighting;
mod surface_reflection;
mod xr;

pub use blit::{ProcessedColorTarget, StorageColorBlit, StorageColorView, TextureBlit};
pub use material::{
    MaterialError, MaterialLayer, MaterialPack, MaterialSet, clearcoat_layer_attenuation,
    conductor_fresnel_complex_rgb, slope_variance_to_roughness,
};
pub use renderer::{
    CameraView, GpuQuad, RenderOutcome, Renderer, RendererError, SkinnedMotionOutput, SurfaceState,
};
mod pose_admission_profile;
mod pose_family_bound;
pub use pose_admission_profile::RenderPoseProfile;
pub use skinned::{SkinnedMesh, SkinnedMeshError, SkinnedUploadError, SkinnedVertex};
mod skinned_morph;
pub use skinned_morph::{
    MAX_MORPH_TARGETS, SkinnedMorphMesh, SkinnedMorphTarget, exact_morph_component,
    exact_morph_component64,
};
pub use voxy_gpu_core::{
    GraphicsBackend, GraphicsCapabilities, GraphicsOptions, PresentPreference,
};
mod skinned_lod;
pub use skinned_lod::{
    PreparedSkinnedLod, SkinnedLodCheckpoint, SkinnedLodCheckpointError,
    SkinnedLodCheckpointLimits, SkinnedLodError, SkinnedLodMesh,
};
mod skinned_lod_gpu;
pub use skinned_lod_gpu::{SkinnedLodGpuError, SkinnedLodResidency};
pub use skinned_motion::{PreparedSkinnedFrame, SkinnedMotionFrame, SkinnedMotionHistory};

pub use scene::{
    DEFAULT_SCENE_SHADER, PreparedSceneEnvironment, SceneDepthMode, SceneDraw, SceneError,
    SceneGeometry, SceneInstanceBatch, SceneInstanceData, SceneInstancedDraw, SceneInstancer,
    SceneLodGeometry, SceneLodHistory, SceneMesh, SceneRenderer, SceneShaderError, SceneSkinError,
    SceneSkinInstance, SceneSkinLodLevel, SceneSkinPose, SceneSkinSource, SceneSkinner,
    SceneTexture, SceneTransform, SceneVertex, SceneView, SceneViewTargets,
};

pub use camera::{InvalidSceneCamera, OrbitView, SceneCamera, SceneProjection};

pub use voxy_gpu_core::{
    ComputeDispatch, ComputeError, ComputeJob, ComputeProgram, PendingComputeReadback,
};

pub use xr::{XrFov, XrMotionHistory, XrView, XrViewError};

pub use sprite::{Sprite, SpriteBatch, SpriteBatchError};

pub use depth_motion::DepthMotionPass;
pub use direct_lighting::{
    DirectLightingJob, PointLightSample, capsule_light_irradiance_factor, cluster_depth_slice,
    lunar_dust_retroreflection, oren_nayar_diffuse_reflectance,
    sphere_light_mrt_representative_point,
};
pub use previous_position::{
    PreviousDepthPass, PreviousPositionPass, PreviousPositionVertex, RasterMotionPass,
};
pub use primary_motion::PrimaryMotionPass;
pub use primary_surface::{PrimarySurfaceJob, PrimarySurfacePipeline};
pub use radiance::{HdrCompositionPipeline, RadianceComposition, restir_reservoir_merge_weight};
pub use ray::{
    RAY_VISIBILITY_SHADER, RayInstanceUpdate, RayScene, RaySceneError, RaySegment,
    RayVisibilityJob, RayVisibilityPipeline, cpu_segment_visibility,
};
pub use reconstruction::{
    RayReconstructionGuides, ReconstructionGuideError, svgf_atrous_stopping_weight,
};
pub use reconstruction_distance::ReconstructionDistancePass;
pub use reconstruction_material::{
    GgxReflectionSample, ReconstructionMaterial, ReconstructionMaterialError,
    ReconstructionMaterialSample,
};
pub use reconstruction_pass::{
    ReconstructionGuideInputs, ReconstructionGuideMesh, ReconstructionGuidePass,
    ReconstructionGuideVertex,
};
pub use reflection::{
    GgxSurfaceSample, MirrorSurfaceSample, SpecularDistanceJob, SpecularDistancePipeline,
    SpecularRay,
};
pub use surface_lighting::{
    GgxLightingBatch, GgxLightingInputs, GgxLightingPipeline, SurfaceLightingJob, SurfacePointLight,
};
pub use surface_reflection::{
    GgxReflectionInputs, GgxReflectionPipeline, ReflectionHit, SurfaceReflectionJob,
    SurfaceReflectionOptions, horizon_specular_occlusion,
};
mod reflection_spatial;
pub use reflection_spatial::{
    ReflectionSpatialJob, ReflectionSpatialOptions, ReflectionSpatialPipeline,
};

pub use image_asset::{ImageAsset, ImageAssetError, ImageLimits};
pub use motion::{InvalidMotionMatrix, MOTION_SCENE_SHADER, MotionHistory, MotionMatrices};
pub use obj::{ObjAsset, ObjError, ObjLimits};
pub use scene::{TextureFilter, TextureSampling, TextureWrap};
pub use surface::{SceneSurface, SurfaceOutput, TemporalFrame};

pub use frame_generation::{
    FrameGenerationCapabilities, FrameGenerationError, FrameGenerationMode,
};

mod xray;
pub use xray::{XrayEffect, XrayError, XrayRegion, XrayStyle};

/// Constant-acceleration physics for independent f32 bodies. Group 0 binding 0
/// contains one acceleration/dt vec4 followed by position/velocity vec4 pairs.
/// Positive finite dt and finite body data must be supplied by the caller.
pub const BALLISTIC_SHADER: &str = include_str!("ballistic.wgsl");

mod ray_lighting_frame;
pub use ray_lighting_frame::{
    GgxRayLightingPipeline, GgxRayLightingState, RayLightingFrame, RayLightingInputs,
    RayLightingResources,
};

mod raster_ray_frame;
pub use raster_ray_frame::{
    GgxRasterRayResources, RasterRayAttachments, RasterRayFrame, RasterRayOptions,
};

mod temporal_history;
mod temporal_resolve;
pub use temporal_history::{TemporalHistory, adaptive_temporal_history_alpha};
pub use temporal_resolve::{
    TemporalResolve, TemporalResolveFrame, TemporalResolveInputs, TemporalResolveOptions,
    variance_clipping_aabb_ycocg,
};

/// Textured GGX point light for the `SceneRenderer` ABI (rigid/uniform-scale world transforms).
pub const TEXTURED_POINT_LIGHT_SHADER: &str = include_str!("scene_point_light.wgsl");

mod shadow_map;
pub use shadow_map::{
    ShadowDraw, ShadowDrawMask, ShadowMap, contact_hardening_shadow_bias, snap_cascade_ortho_bounds,
};

mod shadow_visibility;
pub use shadow_visibility::{
    ShadowFilter, ShadowSettings, pcss_penumbra_filter_radius, practical_cascade_splits,
};

mod reflection_correspondence;
pub use reflection_correspondence::{
    PreviousReflectionTriangle, ReflectionCorrespondenceJob, ReflectionCorrespondencePipeline,
};
mod planar_reflection;
pub use planar_reflection::{
    PlanarReflectionCameras, PlanarReflectionJob, PlanarReflectionPipeline,
    anisotropic_road_streak_roughness, oblique_near_plane_clip_matrix,
};

mod planar_temporal;
pub use planar_temporal::{PlanarTemporalError, PlanarTemporalFrame, PlanarTemporalPipeline};

mod dielectric_boundary;
pub use dielectric_boundary::{
    DielectricBoundarySample, cauchy_ior_rgb, critical_tir_cos_theta, dielectric_boundary_sample,
    refraction_solid_angle_jacobian, thin_film_interference_rgb,
};

mod medium_geometry;
pub use medium_geometry::{
    MediumBoundaryHit, MediumBoundaryMesh, OpaqueRadianceMesh, OpticalMediumId,
    froxel_index_from_depth, froxel_slice_depth_from_index,
};

mod medium_geometry_compute;
mod medium_path;
mod medium_path_compute;
pub use medium_geometry_compute::{
    MEDIUM_GEOMETRY_SHADER, MediumGeometryComputeInput, MediumGeometryGpuHit,
    MediumGeometryHitKind, MediumGeometryRay,
};
pub use medium_path::{
    CpuMediumTransportScene, HomogeneousOpticalMedium, MediumTransportBudget,
    MediumTransportEstimate,
};
pub use medium_path_compute::{
    AcceptedMediumImage, EncodedMediumImage, GpuMediumTransportBudget, GpuMediumTransportEstimate,
    MEDIUM_PATH_SHADER, MediumFrameKey, MediumImage, MediumPathComputeInput,
    MediumTransportCameraRay, PendingMediumImage,
};

mod medium_transport;
pub use medium_transport::{OpticalSegment, integrate_scattered_luminance_step};

mod droplet_extinction;
pub use droplet_extinction::{
    DROPLET_EXTINCTION_REFERENCE_SHADER, DROPLET_EXTINCTION_SHADER, DropletExtinctionComputeInput,
    ExtinctionGridView, double_henyey_greenstein_phase,
};

pub use droplet_extinction::{
    DROPLET_EXTINCTION_COMPOSITE_SHADER, DropletExtinctionCompositeInput,
    extinction_segments_from_depth,
};

pub use droplet_extinction::{
    DROPLET_EXTINCTION_SCENE_SHADER, DirectionalScatteringOptions, DropletExtinctionFrame,
    DropletExtinctionPass, DropletExtinctionSceneInput,
};

mod surface_light_diffusion;
pub use surface_light_diffusion::{
    SurfaceLightDiffusion, SurfaceLightEdge, burley_normalized_diffusion_profile,
    sample_burley_diffusion_radius,
};
mod surface_light_pcg;
pub use surface_light_pcg::{SurfaceLightPcg, owen_scrambled_sobol_2d};
mod surface_light_geometry;
pub use surface_light_geometry::{SurfaceLightGeometry, rect_light_solid_angle_steradians};
mod surface_light_admission;
pub use surface_light_admission::SurfaceLightAdmission;

mod surface_deformation;
pub use surface_deformation::{SurfaceDeformation, SurfaceDeformationWeight};
mod surface_rigid_skinning;
pub use surface_rigid_skinning::SurfaceRigidSkinWeight;

mod surface_normal_transport;

mod fiber_surface;
pub use fiber_surface::{
    FIBER_SURFACE_SHADER, FiberSurfaceFrame, FiberSurfaceInput, FiberSurfaceTransfer,
    dual_scattering_hair_attenuation, hair_melanin_absorption_sigma_a,
    marschner_azimuthal_scattering_n,
};

mod surface_light_transfer;
pub use surface_light_transfer::{SurfaceLightTransfer, SurfaceLightVertexBinding};
mod surface_light_frame;
pub use surface_light_frame::{SurfaceLightFrame, SurfaceLightFrameSettings};

mod surface_color_transfer;
pub use surface_color_transfer::{SurfaceColorBinding, SurfaceColorTransfer};

pub use voxy_gpu_core::{
    EXPERIMENTAL_RESIDENT_CONTACT_SHADER, EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER,
    EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_SHADER, ExperimentalResidentContactBatch,
    ExperimentalResidentContactInput, experimental_seeded_resident_contact_shader,
};

mod liquid_density;
pub use liquid_density::{
    LIQUID_DENSITY_SHADER, LiquidDensityCompute, LiquidDensityDispatch, LiquidDensityError,
    LiquidDensityInput, LiquidDensityKernel, LiquidDensityParticle, LiquidDensityReadError,
    PendingLiquidDensity,
};

mod liquid_pair_forces;
pub use liquid_pair_forces::{
    LIQUID_PAIR_FORCES_SHADER, LiquidPairForceCandidate, LiquidPairForceParameters,
    LiquidPairForcesInput, LiquidPreparedPairForcesInput,
};

mod liquid_normals;
pub use liquid_normals::{LIQUID_NORMALS_SHADER, LiquidNormalParticle, LiquidNormalsInput};

mod liquid_accumulation;
pub use liquid_accumulation::{
    LIQUID_ACCUMULATION_SHADER, LiquidAccumulationCandidate, LiquidAccumulationInput,
};

mod liquid_force_pipeline;
pub use liquid_force_pipeline::{
    LiquidForcePipeline, LiquidForcePipelineCandidate, LiquidForcePipelineDispatch,
    LiquidForcePipelineError, LiquidForcePipelineInput, PendingLiquidForcePipeline,
};

mod compute_chain;

mod liquid_density_normal_pipeline;
pub use liquid_density_normal_pipeline::{
    LiquidDensityNormalCandidate, LiquidDensityNormalDispatch, LiquidDensityNormalInput,
    LiquidDensityNormalPipeline, LiquidPressureParameters, PendingLiquidDensityNormal,
};

mod liquid_pipeline;
pub use liquid_pipeline::{
    LiquidPipeline, LiquidPipelineCandidate, LiquidPipelineDispatch, LiquidPipelineInput,
    LiquidPipelineWorkspace, PendingLiquidPipeline,
};

mod liquid_topology;
pub use liquid_topology::LiquidPairTopology;

pub use voxy_gpu_core::{
    ExperimentalExactFiveContactInput, ExperimentalFiveGapDecision, ExperimentalFiveGapInput,
    ExperimentalFiveQrInput, ExperimentalFiveQrReadback, ExperimentalFiveQrValue,
    ExperimentalFiveRelease, ExperimentalFiveReleaseInput, ExperimentalFiveSelection,
    ExperimentalFiveSelectionInput, ExperimentalTripleActiveContactBatch,
    ExperimentalTripleActiveContactInput, ExperimentalTripleActiveFactors,
    ExperimentalTripleCompletedFactorReadback, ExperimentalTripleContactInput,
    ExperimentalTripleExpandedCandidate, ExperimentalTripleExpansion,
};

pub use skinned::MAX_SKIN_JOINTS;

/// Helpers for integration tests of downstream crates (not part of the rendering API).
#[doc(hidden)]
pub mod test_support {
    pub use crate::scene::{NormalCache, smooth_normals};
    pub use crate::skinned::{GpuSkinnedMesh, create_skin_layout, upload_skinned};
    pub use crate::skinned_lod::test_source as skinned_lod_source;
}

#[cfg(test)]
mod contact_five_qr_tests;

mod compressed_image;
pub use compressed_image::Bc7ImageAsset;

pub mod archive_scene;
mod bloom;
pub use bloom::karis_firefly_clamp_rgb;
pub mod character_material;
mod probe_gi;
pub use probe_gi::{BakedProbeCache, chebyshev_probe_visibility};

mod native_vegetation;
pub use native_vegetation::{
    NativeVegetation, NativeVegetationWindow, foliage_translucent_backlighting,
};

mod archive_hdr_diagnostics;
mod archive_temporal;
pub use archive_temporal::{
    ArchiveAntialiasing, ArchiveCameraLens, catmull_rom_5tap_weights, halton_jitter, rgb_to_ycocg,
    ycocg_to_rgb,
};
#[cfg(target_os = "macos")]
mod metalfx;

pub mod climate;
pub mod weather;

mod archive_culling;
pub use archive_culling::{ArchiveInstanceCulling, project_sphere_screen_aabb};

mod archive_frustum;

mod supersampling;
pub use supersampling::{SupersampleResolve, rcas_sharpening_gain};

#[doc(hidden)]
pub mod archive_depth_bounds;

mod archive_gpu_timing;

pub use voxy_gpu_core::{SharedUploadCopy, SharedUploadRing, SharedUploadSnapshot};

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
mod frame_upload_test_support;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[allow(dead_code)]
#[path = "../tests/fixtures/shared_upload_before.rs"]
mod shared_upload_before;
#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
mod shared_upload_move_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
mod full_frame_upload_bench;

#[cfg(test)]
mod probe_sampling_precision_tests;

#[cfg(test)]
mod nearest_tie_count_tests;

#[cfg(test)]
mod world_wide_order_tests;
