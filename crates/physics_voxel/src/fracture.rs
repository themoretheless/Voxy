//! Continuum fracture to voxel destruction: every released rigid piece
//! claims the solid voxels whose centres lie inside its convex hull, producing
//! one atomic removal transaction plus per-piece voxel inventories that
//! follow the piece as rigid debris.

use std::collections::BTreeSet;

use physics::plasticity::mesh::{DebrisRelease, hull_signed_distance};
use voxy_core::{VoxelPos, split_voxel};
use voxy_world::{BlockStateId, EditSource, EditTxn, Sample, VoxelView, VoxelWrite};

use crate::destruction::{DestructionError, DestructionPlan};

/// World placement of the continuum body: voxel `(i, j, k)` spans
/// `origin + size * [i, i + 1)` on each axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractureFrame {
    pub origin_m: [f64; 3],
    pub voxel_size_m: f64,
    /// Upper bound on removal writes across all pieces.
    pub max_writes: usize,
}

/// Voxels claimed by one released piece, in piece order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoxelChunkPiece {
    pub piece: usize,
    pub voxels: Vec<(VoxelPos, BlockStateId)>,
}

/// Outcome of voxelizing a fracture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FracturePlan {
    pub txn: EditTxn,
    pub pieces: Vec<VoxelChunkPiece>,
}

/// Plans removal of every solid voxel inside the released hulls. The planner
/// aborts before producing writes if any candidate voxel is missing, so the
/// transaction either removes all claimed voxels or none.
///
/// # Errors
/// Invalid frame, coordinate overflow, unavailable chunks, unknown blocks,
/// an inconsistent view, or a write budget overflow.
pub fn plan_fracture(
    view: &impl VoxelView,
    registry: &voxy_world::BlockRegistry,
    source: EditSource,
    release: &DebrisRelease,
    frame: FractureFrame,
) -> Result<Option<FracturePlan>, DestructionError> {
    if !frame.voxel_size_m.is_finite()
        || frame.voxel_size_m <= 0.
        || frame.origin_m.iter().any(|v| !v.is_finite())
        || frame.max_writes == 0
    {
        return Err(DestructionError::InvalidLimits);
    }
    let mut writes = Vec::new();
    let mut touched = BTreeSet::new();
    let mut claimed = BTreeSet::new();
    let mut pieces = Vec::with_capacity(release.pieces.len());
    for (index, piece) in release.pieces.iter().enumerate() {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &piece.hull {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(p[axis]);
                hi[axis] = hi[axis].max(p[axis]);
            }
        }
        // Cell coordinates of the hull bounds; the end bound is exclusive.
        let bound = |cells: f64, round_up: bool| -> Result<i64, DestructionError> {
            let cell = if round_up {
                cells.ceil()
            } else {
                cells.floor()
            };
            if !cell.is_finite() || cell.abs() > 1e15 {
                return Err(DestructionError::CoordinateOverflow);
            }
            Ok(cell as i64)
        };
        let mut range = [[0_i64; 2]; 3];
        for axis in 0..3 {
            let start = (lo[axis] - frame.origin_m[axis]) / frame.voxel_size_m;
            let end = (hi[axis] - frame.origin_m[axis]) / frame.voxel_size_m;
            range[axis] = [bound(start, false)?, bound(end, true)?];
        }
        let mut voxels = Vec::new();
        for x in range[0][0]..range[0][1] {
            for y in range[1][0]..range[1][1] {
                for z in range[2][0]..range[2][1] {
                    let center = [
                        frame.origin_m[0] + (x as f64 + 0.5) * frame.voxel_size_m,
                        frame.origin_m[1] + (y as f64 + 0.5) * frame.voxel_size_m,
                        frame.origin_m[2] + (z as f64 + 0.5) * frame.voxel_size_m,
                    ];
                    if hull_signed_distance(&piece.hull, &piece.hull_faces, center) > 0. {
                        continue;
                    }
                    let pos = VoxelPos { x, y, z };
                    if !claimed.insert(pos) {
                        continue;
                    }
                    match view.sample(pos) {
                        Sample::Loaded(BlockStateId::AIR) => {}
                        Sample::Loaded(block) => {
                            registry
                                .get(block)
                                .ok_or(DestructionError::UnknownBlock(block))?;
                            if writes.len() == frame.max_writes {
                                return Err(DestructionError::WriteBudgetExceeded {
                                    limit: frame.max_writes,
                                });
                            }
                            writes.push(VoxelWrite {
                                pos,
                                block: BlockStateId::AIR,
                            });
                            touched.insert(split_voxel(pos).0);
                            voxels.push((pos, block));
                        }
                        Sample::Unloaded { chunk } => {
                            return Err(DestructionError::Unloaded { at: pos, chunk });
                        }
                        Sample::Unavailable { chunk, cause } => {
                            return Err(DestructionError::Unavailable {
                                at: pos,
                                chunk,
                                cause,
                            });
                        }
                    }
                }
            }
        }
        pieces.push(VoxelChunkPiece {
            piece: index,
            voxels,
        });
    }
    if writes.is_empty() {
        return Ok(None);
    }
    let expected = touched
        .into_iter()
        .map(|pos| {
            view.chunk(pos)
                .map(|snapshot| (pos, snapshot.revision))
                .ok_or(DestructionError::InconsistentView(pos))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(FracturePlan {
        txn: EditTxn {
            source,
            expected,
            writes,
        },
        pieces,
    }))
}

