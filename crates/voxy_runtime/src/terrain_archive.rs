//! Session-local backing store for edited chunks outside the resident terrain window.

use std::collections::BTreeMap;
use std::fs::{self, DirBuilder};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use voxy_core::ChunkPos;
use voxy_storage::{StoredChunk, decode_chunk, encode_chunk};
use voxy_world::{ChunkSnapshot, World};

use crate::BootstrapError;
use crate::bootstrap::terrain_window_positions;

#[derive(Debug, thiserror::Error)]
pub enum TerrainArchiveError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Codec(#[from] voxy_storage::CodecError),
    #[error(transparent)]
    Window(#[from] BootstrapError),
    #[error("terrain archive position mismatch at {0:?}")]
    PositionMismatch(ChunkPos),
    #[error("terrain archive revision mismatch at {0:?}")]
    RevisionMismatch(ChunkPos),
}

/// Stores edited chunks on disk and decodes only the requested terrain halo.
#[derive(Debug)]
pub struct TerrainArchive {
    directory: PathBuf,
}

impl TerrainArchive {
    /// Create an empty archive that is removed when its last owner drops it.
    ///
    /// # Errors
    /// Returns an error if the temporary directory cannot be created.
    pub fn new() -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "voxy-terrain-{}-{timestamp}-{serial}",
            std::process::id()
        ));
        let mut builder = DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        Ok(Self { directory })
    }

    /// Read archived edits in the requested window that are absent from the live world.
    ///
    /// # Errors
    /// Returns invalid window geometry, storage failures or invalid chunk data.
    pub fn load_window(
        &self,
        world: &World,
        center: ChunkPos,
        radius: i32,
    ) -> Result<BTreeMap<ChunkPos, ChunkSnapshot>, TerrainArchiveError> {
        let (_, halo) = terrain_window_positions(center, radius)?;
        let mut snapshots = BTreeMap::new();
        for pos in halo {
            let Some(revision) = world.unloaded_chunk_revision(pos) else {
                continue;
            };
            let bytes = fs::read(self.chunk_path(pos))?;
            let chunk = decode_chunk(&bytes, world.registry())?;
            if chunk.pos != pos {
                return Err(TerrainArchiveError::PositionMismatch(pos));
            }
            if chunk.revision != revision {
                return Err(TerrainArchiveError::RevisionMismatch(pos));
            }
            snapshots.insert(
                pos,
                ChunkSnapshot {
                    pos,
                    revision: chunk.revision,
                    data: Arc::new(chunk.data),
                },
            );
        }
        Ok(snapshots)
    }

    /// Save edits before publishing eviction. The live world remains authoritative
    /// for resident chunks even if a later write or publication fails.
    ///
    /// # Errors
    /// Returns a codec or filesystem failure.
    pub fn save_evictions(
        &self,
        world: &World,
        snapshots: &[ChunkSnapshot],
    ) -> Result<(), TerrainArchiveError> {
        for snapshot in snapshots {
            let chunk = StoredChunk {
                pos: snapshot.pos,
                revision: snapshot.revision,
                data: (*snapshot.data).clone(),
            };
            let bytes = encode_chunk(&chunk, world.registry())?;
            let path = self.chunk_path(snapshot.pos);
            let staging = path.with_extension("tmp");
            fs::write(&staging, bytes)?;
            fs::rename(staging, path)?;
        }
        Ok(())
    }

    fn chunk_path(&self, pos: ChunkPos) -> PathBuf {
        self.directory
            .join(format!("{}_{}_{}.chunk", pos.x, pos.y, pos.z))
    }
}

impl Drop for TerrainArchive {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_core::{CHUNK_EDGE, CancelToken, VoxelPos};
    use voxy_world::{
        EditSource, EditTxn, ProceduralTerrainGenerator, ResourceKey, VoxelView, VoxelWrite,
    };

    #[test]
    fn edited_chunks_round_trip_across_repeated_window_evictions() {
        let (scene, source) =
            crate::build_streamable_generated_scene(19, 0, true, |palette, water| {
                Ok(Box::new(ProceduralTerrainGenerator::new(palette, water)))
            })
            .unwrap();
        let mut world = scene.world;
        let archive = TerrainArchive::new().unwrap();
        let cancel = CancelToken::new();
        let stone = world
            .registry()
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let air = world
            .registry()
            .find(&ResourceKey::parse("voxy:air").unwrap())
            .unwrap();
        let mut expected = BTreeMap::new();
        for (epoch, x) in [0, 3, 6, 9, 6, 3, 0, 3, 6, 9, 6, 3, 0]
            .into_iter()
            .enumerate()
        {
            let center = ChunkPos { x, ..scene.anchor };
            let retained = archive.load_window(&world, center, 0).unwrap();
            let prepared = source
                .prepare_window(&world, &retained, center, 0, epoch as u64 + 1, &cancel)
                .unwrap();
            assert_eq!(prepared.world.resident_chunk_positions().count(), 27);
            if let Some((revision, data)) = expected.get(&center) {
                assert_eq!(prepared.restored, [center]);
                let restored = prepared.world.chunk(center).unwrap();
                assert!(restored.revision > *revision);
                assert_eq!(&restored.data, data);
            } else {
                assert!(prepared.restored.is_empty());
            }
            archive
                .save_evictions(&prepared.world, &prepared.evicted)
                .unwrap();
            world = prepared.world;
            let pos = VoxelPos {
                x: x * CHUNK_EDGE,
                y: 31,
                z: 0,
            };
            let (_, local) = voxy_core::split_voxel(pos);
            let before = world.chunk(center).unwrap();
            let block = if before.data.blocks.get(local.index()) == stone {
                air
            } else {
                stone
            };
            world
                .commit(EditTxn {
                    source: EditSource::Editor,
                    expected: vec![],
                    writes: vec![VoxelWrite { pos, block }],
                })
                .unwrap();
            let snapshot = world.chunk(center).unwrap();
            assert!(snapshot.revision > before.revision);
            expected.insert(center, (snapshot.revision, snapshot.data));
        }
    }
}
