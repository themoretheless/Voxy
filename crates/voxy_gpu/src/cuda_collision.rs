//! CUDA integer broadphase and f64 continuous voxel contacts.
use crate::{GpuSweepError, VoxelRegion, VoxelRegionSnapshot};
use voxy_cuda::{CudaCompute, CudaError};
use voxy_world::{BlockRegistry, VoxelView};
#[derive(Debug)]
pub enum CudaCollisionError {
    World(GpuSweepError),
    Cuda(CudaError),
}
impl std::fmt::Display for CudaCollisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CUDA collision error: {self:?}")
    }
}
impl std::error::Error for CudaCollisionError {}
/// Native synchronous collision adapter. CUDA classifies all broadphase cells;
/// nonempty and unavailable regions run local f64 contact queries on CUDA.
#[derive(Debug)]
pub struct CudaVoxelCollisionWorld<'a, V> {
    pub compute: &'a CudaCompute,
    pub view: &'a V,
    pub registry: &'a BlockRegistry,
}
impl<V: VoxelView> physics::CollisionWorld for CudaVoxelCollisionWorld<'_, V> {
    type Obstacle = physics_voxel::SweepObstacle;
    type Error = CudaCollisionError;
    fn sweep_aabb(
        &self,
        body: physics::AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::SweepResult<Self::Obstacle>, Self::Error> {
        let aabb = physics_voxel::AnchoredAabb {
            anchor: voxy_core::VoxelPos {
                x: body.anchor.x,
                y: body.anchor.y,
                z: body.anchor.z,
            },
            min: body.min,
            max: body.max,
        };
        let config = physics_voxel::SweepConfig {
            max_candidate_voxels: max_candidates,
        };
        let (snapshot, dimensions) =
            crate::voxel_sweep::capture_sweep(self.view, self.registry, aabb, displacement, config)
                .map_err(CudaCollisionError::World)?;
        let results = snapshot
            .classify_cuda(
                self.compute,
                &[VoxelRegion {
                    min: [0; 3],
                    max: dimensions.map(|d| d - 1),
                }],
            )
            .map_err(CudaCollisionError::Cuda)?;
        if !snapshot.is_current(self.view) {
            return Err(CudaCollisionError::World(GpuSweepError::StaleWorld));
        }
        if results[0].solid_count == 0 && results[0].first_fault.is_none() {
            return Ok(physics::SweepResult {
                fraction: 1.0,
                normal: [0; 3],
                obstacle: None,
            });
        }
        let result = cuda_contacts(
            self.compute,
            &snapshot,
            self.view,
            self.registry,
            aabb,
            displacement,
            config,
        )?;
        if !snapshot.is_current(self.view) {
            return Err(CudaCollisionError::World(GpuSweepError::StaleWorld));
        }
        Ok(result)
    }
}

fn cuda_contacts(
    compute: &CudaCompute,
    snapshot: &VoxelRegionSnapshot,
    view: &impl VoxelView,
    registry: &BlockRegistry,
    aabb: physics_voxel::AnchoredAabb,
    displacement: [f64; 3],
    config: physics_voxel::SweepConfig,
) -> Result<physics::SweepResult<physics_voxel::SweepObstacle>, CudaCollisionError> {
    use physics_voxel::{SweepError, SweepObstacle, sweep_candidate_bounds};
    use voxy_world::{CollisionShape, Sample};
    let fail = |error: SweepError| CudaCollisionError::World(error.into());
    let (min, max) = sweep_candidate_bounds(aabb, displacement, config).map_err(fail)?;
    let capacity = compute.box_sweep_capacity().min(4096);
    if capacity == 0 {
        return Err(CudaCollisionError::Cuda(CudaError::BufferLimit));
    }
    let mut best = physics::SweepResult {
        fraction: 1.0,
        normal: [0; 3],
        obstacle: None,
    };
    let mut inputs = Vec::new();
    let mut obstacles = Vec::new();
    for x in min[0]..=max[0] {
        for y in min[1]..=max[1] {
            for z in min[2]..=max[2] {
                let pos = voxy_core::VoxelPos {
                    x: aabb
                        .anchor
                        .x
                        .checked_add(x)
                        .ok_or_else(|| fail(SweepError::CoordinateOverflow))?,
                    y: aabb
                        .anchor
                        .y
                        .checked_add(y)
                        .ok_or_else(|| fail(SweepError::CoordinateOverflow))?,
                    z: aabb
                        .anchor
                        .z
                        .checked_add(z)
                        .ok_or_else(|| fail(SweepError::CoordinateOverflow))?,
                };
                let sample = snapshot
                    .retained_block(pos)
                    .map_or_else(|| view.sample(pos), Sample::Loaded);
                let obstacle = match sample {
                    Sample::Loaded(block) => {
                        let definition = registry
                            .get(block)
                            .ok_or_else(|| fail(SweepError::UnknownBlock(block)))?;
                        if definition.collision == CollisionShape::Empty {
                            continue;
                        }
                        SweepObstacle::Block { pos, block }
                    }
                    Sample::Unloaded { chunk } => SweepObstacle::Unloaded { at: pos, chunk },
                    Sample::Unavailable { chunk, cause } => SweepObstacle::Unavailable {
                        at: pos,
                        chunk,
                        cause,
                    },
                };
                // These are bounded local offsets, never absolute world anchors.
                #[allow(clippy::cast_precision_loss)]
                let base = [x as f64, y as f64, z as f64];
                inputs.push(voxy_cuda::CudaBoxSweep {
                    min: aabb.min,
                    max: aabb.max,
                    displacement,
                    obstacle_min: base,
                    obstacle_max: base.map(|v| v + 1.0),
                });
                obstacles.push(obstacle);
                if inputs.len() == capacity {
                    merge_contacts(compute, &inputs, &obstacles, &mut best)?;
                    inputs.clear();
                    obstacles.clear();
                }
            }
        }
    }
    merge_contacts(compute, &inputs, &obstacles, &mut best)?;
    Ok(best)
}

fn merge_contacts(
    compute: &CudaCompute,
    inputs: &[voxy_cuda::CudaBoxSweep],
    obstacles: &[physics_voxel::SweepObstacle],
    best: &mut physics::SweepResult<physics_voxel::SweepObstacle>,
) -> Result<(), CudaCollisionError> {
    if inputs.is_empty() {
        return Ok(());
    }
    let contacts = compute
        .box_sweeps(inputs)
        .map_err(CudaCollisionError::Cuda)?;
    // Batches preserve canonical x/y/z order; ties retain the first voxel.
    for (contact, obstacle) in contacts.into_iter().zip(obstacles) {
        if let Some((fraction, normal)) = contact
            && (best.obstacle.is_none() || fraction.total_cmp(&best.fraction).is_lt())
        {
            *best = physics::SweepResult {
                fraction,
                normal,
                obstacle: Some(*obstacle),
            };
        }
    }
    Ok(())
}
