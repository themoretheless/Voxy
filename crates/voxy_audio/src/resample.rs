//! Offline bounded PCM rate conversion.
use crate::{AudioError, Clip};
impl Clip {
    /// Offline 65-tap Hann-windowed sinc conversion with low-pass cutoff at
    /// 90% of the smaller Nyquist frequency. Endpoint samples are extended;
    /// filter weights normalize DC gain and output is clamped to normalized PCM.
    /// Fixed support does not guarantee a stopband specification at every ratio.
    /// # Errors
    /// Rejects zero rate, output frame cap and kernel evaluation budget overflow.
    pub fn resample_filtered(
        &self,
        target_rate: u32,
        max_frames: usize,
        max_evaluations: usize,
    ) -> Result<Self, AudioError> {
        if target_rate == 0 {
            return Err(AudioError::InvalidFormat);
        }
        let count =
            (self.frames.len() as u128 * u128::from(target_rate)).div_ceil(u128::from(self.rate));
        let count = usize::try_from(count).map_err(|_| AudioError::Capacity)?;
        if count > max_frames {
            return Err(AudioError::Capacity);
        }
        if target_rate == self.rate {
            return Ok(self.clone());
        }
        if count
            .checked_mul(65)
            .is_none_or(|work| work > max_evaluations)
        {
            return Err(AudioError::Capacity);
        }
        let cutoff = 0.9 * (f64::from(target_rate) / f64::from(self.rate)).min(1.0);
        let g = gcd(self.rate, target_rate);
        let num_phases = (target_rate / g) as usize;
        let rate_reduced = u128::from(self.rate / g);
        let target_reduced = u128::from(target_rate / g);

        // Precompute polyphase filter bank if number of phases is reasonable (<= 1024).
        let polyphase_table: Option<Vec<[f64; 65]>> = if num_phases <= 1024 {
            let mut table = Vec::with_capacity(num_phases);
            for p in 0..num_phases {
                #[allow(clippy::cast_possible_truncation)]
                let remainder = (p as u32) * g;
                let fraction = f64::from(remainder) / f64::from(target_rate);
                table.push(compute_filter_weights(fraction, cutoff));
            }
            Some(table)
        } else {
            None
        };

        let mut frames = Vec::with_capacity(count);
        let input_len = self.frames.len();

        for index in 0..count {
            let phase = index as u128 * u128::from(self.rate);
            let center = usize::try_from(phase / u128::from(target_rate))
                .map_err(|_| AudioError::Capacity)?;

            let kernel = if let Some(ref table) = polyphase_table {
                let phase_idx = usize::try_from((index as u128 * rate_reduced) % target_reduced)
                    .map_err(|_| AudioError::Capacity)?;
                table[phase_idx]
            } else {
                let remainder = u32::try_from(phase % u128::from(target_rate))
                    .map_err(|_| AudioError::Capacity)?;
                let fraction = f64::from(remainder) / f64::from(target_rate);
                compute_filter_weights(fraction, cutoff)
            };

            // Fast path: contiguous interior samples without saturating arithmetic or clamping.
            let (sum0, sum1) = if center >= 32 && center + 32 < input_len {
                let window = &self.frames[center - 32..=center + 32];
                let mut s0 = 0.0f64;
                let mut s1 = 0.0f64;
                for (frame, &weight) in window.iter().zip(kernel.iter()) {
                    s0 += f64::from(frame[0]) * weight;
                    s1 += f64::from(frame[1]) * weight;
                }
                (s0, s1)
            } else {
                let mut s0 = 0.0f64;
                let mut s1 = 0.0f64;
                for (k, offset) in (-32_i32..=32).enumerate() {
                    let magnitude =
                        usize::try_from(offset.unsigned_abs()).map_err(|_| AudioError::Capacity)?;
                    let sample_index = if offset < 0 {
                        center.saturating_sub(magnitude)
                    } else {
                        center.saturating_add(magnitude)
                    }
                    .min(input_len - 1);
                    let weight = kernel[k];
                    s0 += f64::from(self.frames[sample_index][0]) * weight;
                    s1 += f64::from(self.frames[sample_index][1]) * weight;
                }
                (s0, s1)
            };

            #[allow(clippy::cast_possible_truncation)] // Explicit clipping of filter overshoot.
            frames.push([
                sum0.clamp(-1.0, 1.0) as f32,
                sum1.clamp(-1.0, 1.0) as f32,
            ]);
        }
        Self::new(target_rate, frames)
    }

    /// Converts a clip offline with linear interpolation and held last endpoint.
    /// Output length is `ceil(input_frames * target_rate / source_rate)`. Same-rate
    /// conversion shares the original PCM storage. No low-pass filtering occurs;
    /// downsampling can alias and needs a quality resampler for shipping content.
    /// # Errors
    /// Rejects zero target rate, arithmetic overflow and output over the frame cap.
    pub fn resample_linear(&self, target_rate: u32, max_frames: usize) -> Result<Self, AudioError> {
        if target_rate == 0 {
            return Err(AudioError::InvalidFormat);
        }
        let count =
            (self.frames.len() as u128 * u128::from(target_rate)).div_ceil(u128::from(self.rate));
        let count = usize::try_from(count).map_err(|_| AudioError::Capacity)?;
        if count > max_frames {
            return Err(AudioError::Capacity);
        }
        if target_rate == self.rate {
            return Ok(self.clone());
        }
        let mut frames = Vec::with_capacity(count);
        for index in 0..count {
            let phase = index as u128 * u128::from(self.rate);
            let left = usize::try_from(phase / u128::from(target_rate))
                .map_err(|_| AudioError::Capacity)?;
            let remainder =
                u32::try_from(phase % u128::from(target_rate)).map_err(|_| AudioError::Capacity)?;
            let right = (left + 1).min(self.frames.len() - 1);
            let weight = f64::from(remainder) / f64::from(target_rate);
            let sample = std::array::from_fn(|channel| {
                let a = f64::from(self.frames[left][channel]);
                let b = f64::from(self.frames[right][channel]);
                #[allow(clippy::cast_possible_truncation)]
                // Convex combination of normalized samples.
                {
                    (a + (b - a) * weight) as f32
                }
            });
            frames.push(sample);
        }
        Self::new(target_rate, frames)
    }
}

