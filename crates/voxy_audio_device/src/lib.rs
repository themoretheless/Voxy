//! Platform output adapter. Bounded lock-free PCM queue; no hard realtime guarantee.
mod queue;
pub use queue::{
    FrozenOutputQueue, OutputQueueCheckpoint, PreparedOutputQueue, QueueCheckpointError,
};
mod worker;
pub use worker::{OutputConnection, OutputDeviceWorker};
mod pump;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
pub use pump::{MixerPump, PcmPump, PcmPumpCheckpoint, PumpReport};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[derive(Debug, Default)]
struct Counters {
    supplied: AtomicU64,
    missing: AtomicU64,
    errors: AtomicU64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputStats {
    pub supplied: u64,
    pub missing: u64,
    pub errors: u64,
}
impl OutputStats {
    /// Returns the fraction of requested callback samples that suffered an underrun (`[0.0, 1.0]`).
    #[must_use]
    pub fn underrun_ratio(self) -> f64 {
        let total = self.supplied.saturating_add(self.missing);
        if total == 0 {
            0.0
        } else {
            (self.missing as f64) / (total as f64)
        }
    }

    /// Recommends an updated ring-buffer capacity by scaling up on underruns (`missing > 0`)
    /// while respecting `[min_capacity, max_capacity]`.
    #[must_use]
    pub fn recommended_queue_capacity(
        self,
        current_capacity: usize,
        min_capacity: usize,
        max_capacity: usize,
    ) -> usize {
        let lo = min_capacity.max(1);
        let hi = max_capacity.max(lo);
        let base = current_capacity.clamp(lo, hi);
        if self.missing > 0 {
            base.saturating_mul(2).clamp(lo, hi)
        } else {
            base
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    InvalidFrame,
    Full,
    Closed,
}
/// Keeps the native stream alive; dropping closes it and disconnects producers.
/// Only the default stereo f32 configuration is currently supported.
pub struct OutputDevice {
    stream: cpal::Stream,
    rate: u32,
    counters: Arc<Counters>,
}
impl std::fmt::Debug for OutputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputDevice")
            .field("rate", &self.rate)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug)]
pub struct OutputSender(Arc<queue::Queue>);
impl OutputSender {
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Submits an exact prefix while sharing one checkpoint admission across a block.
    /// On backpressure/error the caller retains the unsent suffix for retry.
    #[must_use]
    pub fn submit_frames(&self, frames: &[[f32; 2]]) -> PumpReport {
        self.0.submit_frames(frames)
    }
    /// Nonblocking single-frame submission. Retain and retry on `Full`.
    /// # Errors
    /// Rejects invalid PCM, full queues or disconnected output devices.
    pub fn submit(&self, frame: [f32; 2]) -> Result<(), SubmitError> {
        if frame
            .iter()
            .any(|x| !x.is_finite() || !(-1.0..=1.0).contains(x))
        {
            return Err(SubmitError::InvalidFrame);
        }
        self.0.submit(frame)
    }
}
impl OutputDevice {
    /// Creates a paused stream and bounded frame queue.
    /// # Errors
    /// Rejects zero capacity, absent device, unsupported default format, and host errors.
    pub fn open(capacity: usize) -> Result<(Self, OutputSender), Box<dyn std::error::Error>> {
        if capacity == 0 {
            return Err("zero output capacity".into());
        }
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no output device")?;
        let config = device.default_output_config()?;
        if config.channels() != 2 || config.sample_format() != cpal::SampleFormat::F32 {
            return Err("default output must be stereo f32".into());
        }
        let rate = config.sample_rate().0;
        let (sender, receiver) = queue::output_queue(capacity)?;
        let counters = Arc::new(Counters::default());
        let render_counters = Arc::clone(&counters);
        let error_counters = Arc::clone(&counters);
        let stream = device.build_output_stream(
            &config.into(),
            move |output: &mut [f32], _| {
                let (supplied, missing) = receiver.render(output);
                render_counters
                    .supplied
                    .fetch_add(supplied, Ordering::Relaxed);
                render_counters
                    .missing
                    .fetch_add(missing, Ordering::Relaxed);
            },
            move |_| {
                error_counters.errors.fetch_add(1, Ordering::Relaxed);
            },
            None,
        )?;
        Ok((
            Self {
                stream,
                rate,
                counters,
            },
            sender,
        ))
    }
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }
    #[must_use]
    pub fn stats(&self) -> OutputStats {
        OutputStats {
            supplied: self.counters.supplied.load(Ordering::Relaxed),
            missing: self.counters.missing.load(Ordering::Relaxed),
            errors: self.counters.errors.load(Ordering::Relaxed),
        }
    }
    /// # Errors
    /// Returns the host start failure.
    pub fn start(&self) -> Result<(), cpal::PlayStreamError> {
        self.stream.play()
    }
    /// Pauses the stream on its native owner thread without replacing the application queue.
    /// # Errors
    /// Returns the host pause failure; callers must not assume a fence on failure.
    pub fn pause(&self) -> Result<(), cpal::PauseStreamError> {
        self.stream.pause()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_submission_rejects_full_invalid_and_closed() {
        let (sender, rx) = queue::output_queue(1).unwrap();
        assert_eq!(sender.submit([f32::NAN; 2]), Err(SubmitError::InvalidFrame));
        assert_eq!(sender.submit([0.25; 2]), Ok(()));
        assert_eq!(sender.submit([0.5; 2]), Err(SubmitError::Full));
        assert_eq!(
            rx.recv().unwrap().map(f32::to_bits),
            [0.25_f32.to_bits(); 2]
        );
        drop(rx);
        assert_eq!(sender.submit([0.0; 2]), Err(SubmitError::Closed));
    }
    #[test]
    fn output_stats_underrun_ratio_and_adaptive_capacity() {
        let stats = OutputStats {
            supplied: 900,
            missing: 100,
            errors: 0,
        };
        assert!((stats.underrun_ratio() - 0.1).abs() < 1e-12);
        assert_eq!(stats.recommended_queue_capacity(1024, 512, 4096), 2048);
        let clean = OutputStats {
            supplied: 1000,
            missing: 0,
            errors: 0,
        };
        assert_eq!(clean.underrun_ratio(), 0.0);
        assert_eq!(clean.recommended_queue_capacity(1024, 512, 4096), 1024);
    }
}

mod output_checkpoint;
pub use output_checkpoint::{
    OfflineOutput, OutputCheckpoint, OutputCheckpointLimits, PreparedOutputCheckpoint,
    ReadyOutputPublication,
};

/// Applies a master stereo soft-clipping `tanh` saturator to `[left, right]` with pre-gain `drive`,
/// guaranteeing finite samples strictly within `[-1.0, 1.0]`.
#[must_use]
pub fn soft_clip_tanh_stereo(left: f32, right: f32, drive: f32) -> [f32; 2] {
    let g = if drive.is_finite() && drive > 0.0 {
        drive
    } else {
        1.0
    };
    let sat = |x: f32| -> f32 {
        if !x.is_finite() {
            0.0
        } else {
            (x * g).tanh().clamp(-1.0, 1.0)
        }
    };
    [sat(left), sat(right)]
}

#[cfg(test)]
mod limiter_tests {
    use super::*;

    #[test]
    fn soft_clip_tanh_stereo_bounds_overdriven_signals() {
        let out = soft_clip_tanh_stereo(3.0, -3.0, 1.5);
        assert!(out[0] > 0.99 && out[0] <= 1.0);
        assert!(out[1] < -0.99 && out[1] >= -1.0);
        assert_eq!(soft_clip_tanh_stereo(f32::NAN, 0.0, 1.0), [0.0, 0.0]);
    }
}
