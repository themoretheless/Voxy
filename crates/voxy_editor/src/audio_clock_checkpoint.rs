//! Presentation audio sample clock; device queues and mixer state are other participants.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AudioClockCheckpoint {
    version: u32,
    rate: u32,
    remainder: u64,
    frames: u64,
    peak: u32,
}
impl AudioClockCheckpoint {
    pub(super) fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn validate(&self) -> Result<(), EditorError> {
        let peak = f32::from_bits(self.peak);
        if self.version != 1
            || self.rate == 0
            || self.remainder >= 60
            || !peak.is_finite()
            || !(0.0..=1.0).contains(&peak)
        {
            return Err("invalid audio clock checkpoint".into());
        }
        Ok(())
    }
    pub(super) fn sample_budget(&self, steps: usize) -> Result<(usize, u64), EditorError> {
        self.validate()?;
        let numerator = u64::try_from(steps)?
            .checked_mul(u64::from(self.rate))
            .and_then(|n| n.checked_add(self.remainder))
            .ok_or("audio step overflow")?;
        Ok((usize::try_from(numerator / 60)?, numerator % 60))
    }
}
impl AudioPlay {
    pub(super) fn checkpoint_clock(&self) -> AudioClockCheckpoint {
        AudioClockCheckpoint {
            version: 1,
            rate: self.rate,
            remainder: self.remainder,
            frames: self.frames,
            peak: self.peak.to_bits(),
        }
    }
    /// Stages validation before changing any clock field. Does not restore the mixer.
    #[allow(dead_code)]
    pub(super) fn restore_clock(
        &mut self,
        saved: &AudioClockCheckpoint,
    ) -> Result<(), EditorError> {
        saved.validate()?;
        if saved.rate != self.rate {
            return Err("audio checkpoint output rate mismatch".into());
        }
        self.remainder = saved.remainder;
        self.frames = saved.frames;
        self.peak = f32::from_bits(saved.peak);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn audio() -> AudioPlay {
        AudioPlay {
            catalog: AssetCatalog::new(128, 128).unwrap(),
            runtime: SceneAudioRuntime::new(44101, 128, 16).unwrap(),
            reload: None,
            output: None,
            offline_output: None,
            rate: 44101,
            remainder: 0,
            frames: 0,
            peak: 0.,
        }
    }
    #[test]
    fn fractional_rate_offline_continuation_and_corrupt_restore_are_exact() {
        let scene = SceneGraph::new(1);
        let mut original = audio();
        original.advance(&scene, 7).unwrap();
        assert_ne!(original.remainder, 0);
        assert_eq!(original.frames, 7 * 44101 / 60);
        let saved = original.checkpoint_clock();
        let json = serde_json::to_string(&saved).unwrap();
        let decoded = serde_json::from_str(&json).unwrap();
        let mut restored = audio();
        restored.restore_clock(&decoded).unwrap();
        for step in 0..720 {
            let count = [0, 1, 2, 5][step % 4];
            original.advance(&scene, count).unwrap();
            restored.advance(&scene, count).unwrap();
            assert_eq!(original.checkpoint_clock(), restored.checkpoint_clock());
        }
        let before = restored.checkpoint_clock();
        for bad in [
            AudioClockCheckpoint {
                remainder: 60,
                ..saved.clone()
            },
            AudioClockCheckpoint {
                peak: f32::NAN.to_bits(),
                ..saved.clone()
            },
            AudioClockCheckpoint {
                rate: 48000,
                ..saved.clone()
            },
            AudioClockCheckpoint {
                version: 2,
                ..saved
            },
        ] {
            assert!(restored.restore_clock(&bad).is_err());
            assert_eq!(restored.checkpoint_clock(), before);
        }
    }
}