const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn compute_filter_weights(fraction: f64, cutoff: f64) -> [f64; 65] {
    let mut weights = [0.0f64; 65];
    let mut normalization = 0.0f64;
    for (k, offset) in (-32_i32..=32).enumerate() {
        let distance = f64::from(offset) - fraction;
        let argument = std::f64::consts::PI * cutoff * distance;
        let sinc = if argument.abs() < 1e-12 {
            1.0
        } else {
            argument.sin() / argument
        };
        let window = 0.5 * (1.0 + (std::f64::consts::PI * distance / 33.0).cos());
        let weight = cutoff * sinc * window;
        weights[k] = weight;
        normalization += weight;
    }
    let inv_norm = 1.0 / normalization;
    for w in &mut weights {
        *w *= inv_norm;
    }
    weights
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn filtered_downsampling_rejects_alias_and_preserves_dc() {
        let alternating = Clip::new(
            48_000,
            (0..512)
                .map(|i| [if i % 2 == 0 { 1.0 } else { -1.0 }; 2])
                .collect(),
        )
        .unwrap();
        let linear = alternating.resample_linear(24_000, 256).unwrap();
        let filtered = alternating
            .resample_filtered(24_000, 256, 256 * 65)
            .unwrap();
        assert!(linear.frames[32..224].iter().flatten().all(|x| *x > 0.99));
        assert!(
            filtered.frames[32..224]
                .iter()
                .flatten()
                .all(|x| x.abs() < 0.001)
        );
        let constant = Clip::new(44_100, vec![[0.25, -0.5]; 100]).unwrap();
        let converted = constant.resample_filtered(24_000, 100, 6500).unwrap();
        assert!(
            converted
                .frames
                .iter()
                .all(|x| (x[0] - 0.25).abs() < 1e-6 && (x[1] + 0.5).abs() < 1e-6)
        );
        assert!(matches!(
            constant.resample_filtered(24_000, 100, 1),
            Err(AudioError::Capacity)
        ));
        assert!(Arc::ptr_eq(
            &constant.frames,
            &constant.resample_filtered(44_100, 100, 0).unwrap().frames
        ));
    }
    #[test]
    fn passband_and_stopband_sines_at_two_to_one_ratio() {
        for (frequency, minimum, maximum) in [(1000.0, 0.69, 0.72), (18000.0, 0.0, 0.001)] {
            #[allow(clippy::cast_possible_truncation)] // Unit-amplitude reference sine.
            let source = Clip::new(
                48_000,
                (0..4800)
                    .map(|i| {
                        let sample = (std::f64::consts::TAU * frequency * f64::from(i) / 48_000.0)
                            .sin() as f32;
                        [sample; 2]
                    })
                    .collect(),
            )
            .unwrap();
            let output = source.resample_filtered(24_000, 2400, 156_000).unwrap();
            let power = output.frames[32..2368]
                .iter()
                .map(|frame| f64::from(frame[0]).powi(2))
                .sum::<f64>()
                / 2336.0;
            let rms = power.sqrt();
            assert!(
                (minimum..=maximum).contains(&rms),
                "frequency={frequency}, rms={rms}"
            );
        }
    }
    #[test]
    fn known_samples_duration_and_caps() {
        let source = Clip::new(2, vec![[0.0, 1.0], [1.0, 0.0]]).unwrap();
        let up = source.resample_linear(4, 4).unwrap();
        let expected = [[0.0, 1.0], [0.5, 0.5], [1.0, 0.0], [1.0, 0.0]];
        for (actual, expected) in up.frames.iter().zip(expected) {
            for (a, b) in actual.iter().zip(expected) {
                assert!((*a - b).abs() < f32::EPSILON);
            }
        }
        let down = up.resample_linear(2, 2).unwrap();
        assert_eq!(down.frames.len(), source.frames.len());
        assert!(Arc::ptr_eq(
            &source.frames,
            &source.resample_linear(2, 2).unwrap().frames
        ));
        assert!(matches!(
            source.resample_linear(4, 3),
            Err(AudioError::Capacity)
        ));
        assert!(matches!(
            source.resample_linear(0, 3),
            Err(AudioError::InvalidFormat)
        ));
        let constant = Clip::new(48_000, vec![[0.25; 2]; 101])
            .unwrap()
            .resample_linear(24_000, 51)
            .unwrap();
        assert_eq!(constant.frame_count(), 51);
        assert!(
            constant
                .frames
                .iter()
                .flatten()
                .all(|x| (*x - 0.25).abs() < f32::EPSILON)
        );
    }
}
