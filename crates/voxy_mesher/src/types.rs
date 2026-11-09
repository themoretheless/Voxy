//! Value types, packed GPU quad descriptors, and octahedral normal codecs for chunk meshing.

use std::num::NonZeroU8;

use voxy_core::CHUNK_VOLUME;
use voxy_world::MaterialId;

pub const MAX_DIRECTED_FACES: usize = 6 * CHUNK_VOLUME;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FaceDir {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl FaceDir {
    pub const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    #[inline]
    pub(crate) const fn material_index(self) -> usize {
        match self {
            Self::NegX => 0,
            Self::PosX => 1,
            Self::NegY => 2,
            Self::PosY => 3,
            Self::NegZ => 4,
            Self::PosZ => 5,
        }
    }

    #[inline]
    pub(crate) const fn normal(self) -> [i16; 3] {
        match self {
            Self::NegX => [-1, 0, 0],
            Self::PosX => [1, 0, 0],
            Self::NegY => [0, -1, 0],
            Self::PosY => [0, 1, 0],
            Self::NegZ => [0, 0, -1],
            Self::PosZ => [0, 0, 1],
        }
    }

    #[inline]
    pub(crate) const fn axes(self) -> (usize, usize, usize) {
        match self {
            Self::NegX | Self::PosX => (0, 2, 1),
            Self::NegY | Self::PosY => (1, 0, 2),
            Self::NegZ | Self::PosZ => (2, 0, 1),
        }
    }

    #[inline]
    pub(crate) const fn positive(self) -> bool {
        matches!(self, Self::PosX | Self::PosY | Self::PosZ)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderLayer {
    Opaque,
    Cutout,
    Translucent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuadDiagonal {
    Uv,
    Vu,
}

/// Selects the anisotropy-free triangulation diagonal (`QuadDiagonal::Uv` or `QuadDiagonal::Vu`)
/// from the 4 corner Ambient Occlusion levels `[c00, c10, c01, c11]` (`0..=3`) so that
/// interpolated AO gradients stay symmetric across quad diagonals.
#[inline]
#[must_use]
pub const fn select_ao_quad_diagonal(ao_corners: [u8; 4]) -> QuadDiagonal {
    let diag_uv = (ao_corners[0] as u16) + (ao_corners[3] as u16);
    let diag_vu = (ao_corners[1] as u16) + (ao_corners[2] as u16);
    if diag_uv < diag_vu {
        QuadDiagonal::Vu
    } else {
        QuadDiagonal::Uv
    }
}

/// Returns `true` when the quad triangulation diagonal should flip from `(v00-v11)` to `(v10-v01)`
/// to minimize ambient occlusion interpolation anisotropy across the four quad corners
/// `(ao00, ao10, ao11, ao01)`.
#[inline]
#[must_use]
pub const fn quad_diagonal_flip_for_ao(ao00: u8, ao10: u8, ao11: u8, ao01: u8) -> bool {
    let main_sum = (ao00 as i16) + (ao11 as i16);
    let alt_sum = (ao10 as i16) + (ao01 as i16);
    let main_diff = ((ao00 as i16) - (ao11 as i16)).abs();
    let alt_diff = ((ao10 as i16) - (ao01 as i16)).abs();
    if main_diff != alt_diff {
        alt_diff < main_diff
    } else {
        main_sum < alt_sum
    }
}

/// Symmetric $6 \times 6$ face-to-face visibility bitmask (`u16`, 15 unique unordered face pairs)
/// used by conservative cave culling (Tommaso Checchi's Advanced Cave Culling Algorithm).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ChunkVisibilityGraph6(u16);

impl ChunkVisibilityGraph6 {
    pub const OPAQUE: Self = Self(0);
    pub const ALL_VISIBLE: Self = Self(0x7fff);

    #[inline]
    const fn pair_bit(a: FaceDir, b: FaceDir) -> Option<u32> {
        let ia = a.material_index() as u32;
        let ib = b.material_index() as u32;
        if ia == ib {
            return None;
        }
        let (lo, hi) = if ia < ib { (ia, ib) } else { (ib, ia) };
        Some((hi * (hi - 1)) / 2 + lo)
    }

    /// Marks two chunk boundary faces as mutually visible through an open interior cavity.
    #[inline]
    #[must_use]
    pub const fn with_connection(self, a: FaceDir, b: FaceDir) -> Self {
        match Self::pair_bit(a, b) {
            Some(bit) => Self(self.0 | (1_u16 << bit)),
            None => self,
        }
    }

    /// Returns `true` if a ray can conservatively enter through face `a` and exit through `b`.
    #[inline]
    #[must_use]
    pub const fn can_see_through(self, a: FaceDir, b: FaceDir) -> bool {
        match Self::pair_bit(a, b) {
            Some(bit) => (self.0 & (1_u16 << bit)) != 0,
            None => true,
        }
    }

    /// Returns the number of connected unordered face pairs (`0..=15`).
    #[inline]
    #[must_use]
    pub const fn connected_pair_count(self) -> u32 {
        (self.0 & 0x7fff).count_ones()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Quad {
    pub origin: [u8; 3],
    pub extent_u: NonZeroU8,
    pub extent_v: NonZeroU8,
    pub face: FaceDir,
    pub material: MaterialId,
    pub layer: RenderLayer,
    pub ao: [u8; 4],
    pub diagonal: QuadDiagonal,
}

impl Quad {
    #[inline]
    #[must_use]
    pub fn area(self) -> u16 {
        u16::from(self.extent_u.get()) * u16::from(self.extent_v.get())
    }

    #[inline]
    #[must_use]
    pub fn pack_64(self) -> PackedQuad64 {
        PackedQuad64::pack(self)
    }
}

/// Compact 8-byte (64-bit) GPU-ready greedy quad descriptor.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct PackedQuad64(u64);

impl PackedQuad64 {
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    #[inline]
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    #[inline]
    #[must_use]
    pub fn pack(quad: Quad) -> Self {
        let ox = u64::from(quad.origin[0] & 63);
        let oy = u64::from(quad.origin[1] & 63);
        let oz = u64::from(quad.origin[2] & 63);
        let eu = u64::from(quad.extent_u.get().saturating_sub(1) & 31);
        let ev = u64::from(quad.extent_v.get().saturating_sub(1) & 31);
        let face = quad.face.material_index() as u64;
        let layer = match quad.layer {
            RenderLayer::Opaque => 0_u64,
            RenderLayer::Cutout => 1_u64,
            RenderLayer::Translucent => 2_u64,
        };
        let diag = match quad.diagonal {
            QuadDiagonal::Uv => 0_u64,
            QuadDiagonal::Vu => 1_u64,
        };
        let ao = u64::from(quad.ao[0] & 3)
            | (u64::from(quad.ao[1] & 3) << 2)
            | (u64::from(quad.ao[2] & 3) << 4)
            | (u64::from(quad.ao[3] & 3) << 6);
        let mat = u64::from(quad.material.0);
        Self(
            ox | (oy << 6)
                | (oz << 12)
                | (eu << 18)
                | (ev << 23)
                | (face << 28)
                | (layer << 31)
                | (diag << 33)
                | (ao << 34)
                | (mat << 42),
        )
    }

    #[must_use]
    pub fn unpack(self) -> Option<Quad> {
        let w = self.0;
        let ox = (w & 63) as u8;
        let oy = ((w >> 6) & 63) as u8;
        let oz = ((w >> 12) & 63) as u8;
        if ox > 32 || oy > 32 || oz > 32 {
            return None;
        }
        let eu = NonZeroU8::new((((w >> 18) & 31) as u8) + 1)?;
        let ev = NonZeroU8::new((((w >> 23) & 31) as u8) + 1)?;
        let face = match (w >> 28) & 7 {
            0 => FaceDir::NegX,
            1 => FaceDir::PosX,
            2 => FaceDir::NegY,
            3 => FaceDir::PosY,
            4 => FaceDir::NegZ,
            5 => FaceDir::PosZ,
            _ => return None,
        };
        let layer = match (w >> 31) & 3 {
            0 => RenderLayer::Opaque,
            1 => RenderLayer::Cutout,
            2 => RenderLayer::Translucent,
            _ => return None,
        };
        let diagonal = if ((w >> 33) & 1) == 0 {
            QuadDiagonal::Uv
        } else {
            QuadDiagonal::Vu
        };
        let ao_bits = (w >> 34) & 0xFF;
        let ao = [
            (ao_bits & 3) as u8,
            ((ao_bits >> 2) & 3) as u8,
            ((ao_bits >> 4) & 3) as u8,
            ((ao_bits >> 6) & 3) as u8,
        ];
        let material = MaterialId(((w >> 42) & 0xFFFF) as u16);
        Some(Quad {
            origin: [ox, oy, oz],
            extent_u: eu,
            extent_v: ev,
            face,
            material,
            layer,
            ao,
            diagonal,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalAabb {
    pub min: [u8; 3],
    pub max: [u8; 3],
}

impl LocalAabb {
    pub const EMPTY: Self = Self {
        min: [0; 3],
        max: [0; 3],
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkMesh {
    pub opaque: Box<[Quad]>,
    pub cutout: Box<[Quad]>,
    pub translucent: Box<[Quad]>,
    pub bounds: LocalAabb,
}

impl ChunkMesh {
    #[inline]
    #[must_use]
    pub fn quad_count(&self) -> usize {
        self.opaque.len() + self.cutout.len() + self.translucent.len()
    }

    /// Packs all quads across opaque, cutout, and translucent layers into compact 8-byte
    /// [`PackedQuad64`] GPU descriptors.
    #[must_use]
    pub fn to_packed_64(&self) -> PackedChunkMesh64 {
        PackedChunkMesh64 {
            opaque: self.opaque.iter().copied().map(Quad::pack_64).collect(),
            cutout: self.cutout.iter().copied().map(Quad::pack_64).collect(),
            translucent: self
                .translucent
                .iter()
                .copied()
                .map(Quad::pack_64)
                .collect(),
            bounds: self.bounds,
        }
    }
}

/// Compact GPU-ready representation of a [`ChunkMesh`] where each quad occupies 8 bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedChunkMesh64 {
    pub opaque: Box<[PackedQuad64]>,
    pub cutout: Box<[PackedQuad64]>,
    pub translucent: Box<[PackedQuad64]>,
    pub bounds: LocalAabb,
}

impl PackedChunkMesh64 {
    #[inline]
    #[must_use]
    pub fn quad_count(&self) -> usize {
        self.opaque.len() + self.cutout.len() + self.translucent.len()
    }
}

/// Packs a 3D unit normal into two 16-bit signed normalized integers (`snorm16`) using
/// Cigolle-Donow-Evangelakos-Mara-McGuire-Meyer octahedral projection.
#[must_use]
pub fn pack_octahedral_normal_snorm16(normal: [f32; 3]) -> [i16; 2] {
    if !normal.iter().all(|v| v.is_finite()) {
        return [0, 32_767];
    }
    let l1 = normal[0].abs() + normal[1].abs() + normal[2].abs();
    if l1 <= 1e-8 {
        return [0, 32_767];
    }
    let mut ox = normal[0] / l1;
    let mut oy = normal[1] / l1;
    if normal[2] < 0.0 {
        let sign_x = if ox >= 0.0 { 1.0 } else { -1.0 };
        let sign_y = if oy >= 0.0 { 1.0 } else { -1.0 };
        let new_x = (1.0 - oy.abs()) * sign_x;
        let new_y = (1.0 - ox.abs()) * sign_y;
        ox = new_x;
        oy = new_y;
    }
    let qx = (ox.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
    let qy = (oy.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
    [qx, qy]
}

/// Unpacks a pair of `snorm16` octahedral coordinates back into a normalized 3D unit normal.
#[must_use]
pub fn unpack_octahedral_normal_snorm16(packed: [i16; 2]) -> [f32; 3] {
    let mut x = (f32::from(packed[0]) / 32_767.0).clamp(-1.0, 1.0);
    let mut y = (f32::from(packed[1]) / 32_767.0).clamp(-1.0, 1.0);
    let z = 1.0 - x.abs() - y.abs();
    if z < 0.0 {
        let sign_x = if x >= 0.0 { 1.0 } else { -1.0 };
        let sign_y = if y >= 0.0 { 1.0 } else { -1.0 };
        x += z * sign_x;
        y += z * sign_y;
    }
    let len = (x * x + y * y + z * z).sqrt();
    if len > 1e-8 {
        [x / len, y / len, z / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("meshing failed: {self:?}")]
pub enum MeshError {
    Cancelled,
    OutputLimit,
    Invariant,
}
