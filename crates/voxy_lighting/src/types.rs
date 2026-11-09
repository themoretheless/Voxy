use std::sync::Arc;

use voxy_core::LocalIndex;
use voxy_world::{BlockRegistry, ChunkRevision, ChunkSnapshot};

pub(crate) const EDGE: usize = 32;
pub(crate) const MAX_LIGHT: u8 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceNeighbor {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl FaceNeighbor {
    pub(crate) const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    #[inline]
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LightStamp {
    pub center: ChunkRevision,
    pub faces: [Option<ChunkRevision>; 6],
    pub registry_epoch: u64,
    pub lighting_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct LightingInput {
    pub center: ChunkSnapshot,
    pub neighbors: [Option<ChunkSnapshot>; 6],
    pub registry: Arc<BlockRegistry>,
    /// Whether unobstructed sky enters the top halo at level 15.
    pub sky_from_above: bool,
    pub stamp: LightStamp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LightingBudget {
    pub max_propagation_steps: usize,
}

impl Default for LightingBudget {
    fn default() -> Self {
        Self {
            max_propagation_steps: 2_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LightVolume {
    pub(crate) packed: Box<[u8]>,
    pub stamp: LightStamp,
}

impl LightVolume {
    #[inline]
    #[must_use]
    pub fn packed(&self, index: LocalIndex) -> u8 {
        self.packed[usize::from(index.get())]
    }

    #[inline]
    #[must_use]
    pub fn sky(&self, index: LocalIndex) -> u8 {
        self.packed(index) >> 4
    }

    #[inline]
    #[must_use]
    pub fn block(&self, index: LocalIndex) -> u8 {
        self.packed(index) & 0x0f
    }

    #[inline]
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.packed
    }

    /// Returns `true` when every cell in the chunk has zero skylight and zero blocklight.
    #[must_use]
    pub fn is_all_dark(&self) -> bool {
        self.packed.iter().all(|&b| b == 0)
    }

    /// Returns `true` when every cell in the chunk has full skylight (15) and zero blocklight (`0xF0`).
    #[must_use]
    pub fn is_all_full_sky(&self) -> bool {
        self.packed.iter().all(|&b| b == (MAX_LIGHT << 4))
    }

    /// Downsamples the $32^3$ packed light volume into a $16^3$ (`4096`-byte) LOD mip
    /// volume by taking the component-wise maximum of skylight (`high nibble`) and
    /// blocklight (`low nibble`) across each $2 \times 2 \times 2$ cell octant.
    #[must_use]
    pub fn downsample_2x_max(&self) -> Box<[u8; 4096]> {
        if self.is_all_dark() {
            return Box::new([0_u8; 4096]);
        }
        if self.is_all_full_sky() {
            return Box::new([MAX_LIGHT << 4; 4096]);
        }
        let mut mip = Box::new([0_u8; 4096]);
        for my in 0_usize..16 {
            for mz in 0_usize..16 {
                for mx in 0_usize..16 {
                    let mut max_sky = 0_u8;
                    let mut max_block = 0_u8;
                    for dy in 0_usize..2 {
                        let y = my * 2 + dy;
                        for dz in 0_usize..2 {
                            let z = mz * 2 + dz;
                            let base = EDGE * (z + EDGE * y) + mx * 2;
                            for dx in 0_usize..2 {
                                let p = self.packed[base + dx];
                                max_sky = max_sky.max(p >> 4);
                                max_block = max_block.max(p & 0x0f);
                            }
                        }
                    }
                    mip[mx + 16 * (mz + 16 * my)] = (max_sky << 4) | max_block;
                }
            }
        }
        mip
    }
}

/// Computes the attenuated 4-bit light level (`0..=15`) after passing through a translucent
/// medium (stained glass, water, foliage) with `filter_transmittance` in `0..=15` and an
/// additional medium `step_cost` (`1..=15`).
#[inline]
#[must_use]
pub const fn attenuate_filtered_light(
    incident_level: u8,
    filter_transmittance: u8,
    step_cost: u8,
) -> u8 {
    let level = if incident_level > MAX_LIGHT {
        MAX_LIGHT
    } else {
        incident_level
    };
    let trans = if filter_transmittance > MAX_LIGHT {
        MAX_LIGHT
    } else {
        filter_transmittance
    };
    let cost = if step_cost == 0 { 1 } else { step_cost };
    let scaled = ((level as u16) * (trans as u16) + 7) / (MAX_LIGHT as u16);
    (scaled as u8).saturating_sub(cost)
}

/// Propagates 3-channel RGB light (`0..=15` per channel) through stained/tinted glass
/// with per-channel transmittance `[0..=15]` and Manhattan step decay.
#[inline]
#[must_use]
pub fn attentuate_tinted_glass_rgb_light(
    incoming_rgb: [u8; 3],
    glass_tint_rgb: [u8; 3],
    step_decay: u8,
) -> [u8; 3] {
    [
        attenuate_filtered_light(incoming_rgb[0], glass_tint_rgb[0], step_decay),
        attenuate_filtered_light(incoming_rgb[1], glass_tint_rgb[1], step_decay),
        attenuate_filtered_light(incoming_rgb[2], glass_tint_rgb[2], step_decay),
    ]
}

/// Computes a single-bounce diffuse radiosity block-light injection level (`0..=15`) emitted by
/// a sunlit voxel face given `direct_sun_level` (`0..=15`) and `surface_albedo_01` (`[0, 1]`).
#[inline]
#[must_use]
pub fn bounce_radiosity_injection_level(direct_sun_level: u8, surface_albedo_01: f32) -> u8 {
    if !surface_albedo_01.is_finite() || direct_sun_level == 0 || surface_albedo_01 <= 0.0 {
        return 0;
    }
    let sun = direct_sun_level.min(MAX_LIGHT) as f32;
    let albedo = surface_albedo_01.clamp(0.0, 1.0);
    // One diffuse bounce scales incident skylight by 0.5 * albedo.
    let bounced = (sun * albedo * 0.5).round() as u8;
    bounced.min(MAX_LIGHT)
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("lighting error: {self:?}")]
pub enum LightingError {
    InvalidBudget,
    StampMismatch,
    UnknownBlock(u32),
    BudgetExceeded,
    Cancelled,
}
