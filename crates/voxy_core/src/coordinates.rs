use core::fmt;

pub const CHUNK_EDGE: i64 = 32;
pub const CHUNK_VOLUME: usize = 32 * 32 * 32;
/// Edge of a chunk padded by one neighbour slice on each side, as lighting and
/// meshing kernels consume it.
pub const PADDED_EDGE: usize = 34;
/// Cell count of a padded chunk volume (`PADDED_EDGE` cubed).
pub const PADDED_VOLUME: usize = PADDED_EDGE * PADDED_EDGE * PADDED_EDGE;

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
    pub const fn new(x: u8, y: u8, z: u8) -> Result<Self, InvalidLocalPos> {
        if (x as i64) < CHUNK_EDGE && (y as i64) < CHUNK_EDGE && (z as i64) < CHUNK_EDGE {
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

    /// Interleaves the 5-bit `x`, `y`, `z` coordinates into a 15-bit 3D Morton code.
    #[must_use]
    pub const fn morton(self) -> u16 {
        const fn part1by2(mut n: u16) -> u16 {
            n &= 0x1f;
            n = (n | (n << 8)) & 0x100f;
            n = (n | (n << 4)) & 0x10c3;
            (n | (n << 2)) & 0x1249
        }
        part1by2(self.x as u16) | (part1by2(self.y as u16) << 1) | (part1by2(self.z as u16) << 2)
    }

    /// De-interleaves a 15-bit 3D Morton code into a [`LocalPos`], or `None` if `morton >= 32768`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_morton(morton: u16) -> Option<Self> {
        const fn compact1by2(mut n: u16) -> u8 {
            n &= 0x1249;
            n = (n ^ (n >> 2)) & 0x10c3;
            n = (n ^ (n >> 4)) & 0x100f;
            ((n ^ (n >> 8)) & 0x1f) as u8
        }
        if (morton as usize) >= CHUNK_VOLUME {
            return None;
        }
        Some(Self {
            x: compact1by2(morton),
            y: compact1by2(morton >> 1),
            z: compact1by2(morton >> 2),
        })
    }

    /// Returns a 6-bit mask of chunk faces (`Direction6`) touched by this local voxel.
    #[must_use]
    pub const fn border_faces(self) -> FaceMask {
        let mut bits = 0_u8;
        if self.x == 0 {
            bits |= FaceMask::NEG_X.bits();
        }
        if self.x == 31 {
            bits |= FaceMask::POS_X.bits();
        }
        if self.y == 0 {
            bits |= FaceMask::NEG_Y.bits();
        }
        if self.y == 31 {
            bits |= FaceMask::POS_Y.bits();
        }
        if self.z == 0 {
            bits |= FaceMask::NEG_Z.bits();
        }
        if self.z == 31 {
            bits |= FaceMask::POS_Z.bits();
        }
        FaceMask(bits)
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
    pub const fn new(value: u16) -> Option<Self> {
        if (value as usize) < CHUNK_VOLUME {
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
    pub const fn position(self) -> LocalPos {
        let value = self.0 as usize;
        let x = (value % 32) as u8;
        let z = ((value / 32) % 32) as u8;
        let y = (value / (32 * 32)) as u8;
        LocalPos { x, y, z }
    }

    /// 15-bit 3D Morton (Z-order) code of the cell inside a $32^3$ chunk.
    #[must_use]
    pub const fn morton(self) -> u16 {
        self.position().morton()
    }

    /// Reconstructs a [`LocalIndex`] from a 15-bit 3D Morton (Z-order) code.
    #[must_use]
    pub const fn from_morton(morton: u16) -> Option<Self> {
        match LocalPos::from_morton(morton) {
            Some(pos) => Some(pos.index()),
            None => None,
        }
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
pub const fn split_voxel(pos: VoxelPos) -> (ChunkPos, LocalPos) {
    // CHUNK_EDGE is a power of two, so the Euclidean quotient is an arithmetic shift
    // and the Euclidean remainder a mask, for negative coordinates too.
    const { assert!(CHUNK_EDGE > 0 && CHUNK_EDGE & (CHUNK_EDGE - 1) == 0) };
    const SHIFT: u32 = CHUNK_EDGE.trailing_zeros();
    const MASK: i64 = CHUNK_EDGE - 1;
    let chunk = ChunkPos {
        x: pos.x >> SHIFT,
        y: pos.y >> SHIFT,
        z: pos.z >> SHIFT,
    };
    let local = LocalPos {
        x: (pos.x & MASK) as u8,
        y: (pos.y & MASK) as u8,
        z: (pos.z & MASK) as u8,
    };
    (chunk, local)
}

/// Combines chunk and local coordinates with checked arithmetic.
///
/// # Errors
///
/// Returns [`CoordinateOverflow`] when any reconstructed component does not fit in `i64`.
pub const fn join_voxel(chunk: ChunkPos, local: LocalPos) -> Result<VoxelPos, CoordinateOverflow> {
    const fn component(chunk: i64, local: u8) -> Result<i64, CoordinateOverflow> {
        let Some(base) = chunk.checked_mul(CHUNK_EDGE) else {
            return Err(CoordinateOverflow);
        };
        let Some(sum) = base.checked_add(local as i64) else {
            return Err(CoordinateOverflow);
        };
        Ok(sum)
    }

    let x = match component(chunk.x, local.x) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let y = match component(chunk.y, local.y) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let z = match component(chunk.z, local.z) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    Ok(VoxelPos { x, y, z })
}

/// Six cardinal face directions of a voxel or chunk.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Direction6 {
    NegX = 0,
    PosX = 1,
    NegY = 2,
    PosY = 3,
    NegZ = 4,
    PosZ = 5,
}

impl Direction6 {
    pub const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::NegX => Self::PosX,
            Self::PosX => Self::NegX,
            Self::NegY => Self::PosY,
            Self::PosY => Self::NegY,
            Self::NegZ => Self::PosZ,
            Self::PosZ => Self::NegZ,
        }
    }

    #[must_use]
    pub const fn offset(self) -> [i64; 3] {
        match self {
            Self::NegX => [-1, 0, 0],
            Self::PosX => [1, 0, 0],
            Self::NegY => [0, -1, 0],
            Self::PosY => [0, 1, 0],
            Self::NegZ => [0, 0, -1],
            Self::PosZ => [0, 0, 1],
        }
    }

    #[must_use]
    pub const fn mask(self) -> FaceMask {
        FaceMask(1_u8 << (self as u8))
    }
}

/// Bitmask over the six cardinal cube faces ([`Direction6`]).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FaceMask(u8);

impl FaceMask {
    pub const EMPTY: Self = Self(0);
    pub const ALL: Self = Self(0b0011_1111);
    pub const NEG_X: Self = Self(1 << 0);
    pub const POS_X: Self = Self(1 << 1);
    pub const NEG_Y: Self = Self(1 << 2);
    pub const POS_Y: Self = Self(1 << 3);
    pub const NEG_Z: Self = Self(1 << 4);
    pub const POS_Z: Self = Self(1 << 5);

    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b0011_1111)
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    #[must_use]
    pub const fn contains(self, dir: Direction6) -> bool {
        (self.0 & (1_u8 << (dir as u8))) != 0
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    #[must_use]
    pub const fn invert(self) -> Self {
        Self((!self.0) & 0b0011_1111)
    }
}

/// Integer axis-aligned bounding box in global voxel or chunk coordinates (`min..=max`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AabbI64 {
    pub min: VoxelPos,
    pub max: VoxelPos,
}

impl AabbI64 {
    /// Creates an inclusive bounding box if `min <= max` on every axis.
    #[must_use]
    pub const fn new(min: VoxelPos, max: VoxelPos) -> Option<Self> {
        if min.x <= max.x && min.y <= max.y && min.z <= max.z {
            Some(Self { min, max })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn from_point(point: VoxelPos) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    #[must_use]
    pub const fn contains(self, point: VoxelPos) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        const fn min_i64(a: i64, b: i64) -> i64 {
            if a < b { a } else { b }
        }
        const fn max_i64(a: i64, b: i64) -> i64 {
            if a > b { a } else { b }
        }
        Self {
            min: VoxelPos {
                x: min_i64(self.min.x, other.min.x),
                y: min_i64(self.min.y, other.min.y),
                z: min_i64(self.min.z, other.min.z),
            },
            max: VoxelPos {
                x: max_i64(self.max.x, other.max.x),
                y: max_i64(self.max.y, other.max.y),
                z: max_i64(self.max.z, other.max.z),
            },
        }
    }

    #[must_use]
    pub const fn intersection(self, other: Self) -> Option<Self> {
        const fn min_i64(a: i64, b: i64) -> i64 {
            if a < b { a } else { b }
        }
        const fn max_i64(a: i64, b: i64) -> i64 {
            if a > b { a } else { b }
        }
        Self::new(
            VoxelPos {
                x: max_i64(self.min.x, other.min.x),
                y: max_i64(self.min.y, other.min.y),
                z: max_i64(self.min.z, other.min.z),
            },
            VoxelPos {
                x: min_i64(self.max.x, other.max.x),
                y: min_i64(self.max.y, other.max.y),
                z: min_i64(self.max.z, other.max.z),
            },
        )
    }
}

/// Sub-voxel fixed-point coordinate (`i64` voxel + `u16` fraction in $1/65536$ of a voxel).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SubVoxelCoord {
    pub voxel: i64,
    pub frac: u16,
}

impl SubVoxelCoord {
    #[must_use]
    pub const fn new(voxel: i64, frac: u16) -> Self {
        Self { voxel, frac }
    }

    /// Converts to a local `f32` offset relative to `origin_voxel` without catastrophic cancellation.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn relative_f32(self, origin_voxel: i64) -> Option<f32> {
        let delta = self.voxel.checked_sub(origin_voxel)?;
        Some((delta as f32) + f32::from(self.frac) * (1.0 / 65_536.0))
    }
}

