//! Deterministic, seedable multiply/xor-shift hashing for hot-path maps.
//!
//! The randomized `SipHash` default dominated per-tick component lookups and
//! per-frame normal welding; this fold mixes already well-distributed keys
//! (`TypeId`, generational ids, float bit patterns) well enough for open
//! addressing, is identical on every run and platform, and never affects
//! results, only lookup cost.

use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

/// Hasher state; `Default` starts from zero, [`FastHasher::with_seed`] from a seed.
#[derive(Clone, Copy, Debug, Default)]
pub struct FastHasher(u64);

/// Zero-seeded builder, for `HashMap<K, V, FastHashBuilder>`.
pub type FastHashBuilder = BuildHasherDefault<FastHasher>;

/// Builder that starts every hasher from a fixed seed.
#[derive(Clone, Copy, Debug)]
pub struct SeededFastHashBuilder(pub u64);

impl BuildHasher for SeededFastHashBuilder {
    type Hasher = FastHasher;
    fn build_hasher(&self) -> FastHasher {
        FastHasher::with_seed(self.0)
    }
}

impl FastHasher {
    #[inline]
    #[must_use]
    pub const fn with_seed(seed: u64) -> Self {
        Self(seed)
    }

    #[inline]
    fn mix(&mut self, value: u64) {
        let mut x = (self.0 ^ value).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 31;
        self.0 = x;
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        let x = self.0.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^ (x >> 29)
    }
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let mut word = [0_u8; 8];
            word.copy_from_slice(chunk);
            self.mix(u64::from_le_bytes(word));
        }
        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut word = [0_u8; 8];
            word[..rem.len()].copy_from_slice(rem);
            self.mix(u64::from_le_bytes(word));
        }
    }
    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.mix(u64::from(value));
    }
    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }
    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.mix(value as u64);
    }
    #[inline]
    fn write_u128(&mut self, value: u128) {
        let [low, high] = [value & u128::from(u64::MAX), value >> 64]
            .map(|half| u64::try_from(half).unwrap_or(0));
        self.mix(low);
        self.mix(high);
    }
}

/// Deterministic, forkable SplitMix64 pseudorandom generator for procedural world generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeterministicRng(u64);

impl DeterministicRng {
    #[inline]
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Forks a reproducible child generator from a 3D coordinate `(x, y, z)`.
    #[inline]
    #[must_use]
    pub const fn fork_coord(self, x: i64, y: i64, z: i64) -> Self {
        let mut h = self.0;
        h = (h ^ (x as u64)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = (h ^ (y as u64)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h = (h ^ (z as u64)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Self(h ^ (h >> 31))
    }

    #[inline]
    pub const fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[inline]
    pub const fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform float in $[0.0, 1.0)$ using the top 24 bits of `next_u64`.
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) * (1.0 / ((1_u32 << 24) as f32))
    }
}

const CRC32C_TABLES: [[u32; 256]; 8] = {
    let mut tables = [[0_u32; 256]; 8];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0x82F6_3B78 & mask);
            bit += 1;
        }
        tables[0][i] = crc;
        i += 1;
    }
    let mut k = 1;
    while k < 8 {
        let mut i = 0;
        while i < 256 {
            let c = tables[k - 1][i];
            tables[k][i] = (c >> 8) ^ tables[0][(c & 0xFF) as usize];
            i += 1;
        }
        k += 1;
    }
    tables
};

#[inline]
const fn gf2_mul(mat: &[u32; 32], mut vec: u32) -> u32 {
    let mut sum = 0_u32;
    let mut i = 0;
    while vec != 0 {
        if (vec & 1) != 0 {
            sum ^= mat[i];
        }
        vec >>= 1;
        i += 1;
    }
    sum
}

const CRC32C_ZERO_BYTE_OPS: [[u32; 32]; 64] = {
    let mut ops = [[0_u32; 32]; 64];
    let mut b = 0;
    while b < 32 {
        let mut x = 1_u32 << b;
        let mut bit = 0;
        while bit < 8 {
            let mask = (x & 1).wrapping_neg();
            x = (x >> 1) ^ (0x82F6_3B78 & mask);
            bit += 1;
        }
        ops[0][b] = x;
        b += 1;
    }
    let mut k = 1;
    while k < 64 {
        let mut i = 0;
        while i < 32 {
            ops[k][i] = gf2_mul(&ops[k - 1], ops[k - 1][i]);
            i += 1;
        }
        k += 1;
    }
    ops
};

/// Castagnoli CRC-32C checksum for verifying region frames and WAL records.
#[inline]
#[must_use]
pub const fn crc32c(bytes: &[u8]) -> u32 {
    crc32c_append(0, bytes)
}

