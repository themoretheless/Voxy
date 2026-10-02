//! Immutable dense collision classification captured from chunk snapshots.
use crate::{PendingVoxelRegions, VoxelClass, VoxelRegion, VoxelRegionProgram};
use std::{collections::BTreeMap, sync::Arc};
use voxy_core::{ChunkPos, VoxelPos, split_voxel};
use voxy_render::ComputeError;
use voxy_world::{BlockRegistry, ChunkSnapshot, CollisionShape, VoxelView};

/// Keeps integer world coordinates and chunk identities beside the GPU input.
#[derive(Debug)]
pub struct VoxelRegionSnapshot {
    anchor: VoxelPos,
    dimensions: [u32; 3],
    cells: Vec<VoxelClass>,
    chunks: BTreeMap<ChunkPos, Option<ChunkSnapshot>>,
}
impl VoxelRegionSnapshot {
    /// Captures each chunk once. Missing chunks and unknown block IDs remain faults.
    /// # Errors
    /// Rejects invalid dimensions, excessive grids, coordinate overflow or mismatched chunks.
    pub fn capture(
        view: &impl VoxelView,
        registry: &BlockRegistry,
        anchor: VoxelPos,
        dimensions: [u32; 3],
    ) -> Result<Self, ComputeError> {
        let count = dimensions
            .iter()
            .try_fold(1_u32, |n, &d| {
                if d == 0 || d > 512 {
                    None
                } else {
                    n.checked_mul(d)
                }
            })
            .filter(|&n| n <= 131_072)
            .ok_or(ComputeError::InvalidBuffer)?;
        // Check the complete extent before consulting the world.
        world_position(anchor, dimensions.map(|d| d - 1)).ok_or(ComputeError::InvalidBuffer)?;
        let mut chunks = BTreeMap::new();
        let mut cells =
            Vec::with_capacity(usize::try_from(count).map_err(|_| ComputeError::InvalidBuffer)?);
        for x in 0..dimensions[0] {
            for y in 0..dimensions[1] {
                for z in 0..dimensions[2] {
                    let pos =
                        world_position(anchor, [x, y, z]).ok_or(ComputeError::InvalidBuffer)?;
                    let (chunk, local) = split_voxel(pos);
                    let snapshot = chunks.entry(chunk).or_insert_with(|| view.chunk(chunk));
                    let class = if let Some(snapshot) = snapshot {
                        if snapshot.pos != chunk {
                            return Err(ComputeError::InvalidBuffer);
                        }
                        match registry.get(snapshot.data.blocks.get(local.index())) {
                            Some(def) => match def.collision {
                                CollisionShape::Empty => VoxelClass::Empty,
                                CollisionShape::FullCube => VoxelClass::Solid,
                            },
                            None => VoxelClass::Unknown,
                        }
                    } else {
                        VoxelClass::Unavailable
                    };
                    cells.push(class);
                }
            }
        }
        Ok(Self {
            anchor,
            dimensions,
            cells,
            chunks,
        })
    }

