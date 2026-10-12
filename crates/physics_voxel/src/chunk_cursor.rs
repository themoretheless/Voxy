//! One-entry chunk cache for voxel queries that walk neighbouring cells.

use voxy_core::{ChunkPos, VoxelPos, split_voxel};
use voxy_world::{BlockStateId, ChunkSnapshot, Sample, VoxelView};

/// Samples a [`VoxelView`] while remembering the last chunk it touched.
///
/// Rays and sweeps visit runs of cells inside one `32³` chunk, so resolving the
/// chunk once and reading its snapshot directly replaces one map lookup per cell.
/// The fast path relies on [`VoxelView::chunk`] agreeing with [`VoxelView::sample`]
/// for loaded chunks, which `voxy_world::World` guarantees; a view that returns
/// `None` from `chunk` falls back to `sample` for every cell, so unloaded and
/// unavailable results are reported exactly as before.
pub(crate) struct ChunkCursor<'a, V: VoxelView + ?Sized> {
    view: &'a V,
    cached: Option<(ChunkPos, Option<ChunkSnapshot>)>,
}

impl<'a, V: VoxelView + ?Sized> ChunkCursor<'a, V> {
    pub(crate) fn new(view: &'a V) -> Self {
        Self { view, cached: None }
    }

    pub(crate) fn sample(&mut self, pos: VoxelPos) -> Sample<BlockStateId> {
        let (chunk, local) = split_voxel(pos);
        let snapshot = match &self.cached {
            Some((cached, snapshot)) if *cached == chunk => snapshot.as_ref(),
            _ => {
                self.cached = Some((chunk, self.view.chunk(chunk)));
                self.cached
                    .as_ref()
                    .and_then(|(_, snapshot)| snapshot.as_ref())
            }
        };
        match snapshot {
            Some(snapshot) => Sample::Loaded(snapshot.data.blocks.get(local.index())),
            None => self.view.sample(pos),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_core::ChunkPos;
    use voxy_world::{ChunkData, ChunkRevision};

    struct TwoChunks {
        loaded: ChunkSnapshot,
    }

    impl VoxelView for TwoChunks {
        fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
            let (chunk, local) = split_voxel(pos);
            if chunk == self.loaded.pos {
                Sample::Loaded(self.loaded.data.blocks.get(local.index()))
            } else {
                Sample::Unloaded { chunk }
            }
        }
        fn chunk(&self, pos: ChunkPos) -> Option<ChunkSnapshot> {
            (pos == self.loaded.pos).then(|| self.loaded.clone())
        }
    }

    #[test]
    fn cursor_matches_direct_sampling_across_chunk_edges() {
        let view = TwoChunks {
            loaded: ChunkSnapshot {
                pos: ChunkPos { x: 0, y: 0, z: 0 },
                revision: ChunkRevision::default(),
                data: std::sync::Arc::new(ChunkData::uniform(BlockStateId::AIR)),
            },
        };
        let mut cursor = ChunkCursor::new(&view);
        for x in -3..35 {
            let pos = VoxelPos { x, y: 1, z: 2 };
            assert_eq!(cursor.sample(pos), view.sample(pos), "{pos:?}");
        }
    }
}
