//! 26-neighbor chunk halo indexing and revision-stamped meshing input.

use std::sync::Arc;

use voxy_core::{ChunkPos, LocalPos};
use voxy_world::{BlockRegistry, BlockStateId, ChunkRevision, ChunkSnapshot, Sample};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HaloOffset {
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

pub const HALO_OFFSETS: [HaloOffset; 26] = [
    HaloOffset {
        dx: -1,
        dy: -1,
        dz: -1,
    },
    HaloOffset {
        dx: 0,
        dy: -1,
        dz: -1,
    },
    HaloOffset {
        dx: 1,
        dy: -1,
        dz: -1,
    },
    HaloOffset {
        dx: -1,
        dy: -1,
        dz: 0,
    },
    HaloOffset {
        dx: 0,
        dy: -1,
        dz: 0,
    },
    HaloOffset {
        dx: 1,
        dy: -1,
        dz: 0,
    },
    HaloOffset {
        dx: -1,
        dy: -1,
        dz: 1,
    },
    HaloOffset {
        dx: 0,
        dy: -1,
        dz: 1,
    },
    HaloOffset {
        dx: 1,
        dy: -1,
        dz: 1,
    },
    HaloOffset {
        dx: -1,
        dy: 0,
        dz: -1,
    },
    HaloOffset {
        dx: 0,
        dy: 0,
        dz: -1,
    },
    HaloOffset {
        dx: 1,
        dy: 0,
        dz: -1,
    },
    HaloOffset {
        dx: -1,
        dy: 0,
        dz: 0,
    },
    HaloOffset {
        dx: 1,
        dy: 0,
        dz: 0,
    },
    HaloOffset {
        dx: -1,
        dy: 0,
        dz: 1,
    },
    HaloOffset {
        dx: 0,
        dy: 0,
        dz: 1,
    },
    HaloOffset {
        dx: 1,
        dy: 0,
        dz: 1,
    },
    HaloOffset {
        dx: -1,
        dy: 1,
        dz: -1,
    },
    HaloOffset {
        dx: 0,
        dy: 1,
        dz: -1,
    },
    HaloOffset {
        dx: 1,
        dy: 1,
        dz: -1,
    },
    HaloOffset {
        dx: -1,
        dy: 1,
        dz: 0,
    },
    HaloOffset {
        dx: 0,
        dy: 1,
        dz: 0,
    },
    HaloOffset {
        dx: 1,
        dy: 1,
        dz: 0,
    },
    HaloOffset {
        dx: -1,
        dy: 1,
        dz: 1,
    },
    HaloOffset {
        dx: 0,
        dy: 1,
        dz: 1,
    },
    HaloOffset {
        dx: 1,
        dy: 1,
        dz: 1,
    },
];

impl HaloOffset {
    /// Position in [`HALO_OFFSETS`]: `dx` varies fastest, then `dz`, then `dy`, with the
    /// center excluded. Pinned by `halo_order_is_unique_and_canonical`.
    #[inline]
    #[must_use]
    pub const fn index(self) -> Option<usize> {
        if self.dx < -1 || self.dx > 1 || self.dy < -1 || self.dy > 1 || self.dz < -1 || self.dz > 1
        {
            return None;
        }
        let raw =
            (self.dx + 1) as usize + 3 * (self.dz + 1) as usize + 9 * (self.dy + 1) as usize;
        if raw < 13 {
            Some(raw)
        } else if raw == 13 {
            None
        } else {
            Some(raw - 1)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshStamp {
    pub center: ChunkRevision,
    pub halo: [Option<ChunkRevision>; 26],
    pub registry_epoch: u64,
    pub mesher_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct MeshingInput {
    pub center: ChunkSnapshot,
    pub neighbors: [Option<ChunkSnapshot>; 26],
    pub registry: Arc<BlockRegistry>,
    pub stamp: MeshStamp,
}

#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn sample_relative(input: &MeshingInput, x: i16, y: i16, z: i16) -> Sample<BlockStateId> {
    if (0..32).contains(&x) && (0..32).contains(&y) && (0..32).contains(&z) {
        let local = LocalPos::new(x as u8, y as u8, z as u8).expect("range checked");
        return Sample::Loaded(input.center.data.blocks.get(local.index()));
    }
    let offset = HaloOffset {
        dx: x.div_euclid(32) as i8,
        dy: y.div_euclid(32) as i8,
        dz: z.div_euclid(32) as i8,
    };
    let chunk = ChunkPos {
        x: input.center.pos.x.saturating_add(i64::from(offset.dx)),
        y: input.center.pos.y.saturating_add(i64::from(offset.dy)),
        z: input.center.pos.z.saturating_add(i64::from(offset.dz)),
    };
    let Some(index) = offset.index() else {
        return Sample::Unloaded { chunk };
    };
    let Some(neighbor) = &input.neighbors[index] else {
        return Sample::Unloaded { chunk };
    };
    let local = LocalPos::new(
        x.rem_euclid(32) as u8,
        y.rem_euclid(32) as u8,
        z.rem_euclid(32) as u8,
    )
    .expect("remainder is local");
    Sample::Loaded(neighbor.data.blocks.get(local.index()))
}