impl FracturePlan {
    /// The removal transaction as a generic destruction plan.
    #[must_use]
    pub fn into_destruction_plan(self) -> DestructionPlan {
        DestructionPlan::Transaction(self.txn)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use physics::plasticity::Material;
    use physics::plasticity::mesh::QuadraticBody;
    use voxy_core::{ChunkPos, WorldEpoch};
    use voxy_world::{ChunkData, GeneratedChunk, PalettedBlocks, ResourceKey, World, WorldLimits};

    use super::*;

    fn stone_world() -> (World, Arc<voxy_world::BlockRegistry>, BlockStateId) {
        let registry = Arc::new(crate::test_support::test_registry());
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let mut world = World::new(
            WorldEpoch::new(1).unwrap(),
            Arc::clone(&registry),
            WorldLimits::default(),
        );
        world
            .insert_generated(GeneratedChunk {
                pos: ChunkPos { x: 0, y: 0, z: 0 },
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(stone),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        (world, registry, stone)
    }

    /// Two unit tetrahedra sharing a face at z = 0, already disconnected.
    fn release() -> DebrisRelease {
        let material = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
        let body = QuadraticBody::from_linear(
            vec![
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., -1.],
            ],
            vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
        )
        .unwrap();
        let velocities = vec![[0.; 3]; body.positions().len()];
        body.release_debris(&[1000., 1000.], &velocities, 0.)
            .unwrap()
    }

    fn hull_volume(hull: &[[f64; 3]], faces: &[[usize; 3]]) -> f64 {
        faces
            .iter()
            .map(|&[a, b, c]| {
                let (p, q, r) = (hull[a], hull[b], hull[c]);
                let cross = [
                    q[1] * r[2] - q[2] * r[1],
                    q[2] * r[0] - q[0] * r[2],
                    q[0] * r[1] - q[1] * r[0],
                ];
                (p[0] * cross[0] + p[1] * cross[1] + p[2] * cross[2]) / 6.
            })
            .sum()
    }

    #[test]
    fn pieces_claim_the_voxels_inside_their_hulls_atomically() {
        let (mut world, registry, stone) = stone_world();
        let release = release();
        let frame = FractureFrame {
            // Body spans [-1, 1] m, so it maps to cells 0..20 inside chunk (0, 0, 0).
            origin_m: [-1., -1., -1.],
            voxel_size_m: 0.1,
            max_writes: 100_000,
        };
        let plan = plan_fracture(&world, &registry, EditSource::Simulation, &release, frame)
            .unwrap()
            .expect("solid voxels inside the hulls");
        assert_eq!(plan.pieces.len(), 2);
        let total: usize = plan.pieces.iter().map(|p| p.voxels.len()).sum();
        assert_eq!(plan.txn.writes.len(), total);
        assert_eq!(plan.txn.expected.len(), 1);
        for (piece, inventory) in release.pieces.iter().zip(&plan.pieces) {
            let volume = hull_volume(&piece.hull, &piece.hull_faces);
            assert!((volume - 1. / 6.).abs() < 1e-9);
            let claimed = inventory.voxels.len() as f64 * frame.voxel_size_m.powi(3);
            // Within one voxel layer of the hull surface (≈ 2.4 m² × 0.1 m).
            assert!(
                (claimed - volume).abs() < 2.4 * frame.voxel_size_m,
                "{claimed} vs {volume}"
            );
            assert!(inventory.voxels.iter().all(|(_, block)| *block == stone));
            // Every claimed voxel centre is inside the hull.
            for (pos, _) in &inventory.voxels {
                let center = [
                    frame.origin_m[0] + (pos.x as f64 + 0.5) * frame.voxel_size_m,
                    frame.origin_m[1] + (pos.y as f64 + 0.5) * frame.voxel_size_m,
                    frame.origin_m[2] + (pos.z as f64 + 0.5) * frame.voxel_size_m,
                ];
                assert!(hull_signed_distance(&piece.hull, &piece.hull_faces, center) <= 0.);
            }
        }
        // No voxel is claimed twice.
        let mut all: Vec<VoxelPos> = plan
            .pieces
            .iter()
            .flat_map(|p| p.voxels.iter().map(|v| v.0))
            .collect();
        let before = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), before);
        // The transaction commits atomically and removes exactly the claimed voxels.
        let receipt = world.commit(plan.txn.clone()).unwrap();
        assert_eq!(receipt.inverse.writes.len(), total);
        for (pos, _) in plan.pieces.iter().flat_map(|p| &p.voxels) {
            assert_eq!(world.sample(*pos), Sample::Loaded(BlockStateId::AIR));
        }
        // A second planning pass over air yields nothing.
        assert!(
            plan_fracture(&world, &registry, EditSource::Simulation, &release, frame)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn budgets_and_unloaded_space_abort_before_writes() {
        let (world, registry, _) = stone_world();
        let release = release();
        let tight = FractureFrame {
            origin_m: [-1., -1., -1.],
            voxel_size_m: 0.1,
            max_writes: 5,
        };
        assert!(matches!(
            plan_fracture(&world, &registry, EditSource::Simulation, &release, tight),
            Err(DestructionError::WriteBudgetExceeded { limit: 5 })
        ));
        let outside = FractureFrame {
            origin_m: [-100., -10., -10.],
            voxel_size_m: 0.1,
            max_writes: 100_000,
        };
        assert!(matches!(
            plan_fracture(&world, &registry, EditSource::Simulation, &release, outside),
            Err(DestructionError::Unloaded { .. })
        ));
        let bad = FractureFrame {
            origin_m: [0.; 3],
            voxel_size_m: 0.,
            max_writes: 1,
        };
        assert_eq!(
            plan_fracture(&world, &registry, EditSource::Simulation, &release, bad).err(),
            Some(DestructionError::InvalidLimits)
        );
    }
}
