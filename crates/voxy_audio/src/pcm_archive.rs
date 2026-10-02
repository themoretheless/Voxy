//! Bounded versioned PCM payload for the common artifact cache.
use crate::{AudioError, Clip};
impl Clip {
    /// # Errors
    /// Rejects frame counts that cannot fit the archive format.
    pub fn to_pcm_bytes(&self) -> Result<Vec<u8>, AudioError> {
        let count = u32::try_from(self.frame_count()).map_err(|_| AudioError::Capacity)?;
        let size = self
            .frame_count()
            .checked_mul(8)
            .and_then(|n| n.checked_add(16))
            .ok_or(AudioError::Capacity)?;
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(b"VOXYPCM1");
        bytes.extend_from_slice(&self.sample_rate().to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        for frame in &*self.frames {
            for sample in frame {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }
        Ok(bytes)
    }
    /// # Errors
    /// Rejects version, length, frame budget, rate and invalid normalized samples.
    pub fn from_pcm_bytes(bytes: &[u8], max_frames: usize) -> Result<Self, AudioError> {
        if bytes.len() < 16 || &bytes[..8] != b"VOXYPCM1" {
            return Err(AudioError::InvalidFormat);
        }
        let rate = u32::from_le_bytes(
            bytes[8..12]
                .try_into()
                .map_err(|_| AudioError::InvalidFormat)?,
        );
        let count = usize::try_from(u32::from_le_bytes(
            bytes[12..16]
                .try_into()
                .map_err(|_| AudioError::InvalidFormat)?,
        ))
        .map_err(|_| AudioError::Capacity)?;
        if count > max_frames {
            return Err(AudioError::Capacity);
        }
        if count.checked_mul(8).and_then(|n| n.checked_add(16)) != Some(bytes.len()) {
            return Err(AudioError::InvalidFormat);
        }
        let frames = bytes[16..]
            .chunks_exact(8)
            .map(|frame| {
                Ok([
                    f32::from_le_bytes(
                        frame[..4]
                            .try_into()
                            .map_err(|_| AudioError::InvalidFormat)?,
                    ),
                    f32::from_le_bytes(
                        frame[4..]
                            .try_into()
                            .map_err(|_| AudioError::InvalidFormat)?,
                    ),
                ])
            })
            .collect::<Result<Vec<_>, AudioError>>()?;
        Self::new(rate, frames)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_roundtrip_bounds_and_nonfinite_samples() {
        let clip = Clip::new(48000, vec![[0.25, -0.5]]).unwrap();
        let bytes = clip.to_pcm_bytes().unwrap();
        assert_eq!(
            Clip::from_pcm_bytes(&bytes, 1)
                .unwrap()
                .to_pcm_bytes()
                .unwrap(),
            bytes
        );
        assert!(Clip::from_pcm_bytes(&bytes, 0).is_err());
        assert!(Clip::from_pcm_bytes(&bytes[..bytes.len() - 1], 1).is_err());
        let mut invalid = bytes;
        invalid[16..20].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(Clip::from_pcm_bytes(&invalid, 1).is_err());
    }
}