/// 3D sub-voxel position (`Q64.16` per axis) for deterministic large-world coordinates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SubVoxelPos {
    pub x: SubVoxelCoord,
    pub y: SubVoxelCoord,
    pub z: SubVoxelCoord,
}

impl SubVoxelPos {
    #[must_use]
    pub const fn from_voxel(pos: VoxelPos) -> Self {
        Self {
            x: SubVoxelCoord::new(pos.x, 0),
            y: SubVoxelCoord::new(pos.y, 0),
            z: SubVoxelCoord::new(pos.z, 0),
        }
    }

    #[must_use]
    pub const fn voxel_floor(self) -> VoxelPos {
        VoxelPos {
            x: self.x.voxel,
            y: self.y.voxel,
            z: self.z.voxel,
        }
    }

    /// Returns camera-relative `f32` coordinates relative to a [`FloatingOrigin`] anchor.
    #[must_use]
    pub fn relative_to(self, origin: FloatingOrigin) -> Option<[f32; 3]> {
        let anchor = origin.anchor_voxel()?;
        Some([
            self.x.relative_f32(anchor.x)?,
            self.y.relative_f32(anchor.y)?,
            self.z.relative_f32(anchor.z)?,
        ])
    }
}

/// Camera-relative floating-origin anchor tied to an authoritative [`ChunkPos`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FloatingOrigin {
    pub anchor_chunk: ChunkPos,
}

