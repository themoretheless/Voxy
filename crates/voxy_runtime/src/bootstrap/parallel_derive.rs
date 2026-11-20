use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use voxy_core::{CancelToken, ChunkPos};
use voxy_lighting::{
    LightStamp, LightingBudget, LightingInput, LightingScratch, build_light_with_scratch,
};
use voxy_mesher::{HALO_OFFSETS, MeshStamp, MeshingInput, build_mesh};
use voxy_world::{BlockRegistry, ChunkRevision, ChunkSnapshot, GenerationError, VoxelView};

use super::{BootstrapChunk, BootstrapError};

const fn find_halo_index(dx: i8, dy: i8, dz: i8) -> usize {
    let mut i = 0;
    while i < HALO_OFFSETS.len() {
        let off = HALO_OFFSETS[i];
        if off.dx == dx && off.dy == dy && off.dz == dz {
            return i;
        }
        i += 1;
    }
    panic!("HALO_OFFSETS must contain all six face offsets")
}

const FACE_HALO_INDICES: [usize; 6] = [
    find_halo_index(-1, 0, 0),
    find_halo_index(1, 0, 0),
    find_halo_index(0, -1, 0),
    find_halo_index(0, 1, 0),
    find_halo_index(0, 0, -1),
    find_halo_index(0, 0, 1),
];

#[derive(Clone, Copy)]
pub(super) struct DeriveContext<'a> {
    pub(super) registry: &'a Arc<BlockRegistry>,
    pub(super) derivation_epoch: u64,
    pub(super) token: &'a CancelToken,
}

fn halo_position(pos: ChunkPos, index: usize) -> ChunkPos {
    let offset = HALO_OFFSETS[index];
    ChunkPos {
        x: pos.x + i64::from(offset.dx),
        y: pos.y + i64::from(offset.dy),
        z: pos.z + i64::from(offset.dz),
    }
}

fn halo_snapshots(
    pos: ChunkPos,
    fetch: &impl Fn(ChunkPos) -> Option<ChunkSnapshot>,
) -> [Option<ChunkSnapshot>; 26] {
    std::array::from_fn(|index| fetch(halo_position(pos, index)))
}

fn mesh_stamp_from_halo(
    center: ChunkRevision,
    halo: &[Option<ChunkSnapshot>; 26],
    epoch: u64,
) -> MeshStamp {
    MeshStamp {
        center,
        halo: halo
            .each_ref()
            .map(|snapshot| snapshot.as_ref().map(|snap| snap.revision)),
        registry_epoch: 1,
        mesher_epoch: epoch,
    }
}

fn derive_chunk_with_halo(
    pos: ChunkPos,
    center: ChunkSnapshot,
    neighbors: [Option<ChunkSnapshot>; 26],
    stamp: MeshStamp,
    ctx: DeriveContext<'_>,
    light_scratch: &mut LightingScratch,
) -> Result<BootstrapChunk, BootstrapError> {
    let light_neighbors = FACE_HALO_INDICES.map(|idx| neighbors[idx].clone());
    let light_faces = light_neighbors
        .each_ref()
        .map(|snapshot| snapshot.as_ref().map(|snap| snap.revision));
    let light = build_light_with_scratch(
        &LightingInput {
            center: center.clone(),
            neighbors: light_neighbors,
            registry: Arc::clone(ctx.registry),
            sky_from_above: true,
            stamp: LightStamp {
                center: center.revision,
                faces: light_faces,
                registry_epoch: 1,
                lighting_epoch: ctx.derivation_epoch,
            },
        },
        LightingBudget::default(),
        ctx.token,
        light_scratch,
    )?;
    let mesh = build_mesh(
        &MeshingInput {
            center,
            neighbors,
            registry: Arc::clone(ctx.registry),
            stamp: stamp.clone(),
        },
        ctx.token,
    )
    .map_err(BootstrapError::Mesh)?;
    Ok(BootstrapChunk {
        pos,
        mesh,
        light,
        local_mesh_stamp: Some(stamp),
    })
}

