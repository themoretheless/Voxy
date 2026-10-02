//! Bounded in-memory RIFF/WAVE PCM16 import, independent of filesystem/device APIs.
use crate::{AudioError, Clip};
fn u16le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn u32le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated header"),
    )
}

/// Decodes one PCM16 mono/stereo RIFF/WAVE stream; mono is duplicated to stereo.
/// Unknown chunks are skipped, including their padding. Exactly one format and
/// data chunk are required. No resampling or compressed-codec fallback occurs.
/// Caller owns the input bytes; `max_frames` bounds decoded PCM allocation.
/// # Errors
/// Rejects truncation, inconsistent sizes/rates/alignment, unsupported formats,
/// duplicate chunks, empty data and decoded frames exceeding the limit.
pub fn decode_wav(bytes: &[u8], max_frames: usize) -> Result<Clip, AudioError> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::InvalidFormat);
    }
    let end = usize::try_from(u32le(bytes, 4))
        .map_err(|_| AudioError::InvalidFormat)?
        .checked_add(8)
        .ok_or(AudioError::InvalidFormat)?;
    if end != bytes.len() {
        return Err(AudioError::InvalidFormat);
    }
    let mut offset = 12;
    let mut format = None;
    let mut data = None;
    while offset < end {
        let header_end = offset.checked_add(8).ok_or(AudioError::InvalidFormat)?;
        if header_end > end {
            return Err(AudioError::InvalidFormat);
        }
        let size =
            usize::try_from(u32le(bytes, offset + 4)).map_err(|_| AudioError::InvalidFormat)?;
        let chunk_end = header_end
            .checked_add(size)
            .ok_or(AudioError::InvalidFormat)?;
        let next = chunk_end
            .checked_add(size % 2)
            .ok_or(AudioError::InvalidFormat)?;
        if next > end {
            return Err(AudioError::InvalidFormat);
        }
        let chunk = &bytes[header_end..chunk_end];
        match &bytes[offset..offset + 4] {
            b"fmt " => {
                if format.is_some() || chunk.len() < 16 {
                    return Err(AudioError::InvalidFormat);
                }
                let channels = u16le(chunk, 2);
                let rate = u32le(chunk, 4);
                let align = u16le(chunk, 12);
                if u16le(chunk, 0) != 1
                    || !(1..=2).contains(&channels)
                    || rate == 0
                    || u16le(chunk, 14) != 16
                    || align != channels * 2
                    || rate.checked_mul(u32::from(align)) != Some(u32le(chunk, 8))
                {
                    return Err(AudioError::InvalidFormat);
                }
                format = Some((channels, rate, usize::from(align)));
            }
            b"data" => {
                if data.is_some() {
                    return Err(AudioError::InvalidFormat);
                }
                data = Some(chunk);
            }
            _ => {}
        }
        offset = next;
    }
    let (channels, rate, align) = format.ok_or(AudioError::InvalidFormat)?;
    let data = data.ok_or(AudioError::InvalidFormat)?;
    if data.is_empty() || data.len() % align != 0 {
        return Err(AudioError::InvalidFormat);
    }
    let count = data.len() / align;
    if count > max_frames {
        return Err(AudioError::Capacity);
    }
    let mut frames = Vec::with_capacity(count);
    for frame in data.chunks_exact(align) {
        let sample =
            |offset| f32::from(i16::from_le_bytes([frame[offset], frame[offset + 1]])) / 32768.0;
        let left = sample(0);
        frames.push([left, if channels == 1 { left } else { sample(2) }]);
    }
    Clip::new(rate, frames)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(channels: u16) -> Vec<u8> {
        let mut bytes = b"RIFF\0\0\0\0WAVE".to_vec();
        bytes.extend_from_slice(b"JUNK\x01\0\0\0x\0fmt \x10\0\0\0");
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&(48_000_u32 * u32::from(channels) * 2).to_le_bytes());
        bytes.extend_from_slice(&(channels * 2).to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(&i16::MIN.to_le_bytes());
        bytes.extend_from_slice(&i16::MAX.to_le_bytes());
        let size = u32::try_from(bytes.len() - 8).unwrap();
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        bytes
    }
    #[test]
    fn mono_stereo_and_frame_budget() {
        let mono = decode_wav(&fixture(1), 2).unwrap();
        assert_eq!(mono.rate, 48_000);
        assert!((mono.frames[0][0] + 1.0).abs() < f32::EPSILON);
        assert_eq!(mono.frames[0][0].to_bits(), mono.frames[0][1].to_bits());
        let stereo = decode_wav(&fixture(2), 1).unwrap();
        assert_eq!(stereo.frames.len(), 1);
        assert!(stereo.frames[0][1] > 0.999);
        assert!(matches!(
            decode_wav(&fixture(1), 1),
            Err(AudioError::Capacity)
        ));
    }
    #[test]
    fn malformed_and_all_truncated_prefixes_are_rejected() {
        let bytes = fixture(2);
        for length in 0..bytes.len() {
            assert!(decode_wav(&bytes[..length], 100).is_err());
        }
        let mut bad = bytes.clone();
        bad[32..34].copy_from_slice(&3_u16.to_le_bytes());
        assert!(decode_wav(&bad, 100).is_err());
        let mut bad = bytes.clone();
        bad[38..42].fill(0);
        assert!(decode_wav(&bad, 100).is_err());
        let mut bad = bytes;
        bad.extend_from_slice(b"data\0\0\0\0");
        let size = u32::try_from(bad.len() - 8).unwrap();
        bad[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(decode_wav(&bad, 100).is_err());
    }
}