impl FloatingOrigin {
    #[must_use]
    pub const fn new(anchor_chunk: ChunkPos) -> Self {
        Self { anchor_chunk }
    }

    /// World voxel at the `(0, 0, 0)` corner of `anchor_chunk`.
    #[must_use]
    pub const fn anchor_voxel(self) -> Option<VoxelPos> {
        match join_voxel(self.anchor_chunk, LocalPos { x: 0, y: 0, z: 0 }) {
            Ok(pos) => Some(pos),
            Err(_) => None,
        }
    }

    /// Re-centers the origin when `camera_chunk` strays beyond `max_chunk_distance` (Chebyshev norm).
    #[must_use]
    pub const fn recenter_if_needed(
        self,
        camera_chunk: ChunkPos,
        max_chunk_distance: u64,
    ) -> (Self, bool) {
        let dx = camera_chunk.x.abs_diff(self.anchor_chunk.x);
        let dy = camera_chunk.y.abs_diff(self.anchor_chunk.y);
        let dz = camera_chunk.z.abs_diff(self.anchor_chunk.z);
        if dx > max_chunk_distance || dy > max_chunk_distance || dz > max_chunk_distance {
            (Self::new(camera_chunk), true)
        } else {
            (self, false)
        }
    }
}

/// Dense $32 \times 32 \times 32$ bit-matrix stored as 512 `u64` words (4 KiB).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitMatrix32 {
    words: Box<[u64; 512]>,
}

