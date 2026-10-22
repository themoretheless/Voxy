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
    #[must_use]
    pub const fn with_seed(seed: u64) -> Self {
        Self(seed)
    }

    fn mix(&mut self, value: u64) {
        let mut x = (self.0 ^ value).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 31;
        self.0 = x;
    }
}

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        let x = self.0.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^ (x >> 29)
    }
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0_u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.mix(u64::from_le_bytes(word));
        }
    }
    fn write_u32(&mut self, value: u32) {
        self.mix(u64::from(value));
    }
    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }
    fn write_usize(&mut self, value: usize) {
        self.mix(value as u64);
    }
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
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Forks a reproducible child generator from a 3D coordinate `(x, y, z)`.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub const fn fork_coord(self, x: i64, y: i64, z: i64) -> Self {
        let mut h = self.0;
        h = (h ^ (x as u64)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = (h ^ (y as u64)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h = (h ^ (z as u64)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Self(h ^ (h >> 31))
    }

    pub const fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[allow(clippy::cast_possible_truncation)]
    pub const fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform float in $[0.0, 1.0)$ using the top 24 bits of `next_u64`.
    #[allow(clippy::cast_precision_loss)]
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) * (1.0 / ((1_u32 << 24) as f32))
    }
}

/// Castagnoli CRC-32C checksum for verifying region frames and WAL records.
#[must_use]
pub const fn crc32c(bytes: &[u8]) -> u32 {
    crc32c_append(0, bytes)
}

/// Incrementally updates a Castagnoli CRC-32C checksum over `bytes`.
#[must_use]
pub const fn crc32c_append(previous: u32, bytes: &[u8]) -> u32 {
    let mut crc = !previous;
    let mut i = 0;
    while i < bytes.len() {
        crc ^= bytes[i] as u32;
        let mut bit = 0;
        while bit < 8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0x82F6_3B78 & mask);
            bit += 1;
        }
        i += 1;
    }
    !crc
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
}
