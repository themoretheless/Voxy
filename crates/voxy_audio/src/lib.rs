//! Owner-thread PCM mixer. Device I/O and decoding belong to separate adapters.
mod pcm_archive;
mod resample;
mod stream;
pub use stream::{PcmStream, StreamRead};
mod wav;
pub use wav::decode_wav;
mod spatial;
pub use spatial::spatial_gains;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
static NEXT_MIXER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioError {
    InvalidFormat,
    InvalidGain,
    InvalidSpatial,
    Capacity,
    InvalidVoice,
    InvalidBus,
    Exhausted,
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "audio error: {self:?}")
    }
}
impl std::error::Error for AudioError {}

/// Immutable stereo PCM at an explicit sample rate; samples must be finite and within [-1, 1].
#[derive(Clone, Debug)]
pub struct Clip {
    rate: u32,
    frames: Arc<[[f32; 2]]>,
}
impl Clip {
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// # Errors
    /// Rejects zero rate, empty clips, and samples outside finite [-1, 1].
    pub fn new(rate: u32, frames: Vec<[f32; 2]>) -> Result<Self, AudioError> {
        if rate == 0
            || frames.is_empty()
            || frames
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || !(-1.0..=1.0).contains(x))
        {
            return Err(AudioError::InvalidFormat);
        }
        Ok(Self {
            rate,
            frames: frames.into(),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceId {
    owner: u64,
    serial: u64,
}
#[derive(Clone, Copy, Debug)]
struct Gain {
    current: f64,
    target: f64,
    remaining: u32,
}
impl Gain {
    fn new(value: f32) -> Self {
        Self {
            current: f64::from(value),
            target: f64::from(value),
            remaining: 0,
        }
    }
    fn retarget(&mut self, value: f32, frames: u32) {
        self.target = f64::from(value);
        self.remaining = frames;
        if frames == 0 {
            self.current = self.target;
        }
    }
    fn advance(&mut self) {
        if self.remaining > 0 {
            self.current += (self.target - self.current) / f64::from(self.remaining);
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
            }
        }
    }
}
#[derive(Debug)]
struct Voice {
    id: VoiceId,
    clip: Clip,
    cursor: usize,
    bus: usize,
    gain: Gain,
    channels: [Gain; 2],
    looping: bool,
    paused: bool,
}

/// Fixed voice capacity and fixed bus count. Render performs no allocations.
/// Summed output is clamped to [-1, 1]; this is not a dynamics limiter.
#[derive(Debug)]
pub struct Mixer {
    owner: u64,
    serial: u64,
    rate: u32,
    voices: Vec<Option<Voice>>,
    buses: Vec<Gain>,
}
impl Mixer {
    /// # Errors
    /// Rejects empty configuration or exhausted process identity space.
    pub fn new(rate: u32, voices: usize, buses: usize) -> Result<Self, AudioError> {
        if rate == 0 || voices == 0 || buses == 0 {
            return Err(AudioError::InvalidFormat);
        }
        let owner = NEXT_MIXER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .map_err(|_| AudioError::Exhausted)?;
        Ok(Self {
            owner,
            serial: 0,
            rate,
            voices: (0..voices).map(|_| None).collect(),
            buses: vec![Gain::new(1.0); buses],
        })
    }
    /// # Errors
    /// Rejects invalid gain, bus, mismatched sample rate or full voice capacity.
    pub fn play(
        &mut self,
        clip: Clip,
        bus: usize,
        gain: f32,
        looping: bool,
    ) -> Result<VoiceId, AudioError> {
        valid_gain(gain)?;
        if bus >= self.buses.len() {
            return Err(AudioError::InvalidBus);
        }
        if clip.rate != self.rate {
            return Err(AudioError::InvalidFormat);
        }
        let slot = self
            .voices
            .iter_mut()
            .find(|x| x.is_none())
            .ok_or(AudioError::Capacity)?;
        let serial = self.serial.checked_add(1).ok_or(AudioError::Exhausted)?;
        self.serial = serial;
        let id = VoiceId {
            owner: self.owner,
            serial,
        };
        *slot = Some(Voice {
            id,
            clip,
            cursor: 0,
            bus,
            gain: Gain::new(gain),
            channels: [Gain::new(1.0); 2],
            looping,
            paused: false,
        });
        Ok(id)
    }
    /// # Errors
    /// Rejects foreign, stopped or finished voices.
    pub fn stop(&mut self, id: VoiceId) -> Result<(), AudioError> {
        let slot = self
            .voices
            .iter_mut()
            .find(|v| v.as_ref().is_some_and(|v| v.id == id))
            .ok_or(AudioError::InvalidVoice)?;
        *slot = None;
        Ok(())
    }
    /// # Errors
    /// Rejects foreign, stopped or finished voices.
    pub fn pause(&mut self, id: VoiceId, paused: bool) -> Result<(), AudioError> {
        let voice = self
            .voices
            .iter_mut()
            .flatten()
            .find(|v| v.id == id)
            .ok_or(AudioError::InvalidVoice)?;
        voice.paused = paused;
        Ok(())
    }
    /// # Errors
    /// Rejects unknown buses and gains outside [0, 1].
    pub fn set_bus_gain(&mut self, bus: usize, gain: f32) -> Result<(), AudioError> {
        valid_gain(gain)?;
        self.buses
            .get_mut(bus)
            .ok_or(AudioError::InvalidBus)?
            .retarget(gain, 0);
        Ok(())
    }
    /// Linearly ramps a bus gain over output frames, including silent frames.
    /// Zero duration sets the gain immediately; retargeting starts at current gain.
    /// # Errors
    /// Rejects unknown buses or invalid gains without changing the ramp.
    pub fn ramp_bus_gain(&mut self, bus: usize, gain: f32, frames: u32) -> Result<(), AudioError> {
        valid_gain(gain)?;
        self.buses
            .get_mut(bus)
            .ok_or(AudioError::InvalidBus)?
            .retarget(gain, frames);
        Ok(())
    }
    /// Ramps voice gain over playing frames. Pausing freezes this ramp.
    /// # Errors
    /// Rejects invalid gains and foreign, stopped or finished voices.
    pub fn ramp_voice_gain(
        &mut self,
        id: VoiceId,
        gain: f32,
        frames: u32,
    ) -> Result<(), AudioError> {
        valid_gain(gain)?;
        self.voices
            .iter_mut()
            .flatten()
            .find(|v| v.id == id)
            .ok_or(AudioError::InvalidVoice)?
            .gain
            .retarget(gain, frames);
        Ok(())
    }
    /// Applies stereo gains, typically from `spatial_gains` for a dual-mono clip.
    /// Each channel ramps independently; pausing freezes both ramps.
    /// # Errors
    /// Rejects either invalid gain or invalid voice without partial changes.
    pub fn ramp_channel_gains(
        &mut self,
        id: VoiceId,
        gains: [f32; 2],
        frames: u32,
    ) -> Result<(), AudioError> {
        for gain in gains {
            valid_gain(gain)?;
        }
        let voice = self
            .voices
            .iter_mut()
            .flatten()
            .find(|v| v.id == id)
            .ok_or(AudioError::InvalidVoice)?;
        for (channel, gain) in voice.channels.iter_mut().zip(gains) {
            channel.retarget(gain, frames);
        }
        Ok(())
    }
    /// Overwrites caller-owned stereo output. Muted voices still advance;
    /// paused voices preserve their cursor. Finished voices release capacity.
    /// The first output frame applies the first step of each active gain ramp.
    pub fn render(&mut self, output: &mut [[f32; 2]]) {
        for frame in output {
            for bus in &mut self.buses {
                bus.advance();
            }
            let mut sum = [0.0_f64; 2];
            for slot in &mut self.voices {
                let Some(voice) = slot.as_mut() else { continue };
                if voice.paused {
                    continue;
                }
                voice.gain.advance();
                let gain = voice.gain.current * self.buses[voice.bus].current;
                let sample = voice.clip.frames[voice.cursor];
                for channel in 0..2 {
                    voice.channels[channel].advance();
                    sum[channel] +=
                        f64::from(sample[channel]) * gain * voice.channels[channel].current;
                }
                voice.cursor += 1;
                if voice.cursor == voice.clip.frames.len() {
                    if voice.looping {
                        voice.cursor = 0;
                    } else {
                        *slot = None;
                    }
                }
            }
            #[allow(clippy::cast_possible_truncation)] // Clamped normalized output.
            for channel in 0..2 {
                frame[channel] = sum[channel].clamp(-1.0, 1.0) as f32;
            }
        }
    }
}
fn valid_gain(gain: f32) -> Result<(), AudioError> {
    if gain.is_finite() && (0.0..=1.0).contains(&gain) {
        Ok(())
    } else {
        Err(AudioError::InvalidGain)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact PCM copy, zero and saturation are contractual here.
mod tests {
    use super::*;
    fn clip(frames: Vec<[f32; 2]>) -> Clip {
        Clip::new(48_000, frames).unwrap()
    }
    #[test]
    fn spatial_gains_route_mono_voice_without_affecting_bus() {
        let mut mixer = Mixer::new(48_000, 1, 1).unwrap();
        let id = mixer.play(clip(vec![[1.0; 2]]), 0, 1.0, true).unwrap();
        let gains = spatial_gains([0.0; 3], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0, 3.0).unwrap();
        mixer.ramp_channel_gains(id, gains, 0).unwrap();
        assert_eq!(
            mixer.ramp_channel_gains(id, [0.5, f32::NAN], 0),
            Err(AudioError::InvalidGain)
        );
        let mut out = [[0.0; 2]; 1];
        mixer.render(&mut out);
        assert_eq!(out[0], [0.0, 1.0]);
    }
    #[test]
    fn ramps_retarget_and_pause_on_sample_clock() {
        let pcm = clip(vec![[1.0; 2]]);
        let mut mixer = Mixer::new(48_000, 1, 1).unwrap();
        let id = mixer.play(pcm, 0, 1.0, true).unwrap();
        mixer.ramp_bus_gain(0, 0.0, 4).unwrap();
        let mut first = [[0.0; 2]; 2];
        mixer.render(&mut first);
        assert_eq!(first, [[0.75; 2], [0.5; 2]]);
        assert_eq!(
            mixer.ramp_bus_gain(0, f32::NAN, 1),
            Err(AudioError::InvalidGain)
        );
        mixer.ramp_bus_gain(0, 1.0, 2).unwrap();
        mixer.ramp_voice_gain(id, 0.0, 2).unwrap();
        mixer.pause(id, true).unwrap();
        let mut silent = [[1.0; 2]; 2];
        mixer.render(&mut silent);
        assert_eq!(silent, [[0.0; 2]; 2]);
        mixer.pause(id, false).unwrap();
        mixer.render(&mut first);
        assert_eq!(first, [[0.5; 2], [0.0; 2]]);
        mixer.ramp_voice_gain(id, 1.0, 0).unwrap();
        mixer.render(&mut first);
        assert_eq!(first, [[1.0; 2]; 2]);
    }
    #[test]
    fn ramp_results_are_block_partition_independent() {
        let mut a = Mixer::new(48_000, 1, 1).unwrap();
        let mut b = Mixer::new(48_000, 1, 1).unwrap();
        for mixer in [&mut a, &mut b] {
            let id = mixer.play(clip(vec![[0.8; 2]]), 0, 1.0, true).unwrap();
            mixer.ramp_bus_gain(0, 0.25, 7).unwrap();
            mixer.ramp_voice_gain(id, 0.5, 11).unwrap();
        }
        let mut whole = [[0.0; 2]; 16];
        let mut split = whole;
        a.render(&mut whole);
        b.render(&mut split[..3]);
        b.render(&mut []);
        b.render(&mut split[3..9]);
        b.render(&mut split[9..]);
        assert_eq!(whole, split);
    }
    #[test]
    fn rendering_is_independent_of_block_partition() {
        let pcm = clip(vec![[0.125, -0.25], [0.5, -0.5], [1.0, -1.0]]);
        let mut whole = Mixer::new(48_000, 1, 1).unwrap();
        let mut split = Mixer::new(48_000, 1, 1).unwrap();
        whole.play(pcm.clone(), 0, 0.5, true).unwrap();
        split.play(pcm, 0, 0.5, true).unwrap();
        let mut expected = [[0.0; 2]; 11];
        let mut actual = [[0.0; 2]; 11];
        whole.render(&mut expected);
        split.render(&mut actual[..2]);
        split.render(&mut []);
        split.render(&mut actual[2..7]);
        split.render(&mut actual[7..]);
        assert_eq!(actual, expected);
    }
    #[test]
    fn chunk_boundaries_pause_loop_and_completion() {
        let mut mixer = Mixer::new(48_000, 1, 1).unwrap();
        let id = mixer
            .play(clip(vec![[0.2, 0.4], [0.6, 0.8]]), 0, 1.0, false)
            .unwrap();
        let mut out = [[0.0; 2]; 1];
        mixer.render(&mut out);
        assert_eq!(out[0], [0.2, 0.4]);
        mixer.pause(id, true).unwrap();
        mixer.render(&mut out);
        assert_eq!(out[0], [0.0; 2]);
        mixer.pause(id, false).unwrap();
        mixer.render(&mut out);
        assert_eq!(out[0], [0.6, 0.8]);
        assert_eq!(mixer.stop(id), Err(AudioError::InvalidVoice));
        let next = mixer.play(clip(vec![[0.25; 2]]), 0, 1.0, true).unwrap();
        assert_ne!(id, next);
        let mut out = [[0.0; 2]; 3];
        mixer.render(&mut out);
        assert_eq!(out, [[0.25; 2]; 3]);
    }
    #[test]
    fn buses_capacity_foreign_handles_and_invalid_inputs() {
        let mut mixer = Mixer::new(48_000, 2, 2).unwrap();
        let pcm = clip(vec![[0.75; 2]; 2]);
        let id = mixer.play(pcm.clone(), 0, 1.0, false).unwrap();
        mixer.play(pcm.clone(), 1, 1.0, false).unwrap();
        assert_eq!(
            mixer.play(pcm.clone(), 0, 1.0, false),
            Err(AudioError::Capacity)
        );
        let mut other = Mixer::new(48_000, 1, 1).unwrap();
        assert_eq!(other.stop(id), Err(AudioError::InvalidVoice));
        assert_eq!(
            mixer.set_bus_gain(0, f32::NAN),
            Err(AudioError::InvalidGain)
        );
        mixer.set_bus_gain(1, 0.0).unwrap();
        let mut out = [[0.0; 2]; 3];
        mixer.render(&mut out);
        assert_eq!(out, [[0.75; 2], [0.75; 2], [0.0; 2]]);
        mixer.play(pcm.clone(), 0, 1.0, false).unwrap();
        mixer.play(pcm, 1, 1.0, false).unwrap();
        mixer.set_bus_gain(1, 1.0).unwrap();
        mixer.render(&mut out);
        assert_eq!(out[0], [1.0; 2]);
        assert!(Clip::new(0, vec![[0.0; 2]]).is_err());
        assert!(Clip::new(48_000, vec![[f32::INFINITY; 2]]).is_err());
    }
}
