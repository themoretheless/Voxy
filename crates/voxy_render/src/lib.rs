//! GPU ownership, surface lifecycle, mesh upload formats, and material textures.

mod accumulation;
pub use accumulation::{RadianceAccumulationFrame, RadianceAccumulator};
mod joint_contact_dot;
pub use joint_contact_dot::{
    JOINT_CONTACT_DOT_SHADER, JOINT_CONTACT_NORMALIZE_SERIAL_SHADER,
    JOINT_CONTACT_NORMALIZE_SHADER, JointContactDotInput, JointDotError,
};
pub use joint_contact_dot::{
    JOINT_CONTACT_EQUALITY_SHADER, ResidentContactEqualityInput, ResidentContactEqualityOutput,
    ResidentContactQrPrefix, ValidatedResidentContactEquality,
};
pub use joint_contact_dot::{
    JOINT_CONTACT_QR_SHADER, ResidentContactQrInput, ResidentContactQrOutput,
};
mod banded_solve;
mod banded_transfer;
pub use banded_solve::{
    BANDED_SOLVE_SHADER, BandedPackedReference, BandedSolveError, BandedSolveInput, BandedSystem,
};
pub use banded_transfer::BandedTransferProgram;
mod blit;
mod bodycam;
pub use bodycam::{
    BodycamPostProcess, BodycamPostUniforms, BodycamRig, PhotorealLensProfile, SpringDamper3D,
};
mod camera;
mod planar_capture;
pub use planar_capture::{
    PLANAR_REFLECTION_CLIP_SHADER, PLANAR_REFLECTION_FRESNEL_SHADER,
    PLANAR_REFLECTION_ROUGH_SHADER, PLANAR_REFLECTION_SURFACE_SHADER, PlanarReflectionCapture,
    planar_reflection_pbr_clip_shader,
};
mod camera_lod;
pub use camera_lod::SceneLodBounds;
mod certified_lod;
pub use certified_lod::{
    CertifiedLodError, CertifiedLodIndexSet, CertifiedLodSubdivisionVariant, CertifiedLodVariant,
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
pub use exposure::{AutoExposure, AutoExposureFrame, ExposureSettings};
mod hdr_mips;
mod hdr_resolve;
pub use hdr_mips::HdrMipPyramid;
mod dfg;
pub use dfg::{
    GgxEnergyAtlasBuild, GgxEnergyAtlasSlot, GgxEnergyBuildBudget, GgxEnergyBuildError,
    GgxEnergyBuildPhase, GgxEnergyBuildProgress, GgxEnergyOracleSample, ReadyGgxEnergyAtlas,
};
pub use dfg::{
    EncodedGgxEnergyBatch, GGX_ENERGY_GPU_BATCH_LIMIT, GgxEnergyBatchKey,
    GgxEnergyBatchOutput, GgxEnergyBatchStage, GgxEnergyGpuPrecompute, PendingGgxEnergyBatch,
};
pub use dfg::{
    GgxDfgLut, GgxEnergyPatchData, GgxEnergyPatchError,
    GgxEnergyRefinementAssessment, GgxEnergyRefinementDecision, GgxEnergyRefinementError,
    GgxEnergyRefinementObservation, GgxEnergyRefinementPlan, GgxEnergyRefinementPolicy,
    GgxEnergyRefinementProbe,
};
mod environment_diffuse;
mod environment_lighting;
pub use environment_diffuse::DiffuseEnvironmentConvolution;
mod environment_ggx;
pub use environment_ggx::GgxEnvironmentPrefilter;
mod imported_environment;
pub use imported_environment::{ImportedEnvironment, ImportedEnvironmentError};
mod hdr_probe;
pub use hdr_probe::HdrPixelProbe;
mod hdr_asset;
pub use hdr_asset::{HdrImageAsset, HdrImageError};
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
mod previous_position;
mod primary_motion;
mod primary_surface;
mod radiance;
pub use hdr_resolve::{HdrHalfResolveJob, HdrHalfResolvePipeline};
mod fluid_screen;
mod ray;
mod reconstruction;
mod reconstruction_distance;
mod reconstruction_material;
mod reconstruction_pass;
mod reflection;
mod renderer;
mod scene;
pub use fluid_screen::{
    FluidDepthFilter, FluidDiagnostic, FluidRenderFilmTriangle, FluidRenderParticle,
    ScreenSpaceFluidRenderer,
};
mod skinned;
mod skinned_motion;
mod sprite;
mod surface;
mod surface_lighting;
mod surface_reflection;
mod xr;

pub use blit::{ProcessedColorTarget, StorageColorBlit, StorageColorView, TextureBlit};
pub use material::{MaterialError, MaterialLayer, MaterialPack, MaterialSet};
pub use renderer::{
    CameraView, GpuQuad, RenderOutcome, Renderer, RendererError, SkinnedMotionOutput, SurfaceState,
};
pub use skinned::{SkinnedMesh, SkinnedMeshError, SkinnedUploadError, SkinnedVertex};
pub use voxy_gpu_core::{
    GraphicsBackend, GraphicsCapabilities, GraphicsOptions, PresentPreference,
};
mod skinned_lod;
pub use skinned_lod::{PreparedSkinnedLod, SkinnedLodError, SkinnedLodMesh};
mod skinned_lod_gpu;
pub use skinned_lod_gpu::{SkinnedLodGpuError, SkinnedLodResidency};
pub use skinned_motion::{PreparedSkinnedFrame, SkinnedMotionFrame, SkinnedMotionHistory};

pub use scene::{
    DEFAULT_SCENE_SHADER, PreparedSceneEnvironment, SceneDepthMode, SceneDraw, SceneError, SceneGeometry, SceneLodGeometry,
    SceneLodHistory, SceneMesh, SceneRenderer, SceneShaderError, SceneSkinError, SceneSkinInstance,
    SceneSkinLodLevel, SceneSkinPose, SceneSkinSource, SceneSkinner, SceneTexture, SceneTransform,
    SceneVertex, SceneView, SceneViewTargets,
};

pub use camera::{InvalidSceneCamera, SceneCamera, SceneProjection};

pub use voxy_gpu_core::{
    ComputeDispatch, ComputeError, ComputeJob, ComputeProgram, PendingComputeReadback,
};

pub use xr::{XrFov, XrMotionHistory, XrView, XrViewError};

pub use sprite::{Sprite, SpriteBatch, SpriteBatchError};

pub use depth_motion::DepthMotionPass;
pub use direct_lighting::{DirectLightingJob, PointLightSample};
pub use previous_position::{
    PreviousDepthPass, PreviousPositionPass, PreviousPositionVertex, RasterMotionPass,
};
pub use primary_motion::PrimaryMotionPass;
pub use primary_surface::{PrimarySurfaceJob, PrimarySurfacePipeline};
pub use radiance::{HdrCompositionPipeline, RadianceComposition};
pub use ray::{
    RAY_VISIBILITY_SHADER, RayInstanceUpdate, RayScene, RaySceneError, RaySegment,
    RayVisibilityJob, RayVisibilityPipeline, cpu_segment_visibility,
};
pub use reconstruction::{RayReconstructionGuides, ReconstructionGuideError};
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
    SurfaceReflectionOptions,
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
pub use temporal_history::TemporalHistory;
pub use temporal_resolve::{
    TemporalResolve, TemporalResolveFrame, TemporalResolveInputs, TemporalResolveOptions,
};

/// Textured GGX point light for the `SceneRenderer` ABI (rigid/uniform-scale world transforms).
pub const TEXTURED_POINT_LIGHT_SHADER: &str = include_str!("scene_point_light.wgsl");

mod shadow_map;
pub use shadow_map::{ShadowDraw, ShadowDrawMask, ShadowMap};

mod shadow_visibility;
pub use shadow_visibility::{ShadowFilter, ShadowSettings};

mod reflection_correspondence;
pub use reflection_correspondence::{
    PreviousReflectionTriangle, ReflectionCorrespondenceJob, ReflectionCorrespondencePipeline,
};
mod planar_reflection;
pub use planar_reflection::{
    PlanarReflectionCameras, PlanarReflectionJob, PlanarReflectionPipeline,
};

mod planar_temporal;
pub use planar_temporal::{PlanarTemporalError, PlanarTemporalFrame, PlanarTemporalPipeline};

mod dielectric_boundary;
pub use dielectric_boundary::{DielectricBoundarySample, dielectric_boundary_sample};

mod medium_geometry;
pub use medium_geometry::{
    MediumBoundaryHit, MediumBoundaryMesh, OpaqueRadianceMesh, OpticalMediumId,
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
pub use medium_transport::OpticalSegment;

mod droplet_extinction;
pub use droplet_extinction::{
    DROPLET_EXTINCTION_REFERENCE_SHADER, DROPLET_EXTINCTION_SHADER, DropletExtinctionComputeInput,
    ExtinctionGridView,
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
pub use surface_light_diffusion::{SurfaceLightDiffusion, SurfaceLightEdge};
mod surface_light_pcg;
pub use surface_light_pcg::SurfaceLightPcg;
mod surface_light_geometry;
pub use surface_light_geometry::SurfaceLightGeometry;
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
};

mod surface_light_transfer;
pub use surface_light_transfer::{SurfaceLightTransfer, SurfaceLightVertexBinding};
mod surface_light_frame;
pub use surface_light_frame::{SurfaceLightFrame, SurfaceLightFrameSettings};

mod surface_color_transfer;
pub use surface_color_transfer::{SurfaceColorBinding, SurfaceColorTransfer};

pub use joint_contact_dot::{
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

pub use joint_contact_dot::{
    ExperimentalTripleActiveContactBatch, ExperimentalTripleActiveContactInput,
    ExperimentalTripleActiveFactors, ExperimentalTripleCompletedFactorReadback,
    ExperimentalTripleContactInput, ExperimentalTripleExpandedCandidate,
    ExperimentalTripleExpansion,
};

pub use skinned::MAX_SKIN_JOINTS;

/// Helpers for integration tests of downstream crates (not part of the rendering API).
#[doc(hidden)]
pub mod test_support {
    pub use crate::scene::{NormalCache, smooth_normals};
    pub use crate::skinned::{GpuSkinnedMesh, create_skin_layout, upload_skinned};
    pub use crate::skinned_lod::test_source as skinned_lod_source;
}

mod contact_exact_five;
pub use contact_exact_five::ExperimentalExactFiveContactInput;

#[cfg(test)]
mod contact_five_product_tests;

mod contact_five_qr;
pub use contact_five_qr::{ExperimentalFiveQrInput, ExperimentalFiveQrReadback, ExperimentalFiveQrValue};

#[cfg(test)]
mod contact_five_qr_tests;

mod contact_five_gaps;
pub use contact_five_gaps::{ExperimentalFiveGapInput,ExperimentalFiveGapDecision};

mod contact_five_select;
pub use contact_five_select::{ExperimentalFiveSelection,ExperimentalFiveSelectionInput};

mod contact_five_release;
pub use contact_five_release::{ExperimentalFiveRelease,ExperimentalFiveReleaseInput};

mod compressed_image;
pub use compressed_image::Bc7ImageAsset;

pub mod archive_scene;

mod native_vegetation;
pub use native_vegetation::NativeVegetation;

pub mod weather;