fn map_chunks_parallel<T, F>(
    positions: &[ChunkPos],
    token: &CancelToken,
    op: F,
) -> Result<Vec<T>, BootstrapError>
where
    T: Send,
    F: Fn(ChunkPos, &mut LightingScratch) -> Result<T, BootstrapError> + Sync,
{
    if token.is_cancelled() {
        return Err(GenerationError::Cancelled.into());
    }
    if positions.is_empty() {
        return Ok(Vec::new());
    }
    let workers = std::thread::available_parallelism()
        .map_or(4, std::num::NonZeroUsize::get)
        .min(positions.len());
    if positions.len() <= 2 || workers <= 1 {
        let mut scratch = LightingScratch::new();
        let mut out = Vec::with_capacity(positions.len());
        for &pos in positions {
            if token.is_cancelled() {
                return Err(GenerationError::Cancelled.into());
            }
            out.push(op(pos, &mut scratch)?);
        }
        return Ok(out);
    }

    let next_idx = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let mut slots: Vec<Option<Result<T, BootstrapError>>> =
        (0..positions.len()).map(|_| None).collect();
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let next_ref = &next_idx;
            let stopped_ref = &stopped;
            let op_ref = &op;
            handles.push(scope.spawn(move || {
                let mut scratch = LightingScratch::new();
                let mut local = Vec::new();
                while !stopped_ref.load(Ordering::Relaxed) {
                    if token.is_cancelled() {
                        stopped_ref.store(true, Ordering::Relaxed);
                        break;
                    }
                    let idx = next_ref.fetch_add(1, Ordering::Relaxed);
                    if idx >= positions.len() {
                        break;
                    }
                    let res = op_ref(positions[idx], &mut scratch);
                    if res.is_err() {
                        stopped_ref.store(true, Ordering::Relaxed);
                    }
                    local.push((idx, res));
                }
                local
            }));
        }
        for handle in handles {
            let local = handle.join().expect("bootstrap chunk derivation panicked");
            for (idx, res) in local {
                slots[idx] = Some(res);
            }
        }
    });

    let mut out = Vec::with_capacity(positions.len());
    for slot in slots {
        match slot {
            Some(res) => out.push(res?),
            None if token.is_cancelled() => return Err(GenerationError::Cancelled.into()),
            None => return Err(BootstrapError::MissingGeneratedChunk),
        }
    }
    Ok(out)
}

pub(super) fn derive_positions_parallel<F>(
    positions: &[ChunkPos],
    ctx: DeriveContext<'_>,
    fetch: F,
) -> Result<Vec<BootstrapChunk>, BootstrapError>
where
    F: Fn(ChunkPos) -> Option<ChunkSnapshot> + Sync,
{
    let mut chunks = map_chunks_parallel(positions, ctx.token, |pos, scratch| {
        let center = fetch(pos).ok_or(BootstrapError::MissingGeneratedChunk)?;
        let neighbors = halo_snapshots(pos, &fetch);
        let stamp = mesh_stamp_from_halo(center.revision, &neighbors, ctx.derivation_epoch);
        derive_chunk_with_halo(pos, center, neighbors, stamp, ctx, scratch)
    })?;
    chunks.sort_by_key(|chunk| chunk.pos);
    Ok(chunks)
}

pub(super) fn derive_window_reusing(
    visible: &[ChunkPos],
    existing: &BTreeMap<ChunkPos, BootstrapChunk>,
    world: &voxy_world::World,
    ctx: DeriveContext<'_>,
) -> Result<(Vec<BootstrapChunk>, usize), BootstrapError> {
    let entries = map_chunks_parallel(visible, ctx.token, |pos, scratch| {
        let center = world
            .chunk(pos)
            .ok_or(BootstrapError::MissingGeneratedChunk)?;
        let neighbors = halo_snapshots(pos, &|neighbor| world.chunk(neighbor));
        let stamp = mesh_stamp_from_halo(center.revision, &neighbors, ctx.derivation_epoch);
        if let Some(chunk) = existing.get(&pos)
            && let Some(previous) = &chunk.local_mesh_stamp
            && previous.center == stamp.center
            && previous.halo == stamp.halo
            && previous.registry_epoch == stamp.registry_epoch
        {
            let mut chunk = chunk.clone();
            chunk.local_mesh_stamp = Some(stamp);
            chunk.light.stamp.lighting_epoch = ctx.derivation_epoch;
            return Ok((chunk, true));
        }
        let chunk = derive_chunk_with_halo(pos, center, neighbors, stamp, ctx, scratch)?;
        Ok((chunk, false))
    })?;
    let mut chunks = Vec::with_capacity(entries.len());
    let mut reused_chunks = 0_usize;
    for (chunk, reused) in entries {
        reused_chunks += usize::from(reused);
        chunks.push(chunk);
    }
    chunks.sort_by_key(|chunk| chunk.pos);
    Ok((chunks, reused_chunks))
}
