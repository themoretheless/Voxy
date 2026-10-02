//! Owns playback preparation and the native audio device lifecycle.
use super::audio_play;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AudioOutputMode {
    Offline,
    Native,
}

#[derive(Debug)]
pub(super) struct AudioSession {
    play: Option<audio_play::AudioPlay>,
    output_mode: AudioOutputMode,
    device: Option<voxy_audio_device::OutputDeviceWorker>,
    connection: Option<voxy_audio_device::OutputConnection>,
    pending_play: Option<Instant>,
    preparation: Option<audio_play::AudioPreparation>,
    prepared: Option<audio_play::AudioPlay>,
    retired: Vec<std::thread::JoinHandle<()>>,
}
impl Default for AudioSession {
    fn default() -> Self {
        Self {
            play: None,
            output_mode: AudioOutputMode::Offline,
            device: None,
            connection: None,
            pending_play: None,
            preparation: None,
            prepared: None,
            retired: Vec::new(),
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
enum DevicePoll {
    Pending,
    Connected,
    Stopped,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum AudioPoll {
    Pending,
    Ready,
    RetryPlay,
}
pub(super) enum AudioStart {
    Pending,
    Ready(Option<audio_play::AudioPlay>),
}
impl AudioSession {
    /// Poll the owned device without changing the editor play state.
    fn poll_device(&mut self) -> Result<DevicePoll, Box<dyn std::error::Error>> {
        if self.connection.is_none()
            && let Some(worker) = &mut self.device
        {
            match worker.poll() {
                Ok(Some(connection)) => {
                    self.connection = Some(connection);
                    return Ok(DevicePoll::Connected);
                }
                Ok(None) => {
                    if worker.is_finished() {
                        worker.join()?;
                        self.device = None;
                        return Ok(DevicePoll::Stopped);
                    }
                }
                Err(error) => {
                    self.close_audio_device();
                    return Err(error.into());
                }
            }
        }
        Ok(DevicePoll::Pending)
    }

    pub(super) fn close_play_audio(&mut self) {
        if let Some(audio) = self.play.take() {
            self.retired.extend(audio.close());
        }
    }
    pub(super) fn close_audio_device(&mut self) {
        self.pending_play = None;
        if let Some(audio) = self.prepared.take() {
            self.retired.extend(audio.close());
        }
        if let Some(preparation) = self.preparation.take() {
            self.retired.push(preparation.close());
        }
        self.connection = None;
        if let Some(worker) = &mut self.device {
            worker.close();
        }
    }
    fn poll_preparation(
        &mut self,
        scene: &voxy_scene::SceneGraph,
        project: &super::prefab_authoring::AuthoringProject,
    ) -> Result<AudioPoll, Box<dyn std::error::Error>> {
        if self.retired.iter().any(|worker| !worker.is_finished()) {
            return Ok(AudioPoll::Pending);
        }
        for worker in self.retired.drain(..) {
            worker.join().map_err(|_| "audio import worker panicked")?;
        }
        if let Some(preparation) = &self.preparation
            && !preparation.matches(scene)?
        {
            if let Some(preparation) = self.preparation.take() {
                self.retired.push(preparation.close());
            }
            return Ok(AudioPoll::Pending);
        }
        if self.preparation.is_none() {
            let rate = self
                .connection
                .as_ref()
                .ok_or("missing audio connection")?
                .sample_rate();
            self.preparation = Some(audio_play::AudioPreparation::new(scene, project, rate)?);
        }
        if self
            .preparation
            .as_mut()
            .ok_or("missing audio preparation")?
            .poll()?
        {
            let preparation = self
                .preparation
                .take()
                .ok_or("missing ready audio preparation")?;
            let connection = self.connection.clone().ok_or("missing audio connection")?;
            self.prepared = Some(preparation.finish(connection, project)?);
            self.pending_play = None;
            return Ok(AudioPoll::Ready);
        }
        Ok(AudioPoll::Pending)
    }

    pub(super) fn play(&self) -> Option<&audio_play::AudioPlay> {
        self.play.as_ref()
    }
    pub(super) fn play_mut(&mut self) -> Option<&mut audio_play::AudioPlay> {
        self.play.as_mut()
    }
    pub(super) fn set_play(&mut self, play: Option<audio_play::AudioPlay>) {
        self.play = play;
    }
    pub(super) fn set_output_mode(&mut self, mode: AudioOutputMode) {
        self.output_mode = mode;
    }
    pub(super) fn is_pending(&self) -> bool {
        self.pending_play.is_some()
    }
    pub(super) fn request_play(&mut self) {
        self.pending_play = Some(Instant::now());
    }
    pub(super) fn poll(
        &mut self,
        scene: &voxy_scene::SceneGraph,
        project: &super::prefab_authoring::AuthoringProject,
    ) -> Result<AudioPoll, Box<dyn std::error::Error>> {
        if self
            .pending_play
            .is_some_and(|started| started.elapsed() > std::time::Duration::from_mins(3))
        {
            self.close_audio_device();
            return Err("audio preparation timed out".into());
        }
        if self.poll_device()? == DevicePoll::Stopped && self.pending_play.take().is_some() {
            return Ok(AudioPoll::RetryPlay);
        }
        if self.connection.is_some() && self.is_pending() {
            let result = self.poll_preparation(scene, project);
            if result.is_err() {
                self.close_audio_device();
            }
            return result;
        }
        Ok(AudioPoll::Pending)
    }
    pub(super) fn prepare_start(
        &mut self,
        scene: &voxy_scene::SceneGraph,
        project: &super::prefab_authoring::AuthoringProject,
        has_window: bool,
    ) -> Result<AudioStart, Box<dyn std::error::Error>> {
        let native = (self.output_mode == AudioOutputMode::Native || has_window)
            && scene
                .components::<voxy_gameplay::AudioSource>()
                .next()
                .is_some();
        if native && self.connection.is_none() {
            voxy_gameplay::extract_scene_audio(scene, 128)?;
            if self.device.is_none() {
                self.device = Some(voxy_audio_device::OutputDeviceWorker::spawn(4096)?);
            }
            self.request_play();
            return Ok(AudioStart::Pending);
        }
        if native {
            if self.prepared.is_none() {
                self.request_play();
                return Ok(AudioStart::Pending);
            }
            Ok(AudioStart::Ready(self.prepared.take()))
        } else {
            self.close_audio_device();
            if !has_window {
                for worker in self.retired.drain(..) {
                    worker.join().map_err(|_| "audio worker panicked")?;
                }
            }
            Ok(AudioStart::Ready(audio_play::AudioPlay::prepare(
                scene, project, None,
            )?))
        }
    }
    pub(super) fn join_workers(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        if let Some(mut worker) = self.device.take()
            && let Err(error) = worker.join()
        {
            errors.push(format!("audio device worker: {error}"));
        }
        for worker in self.retired.drain(..) {
            if let Err(payload) = worker.join() {
                errors.push(format!(
                    "audio worker panicked: {}",
                    super::panic_message(payload.as_ref())
                ));
            }
        }
        errors
    }
    #[cfg(test)]
    pub(super) fn inject_preparation(&mut self, preparation: audio_play::AudioPreparation) {
        self.preparation = Some(preparation);
        self.request_play();
    }
    #[cfg(test)]
    pub(super) fn has_preparation(&self) -> bool {
        self.preparation.is_some()
    }
    #[cfg(test)]
    pub(super) fn has_retired_workers(&self) -> bool {
        !self.retired.is_empty()
    }
}
