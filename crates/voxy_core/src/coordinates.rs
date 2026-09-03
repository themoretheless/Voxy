use core::fmt;

pub const CHUNK_EDGE: i64 = 32;
pub const CHUNK_VOLUME: usize = 32 * 32 * 32;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VoxelPos {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkPos {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalPos {
    x: u8,
    y: u8,
    z: u8,
}

impl LocalPos {
    /// Creates a position inside one chunk.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLocalPos`] when any component is at least [`CHUNK_EDGE`].
    pub fn new(x: u8, y: u8, z: u8) -> Result<Self, InvalidLocalPos> {
        if i64::from(x) < CHUNK_EDGE && i64::from(y) < CHUNK_EDGE && i64::from(z) < CHUNK_EDGE {
            Ok(Self { x, y, z })
        } else {
            Err(InvalidLocalPos { x, y, z })
        }
    }

    #[must_use]
    pub const fn x(self) -> u8 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> u8 {
        self.y
    }

    #[must_use]
    pub const fn z(self) -> u8 {
        self.z
    }

    #[must_use]
    pub const fn index(self) -> LocalIndex {
        let value = (self.x as u16) + 32 * ((self.z as u16) + 32 * (self.y as u16));
        LocalIndex(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidLocalPos {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl fmt::Display for InvalidLocalPos {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "local position ({}, {}, {}) is outside 0..{CHUNK_EDGE}",
            self.x, self.y, self.z
        )
    }
}

impl std::error::Error for InvalidLocalPos {}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalIndex(u16);

impl LocalIndex {
    #[must_use]
    pub fn new(value: u16) -> Option<Self> {
        if usize::from(value) < CHUNK_VOLUME {
            Some(Self(value))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    #[must_use]
    pub fn all() -> impl ExactSizeIterator<Item = Self> {
        (0_u16..32_768).map(Self)
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn position(self) -> LocalPos {
        let value = usize::from(self.0);
        let x = (value % 32) as u8;
        let z = ((value / 32) % 32) as u8;
        let y = (value / (32 * 32)) as u8;
        LocalPos { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoordinateOverflow;

impl fmt::Display for CoordinateOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("chunk and local coordinates do not fit in an i64 voxel position")
    }
}

impl std::error::Error for CoordinateOverflow {}

#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn split_voxel(pos: VoxelPos) -> (ChunkPos, LocalPos) {
    let chunk = ChunkPos {
        x: pos.x.div_euclid(CHUNK_EDGE),
        y: pos.y.div_euclid(CHUNK_EDGE),
        z: pos.z.div_euclid(CHUNK_EDGE),
    };
    let local = LocalPos {
        x: pos.x.rem_euclid(CHUNK_EDGE) as u8,
        y: pos.y.rem_euclid(CHUNK_EDGE) as u8,
        z: pos.z.rem_euclid(CHUNK_EDGE) as u8,
    };
    (chunk, local)
}

/// Combines chunk and local coordinates with checked arithmetic.
///
/// # Errors
///
/// Returns [`CoordinateOverflow`] when any reconstructed component does not fit in `i64`.
pub fn join_voxel(chunk: ChunkPos, local: LocalPos) -> Result<VoxelPos, CoordinateOverflow> {
    fn component(chunk: i64, local: u8) -> Result<i64, CoordinateOverflow> {
        chunk
            .checked_mul(CHUNK_EDGE)
            .and_then(|base| base.checked_add(i64::from(local)))
            .ok_or(CoordinateOverflow)
    }

    Ok(VoxelPos {
        x: component(chunk.x, local.x)?,
        y: component(chunk.y, local.y)?,
        z: component(chunk.z, local.z)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_uses_euclidean_coordinates_around_boundaries() {
        for value in [-65, -64, -63, -33, -32, -31, -1, 0, 1, 31, 32, 33, 63, 64] {
            let pos = VoxelPos {
                x: value,
                y: value,
                z: value,
            };
            let (chunk, local) = split_voxel(pos);
            assert_eq!(join_voxel(chunk, local), Ok(pos));
            assert!(local.x() < 32 && local.y() < 32 && local.z() < 32);
        }
    }

    #[test]
    fn split_join_round_trips_representative_domain() {
        let values = [
            i64::MIN,
            i64::MIN + 1,
            -1_000_000_001,
            -33,
            -32,
            -1,
            0,
            1,
            31,
            32,
            33,
            1_000_000_001,
            i64::MAX - 1,
            i64::MAX,
        ];
        for &x in &values {
            for &y in &values {
                let pos = VoxelPos { x, y, z: x ^ y };
                let (chunk, local) = split_voxel(pos);
                assert_eq!(join_voxel(chunk, local), Ok(pos));
            }
        }
    }

    #[test]
    fn join_detects_both_overflow_directions() {
        let zero = LocalPos::new(0, 0, 0).unwrap();
        let max = LocalPos::new(31, 31, 31).unwrap();
        assert_eq!(
            join_voxel(
                ChunkPos {
                    x: i64::MAX,
                    y: 0,
                    z: 0
                },
                zero
            ),
            Err(CoordinateOverflow)
        );
        assert_eq!(
            join_voxel(
                ChunkPos {
                    x: i64::MIN,
                    y: 0,
                    z: 0
                },
                max
            ),
            Err(CoordinateOverflow)
        );
    }

    #[test]
    fn local_index_layout_is_bijective() {
        let mut seen = vec![false; CHUNK_VOLUME];
        for y in 0..32 {
            for z in 0..32 {
                for x in 0..32 {
                    let pos = LocalPos::new(x, y, z).unwrap();
                    let index = pos.index();
                    assert_eq!(index.position(), pos);
                    assert!(!core::mem::replace(
                        &mut seen[usize::from(index.get())],
                        true
                    ));
                }
            }
        }
        assert!(seen.into_iter().all(core::convert::identity));
    }
}
