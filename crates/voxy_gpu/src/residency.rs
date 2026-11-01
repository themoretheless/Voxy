//! Exact integer residency footprints for actor bodies.
use physics_voxel::{AnchoredAabb, SweepConfig, SweepError, sweep_candidate_bounds};
use voxy_core::{ChunkPos, VoxelPos, split_voxel};

/// Enumerates chunks occupied by a body, preserving exclusive maxima and far coordinates.
/// # Errors
/// Rejects invalid bodies, coordinate overflow and excessive candidate volumes.
pub fn body_chunks(body: physics::AnchoredAabb) -> Result<Vec<ChunkPos>, SweepError> {
    let anchor = VoxelPos {
        x: body.anchor.x,
        y: body.anchor.y,
        z: body.anchor.z,
    };
    let (min, max) = sweep_candidate_bounds(
        AnchoredAabb {
            anchor,
            min: body.min,
            max: body.max,
        },
        [0.0; 3],
        SweepConfig::default(),
    )?;
    let position = |offset: [i64; 3]| VoxelPos {
        x: anchor.x + offset[0],
        y: anchor.y + offset[1],
        z: anchor.z + offset[2],
    };
    let first = split_voxel(position(min)).0;
    let last = split_voxel(position(max)).0;
    let mut chunks = Vec::new();
    for x in first.x..=last.x {
        for y in first.y..=last.y {
            for z in first.z..=last.z {
                chunks.push(ChunkPos { x, y, z });
            }
        }
    }
    Ok(chunks)
}

/// Candidate chunk allocation considered for VRAM residency eviction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidencyEvictionCandidate {
    pub chunk: ChunkPos,
    pub bytes: u64,
    /// Number of frames since this chunk was last visible or touched.
    pub age_frames: u32,
    /// Estimated screen-space coverage fraction in `[0.0, 1.0]`.
    pub screen_coverage: f32,
}

impl ResidencyEvictionCandidate {
    /// Higher eviction score means the candidate should be evicted earlier:
    /// old chunks with negligible screen coverage score highest.
    #[must_use]
    pub fn eviction_score(self) -> f64 {
        let coverage = if self.screen_coverage.is_finite() {
            f64::from(self.screen_coverage.clamp(0.0, 1.0))
        } else {
            0.0
        };
        (f64::from(self.age_frames) + 1.0) / (coverage + 1e-4)
    }
}

/// Selects the minimal set of [`ResidencyEvictionCandidate`]s to evict (ordered highest-priority
/// victim first) until at least `bytes_to_free` bytes have been reclaimed.
#[must_use]
pub fn select_eviction_victims(
    candidates: &[ResidencyEvictionCandidate],
    bytes_to_free: u64,
) -> Vec<ChunkPos> {
    if bytes_to_free == 0 || candidates.is_empty() {
        return Vec::new();
    }
    let mut indexed: Vec<(usize, ResidencyEvictionCandidate)> =
        candidates.iter().copied().enumerate().collect();
    indexed.sort_by(|(idx_a, a), (idx_b, b)| {
        b.eviction_score()
            .total_cmp(&a.eviction_score())
            .then_with(|| idx_a.cmp(idx_b))
    });

    let mut freed = 0_u64;
    let mut victims = Vec::new();
    for (_, candidate) in indexed {
        if candidate.bytes == 0 {
            continue;
        }
        victims.push(candidate.chunk);
        freed = freed.saturating_add(candidate.bytes);
        if freed >= bytes_to_free {
            break;
        }
    }
    victims
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exclusive_maximum_and_negative_boundary() {
        let mut body = physics::AnchoredAabb {
            anchor: physics::Origin { x: 0, y: 0, z: 0 },
            min: [-0.3, 0.0, 0.0],
            max: [0.3, 1.8, 32.0],
        };
        assert_eq!(
            body_chunks(body).unwrap(),
            vec![
                ChunkPos { x: -1, y: 0, z: 0 },
                ChunkPos { x: 0, y: 0, z: 0 },
            ]
        );
        body.anchor.x = 9_007_199_254_740_993;
        body.min[0] = 0.0;
        body.max[0] = 1.0;
        assert_eq!(
            body_chunks(body).unwrap()[0].x,
            body.anchor.x.div_euclid(32)
        );
        body.anchor.x = i64::MAX;
        body.max[0] = 2.0;
        assert!(matches!(
            body_chunks(body),
            Err(SweepError::CoordinateOverflow)
        ));
    }

    #[test]
    fn select_eviction_victims_prioritizes_stale_low_coverage_chunks() {
        let candidates = [
            ResidencyEvictionCandidate {
                chunk: ChunkPos { x: 0, y: 0, z: 0 },
                bytes: 4096,
                age_frames: 2,
                screen_coverage: 0.45,
            },
            ResidencyEvictionCandidate {
                chunk: ChunkPos { x: 10, y: 0, z: 10 },
                bytes: 4096,
                age_frames: 60,
                screen_coverage: 0.0,
            },
            ResidencyEvictionCandidate {
                chunk: ChunkPos { x: 5, y: 0, z: 5 },
                bytes: 4096,
                age_frames: 15,
                screen_coverage: 0.05,
            },
        ];
        let victims = select_eviction_victims(&candidates, 6000);
        assert_eq!(
            victims,
            vec![
                ChunkPos { x: 10, y: 0, z: 10 },
                ChunkPos { x: 5, y: 0, z: 5 },
            ]
        );
    }
}
