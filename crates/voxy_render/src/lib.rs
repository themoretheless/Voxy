//! GPU ownership, surface lifecycle, mesh upload formats, and material textures.

mod accumulation;
pub use accumulation::{RadianceAccumulationFrame, RadianceAccumulator};
mod backend;
mod blit;
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
mod compute;
mod compute_memory;
pub use compute_memory::{
    ComputeMemoryBudget, ComputeMemoryStats, ComputeStorage, PendingComputeRetirement,
};
mod compute_readback;
pub use certified_lod::{
    CertifiedLodError, CertifiedLodIndexSet, CertifiedLodSubdivisionVariant, CertifiedLodVariant,
};
pub use compute_readback::{ComputeReadbackLimits, ComputeReadbackPool, ComputeReadbackStats};
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
pub use dfg::GgxDfgLut;
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
mod model;
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
    FluidDepthFilter, FluidDiagnostic, FluidRenderParticle, ScreenSpaceFluidRenderer,
};
mod skinned;
mod skinned_motion;
mod sprite;
mod surface;
mod surface_lighting;
mod surface_reflection;
mod xr;

pub use backend::{GraphicsBackend, GraphicsCapabilities, GraphicsOptions};
pub use blit::{ProcessedColorTarget, TextureBlit};
pub use material::{MaterialError, MaterialLayer, MaterialPack, MaterialSet};
pub use model::{ModelAsset, ModelError, ModelGeometry, ModelLimits, ModelPrimitive, ModelTexture};
pub use renderer::{
    CameraView, GpuQuad, RenderOutcome, Renderer, RendererError, SkinnedMotionOutput, SurfaceState,
};
pub use skinned::{SkinnedMesh, SkinnedMeshError, SkinnedUploadError, SkinnedVertex};
mod skinned_lod;
pub use skinned_lod::{PreparedSkinnedLod, SkinnedLodError, SkinnedLodMesh};
mod skinned_lod_gpu;
pub use skinned_lod_gpu::{SkinnedLodGpuError, SkinnedLodResidency};
pub use skinned_motion::{PreparedSkinnedFrame, SkinnedMotionFrame, SkinnedMotionHistory};

pub use scene::{
    DEFAULT_SCENE_SHADER, SceneDepthMode, SceneDraw, SceneError, SceneGeometry, SceneLodGeometry,
    SceneLodHistory, SceneMesh, SceneRenderer, SceneShaderError, SceneSkinError, SceneSkinInstance,
    SceneSkinLodLevel, SceneSkinPose, SceneSkinSource, SceneSkinner, SceneTexture, SceneTransform,
    SceneVertex, SceneView, SceneViewTargets,
};

pub use camera::{InvalidSceneCamera, SceneCamera, SceneProjection};

pub use compute::{
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
pub use shadow_map::{ShadowDraw, ShadowMap};

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