/// Incrementally updates a Castagnoli CRC-32C checksum over `bytes` using slice-by-8 lookup tables.
#[inline]
#[must_use]
pub const fn crc32c_append(previous: u32, bytes: &[u8]) -> u32 {
    let mut crc = !previous;
    let mut i = 0;
    while i + 8 <= bytes.len() {
        let w = u64::from_le_bytes([
            bytes[i],
            bytes[i + 1],
            bytes[i + 2],
            bytes[i + 3],
            bytes[i + 4],
            bytes[i + 5],
            bytes[i + 6],
            bytes[i + 7],
        ]) ^ (crc as u64);
        crc = CRC32C_TABLES[7][(w & 0xFF) as usize]
            ^ CRC32C_TABLES[6][((w >> 8) & 0xFF) as usize]
            ^ CRC32C_TABLES[5][((w >> 16) & 0xFF) as usize]
            ^ CRC32C_TABLES[4][((w >> 24) & 0xFF) as usize]
            ^ CRC32C_TABLES[3][((w >> 32) & 0xFF) as usize]
            ^ CRC32C_TABLES[2][((w >> 40) & 0xFF) as usize]
            ^ CRC32C_TABLES[1][((w >> 48) & 0xFF) as usize]
            ^ CRC32C_TABLES[0][((w >> 56) & 0xFF) as usize];
        i += 8;
    }
    while i < bytes.len() {
        let idx = ((crc ^ (bytes[i] as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLES[0][idx];
        i += 1;
    }
    !crc
}

/// Combines two independent Castagnoli CRC-32C checksums (`crc1` over `a`, `crc2` over `b` of
/// byte length `len2`) into `crc32c(&[a, b].concat())` in $O(\log(\text{len2}))$ GF(2) steps.
#[inline]
#[must_use]
pub const fn crc32c_combine(mut crc1: u32, crc2: u32, len2: usize) -> u32 {
    if len2 == 0 {
        return crc1;
    }
    let mut n = len2 as u64;
    let mut bit = 0;
    while n != 0 {
        if (n & 1) != 0 {
            crc1 = gf2_mul(&CRC32C_ZERO_BYTE_OPS[bit], crc1);
        }
        n >>= 1;
        bit += 1;
    }
    crc1 ^ crc2
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every typed `write_*` must agree with the byte path, so a key hashes the
    /// same whichever `Hash` impl feeds it.
    #[test]
    fn typed_writes_match_byte_writes() {
        for seed in [0, 0x9E37_79B9_7F4A_7C15] {
            for value in [0_u64, 1, 0xFFFF_FFFF, u64::MAX, 0x0123_4567_89AB_CDEF] {
                let mut typed = FastHasher::with_seed(seed);
                typed.write_u64(value);
                let mut bytes = FastHasher::with_seed(seed);
                bytes.write(&value.to_le_bytes());
                assert_eq!(typed.finish(), bytes.finish());

                let mut typed = FastHasher::with_seed(seed);
                typed.write_u32(value as u32);
                let mut bytes = FastHasher::with_seed(seed);
                bytes.write(&(value as u32).to_le_bytes());
                assert_eq!(typed.finish(), bytes.finish());

                let wide = u128::from(value) << 64 | u128::from(!value);
                let mut typed = FastHasher::with_seed(seed);
                typed.write_u128(wide);
                let mut bytes = FastHasher::with_seed(seed);
                bytes.write(&wide.to_le_bytes());
                assert_eq!(typed.finish(), bytes.finish());
            }
        }
    }

    #[test]
    fn pinned_values_keep_iteration_order_stable_across_releases() {
        let mut hasher = FastHasher::default();
        hasher.write_u64(42);
        let zero_seeded = hasher.finish();
        let mut hasher = SeededFastHashBuilder(0x9E37_79B9_7F4A_7C15).build_hasher();
        hasher.write_u64(42);
        assert_ne!(zero_seeded, hasher.finish());
        // Pinned output of the fold for the key 42 from a zero seed.
        let folded = 42_u64.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let folded = folded ^ (folded >> 31);
        let folded = folded.wrapping_mul(0x94D0_49BB_1331_11EB);
        assert_eq!(zero_seeded, folded ^ (folded >> 29));
    }

    #[test]
    fn slice_by_8_and_combine_match_byte_oracle() {
        fn byte_oracle(bytes: &[u8]) -> u32 {
            let mut crc = !0_u32;
            for &b in bytes {
                crc = (crc >> 8) ^ CRC32C_TABLES[0][((crc ^ u32::from(b)) & 0xFF) as usize];
            }
            !crc
        }
        for la in [0_usize, 1, 7, 8, 13, 129, 1024] {
            for lb in [0_usize, 1, 7, 8, 19, 1024, 65_537] {
                let a: Vec<u8> = (0..la).map(|i| ((i * 37 + 11) & 0xFF) as u8).collect();
                let b: Vec<u8> = (0..lb).map(|i| ((i * 91 + 5) & 0xFF) as u8).collect();
                let mut joined = a.clone();
                joined.extend_from_slice(&b);
                let oracle = byte_oracle(&joined);
                assert_eq!(crc32c(&joined), oracle);
                assert_eq!(crc32c_append(crc32c(&a), &b), oracle);
                assert_eq!(crc32c_combine(crc32c(&a), crc32c(&b), b.len()), oracle);
            }
        }
    }
}