impl Default for BitMatrix32 {
    fn default() -> Self {
        Self::new_clear()
    }
}

impl BitMatrix32 {
    #[must_use]
    pub fn new_clear() -> Self {
        Self {
            words: Box::new([0_u64; 512]),
        }
    }

    #[must_use]
    pub fn new_full() -> Self {
        Self {
            words: Box::new([u64::MAX; 512]),
        }
    }

    #[must_use]
    pub const fn get(&self, index: LocalIndex) -> bool {
        let cell = index.get() as usize;
        let word = cell >> 6;
        let bit = cell & 63;
        ((self.words[word] >> bit) & 1) != 0
    }

    pub const fn set(&mut self, index: LocalIndex, value: bool) {
        let cell = index.get() as usize;
        let word = cell >> 6;
        let bit = cell & 63;
        let mask = 1_u64 << bit;
        if value {
            self.words[word] |= mask;
        } else {
            self.words[word] &= !mask;
        }
    }

    #[must_use]
    pub fn count_ones(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    #[must_use]
    pub fn is_all_zero(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    #[must_use]
    pub fn is_all_ones(&self) -> bool {
        self.words.iter().all(|&w| w == u64::MAX)
    }

    #[must_use]
    pub const fn words(&self) -> &[u64; 512] {
        &self.words
    }
}

/// 2D horizontal spiral chunk iterator (`XZ` shells around `center`).
#[derive(Clone, Debug)]
pub struct SpiralChunkIterator {
    center: ChunkPos,
    max_radius: i64,
    ring: i64,
    leg: u8,
    step: i64,
    x: i64,
    z: i64,
    done: bool,
}

impl SpiralChunkIterator {
    #[must_use]
    pub const fn new(center: ChunkPos, max_radius: u32) -> Self {
        Self {
            center,
            max_radius: max_radius as i64,
            ring: 0,
            leg: 0,
            step: 0,
            x: 0,
            z: 0,
            done: false,
        }
    }
}

impl Iterator for SpiralChunkIterator {
    type Item = ChunkPos;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if self.ring == 0 {
            self.ring = 1;
            if self.max_radius == 0 {
                self.done = true;
            } else {
                self.x = 1;
                self.z = 0;
                self.leg = 0;
                self.step = 0;
            }
            return Some(self.center);
        }
        let current = ChunkPos {
            x: self.center.x.saturating_add(self.x),
            y: self.center.y,
            z: self.center.z.saturating_add(self.z),
        };
        let side_len = 2 * self.ring;
        self.step += 1;
        match self.leg {
            0 => {
                self.z += 1;
                if self.z == self.ring {
                    self.leg = 1;
                    self.step = 0;
                }
            }
            1 => {
                self.x -= 1;
                if self.step == side_len {
                    self.leg = 2;
                    self.step = 0;
                }
            }
            2 => {
                self.z -= 1;
                if self.step == side_len {
                    self.leg = 3;
                    self.step = 0;
                }
            }
            3 => {
                self.x += 1;
                if self.step == side_len {
                    self.leg = 4;
                    self.step = 0;
                }
            }
            _ => {
                self.z += 1;
                if self.z == 0 {
                    self.ring += 1;
                    if self.ring > self.max_radius {
                        self.done = true;
                    } else {
                        self.x = self.ring;
                        self.z = 0;
                        self.leg = 0;
                        self.step = 0;
                    }
                }
            }
        }
        Some(current)
    }
}

/// 3D spherical chunk iterator around `center` within `radius` (in chunk units).
#[derive(Clone, Debug)]
pub struct ChunkSphereIterator {
    center: ChunkPos,
    radius: i64,
    radius_sq: i64,
    dx: i64,
    dy: i64,
    dz: i64,
}

impl ChunkSphereIterator {
    #[must_use]
    pub const fn new(center: ChunkPos, radius: u32) -> Self {
        let r = radius as i64;
        Self {
            center,
            radius: r,
            radius_sq: r.saturating_mul(r),
            dx: -r,
            dy: -r,
            dz: -r,
        }
    }
}

impl Iterator for ChunkSphereIterator {
    type Item = ChunkPos;

