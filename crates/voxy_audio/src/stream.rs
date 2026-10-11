//! Single-owner streaming buffer; cross-thread transport belongs to device adapters.
use crate::AudioError;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StreamRead {
    pub supplied: usize,
    pub missing: usize,
}
/// Preallocated bounded stereo frame FIFO. Push and read do not grow storage.
/// This is single-owner and does not provide lock-free cross-thread guarantees.
#[derive(Debug)]
pub struct PcmStream {
    rate: u32,
    capacity: usize,
    frames: VecDeque<[f32; 2]>,
    missing_total: u64,
}
impl PcmStream {
    /// # Errors
    /// Rejects a zero rate or capacity.
    pub fn new(rate: u32, capacity: usize) -> Result<Self, AudioError> {
        if rate == 0 || capacity == 0 {
            return Err(AudioError::InvalidFormat);
        }
        Ok(Self {
            rate,
            capacity,
            frames: VecDeque::with_capacity(capacity),
            missing_total: 0,
        })
    }
    #[must_use]
    pub fn buffered_frames(&self) -> usize {
        self.frames.len()
    }
    #[must_use]
    pub fn available_frames(&self) -> usize {
        self.capacity - self.frames.len()
    }
    /// Saturating lifetime count, including silence emitted before any input.
    #[must_use]
    pub fn missing_frames(&self) -> u64 {
        self.missing_total
    }
    /// Appends a complete block or rejects it without consuming existing audio.
    /// Caller can retain a rejected block and retry after output drains frames.
    /// # Errors
    /// Rejects rate mismatches, non-normalized/nonfinite samples and full capacity.
    pub fn push(&mut self, rate: u32, input: &[[f32; 2]]) -> Result<(), AudioError> {
        if rate != self.rate
            || input
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || !(-1.0..=1.0).contains(x))
        {
            return Err(AudioError::InvalidFormat);
        }
        if input.len() > self.available_frames() {
            return Err(AudioError::Capacity);
        }
        self.frames.extend(input.iter().copied());
        Ok(())
    }
    /// Overwrites output with available frames then silence, preserving FIFO order.
    /// No time-stretch, sample repetition, waiting or allocation occurs.
    pub fn read(&mut self, output: &mut [[f32; 2]]) -> StreamRead {
        let supplied = output.len().min(self.frames.len());
        let (front, back) = self.frames.as_slices();
        if supplied <= front.len() {
            output[..supplied].copy_from_slice(&front[..supplied]);
        } else {
            output[..front.len()].copy_from_slice(front);
            let rem = supplied - front.len();
            output[front.len()..supplied].copy_from_slice(&back[..rem]);
        }
        self.frames.drain(..supplied);
        output[supplied..].fill([0.0; 2]);
        let missing = output.len() - supplied;
        self.missing_total = self
            .missing_total
            .saturating_add(u64::try_from(missing).unwrap_or(u64::MAX));
        StreamRead { supplied, missing }
    }
    /// Discards queued data for a seek/reset. Lifetime underrun telemetry persists.
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}
#[cfg(test)]
#[allow(clippy::float_cmp)] // FIFO must preserve PCM bits and exact silence.
mod tests {
    use super::*;
    #[test]
    fn wraparound_backpressure_and_underrun() {
        let mut stream = PcmStream::new(48_000, 3).unwrap();
        stream
            .push(48_000, &[[0.1; 2], [0.2; 2], [0.3; 2]])
            .unwrap();
        assert_eq!(stream.push(48_000, &[[0.4; 2]]), Err(AudioError::Capacity));
        let mut first = [[0.0; 2]; 2];
        assert_eq!(
            stream.read(&mut first),
            StreamRead {
                supplied: 2,
                missing: 0
            }
        );
        assert_eq!(first, [[0.1; 2], [0.2; 2]]);
        stream.push(48_000, &[[0.4; 2], [0.5; 2]]).unwrap();
        let mut tail = [[1.0; 2]; 4];
        assert_eq!(
            stream.read(&mut tail),
            StreamRead {
                supplied: 3,
                missing: 1
            }
        );
        assert_eq!(tail, [[0.3; 2], [0.4; 2], [0.5; 2], [0.0; 2]]);
        assert_eq!(stream.missing_frames(), 1);
        assert_eq!(stream.available_frames(), 3);
        assert_eq!(stream.read(&mut []), StreamRead::default());
    }
    #[test]
    fn invalid_blocks_and_seek_preserve_accounting() {
        let mut stream = PcmStream::new(48_000, 2).unwrap();
        stream.push(48_000, &[[0.25; 2]]).unwrap();
        assert_eq!(
            stream.push(44_100, &[[0.5; 2]]),
            Err(AudioError::InvalidFormat)
        );
        assert_eq!(
            stream.push(48_000, &[[f32::NAN; 2]]),
            Err(AudioError::InvalidFormat)
        );
        assert_eq!(stream.buffered_frames(), 1);
        stream.clear();
        let mut output = [[1.0; 2]; 2];
        stream.read(&mut output);
        assert_eq!(output, [[0.0; 2]; 2]);
        stream.clear();
        assert_eq!(stream.missing_frames(), 2);
    }
}
