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

    /// Encodes this stereo clip into a compact 4-bit IMA ADPCM archive (`VOXYADP1`),
    /// achieving 8x compression compared to `f32` stereo frames (`1 byte` per stereo frame).
    ///
    /// # Errors
    /// Rejects frame counts exceeding `u32::MAX`.
    pub fn to_adpcm_bytes(&self) -> Result<Vec<u8>, AudioError> {
        let count = u32::try_from(self.frame_count()).map_err(|_| AudioError::Capacity)?;
        let size = self
            .frame_count()
            .checked_add(16)
            .ok_or(AudioError::Capacity)?;
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(b"VOXYADP1");
        bytes.extend_from_slice(&self.sample_rate().to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        let mut state_l = ImaAdpcmState::default();
        let mut state_r = ImaAdpcmState::default();
        for &[left, right] in &*self.frames {
            let pcm_l = (left.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            let pcm_r = (right.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            let nib_l = state_l.encode_sample(pcm_l);
            let nib_r = state_r.encode_sample(pcm_r);
            bytes.push((nib_l & 0x0F) | ((nib_r & 0x0F) << 4));
        }
        Ok(bytes)
    }

    /// Decodes a 4-bit IMA ADPCM stereo archive (`VOXYADP1`) back into a normalized [`Clip`].
    ///
    /// # Errors
    /// Rejects invalid headers, frame count mismatches, or budget overflows.
    pub fn from_adpcm_bytes(bytes: &[u8], max_frames: usize) -> Result<Self, AudioError> {
        if bytes.len() < 16 || &bytes[..8] != b"VOXYADP1" {
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
        if count.checked_add(16) != Some(bytes.len()) {
            return Err(AudioError::InvalidFormat);
        }
        let mut state_l = ImaAdpcmState::default();
        let mut state_r = ImaAdpcmState::default();
        let mut frames = Vec::with_capacity(count);
        for &byte in &bytes[16..] {
            let nib_l = byte & 0x0F;
            let nib_r = (byte >> 4) & 0x0F;
            let l = f32::from(state_l.decode_sample(nib_l)) / 32768.0;
            let r = f32::from(state_r.decode_sample(nib_r)) / 32768.0;
            frames.push([l.clamp(-1.0, 1.0), r.clamp(-1.0, 1.0)]);
        }
        Self::new(rate, frames)
    }
}

const IMA_INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
const IMA_STEP_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

#[derive(Clone, Copy, Debug, Default)]
struct ImaAdpcmState {
    predictor: i32,
    step_index: i32,
}

impl ImaAdpcmState {
    fn encode_sample(&mut self, sample: i16) -> u8 {
        let step = IMA_STEP_TABLE[self.step_index as usize];
        let mut diff = i32::from(sample) - self.predictor;
        let mut nibble = if diff < 0 {
            diff = -diff;
            8_u8
        } else {
            0_u8
        };
        let mut vpdiff = step >> 3;
        if diff >= step {
            nibble |= 4;
            diff -= step;
            vpdiff += step;
        }
        if diff >= (step >> 1) {
            nibble |= 2;
            diff -= step >> 1;
            vpdiff += step >> 1;
        }
        if diff >= (step >> 2) {
            nibble |= 1;
            vpdiff += step >> 2;
        }
        if (nibble & 8) != 0 {
            self.predictor -= vpdiff;
        } else {
            self.predictor += vpdiff;
        }
        self.predictor = self.predictor.clamp(-32768, 32767);
        self.step_index = (self.step_index + IMA_INDEX_TABLE[nibble as usize]).clamp(0, 88);
        nibble
    }

    fn decode_sample(&mut self, nibble: u8) -> i16 {
        let step = IMA_STEP_TABLE[self.step_index as usize];
        let mut vpdiff = step >> 3;
        if (nibble & 4) != 0 {
            vpdiff += step;
        }
        if (nibble & 2) != 0 {
            vpdiff += step >> 1;
        }
        if (nibble & 1) != 0 {
            vpdiff += step >> 2;
        }
        if (nibble & 8) != 0 {
            self.predictor -= vpdiff;
        } else {
            self.predictor += vpdiff;
        }
        self.predictor = self.predictor.clamp(-32768, 32767);
        self.step_index =
            (self.step_index + IMA_INDEX_TABLE[(nibble & 0x0F) as usize]).clamp(0, 88);
        self.predictor as i16
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

        let adpcm = clip.to_adpcm_bytes().unwrap();
        let decoded = Clip::from_adpcm_bytes(&adpcm, 1).unwrap();
        assert_eq!(decoded.frame_count(), 1);
        assert_eq!(decoded.sample_rate(), 48000);
    }
}
