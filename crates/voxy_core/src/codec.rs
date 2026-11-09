//! Zero-dependency binary checkpoint primitives and reusable byte cursors.

use core::fmt;

/// Error returned when encoding or decoding a binary checkpoint or wire frame fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecError {
    /// Input buffer ended before the expected number of bytes could be read.
    UnexpectedEof { needed: usize, remaining: usize },
    /// Frame still had trailing unconsumed bytes after decoding finished.
    TrailingBytes { remaining: usize },
    /// Magic header or format tag did not match the expected value.
    InvalidMagic,
    /// Schema version is unsupported by this decoder.
    UnsupportedVersion(u32),
    /// Checksum (`CRC-32C`) mismatch between frame header/footer and payload.
    ChecksumMismatch { expected: u32, actual: u32 },
    /// Decoded value violated a domain invariant (e.g., non-finite float, out-of-range ID).
    InvalidValue(&'static str),
    /// Output or input exceeded a configured byte or element count limit.
    CapacityExceeded { count: usize, limit: usize },
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnexpectedEof { needed, remaining } => write!(
                formatter,
                "unexpected EOF: needed {needed} bytes, only {remaining} remaining"
            ),
            Self::TrailingBytes { remaining } => {
                write!(formatter, "trailing unconsumed bytes: {remaining}")
            }
            Self::InvalidMagic => formatter.write_str("invalid checkpoint magic header"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported checkpoint version: {version}")
            }
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "checksum mismatch: expected {expected:#010x}, got {actual:#010x}"
            ),
            Self::InvalidValue(reason) => write!(formatter, "invalid checkpoint value: {reason}"),
            Self::CapacityExceeded { count, limit } => {
                write!(formatter, "capacity {count} exceeds limit {limit}")
            }
        }
    }
}

impl std::error::Error for CodecError {}

/// Trait for deterministic binary checkpoint serialization into reusable buffers.
pub trait BinaryCheckpoint: Sized {
    /// Appends the canonical binary representation of `self` to `out` without reallocating if capacity suffices.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] if `self` contains state that cannot be represented validly.
    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), CodecError>;

    /// Decodes an instance from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] if `bytes` is truncated, malformed, or violates domain invariants.
    fn decode_from(bytes: &[u8]) -> Result<Self, CodecError>;

    /// Convenience helper that encodes `self` into a newly allocated `Vec<u8>`.
    ///
    /// # Errors
    ///
    /// Propagates any error from [`BinaryCheckpoint::encode_into`].
    #[inline]
    fn to_checkpoint_bytes(&self) -> Result<Vec<u8>, CodecError> {
        let mut out = Vec::new();
        self.encode_into(&mut out)?;
        Ok(out)
    }
}

/// Zero-copy little-endian cursor over a byte slice for deterministic checkpoint decoding.
#[derive(Clone, Copy, Debug)]
pub struct ByteReader<'a> {
    bytes: &'a [u8],
}

impl<'a> ByteReader<'a> {
    #[inline]
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    #[inline]
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.bytes.len()
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    #[inline]
    #[must_use]
    pub const fn as_slice(&self) -> &'a [u8] {
        self.bytes
    }

    /// Consumes and returns exactly `len` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::UnexpectedEof`] when fewer than `len` bytes remain.
    #[inline]
    pub fn read_slice(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        if self.bytes.len() < len {
            return Err(CodecError::UnexpectedEof {
                needed: len,
                remaining: self.bytes.len(),
            });
        }
        let (head, tail) = self.bytes.split_at(len);
        self.bytes = tail;
        Ok(head)
    }

    /// Alias for [`ByteReader::read_slice`].
    #[inline]
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        self.read_slice(len)
    }

    /// Consumes a fixed-size byte array `[u8; N]`.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::UnexpectedEof`] when fewer than `N` bytes remain.
    #[inline]
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let slice = self.read_slice(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    #[inline]
    pub fn read_u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.read_array::<1>()?[0])
    }

    #[inline]
    pub fn read_u16_le(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.read_array::<2>()?))
    }

    #[inline]
    pub fn read_u32_le(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    #[inline]
    pub fn read_u64_le(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    #[inline]
    pub fn read_i64_le(&mut self) -> Result<i64, CodecError> {
        Ok(i64::from_le_bytes(self.read_array::<8>()?))
    }

    /// Reads a finite `f32` in little-endian IEEE 754 format, rejecting `NaN` and `±Infinity`.
    #[inline]
    pub fn read_finite_f32_le(&mut self) -> Result<f32, CodecError> {
        let value = f32::from_bits(self.read_u32_le()?);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(CodecError::InvalidValue("non-finite f32"))
        }
    }

    /// Reads a finite `f64` in little-endian IEEE 754 format, rejecting `NaN` and `±Infinity`.
    #[inline]
    pub fn read_finite_f64_le(&mut self) -> Result<f64, CodecError> {
        let value = f64::from_bits(self.read_u64_le()?);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(CodecError::InvalidValue("non-finite f64"))
        }
    }

    /// Verifies that no trailing bytes remain in the cursor.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::TrailingBytes`] if any bytes are left unconsumed.
    #[inline]
    pub const fn finish(self) -> Result<(), CodecError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(CodecError::TrailingBytes {
                remaining: self.bytes.len(),
            })
        }
    }
}

/// Little-endian writer wrapping a reusable `Vec<u8>`.
#[derive(Debug)]
pub struct ByteWriter<'a> {
    buf: &'a mut Vec<u8>,
}

impl<'a> ByteWriter<'a> {
    #[inline]
    pub fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { buf }
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.buf.as_slice()
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional);
    }

    #[inline]
    pub fn write_slice(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    #[inline]
    pub fn write_u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    #[inline]
    pub fn write_u16_le(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_u32_le(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_u64_le(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_i64_le(&mut self, value: i64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_f32_le(&mut self, value: f32) {
        self.buf.extend_from_slice(&value.to_bits().to_le_bytes());
    }

    #[inline]
    pub fn write_f64_le(&mut self, value: f64) {
        self.buf.extend_from_slice(&value.to_bits().to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_reader_and_writer_round_trip_primitives_and_reject_corrupt_frames() {
        let mut buf = Vec::with_capacity(64);
        let mut writer = ByteWriter::new(&mut buf);
        writer.write_u8(7);
        writer.write_u16_le(1024);
        writer.write_u32_le(0xDEAD_BEEF);
        writer.write_i64_le(-42);
        writer.write_f32_le(1.5);
        writer.write_f64_le(-3.25);

        let mut reader = ByteReader::new(&buf);
        assert_eq!(reader.read_u8(), Ok(7));
        assert_eq!(reader.read_u16_le(), Ok(1024));
        assert_eq!(reader.read_u32_le(), Ok(0xDEAD_BEEF));
        assert_eq!(reader.read_i64_le(), Ok(-42));
        assert_eq!(reader.read_finite_f32_le(), Ok(1.5));
        assert_eq!(reader.read_finite_f64_le(), Ok(-3.25));
        assert_eq!(reader.finish(), Ok(()));

        // Non-finite floats are rejected.
        let mut bad = Vec::new();
        ByteWriter::new(&mut bad).write_f64_le(f64::NAN);
        assert_eq!(
            ByteReader::new(&bad).read_finite_f64_le(),
            Err(CodecError::InvalidValue("non-finite f64"))
        );
    }
}
