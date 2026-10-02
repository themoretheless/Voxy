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
}
