//! GPU empty-region rejection followed by exact CPU continuous collision.
use crate::{PendingVoxelRegions, VoxelRegion, VoxelRegionProgram, VoxelRegionSnapshot};
use physics_voxel::{
    AnchoredAabb, SweepConfig, SweepError, SweepResult, sweep_aabb, sweep_candidate_bounds,
};
use voxy_core::VoxelPos;
use voxy_render::ComputeError;
use voxy_world::{BlockRegistry, VoxelView};

#[derive(Debug)]
pub enum GpuSweepError {
    Sweep(SweepError),
    Compute(ComputeError),
    StaleWorld,
}
impl From<ComputeError> for GpuSweepError {
    fn from(error: ComputeError) -> Self {
        Self::Compute(error)
    }
}
impl From<SweepError> for GpuSweepError {
    fn from(error: SweepError) -> Self {
        Self::Sweep(error)
    }
}
impl std::fmt::Display for GpuSweepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU sweep error: {self:?}")
    }
}
impl std::error::Error for GpuSweepError {}

#[derive(Debug)]
pub struct PendingVoxelSweep {
    snapshot: VoxelRegionSnapshot,
    pending: PendingVoxelRegions,
    aabb: AnchoredAabb,
    displacement: [f64; 3],
    config: SweepConfig,
    registry: BlockRegistry,
}
impl VoxelRegionProgram {
    /// Submits exact integer broadphase classification for a continuous voxel sweep.
    /// GPU rejects empty regions; nonempty/fault regions use the existing precise CPU solver.
    /// # Errors
    /// Returns sweep validation, dense grid limits or submission errors.
    pub fn begin_sweep(
        &self,
        queue: &wgpu::Queue,
        view: &impl VoxelView,
        registry: &BlockRegistry,
        aabb: AnchoredAabb,
        displacement: [f64; 3],
        config: SweepConfig,
    ) -> Result<PendingVoxelSweep, GpuSweepError> {
        let (snapshot, dimensions) = capture_sweep(view, registry, aabb, displacement, config)?;
        let pending = snapshot.begin_classify(
            self,
            queue,
            &[VoxelRegion {
                min: [0; 3],
                max: dimensions.map(|d| d - 1),
            }],
        )?;
        Ok(PendingVoxelSweep {
            snapshot,
            pending,
            aabb,
            displacement,
            config,
            registry: registry.clone(),
        })
    }
}
impl PendingVoxelSweep {
    /// Chunks read by the sweep, including missing chunks awaiting recovery.
    pub fn captured_chunks(&self) -> impl Iterator<Item = voxy_core::ChunkPos> + '_ {
        self.snapshot.captured_chunks()
    }

    /// Checks whether all captured chunk identities still match the world.
    #[must_use]
    pub fn is_current(&self, view: &impl VoxelView) -> bool {
        self.snapshot.is_current(view)
    }

    /// Completes once. World edits invalidate the request rather than allowing stale skips.
    /// The captured immutable registry is retained. Serialize world edits during this call.
    /// # Errors
    /// Returns stale world, consumed readback, compute or exact sweep errors.
    pub fn try_sweep(
        &mut self,
        view: &impl VoxelView,
    ) -> Result<Option<SweepResult>, GpuSweepError> {
        let Some(result) = self.pending.try_result()? else {
            return Ok(None);
        };
        if !self.snapshot.is_current(view) {
            return Err(GpuSweepError::StaleWorld);
        }
        if result[0].solid_count == 0 && result[0].first_fault.is_none() {
            Ok(Some(SweepResult {
                fraction: 1.0,
                normal: [0; 3],
                obstacle: None,
            }))
        } else {
            Ok(Some(sweep_aabb(
                &CapturedCollisionView {
                    snapshot: &self.snapshot,
                    fallback: view,
                },
                &self.registry,
                self.aabb,
                self.displacement,
                self.config,
            )?))
        }
    }
}

/// Native synchronous adapter for the shared character/vehicle collision controller.
/// Each query submits GPU broadphase; occupied regions retain the exact CPU sweep.
/// This adapter waits for readback and makes no frame-latency claim.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct GpuVoxelCollisionWorld<'a, V> {
    pub view: &'a V,
    pub registry: &'a BlockRegistry,
    pub program: &'a VoxelRegionProgram,
    pub queue: &'a wgpu::Queue,
}
#[cfg(not(target_arch = "wasm32"))]
impl<V: VoxelView> physics::CollisionWorld for GpuVoxelCollisionWorld<'_, V> {
    type Obstacle = physics_voxel::SweepObstacle;
    type Error = GpuSweepError;
    fn sweep_aabb(
        &self,
        body: physics::AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::SweepResult<Self::Obstacle>, Self::Error> {
        let aabb = AnchoredAabb {
            anchor: VoxelPos {
                x: body.anchor.x,
                y: body.anchor.y,
                z: body.anchor.z,
            },
            min: body.min,
            max: body.max,
        };
        let mut pending = self.program.begin_sweep(
            self.queue,
            self.view,
            self.registry,
            aabb,
            displacement,
            SweepConfig {
                max_candidate_voxels: max_candidates,
            },
        )?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            self.program.poll_native()?;
            if let Some(result) = pending.try_sweep(self.view)? {
                return Ok(physics::SweepResult {
                    fraction: result.fraction,
                    normal: result.normal,
                    obstacle: result.obstacle,
                });
            }
            if std::time::Instant::now() >= deadline {
                return Err(
                    ComputeError::Mapping("GPU collision readback timed out".into()).into(),
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

pub(crate) fn capture_sweep(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    aabb: AnchoredAabb,
    displacement: [f64; 3],
    config: SweepConfig,
) -> Result<(VoxelRegionSnapshot, [u32; 3]), GpuSweepError> {
    let (min, max) = sweep_candidate_bounds(aabb, displacement, config)?;
    let mut dimensions = [0; 3];
    for axis in 0..3 {
        dimensions[axis] =
            u32::try_from(max[axis] - min[axis] + 1).map_err(|_| ComputeError::InvalidBuffer)?;
    }
    let anchor = VoxelPos {
        x: aabb
            .anchor
            .x
            .checked_add(min[0])
            .ok_or(SweepError::CoordinateOverflow)?,
        y: aabb
            .anchor
            .y
            .checked_add(min[1])
            .ok_or(SweepError::CoordinateOverflow)?,
        z: aabb
            .anchor
            .z
            .checked_add(min[2])
            .ok_or(SweepError::CoordinateOverflow)?,
    };
    let snapshot = VoxelRegionSnapshot::capture(view, registry, anchor, dimensions)?;
    Ok((snapshot, dimensions))
}

/// Precise contact sampling shares broadphase's loaded chunk identities.
struct CapturedCollisionView<'a, V> {
    snapshot: &'a VoxelRegionSnapshot,
    fallback: &'a V,
}
impl<V: VoxelView> VoxelView for CapturedCollisionView<'_, V> {
    fn sample(&self, pos: VoxelPos) -> voxy_world::Sample<voxy_world::BlockStateId> {
        self.snapshot
            .retained_block(pos)
            .map_or_else(|| self.fallback.sample(pos), voxy_world::Sample::Loaded)
    }
    fn chunk(&self, pos: voxy_core::ChunkPos) -> Option<voxy_world::ChunkSnapshot> {
        self.fallback.chunk(pos)
    }
}
