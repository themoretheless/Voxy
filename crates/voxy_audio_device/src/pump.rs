use crate::{OutputSender, SubmitError};
use voxy_audio::{AudioError, Mixer};
/// Result of one bounded pump call. A stop reason preserves the pending frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PumpReport {
    pub submitted: usize,
    pub stop: Option<SubmitError>,
}
/// Caller-owned PCM renderer plus one reusable output block. Full queues preserve the
/// unsent suffix; mixing resumes only after the previous block is submitted.
/// Renderer changes take effect after already-rendered/queued frames.
#[derive(Debug)]
pub struct PcmPump {
    block: Vec<[f32; 2]>,
    cursor: usize,
}
impl PcmPump {
    /// # Errors
    /// Rejects empty blocks.
    pub fn new(block_frames: usize) -> Result<Self, AudioError> {
        if block_frames == 0 {
            return Err(AudioError::InvalidFormat);
        }
        Ok(Self {
            block: vec![[0.0; 2]; block_frames],
            cursor: block_frames,
        })
    }
    #[must_use]
    pub fn pending_frames(&self) -> usize {
        self.block.len() - self.cursor
    }
    /// Submits at most `budget` frames, never waiting on the output queue.
    /// A zero budget does not render or advance playback. Rate matching remains
    /// the caller's responsibility when constructing the mixer/device pair.
    pub fn pump(
        &mut self,
        sender: &OutputSender,
        budget: usize,
        mut render: impl FnMut(&mut [[f32; 2]]),
    ) -> PumpReport {
        let mut submitted = 0;
        while submitted < budget {
            if self.cursor == self.block.len() {
                render(&mut self.block);
                self.cursor = 0;
            }
            if let Err(error) = sender.submit(self.block[self.cursor]) {
                return PumpReport {
                    submitted,
                    stop: Some(error),
                };
            }
            self.cursor += 1;
            submitted += 1;
        }
        PumpReport {
            submitted,
            stop: None,
        }
    }
}
/// Mixer compatibility wrapper around the shared PCM queue pump.
#[derive(Debug)]
pub struct MixerPump {
    mixer: Mixer,
    pcm: PcmPump,
}
impl MixerPump {
    /// # Errors
    /// Rejects empty output blocks.
    pub fn new(mixer: Mixer, block_frames: usize) -> Result<Self, AudioError> {
        Ok(Self {
            mixer,
            pcm: PcmPump::new(block_frames)?,
        })
    }
    pub fn mixer_mut(&mut self) -> &mut Mixer {
        &mut self.mixer
    }
    #[must_use]
    pub fn pending_frames(&self) -> usize {
        self.pcm.pending_frames()
    }
    pub fn pump(&mut self, sender: &OutputSender, budget: usize) -> PumpReport {
        self.pcm
            .pump(sender, budget, |output| self.mixer.render(output))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use voxy_audio::Clip;
    #[test]
    fn queue_backpressure_preserves_pcm_and_zero_budget_preserves_clock() {
        let mut mixer = Mixer::new(48_000, 1, 1).unwrap();
        mixer
            .play(
                Clip::new(48_000, vec![[0.125; 2], [0.25; 2], [0.5; 2]]).unwrap(),
                0,
                1.0,
                true,
            )
            .unwrap();
        let mut pump = MixerPump::new(mixer, 2).unwrap();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let sender = OutputSender(tx);
        assert_eq!(pump.pump(&sender, 0).submitted, 0);
        assert_eq!(pump.pending_frames(), 0);
        for expected in [0.125_f32, 0.25, 0.5, 0.125, 0.25] {
            let report = pump.pump(&sender, 8);
            assert_eq!(report.submitted, 1);
            assert_eq!(report.stop, Some(SubmitError::Full));
            assert_eq!(
                rx.recv().unwrap().map(f32::to_bits),
                [expected.to_bits(); 2]
            );
        }
        drop(rx);
        let pending = pump.pending_frames();
        assert_eq!(pump.pump(&sender, 1).stop, Some(SubmitError::Closed));
        assert_eq!(pump.pending_frames(), pending);
    }
}