    fn next(&mut self) -> Option<Self::Item> {
        while self.dy <= self.radius {
            while self.dz <= self.radius {
                while self.dx <= self.radius {
                    let dx = self.dx;
                    let dy = self.dy;
                    let dz = self.dz;
                    self.dx += 1;
                    if dx * dx + dy * dy + dz * dz <= self.radius_sq {
                        return Some(ChunkPos {
                            x: self.center.x.saturating_add(dx),
                            y: self.center.y.saturating_add(dy),
                            z: self.center.z.saturating_add(dz),
                        });
                    }
                }
                self.dx = -self.radius;
                self.dz += 1;
            }
            self.dz = -self.radius;
            self.dy += 1;
        }
        None
    }
}

/// Single step reported by [`RayGridIterator`] (3D Amanatides-Woo DDA voxel traversal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayVoxelStep {
    pub voxel: VoxelPos,
    pub t_enter: f64,
    pub entered_face: Option<Direction6>,
}

/// 3D Amanatides-Woo fast voxel traversal iterator (`RayGridIterator`).
#[derive(Clone, Debug)]
pub struct RayGridIterator {
    current: VoxelPos,
    step: [i64; 3],
    t_max: [f64; 3],
    t_delta: [f64; 3],
    t_current: f64,
    max_t: f64,
    last_face: Option<Direction6>,
    remaining_steps: usize,
}

impl RayGridIterator {
    /// Creates a 3D DDA voxel iterator for `origin + t * dir` with `t in [0, max_distance]`.
    #[must_use]
    pub fn new(
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        max_steps: usize,
    ) -> Option<Self> {
        if !origin.iter().all(|v| v.is_finite())
            || !direction.iter().all(|v| v.is_finite())
            || !max_distance.is_finite()
            || max_distance < 0.0
        {
            return None;
        }
        let len_sq =
            direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2];
        if len_sq <= 0.0 || !len_sq.is_finite() {
            return None;
        }
        let inv_len = 1.0 / len_sq.sqrt();
        let dir = [
            direction[0] * inv_len,
            direction[1] * inv_len,
            direction[2] * inv_len,
        ];
        let vx = origin[0].floor() as i64;
        let vy = origin[1].floor() as i64;
        let vz = origin[2].floor() as i64;
        let coords = [vx, vy, vz];
        let mut step = [0_i64; 3];
        let mut t_max = [f64::INFINITY; 3];
        let mut t_delta = [f64::INFINITY; 3];
        for axis in 0..3 {
            let d = dir[axis];
            if d > 0.0 {
                step[axis] = 1;
                let next_boundary = (coords[axis] as f64) + 1.0;
                t_max[axis] = (next_boundary - origin[axis]) / d;
                t_delta[axis] = 1.0 / d;
            } else if d < 0.0 {
                step[axis] = -1;
                let next_boundary = coords[axis] as f64;
                t_max[axis] = (next_boundary - origin[axis]) / d;
                t_delta[axis] = -1.0 / d;
            }
        }
        Some(Self {
            current: VoxelPos {
                x: vx,
                y: vy,
                z: vz,
            },
            step,
            t_max,
            t_delta,
            t_current: 0.0,
            max_t: max_distance,
            last_face: None,
            remaining_steps: max_steps,
        })
    }
}

