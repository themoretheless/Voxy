//! Platform output adapter. Bounded standard channel; no hard realtime guarantee.
mod worker;
pub use worker::{OutputConnection, OutputDeviceWorker};
mod pump;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
pub use pump::{MixerPump, PcmPump, PumpReport};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, SyncSender, TrySendError},
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
pub struct OutputSender(SyncSender<[f32; 2]>);
impl OutputSender {
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
        self.0.try_send(frame).map_err(|error| match error {
            TrySendError::Full(_) => SubmitError::Full,
            TrySendError::Disconnected(_) => SubmitError::Closed,
        })
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
        let (sender, receiver) = mpsc::sync_channel::<[f32; 2]>(capacity);
        let counters = Arc::new(Counters::default());
        let render_counters = Arc::clone(&counters);
        let error_counters = Arc::clone(&counters);
        let stream = device.build_output_stream(
            &config.into(),
            move |output: &mut [f32], _| {
                let mut supplied = 0;
                let mut missing = 0;
                for frame in output.chunks_exact_mut(2) {
                    if let Ok(sample) = receiver.try_recv() {
                        frame.copy_from_slice(&sample);
                        supplied += 1;
                    } else {
                        frame.fill(0.0);
                        missing += 1;
                    }
                }
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
            OutputSender(sender),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_submission_rejects_full_invalid_and_closed() {
        let (tx, rx) = mpsc::sync_channel(1);
        let sender = OutputSender(tx);
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
}
