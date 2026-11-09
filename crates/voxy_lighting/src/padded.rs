use voxy_core::{LocalIndex, PADDED_EDGE, PADDED_VOLUME};
use voxy_world::{BlockRegistry, BlockStateId, ChunkSnapshot, Occlusion, PalettedBlocks};

use crate::types::{EDGE, FaceNeighbor, LightingError};

/// Padded occlusion and emission of the center chunk plus its one-cell face halo.
pub(crate) struct PaddedCells {
    pub(crate) opaque: Vec<bool>,
    pub(crate) emission: Vec<u8>,
    pub(crate) has_emission: bool,
}

impl PaddedCells {
    pub(crate) fn new() -> Self {
        Self {
            opaque: vec![true; PADDED_VOLUME],
            emission: vec![0_u8; PADDED_VOLUME],
            has_emission: false,
        }
    }

    #[inline]
    pub(crate) fn fill(
        &mut self,
        registry: &BlockRegistry,
        block: BlockStateId,
        index: usize,
    ) -> Result<(), LightingError> {
        let definition = registry
            .get(block)
            .ok_or(LightingError::UnknownBlock(block.get()))?;
        self.opaque[index] = definition.occlusion == Occlusion::FullCube;
        self.emission[index] = definition.emission;
        if definition.emission > 0 {
            self.has_emission = true;
        }
        Ok(())
    }

    /// Fills the center cells by classifying each distinct block once and expanding the
    /// chunk's palette indices instead of resolving every cell through the registry.
    pub(crate) fn fill_center_paletted(
        &mut self,
        registry: &BlockRegistry,
        blocks: &PalettedBlocks,
    ) -> Result<(), LightingError> {
        let (palette, slots) = blocks.palette_indices();
        let mut classes = Vec::with_capacity(palette.len());
        let mut any_emission = false;
        for &block in &palette {
            let definition = registry
                .get(block)
                .ok_or(LightingError::UnknownBlock(block.get()))?;
            let blocked = definition.occlusion == Occlusion::FullCube;
            let emission = definition.emission;
            if emission > 0 {
                any_emission = true;
            }
            classes.push((blocked, emission));
        }
        for y in 0..EDGE {
            for z in 0..EDGE {
                let base_local = EDGE * (z + EDGE * y);
                let base_padded = ((y + 1) * PADDED_EDGE + (z + 1)) * PADDED_EDGE + 1;
                for x in 0..EDGE {
                    let (blocked, emission) = classes[usize::from(slots[base_local + x])];
                    self.opaque[base_padded + x] = blocked;
                    self.emission[base_padded + x] = emission;
                }
            }
        }
        if any_emission {
            self.has_emission = true;
        }
        Ok(())
    }

    /// Fills every center cell from one definition without expanding the chunk.
    pub(crate) fn fill_center_uniform(
        &mut self,
        registry: &BlockRegistry,
        block: BlockStateId,
    ) -> Result<(), LightingError> {
        let definition = registry
            .get(block)
            .ok_or(LightingError::UnknownBlock(block.get()))?;
        let blocked = definition.occlusion == Occlusion::FullCube;
        for y in 0..EDGE {
            for z in 0..EDGE {
                let start = ((y + 1) * PADDED_EDGE + (z + 1)) * PADDED_EDGE + 1;
                self.opaque[start..start + EDGE].fill(blocked);
                self.emission[start..start + EDGE].fill(definition.emission);
            }
        }
        if definition.emission > 0 {
            self.has_emission = true;
        }
        Ok(())
    }
}

/// Copies the neighbor's boundary slice into the matching halo face.
pub(crate) fn fill_face_halo(
    cells: &mut PaddedCells,
    registry: &BlockRegistry,
    neighbor: &ChunkSnapshot,
    face: FaceNeighbor,
) -> Result<(), LightingError> {
    let last = EDGE - 1;
    // (neighbor-local fixed coordinate, padded fixed coordinate) along the face axis.
    let (fixed_local, fixed_padded) = match face {
        FaceNeighbor::NegX | FaceNeighbor::NegY | FaceNeighbor::NegZ => (last, 0),
        FaceNeighbor::PosX | FaceNeighbor::PosY | FaceNeighbor::PosZ => (0, PADDED_EDGE - 1),
    };
    if let PalettedBlocks::Uniform(block) = &neighbor.data.blocks {
        let definition = registry
            .get(*block)
            .ok_or(LightingError::UnknownBlock(block.get()))?;
        let blocked = definition.occlusion == Occlusion::FullCube;
        let emission = definition.emission;
        if emission > 0 {
            cells.has_emission = true;
        }
        for a in 0..EDGE {
            for b in 0..EDGE {
                let (px, py, pz) = match face {
                    FaceNeighbor::NegX | FaceNeighbor::PosX => (fixed_padded, a + 1, b + 1),
                    FaceNeighbor::NegY | FaceNeighbor::PosY => (b + 1, fixed_padded, a + 1),
                    FaceNeighbor::NegZ | FaceNeighbor::PosZ => (b + 1, a + 1, fixed_padded),
                };
                let padded_idx = (py * PADDED_EDGE + pz) * PADDED_EDGE + px;
                cells.opaque[padded_idx] = blocked;
                cells.emission[padded_idx] = emission;
            }
        }
        return Ok(());
    }
    for a in 0..EDGE {
        for b in 0..EDGE {
            let (x, y, z) = match face {
                FaceNeighbor::NegX | FaceNeighbor::PosX => (fixed_local, a, b),
                FaceNeighbor::NegY | FaceNeighbor::PosY => (b, fixed_local, a),
                FaceNeighbor::NegZ | FaceNeighbor::PosZ => (b, a, fixed_local),
            };
            let (px, py, pz) = match face {
                FaceNeighbor::NegX | FaceNeighbor::PosX => (fixed_padded, a + 1, b + 1),
                FaceNeighbor::NegY | FaceNeighbor::PosY => (b + 1, fixed_padded, a + 1),
                FaceNeighbor::NegZ | FaceNeighbor::PosZ => (b + 1, a + 1, fixed_padded),
            };
            let Some(local) =
                LocalIndex::new(u16::try_from(x + EDGE * (z + EDGE * y)).unwrap_or(u16::MAX))
            else {
                continue;
            };
            let block = neighbor.data.blocks.get(local);
            cells.fill(registry, block, (py * PADDED_EDGE + pz) * PADDED_EDGE + px)?;
        }
    }
    Ok(())
}