    /// Exact chunk footprint consulted by this request, including missing chunks.
    /// Streaming callers must retain these chunks until collision publication completes.
    pub fn captured_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.keys().copied()
    }

    /// Returns block identity from the retained chunk, without another world read.
    /// Missing chunks require a live sample to preserve unavailable fault details.
    pub(crate) fn retained_block(&self, pos: VoxelPos) -> Option<voxy_world::BlockStateId> {
        let (chunk, local) = split_voxel(pos);
        self.chunks
            .get(&chunk)?
            .as_ref()
            .map(|snapshot| snapshot.data.blocks.get(local.index()))
    }

    /// Checks original revisions and retained data identities before accepting a result.
    /// Missing-to-loaded transitions also invalidate the snapshot. Callers must prevent
    /// concurrent world edits between this check and collision publication.
    #[must_use]
    pub fn is_current(&self, view: &impl VoxelView) -> bool {
        self.chunks
            .iter()
            .all(|(&pos, original)| match (original, view.chunk(pos)) {
                (None, None) => true,
                (Some(old), Some(new)) => {
                    new.pos == pos
                        && old.revision == new.revision
                        && Arc::ptr_eq(&old.data, &new.data)
                }
                _ => false,
            })
    }

    /// Converts a GPU candidate index back to exact world coordinates.
    #[must_use]
    pub fn position(&self, index: u32) -> Option<VoxelPos> {
        if usize::try_from(index).ok()? >= self.cells.len() {
            return None;
        }
        world_position(
            self.anchor,
            [
                index / (self.dimensions[1] * self.dimensions[2]),
                index / self.dimensions[2] % self.dimensions[1],
                index % self.dimensions[2],
            ],
        )
    }

    /// Runs CUDA classification against this exact retained world snapshot.
    /// Publication still requires `is_current`; this API performs synchronous readback.
    /// # Errors
    /// Returns CUDA input, budget, compilation or device errors without CPU fallback.
    pub fn classify_cuda(
        &self,
        compute: &voxy_cuda::CudaCompute,
        regions: &[VoxelRegion],
    ) -> Result<Vec<crate::VoxelRegionResult>, voxy_cuda::CudaError> {
        if regions.is_empty() || regions.len() > 16_384 {
            return Err(voxy_cuda::CudaError::InvalidVoxelInput);
        }
        let cells: Vec<u32> = self
            .cells
            .iter()
            .map(|class| match class {
                VoxelClass::Empty => 0,
                VoxelClass::Solid => 1,
                VoxelClass::Unavailable => 2,
                VoxelClass::Unknown => 3,
            })
            .collect();
        let queries: Vec<[u32; 6]> = regions
            .iter()
            .map(|r| [r.min[0], r.min[1], r.min[2], r.max[0], r.max[1], r.max[2]])
            .collect();
        compute
            .voxel_regions(self.dimensions, &cells, &queries)
            .map(|results| {
                results
                    .into_iter()
                    .map(|r| crate::VoxelRegionResult {
                        solid_count: r[0],
                        first_solid: (r[1] != u32::MAX).then_some(r[1]),
                        first_fault: (r[2] != u32::MAX).then_some(r[2]),
                    })
                    .collect()
            })
    }

    /// Submits the frozen classification without waiting. Retain this snapshot
    /// until result publication and check `is_current` against the original world.
    /// # Errors
    /// Returns invalid query or GPU submission errors.
    pub fn begin_classify(
        &self,
        program: &VoxelRegionProgram,
        queue: &wgpu::Queue,
        regions: &[VoxelRegion],
    ) -> Result<PendingVoxelRegions, ComputeError> {
        program.begin_classify(queue, self.dimensions, &self.cells, regions)
    }
}
fn world_position(anchor: VoxelPos, local: [u32; 3]) -> Option<VoxelPos> {
    Some(VoxelPos {
        x: anchor.x.checked_add(i64::from(local[0]))?,
        y: anchor.y.checked_add(i64::from(local[1]))?,
        z: anchor.z.checked_add(i64::from(local[2]))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use voxy_world::{BlockStateId, ChunkData, ChunkRevision, Sample};

    struct View {
        chunk: Option<ChunkSnapshot>,
        calls: Cell<usize>,
    }
    impl VoxelView for View {
        fn sample(&self, _: VoxelPos) -> Sample<BlockStateId> {
            panic!("capture must use retained chunks")
        }
        fn chunk(&self, pos: ChunkPos) -> Option<ChunkSnapshot> {
            self.calls.set(self.calls.get() + 1);
            self.chunk.as_ref().filter(|s| s.pos == pos).cloned()
        }
    }
    #[test]
    fn footprint_includes_missing_chunks_across_negative_boundaries() {
        let scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let view = View {
            chunk: Some(ChunkSnapshot {
                pos: ChunkPos { x: -1, y: 0, z: 0 },
                revision: ChunkRevision::from_raw(1),
                data: Arc::new(ChunkData::uniform(BlockStateId::AIR)),
            }),
            calls: Cell::new(0),
        };
        let snapshot = VoxelRegionSnapshot::capture(
            &view,
            scene.world.registry(),
            VoxelPos { x: -1, y: 0, z: 0 },
            [2, 1, 1],
        )
        .unwrap();
        let before = view.calls.get();
        assert_eq!(
            snapshot.captured_chunks().collect::<Vec<_>>(),
            vec![
                ChunkPos { x: -1, y: 0, z: 0 },
                ChunkPos { x: 0, y: 0, z: 0 },
            ]
        );
        assert_eq!(view.calls.get(), before);
        assert!(matches!(snapshot.cells[0], VoxelClass::Empty));
        assert!(matches!(snapshot.cells[1], VoxelClass::Unavailable));
    }

    #[test]
    fn frozen_grid_preserves_far_coordinates_and_rejects_replacements() {
        let scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let anchor = VoxelPos {
            x: 9_007_199_254_740_993,
            y: -2,
            z: -3,
        };
        let chunk = split_voxel(anchor).0;
        let mut view = View {
            chunk: Some(ChunkSnapshot {
                pos: chunk,
                revision: ChunkRevision::from_raw(7),
                data: Arc::new(ChunkData::uniform(BlockStateId::AIR)),
            }),
            calls: Cell::new(0),
        };
        let snapshot =
            VoxelRegionSnapshot::capture(&view, scene.world.registry(), anchor, [2; 3]).unwrap();
        assert_eq!(view.calls.get(), 1);
        assert_eq!(snapshot.retained_block(anchor), Some(BlockStateId::AIR));
        assert_eq!(view.calls.get(), 1);
        assert!(
            snapshot
                .cells
                .iter()
                .all(|c| matches!(c, VoxelClass::Empty))
        );
        assert_eq!(
            snapshot.position(7),
            Some(VoxelPos {
                x: anchor.x + 1,
                y: -1,
                z: -2
            })
        );
        assert_eq!(snapshot.position(8), None);
        assert!(snapshot.is_current(&view));
        view.chunk.as_mut().unwrap().revision = ChunkRevision::from_raw(8);
        assert!(!snapshot.is_current(&view));
        view.chunk.as_mut().unwrap().revision = ChunkRevision::from_raw(7);
        view.chunk.as_mut().unwrap().data = Arc::new(ChunkData::uniform(BlockStateId::AIR));
        assert!(!snapshot.is_current(&view));
        view.chunk = None;
        assert!(!snapshot.is_current(&view));
        assert_eq!(snapshot.retained_block(anchor), Some(BlockStateId::AIR));
        let missing =
            VoxelRegionSnapshot::capture(&view, scene.world.registry(), anchor, [1; 3]).unwrap();
        assert!(matches!(missing.cells[0], VoxelClass::Unavailable));
        assert_eq!(missing.retained_block(anchor), None);
        assert!(missing.is_current(&view));
        view.chunk = Some(ChunkSnapshot {
            pos: chunk,
            revision: ChunkRevision::from_raw(7),
            data: Arc::new(ChunkData::uniform(BlockStateId::AIR)),
        });
        assert!(!missing.is_current(&view));
        let before = view.calls.get();
        assert!(
            VoxelRegionSnapshot::capture(
                &view,
                scene.world.registry(),
                VoxelPos {
                    x: i64::MAX,
                    y: 0,
                    z: 0
                },
                [2; 3]
            )
            .is_err()
        );
        assert_eq!(view.calls.get(), before);
    }
}
