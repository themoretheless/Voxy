#![recursion_limit = "256"]
//! Engine compute workloads with explicit hardware ownership and validated data.
mod terrain;
pub use terrain::GpuTerrainError;
#[cfg(not(target_arch = "wasm32"))]
pub use terrain::GpuTerrainGenerator;

mod cuda_terrain;
pub use cuda_terrain::{CudaTerrainError, CudaTerrainGenerator};

mod terrain_task;
pub use terrain_task::{PendingTerrain, TerrainDispatch, TerrainJob, TerrainProgram};

mod gravity;
pub use gravity::{
    GravityBody, GravityBudget, GravityComputeError, GravityJob, GravityParameters, GravityProgram,
    GravityReadback, PendingGravity,
};

mod gravity_view;
pub use gravity_view::GravityView;

mod water;
pub use water::{
    CudaWaterTransferProgram, PendingWaterTransfer, WaterComputeError, WaterNode, WaterNodeStatus,
    WaterTransferProgram, WaterTransferResult,
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
pub use voxel_sweep::{GpuSweepError, PendingVoxelSweep};

#[cfg(not(target_arch = "wasm32"))]
pub use voxel_sweep::GpuVoxelCollisionWorld;

mod character_task;
pub use character_task::{CharacterGpuQueryError, PendingGpuCharacter};

mod cuda_collision;
pub use cuda_collision::{CudaCollisionError, CudaVoxelCollisionWorld};

mod vehicle_task;
pub use vehicle_task::{ChassisMotion, PendingGpuVehicle, VehicleGpuError};

mod tissue_task;
pub use tissue_task::{
    is_gpu_tissue_search_failure, GpuTissueError, GpuTissueProgram,
};

mod voxel_mesh;
pub use voxel_mesh::{
    GpuDrawIndirectArgs, GpuMeshResult, GpuVoxelMeshError, GpuVoxelMesher, GpuVoxelPipeline,
    GpuVoxelPipelineError, GpuVoxelVertex, PADDED_CHUNK_VOLUME,
};

mod lighting;
pub use lighting::{GpuLightingError, GpuLightingProgram};

mod morph;
pub use morph::{GpuMorphControl, GpuMorphError, GpuMorphProgram};

mod residency;
pub use residency::body_chunks;
