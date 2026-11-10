#![recursion_limit = "256"]
//! Engine compute workloads with explicit hardware ownership and validated data.
mod terrain;
#[cfg(not(target_arch = "wasm32"))]
pub use terrain::GpuTerrainGenerator;
pub use terrain::{GpuTerrainError, clipmap_toroidal_wrap_coord};

mod cuda_terrain;
pub use cuda_terrain::{CudaTerrainError, CudaTerrainGenerator};

mod terrain_task;
pub use terrain_task::{PendingTerrain, TerrainDispatch, TerrainJob, TerrainProgram};

mod gravity;
pub use gravity::{
    BlockingGravityJob, GravityBody, GravityBudget, GravityComputeError, GravityJob,
    GravityParameters, GravityProgram, GravityReadback, PendingGravity, barnes_hut_mac_satisfied,
};

mod gravity_view;
pub use gravity_view::GravityView;

mod water;
pub use water::{
    CudaWaterTransferProgram, PendingWaterTransfer, WaterComputeError, WaterNode, WaterNodeStatus,
    WaterTransferProgram, WaterTransferResult, courant_friedrichs_lewy_stable_dt,
};

mod water_snapshot;
pub use water_snapshot::{PendingWaterPlan, WaterSnapshotError, WaterWorldSnapshot};

mod voxel_regions;
pub use voxel_regions::{
    PendingVoxelRegions, VoxelClass, VoxelRegion, VoxelRegionProgram, VoxelRegionResult,
};

mod voxel_snapshot;
pub use voxel_snapshot::VoxelRegionSnapshot;

mod voxel_sweep;
pub use voxel_sweep::{GpuSweepError, PendingVoxelSweep, morton_lbvh_common_upper_bits};

#[cfg(not(target_arch = "wasm32"))]
pub use voxel_sweep::GpuVoxelCollisionWorld;

mod character_task;
pub use character_task::{CharacterGpuQueryError, PendingGpuCharacter};

mod cuda_collision;
pub use cuda_collision::{CudaCollisionError, CudaVoxelCollisionWorld};

mod vehicle_task;
pub use vehicle_task::{ChassisMotion, PendingGpuVehicle, VehicleGpuError};

mod tissue_task;
pub use tissue_task::{GpuTissueError, GpuTissueProgram, is_gpu_tissue_search_failure};

mod voxel_mesh;
pub use voxel_mesh::{
    GpuDrawIndirectArgs, GpuMeshResult, GpuVoxelMeshError, GpuVoxelMesher, GpuVoxelPipeline,
    GpuVoxelPipelineError, GpuVoxelVertex, PADDED_CHUNK_VOLUME,
};

mod lighting;
pub use lighting::{GpuLightingError, GpuLightingProgram, estimate_dirty_light_mip_voxels};

mod morph;
pub use morph::{GpuMorphControl, GpuMorphError, GpuMorphProgram};

mod pose_eval;
pub use pose_eval::{GpuClipSet, GpuPoseEvalError, GpuPoseEvalProgram};

mod medium_solver;
pub use medium_solver::{
    GpuMediumError, GpuMediumProgram, MediumTransportJob, MediumTransportResult,
    henyey_greenstein_phase,
};

mod residency;
pub use residency::{ResidencyEvictionCandidate, body_chunks, select_eviction_victims};

pub use voxy_gpu_core::{
    BANDED_SOLVE_SHADER, BandedPackedReference, BandedSolveError, BandedSolveInput, BandedSystem,
    BandedTransferProgram, EXPERIMENTAL_RESIDENT_CONTACT_SHADER,
    EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_64_SHADER, EXPERIMENTAL_SEEDED_RESIDENT_CONTACT_SHADER,
    ExperimentalExactFiveContactInput, ExperimentalFiveGapDecision, ExperimentalFiveGapInput,
    ExperimentalFiveQrInput, ExperimentalFiveQrReadback, ExperimentalFiveQrValue,
    ExperimentalFiveRelease, ExperimentalFiveReleaseInput, ExperimentalFiveSelection,
    ExperimentalFiveSelectionInput, ExperimentalResidentContactBatch,
    ExperimentalResidentContactInput, ExperimentalTripleActiveContactBatch,
    ExperimentalTripleActiveContactInput, ExperimentalTripleActiveFactors,
    ExperimentalTripleCompletedFactorReadback, ExperimentalTripleContactInput,
    ExperimentalTripleExpandedCandidate, ExperimentalTripleExpansion, JOINT_CONTACT_DOT_SHADER,
    JOINT_CONTACT_EQUALITY_SHADER, JOINT_CONTACT_NORMALIZE_SERIAL_SHADER,
    JOINT_CONTACT_NORMALIZE_SHADER, JOINT_CONTACT_QR_SHADER, JointContactDotInput, JointDotError,
    ResidentContactEqualityInput, ResidentContactEqualityOutput, ResidentContactQrInput,
    ResidentContactQrOutput, ResidentContactQrPrefix, ValidatedResidentContactEquality,
    experimental_seeded_resident_contact_shader,
};
