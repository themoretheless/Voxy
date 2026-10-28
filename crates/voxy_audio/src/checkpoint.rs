//! Codec-neutral mixer state; hosts persist immutable clip revision keys separately.
use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GainCheckpoint {
    pub current: u64,
    pub target: u64,
    pub remaining: u32,
}
impl Gain {
    fn checkpoint(self) -> GainCheckpoint {
        GainCheckpoint {
            current: self.current.to_bits(),
            target: self.target.to_bits(),
            remaining: self.remaining,
        }
    }
}
impl GainCheckpoint {
    fn prepare(&self) -> Result<Gain, AudioError> {
        let current = f64::from_bits(self.current);
        let target = f64::from_bits(self.target);
        if !current.is_finite()
            || !target.is_finite()
            || !(0.0..=1.0).contains(&current)
            || !(0.0..=1.0).contains(&target)
            || (self.remaining == 0 && current.to_bits() != target.to_bits())
        {
            return Err(AudioError::InvalidGain);
        }
        Ok(Gain {
            current,
            target,
            remaining: self.remaining,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceCheckpoint {
    pub serial: u64,
    pub clip: String,
    pub cursor: usize,
    pub bus: usize,
    pub gain: GainCheckpoint,
    pub channels: [GainCheckpoint; 2],
    pub looping: bool,
    pub paused: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MixerCheckpoint {
    pub version: u32,
    pub serial: u64,
    pub rate: u32,
    /// Slot order is retained because floating-point accumulation follows it.
    pub voices: Vec<Option<VoiceCheckpoint>>,
    pub buses: Vec<GainCheckpoint>,
}
impl Mixer {
    /// Stable within this saved mixer; restored VoiceIds use a fresh owner identity.
    /// # Errors
    /// Rejects foreign, finished or stopped voices.
    pub fn voice_checkpoint_key(&self, id: VoiceId) -> Result<u64, AudioError> {
        self.voices
            .iter()
            .flatten()
            .find(|v| v.id == id)
            .map(|v| v.id.serial)
            .ok_or(AudioError::InvalidVoice)
    }
    /// # Errors
    /// Rejects unresolved or conflicting immutable resource revision keys.
    pub fn checkpoint(
        &self,
        mut identify: impl FnMut(&Clip) -> Option<String>,
    ) -> Result<MixerCheckpoint, AudioError> {
        let mut keys: HashMap<String, &Clip> = HashMap::new();
        let mut voices = Vec::with_capacity(self.voices.len());
        for slot in &self.voices {
            let saved = if let Some(v) = slot {
                let clip = identify(&v.clip)
                    .filter(|k| !k.is_empty())
                    .ok_or(AudioError::InvalidFormat)?;
                if let Some(prior) = keys.get(&clip) {
                    if !Arc::ptr_eq(&prior.frames, &v.clip.frames) {
                        return Err(AudioError::InvalidFormat);
                    }
                } else {
                    keys.insert(clip.clone(), &v.clip);
                }
                Some(VoiceCheckpoint {
                    serial: v.id.serial,
                    clip,
                    cursor: v.cursor,
                    bus: v.bus,
                    gain: v.gain.checkpoint(),
                    channels: v.channels.map(Gain::checkpoint),
                    looping: v.looping,
                    paused: v.paused,
                })
            } else {
                None
            };
            voices.push(saved);
        }
        Ok(MixerCheckpoint {
            version: 1,
            serial: self.serial,
            rate: self.rate,
            voices,
            buses: self.buses.iter().copied().map(Gain::checkpoint).collect(),
        })
    }
    /// Captures an allocated reference even after its one-shot voice completed.
    /// # Errors
    /// Rejects foreign or never-allocated references.
    pub fn voice_reference_key(&self, id: VoiceId) -> Result<u64, AudioError> {
        if id.owner != self.owner || id.serial == 0 || id.serial > self.serial {
            return Err(AudioError::InvalidVoice);
        }
        Ok(id.serial)
    }
    /// Rebinds an allocated reference, including a completed voice tombstone.
    /// # Errors
    /// Rejects zero and never-allocated serials.
    pub fn restore_voice_reference(&self, serial: u64) -> Result<VoiceId, AudioError> {
        if serial == 0 || serial > self.serial {
            return Err(AudioError::InvalidVoice);
        }
        Ok(VoiceId {
            owner: self.owner,
            serial,
        })
    }
    /// Resolve a saved serial in this newly restored mixer. Old VoiceIds stay invalid.
    #[must_use]
    pub fn restored_voice(&self, serial: u64) -> Option<VoiceId> {
        self.voices
            .iter()
            .flatten()
            .find(|v| v.id.serial == serial)
            .map(|v| v.id)
    }
}
impl MixerCheckpoint {
    /// Rebuilds an independent mixer with exact cursors, ramps and slot ordering.
    /// The resolver must return the exact saved immutable PCM revision.
    /// # Errors
    /// Rejects resource/format mismatch, invalid state and caller capacity limits.
    pub fn prepare(
        &self,
        mut resolve: impl FnMut(&str) -> Option<Clip>,
        max_voices: usize,
        max_buses: usize,
    ) -> Result<Mixer, AudioError> {
        if self.voices.len() > max_voices || self.buses.len() > max_buses {
            return Err(AudioError::Capacity);
        }
        if self.version != 1 || self.rate == 0 || self.voices.is_empty() || self.buses.is_empty() {
            return Err(AudioError::InvalidFormat);
        }
        let buses = self
            .buses
            .iter()
            .map(GainCheckpoint::prepare)
            .collect::<Result<Vec<_>, _>>()?;
        let mut ids = HashSet::new();
        let mut clips = HashMap::new();
        let mut voices = Vec::with_capacity(self.voices.len());
        for slot in &self.voices {
            let voice = if let Some(v) = slot {
                if v.serial == 0
                    || v.serial > self.serial
                    || !ids.insert(v.serial)
                    || v.clip.is_empty()
                {
                    return Err(AudioError::InvalidVoice);
                }
                if v.bus >= buses.len() {
                    return Err(AudioError::InvalidBus);
                }
                if !clips.contains_key(&v.clip) {
                    clips.insert(
                        v.clip.clone(),
                        resolve(&v.clip).ok_or(AudioError::InvalidFormat)?,
                    );
                }
                let clip: Clip = clips.get(&v.clip).ok_or(AudioError::InvalidFormat)?.clone();
                if clip.rate != self.rate || v.cursor >= clip.frames.len() {
                    return Err(AudioError::InvalidFormat);
                }
                Some(Voice {
                    id: VoiceId {
                        owner: 0,
                        serial: v.serial,
                    },
                    clip,
                    cursor: v.cursor,
                    bus: v.bus,
                    gain: v.gain.prepare()?,
                    channels: [v.channels[0].prepare()?, v.channels[1].prepare()?],
                    looping: v.looping,
                    paused: v.paused,
                })
            } else {
                None
            };
            voices.push(voice);
        }
        let mut mixer = Mixer::new(self.rate, self.voices.len(), buses.len())?;
        for voice in voices.iter_mut().flatten() {
            voice.id.owner = mixer.owner;
        }
        mixer.serial = self.serial;
        mixer.voices = voices;
        mixer.buses = buses;
        Ok(mixer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_pcm_continuation_preserves_ramps_pauses_holes_and_new_identity() {
        let clip = Clip::new(
            48000,
            (0..97)
                .map(|i| {
                    let x = f32::from(i as u16) / 200.;
                    [x, -x]
                })
                .collect(),
        )
        .unwrap();
        let mut original = Mixer::new(48000, 4, 2).unwrap();
        let dead = original.play(clip.clone(), 0, 0.2, true).unwrap();
        let active = original.play(clip.clone(), 1, 0.3, true).unwrap();
        let paused = original.play(clip.clone(), 0, 0.4, false).unwrap();
        original.stop(dead).unwrap();
        original.ramp_voice_gain(active, 0.9, 111).unwrap();
        original
            .ramp_channel_gains(active, [0.13, 0.71], 137)
            .unwrap();
        original.ramp_bus_gain(1, 0.17, 151).unwrap();
        original.pause(paused, true).unwrap();
        original.render(&mut [[0.; 2]; 19]);
        let saved = original.checkpoint(|_| Some("pcm-rev-1".into())).unwrap();
        let mut restored = saved.prepare(|_| Some(clip.clone()), 4, 2).unwrap();
        assert!(restored.pause(active, true).is_err());
        let new_paused = restored
            .restored_voice(original.voice_checkpoint_key(paused).unwrap())
            .unwrap();
        for block in 0..120 {
            if block == 3 {
                original.pause(paused, false).unwrap();
                restored.pause(new_paused, false).unwrap();
            }
            let mut a = vec![[0.; 2]; [1, 7, 31, 0][block % 4]];
            let mut b = a.clone();
            original.render(&mut a);
            restored.render(&mut b);
            assert_eq!(
                a.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>(),
                b.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
            assert_eq!(
                original.checkpoint(|_| Some("pcm-rev-1".into())).unwrap(),
                restored.checkpoint(|_| Some("pcm-rev-1".into())).unwrap()
            );
        }
        let mut bad = saved.clone();
        bad.voices[1].as_mut().unwrap().cursor = clip.frame_count();
        assert!(bad.prepare(|_| Some(clip.clone()), 4, 2).is_err());
        assert!(saved.prepare(|_| None, 4, 2).is_err());
        assert!(saved.prepare(|_| Some(clip.clone()), 3, 2).is_err());
        let mut bad = saved;
        bad.buses[0].current = f64::NAN.to_bits();
        assert!(bad.prepare(|_| Some(clip.clone()), 4, 2).is_err());
    }
}
