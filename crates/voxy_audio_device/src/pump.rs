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
/// Already rendered PCM and the submission cursor. The device queue is separate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcmPumpCheckpoint {
    pub block: Vec<[u32; 2]>,
    pub cursor: usize,
}
impl PcmPumpCheckpoint {
    /// # Errors
    /// Rejects capacity, malformed cursors and invalid normalized PCM.
    pub fn prepare(&self, max_block_frames: usize) -> Result<PcmPump, AudioError> {
        if self.block.len() > max_block_frames {
            return Err(AudioError::Capacity);
        }
        if self.block.is_empty() || self.cursor > self.block.len() {
            return Err(AudioError::InvalidFormat);
        }
        let block: Vec<_> = self
            .block
            .iter()
            .map(|frame| frame.map(f32::from_bits))
            .collect();
        if block
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
        {
            return Err(AudioError::InvalidFormat);
        }
        Ok(PcmPump {
            block,
            cursor: self.cursor,
        })
    }
}
impl PcmPump {
    pub(crate) fn block_frames(&self) -> usize {
        self.block.len()
    }

    #[must_use]
    pub fn checkpoint(&self) -> PcmPumpCheckpoint {
        PcmPumpCheckpoint {
            block: self
                .block
                .iter()
                .map(|frame| frame.map(f32::to_bits))
                .collect(),
            cursor: self.cursor,
        }
    }
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
            let count = (budget - submitted).min(self.block.len() - self.cursor);
            let report = sender.submit_frames(&self.block[self.cursor..self.cursor + count]);
            self.cursor += report.submitted;
            submitted += report.submitted;
            if report.stop.is_some() {
                return PumpReport {
                    submitted,
                    stop: report.stop,
                };
            }
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
        let (sender, rx) = crate::queue::output_queue(1).unwrap();
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

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    #[test]
    fn unsent_pcm_resumes_after_backpressure_without_rendering_it_twice() {
        let (sender, rx) = crate::queue::output_queue(1).unwrap();
        let mut original = PcmPump::new(4).unwrap();
        let mut rendered = 0_u16;
        let render = |block: &mut [[f32; 2]], next: &mut u16| {
            for frame in block {
                *frame = [f32::from(*next) / 1000.; 2];
                *next += 1;
            }
        };
        let first = original.pump(&sender, 8, |b| render(b, &mut rendered));
        assert_eq!(first.submitted, 1);
        assert_eq!(first.stop, Some(SubmitError::Full));
        rx.recv().unwrap(); // Already submitted device frames are a separate owner.
        let saved = original.checkpoint();
        assert_eq!(original.pending_frames(), 3);
        let mut restored = saved.prepare(4).unwrap();
        let mut restored_rendered = rendered;
        let (sender2, rx2) = crate::queue::output_queue(1).unwrap();
        for _ in 0..120 {
            assert_eq!(
                original.pump(&sender, 8, |b| render(b, &mut rendered)),
                restored.pump(&sender2, 8, |b| render(b, &mut restored_rendered))
            );
            assert_eq!(
                rx.recv().unwrap().map(f32::to_bits),
                rx2.recv().unwrap().map(f32::to_bits)
            );
            assert_eq!(original.checkpoint(), restored.checkpoint());
            assert_eq!(rendered, restored_rendered);
        }
        assert!(saved.prepare(3).is_err());
        let mut bad = saved.clone();
        bad.cursor = 5;
        assert!(bad.prepare(4).is_err());
        let mut bad = saved;
        bad.block[1][0] = f32::NAN.to_bits();
        assert!(bad.prepare(4).is_err());
    }
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    // Original single-frame pump algorithm, kept as a semantic control.
    fn reference(
        pump: &mut PcmPump,
        sender: &OutputSender,
        budget: usize,
        mut render: impl FnMut(&mut [[f32; 2]]),
    ) -> PumpReport {
        let mut submitted = 0;
        while submitted < budget {
            if pump.cursor == pump.block.len() {
                render(&mut pump.block);
                pump.cursor = 0;
            }
            if let Err(error) = sender.submit(pump.block[pump.cursor]) {
                return PumpReport {
                    submitted,
                    stop: Some(error),
                };
            }
            pump.cursor += 1;
            submitted += 1;
        }
        PumpReport {
            submitted,
            stop: None,
        }
    }
    #[test]
    fn batched_pump_matches_original_cursor_clock_and_pcm_for_720_irregular_callbacks() {
        for capacity in [1, 5, 128] {
            let (old_sender, old_receiver) = crate::queue::output_queue(capacity).unwrap();
            let (new_sender, new_receiver) = crate::queue::output_queue(capacity).unwrap();
            let mut old = PcmPump::new(7).unwrap();
            let mut new = PcmPump::new(7).unwrap();
            let mut old_clock = 0_u32;
            let mut new_clock = 0_u32;
            let render = |block: &mut [[f32; 2]], clock: &mut u32| {
                for frame in block {
                    *frame = [f32::from_bits(0x3e00_0000 + *clock), -0.0];
                    *clock += 1;
                }
            };
            for index in 0..720 {
                let budget = index % 29;
                assert_eq!(
                    reference(&mut old, &old_sender, budget, |b| render(b, &mut old_clock)),
                    new.pump(&new_sender, budget, |b| render(b, &mut new_clock))
                );
                let mut a = [0.0_f32; 16];
                let mut b = [0.0_f32; 16];
                assert_eq!(old_receiver.render(&mut a), new_receiver.render(&mut b));
                assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
                assert_eq!(old.checkpoint(), new.checkpoint());
                assert_eq!(old_clock, new_clock);
            }
        }
    }
}