impl Iterator for RayGridIterator {
    type Item = RayVoxelStep;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_steps == 0 || self.t_current > self.max_t {
            return None;
        }
        self.remaining_steps -= 1;
        let out = RayVoxelStep {
            voxel: self.current,
            t_enter: self.t_current,
            entered_face: self.last_face,
        };
        if self.t_max[0] <= self.t_max[1] && self.t_max[0] <= self.t_max[2] {
            self.t_current = self.t_max[0];
            self.t_max[0] += self.t_delta[0];
            self.current.x = self.current.x.saturating_add(self.step[0]);
            self.last_face = Some(if self.step[0] > 0 {
                Direction6::NegX
            } else {
                Direction6::PosX
            });
        } else if self.t_max[1] <= self.t_max[2] {
            self.t_current = self.t_max[1];
            self.t_max[1] += self.t_delta[1];
            self.current.y = self.current.y.saturating_add(self.step[1]);
            self.last_face = Some(if self.step[1] > 0 {
                Direction6::NegY
            } else {
                Direction6::PosY
            });
        } else {
            self.t_current = self.t_max[2];
            self.t_max[2] += self.t_delta[2];
            self.current.z = self.current.z.saturating_add(self.step[2]);
            self.last_face = Some(if self.step[2] > 0 {
                Direction6::NegZ
            } else {
                Direction6::PosZ
            });
        }
        Some(out)
    }
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

    #[test]
    fn morton_z_order_is_bijective_and_preserves_locality() {
        let mut seen = vec![false; CHUNK_VOLUME];
        for code in 0_u16..32_768 {
            let pos = LocalPos::from_morton(code).unwrap();
            assert_eq!(pos.morton(), code);
            let idx = pos.index();
            assert_eq!(LocalIndex::from_morton(code), Some(idx));
            assert_eq!(idx.morton(), code);
            assert!(!core::mem::replace(&mut seen[usize::from(idx.get())], true));
        }
        assert!(seen.into_iter().all(core::convert::identity));
        assert!(LocalPos::from_morton(32_768).is_none());
    }

    #[test]
    fn floating_origin_and_sub_voxel_preserve_precision_at_large_coordinates() {
        let anchor_chunk = ChunkPos {
            x: 1_000_000,
            y: -500_000,
            z: 2_000_000,
        };
        let origin = FloatingOrigin::new(anchor_chunk);
        let anchor = origin.anchor_voxel().unwrap();
        let sub = SubVoxelPos {
            x: SubVoxelCoord::new(anchor.x + 15, 32_768),
            y: SubVoxelCoord::new(anchor.y + 4, 16_384),
            z: SubVoxelCoord::new(anchor.z - 2, 0),
        };
        let rel = sub.relative_to(origin).unwrap();
        assert!((rel[0] - 15.5).abs() < 1e-6);
        assert!((rel[1] - 4.25).abs() < 1e-6);
        assert!((rel[2] - (-2.0)).abs() < 1e-6);

        let (kept, moved) = origin.recenter_if_needed(
            ChunkPos {
                x: anchor_chunk.x + 2,
                ..anchor_chunk
            },
            4,
        );
        assert!(!moved);
        assert_eq!(kept, origin);
        let (recentered, moved) = origin.recenter_if_needed(
            ChunkPos {
                x: anchor_chunk.x + 9,
                ..anchor_chunk
            },
            4,
        );
        assert!(moved);
        assert_eq!(recentered.anchor_chunk.x, anchor_chunk.x + 9);
    }

    #[test]
    fn bit_matrix_and_border_faces_round_trip() {
        let mut matrix = BitMatrix32::new_clear();
        assert!(matrix.is_all_zero());
        let corner = LocalPos::new(0, 31, 0).unwrap();
        assert_eq!(
            corner.border_faces(),
            FaceMask::NEG_X
                .union(FaceMask::POS_Y)
                .union(FaceMask::NEG_Z)
        );
        matrix.set(corner.index(), true);
        assert!(matrix.get(corner.index()));
        assert_eq!(matrix.count_ones(), 1);
    }

    #[test]
    fn spiral_sphere_and_ray_grid_iterators_are_exact() {
        let center = ChunkPos { x: 0, y: 2, z: 0 };
        let spiral: Vec<_> = SpiralChunkIterator::new(center, 1).collect();
        assert_eq!(spiral.len(), 9);
        assert_eq!(spiral[0], center);

        let sphere: Vec<_> = ChunkSphereIterator::new(center, 1).collect();
        // Distance^2 <= 1 in 3D grid has 1 center + 6 face neighbors = 7 chunks.
        assert_eq!(sphere.len(), 7);

        let ray: Vec<_> = RayGridIterator::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], 2.1, 16)
            .unwrap()
            .collect();
        assert_eq!(ray.len(), 3);
        assert_eq!(ray[0].voxel, VoxelPos { x: 0, y: 0, z: 0 });
        assert_eq!(ray[1].voxel, VoxelPos { x: 1, y: 0, z: 0 });
        assert_eq!(ray[1].entered_face, Some(Direction6::NegX));
        assert_eq!(ray[2].voxel, VoxelPos { x: 2, y: 0, z: 0 });
    }
}
