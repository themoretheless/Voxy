//! Host publication of a staged audio checkpoint; native device transfer remains gated.
use super::*;
use crate::{
    EditorError, audio_play::owner_checkpoint::AudioOwnerCheckpoint,
    prefab_authoring::AuthoringProject,
};
use serde::{Deserialize, Serialize};
use voxy_scene::{NodeId, ObjectId, SceneGraph};
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AudioSessionCheckpoint {
    version: u32,
    play: Option<AudioOwnerCheckpoint>,
}
#[derive(Debug)]
pub(super) struct NativeCheckpoint {
    connection: voxy_audio_device::OutputConnection,
    frozen: Option<voxy_audio_device::FrozenOutputQueue>,
    resuming: bool,
    epoch: std::sync::Arc<()>,
    publication_resuming: bool,
    publication_committed: bool,
}
/// Owns staged workers until publication or explicit retirement by the host.
#[allow(dead_code)]
pub(crate) struct PreparedAudioSession {
    play: Option<audio_play::AudioPlay>,
}
/// Detached native candidate. It does not alter the live device or publish PCM.
#[allow(dead_code)]
pub(crate) struct PreparedNativeAudioSession {
    staged: PreparedAudioSession,
    idle_pump: Option<voxy_audio_device::PcmPump>,
    output: voxy_audio_device::PreparedOutputCheckpoint,
    scene: voxy_scene::SceneChangeToken,
    project: std::sync::Arc<()>,
    connection: voxy_audio_device::OutputConnection,
    origin: Option<(std::sync::Arc<()>, String)>,
    ready: bool,
    max_bytes: usize,
}
#[allow(dead_code)]
impl PreparedNativeAudioSession {
    pub(crate) fn validate_destination(
        &self,
        scene: &SceneGraph,
        project: &AuthoringProject,
        connection: &voxy_audio_device::OutputConnection,
    ) -> Result<(), EditorError> {
        let native_play = self
            .staged
            .play
            .as_ref()
            .and_then(audio_play::AudioPlay::output_connection);
        if native_play.is_some() == self.idle_pump.is_some()
            || self.scene != scene.change_token()
            || !std::sync::Arc::ptr_eq(&self.project, &project.observation_token())
            || !self.connection.shares_output(connection)
        {
            return Err("native audio destination changed after preparation".into());
        }
        Ok(self.output.validate_connection(connection)?)
    }
    pub(crate) fn discard(self) -> Vec<std::thread::JoinHandle<()>> {
        self.staged.discard()
    }
}
/// Final native admission. Dropping before publish restores the exact held source.
#[allow(dead_code)]
pub(crate) struct NativeAudioPublicationPermit<'a, 'b, 'c> {
    session: Option<&'a mut AudioSession>,
    candidate: &'b mut PreparedNativeAudioSession,
    installation: Option<voxy_audio_device::ReadyOutputPublication>,
    _scene: &'c SceneGraph,
    _project: &'c AuthoringProject,
}
impl Drop for NativeAudioPublicationPermit<'_, '_, '_> {
    fn drop(&mut self) {
        if let Some(installation) = self.installation.take() {
            let (frozen, output) = installation.cancel();
            self.candidate.output = output;
            self.session
                .as_mut()
                .expect("unpublished native session")
                .native_checkpoint
                .as_mut()
                .expect("admitted checkpoint")
                .frozen = Some(frozen);
        }
    }
}
/// Audio has committed; the gate stays owned until all other host slots are swapped.
#[allow(dead_code)]
pub(crate) struct CommittedNativeAudio<'a> {
    session: Option<&'a mut AudioSession>,
    frozen: Option<voxy_audio_device::FrozenOutputQueue>,
    retired: Vec<std::thread::JoinHandle<()>>,
    retire_output: bool,
}
#[allow(dead_code)]
impl CommittedNativeAudio<'_> {
    /// Infallible final gate release. No native API, worker wait or re-admission.
    pub(crate) fn finish(mut self) -> Vec<std::thread::JoinHandle<()>> {
        self.release();
        std::mem::take(&mut self.retired)
    }
    fn release(&mut self) {
        if let Some(session) = self.session.take() {
            session.native_checkpoint = None;
            session.checkpoint_requested = false;
            session.checkpoint_startup = None;
            if self.retire_output {
                // Nonblocking close command; the original worker owns native
                // destruction and normal session polling joins only when done.
                session.close_audio_device();
            }
            drop(self.frozen.take());
        }
    }
    /// Native audio-only replacement waits for explicit resume. A logical
    /// target completes retirement of the old native output here.
    fn hold(mut self) -> Vec<std::thread::JoinHandle<()>> {
        if self.retire_output {
            return self.finish();
        }
        let session = self.session.take().expect("committed native session");
        session
            .native_checkpoint
            .as_mut()
            .expect("admitted native checkpoint")
            .frozen = self.frozen.take();
        std::mem::take(&mut self.retired)
    }
}
impl Drop for CommittedNativeAudio<'_> {
    fn drop(&mut self) {
        self.release();
    }
}
#[allow(dead_code)]
impl<'a> NativeAudioPublicationPermit<'a, '_, '_> {
    /// All fallible admission and allocation completed when this permit was acquired.
    pub(crate) fn publish(mut self) -> CommittedNativeAudio<'a> {
        let session = self.session.take().expect("admitted native session");
        let installation = self
            .installation
            .take()
            .expect("admitted output publication");
        let retire_output = self.candidate.idle_pump.is_some();
        let frozen = if let Some(pump) = &mut self.candidate.idle_pump {
            installation.publish(pump).0
        } else {
            self.candidate
                .staged
                .play
                .as_mut()
                .expect("admitted native playback")
                .publish_ready_output(installation)
        };
        let old = std::mem::replace(&mut session.play, self.candidate.staged.play.take());
        session
            .native_checkpoint
            .as_mut()
            .expect("admitted native checkpoint")
            .publication_committed = true;
        CommittedNativeAudio {
            session: Some(session),
            frozen: Some(frozen),
            retired: old.map_or_else(Vec::new, audio_play::AudioPlay::close),
            retire_output,
        }
    }
}
impl Drop for PreparedAudioSession {
    fn drop(&mut self) {
        if let Some(play) = self.play.take() {
            // Closing channels is nonblocking; callers needing joined cleanup use discard.
            drop(play.close());
        }
    }
}
#[allow(dead_code)]
impl PreparedAudioSession {
    pub(crate) fn validate_output_mode(&self, mode: AudioOutputMode) -> Result<(), EditorError> {
        if mode == AudioOutputMode::Native && self.play.is_some() {
            return Err("native playback restore requires an owned device; offline audio cannot be published silently".into());
        }
        Ok(())
    }
    pub(crate) fn discard(mut self) -> Vec<std::thread::JoinHandle<()>> {
        self.play
            .take()
            .map_or_else(Vec::new, audio_play::AudioPlay::close)
    }
}
/// Admission borrows the destination until the infallible publication step.
#[allow(dead_code)]
pub(crate) struct AudioPublicationPermit<'a> {
    session: &'a mut AudioSession,
}
#[allow(dead_code)]
impl AudioPublicationPermit<'_> {
    pub(crate) fn publish(self, mut prepared: PreparedAudioSession) {
        let old = std::mem::replace(&mut self.session.play, prepared.play.take());
        self.session.checkpoint_requested = false;
        if let Some(old) = old {
            self.session.retired.extend(old.close());
        }
    }
}
#[allow(dead_code)]
impl AudioSessionCheckpoint {
    pub(crate) fn playback_rate(&self) -> Option<u32> {
        self.play.as_ref().map(AudioOwnerCheckpoint::playback_rate)
    }
    pub(crate) fn native_output_rate(&self) -> Option<u32> {
        self.play
            .as_ref()
            .and_then(AudioOwnerCheckpoint::native_output_rate)
    }
    pub(crate) fn validate_native_output(&self, max_bytes: usize) -> Result<(), EditorError> {
        self.to_json(max_bytes)?;
        if self.version != 1 {
            return Err("unsupported audio session checkpoint".into());
        }
        match &self.play {
            Some(play) => play.validate_native_output(max_bytes),
            None => Ok(()),
        }
    }
    pub(crate) fn prepare_offline_replacing_native(
        &self,
        scene: &SceneGraph,
        project: &AuthoringProject,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
        connection: &voxy_audio_device::OutputConnection,
    ) -> Result<PreparedNativeAudioSession, EditorError> {
        let staged = self.prepare(scene, project, resolve, max_bytes)?;
        let (pump, output) =
            audio_play::playback_checkpoint::empty_native_output(connection, max_bytes)?;
        Ok(PreparedNativeAudioSession {
            staged,
            idle_pump: Some(pump),
            output,
            scene: scene.change_token(),
            project: project.observation_token(),
            connection: connection.clone(),
            origin: None,
            ready: false,
            max_bytes,
        })
    }
    pub(crate) fn prepare_native(
        &self,
        scene: &SceneGraph,
        project: &AuthoringProject,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
        connection: &voxy_audio_device::OutputConnection,
    ) -> Result<PreparedNativeAudioSession, EditorError> {
        self.to_json(max_bytes)?;
        if self.version != 1 {
            return Err("unsupported audio session checkpoint".into());
        }
        let Some(saved) = self.play.as_ref() else {
            // An archive without playback replaces the active output with an
            // admitted empty queue before its device owner is retired at commit.
            let (pump, output) =
                audio_play::playback_checkpoint::empty_native_output(connection, max_bytes)?;
            return Ok(PreparedNativeAudioSession {
                staged: PreparedAudioSession { play: None },
                idle_pump: Some(pump),
                output,
                scene: scene.change_token(),
                project: project.observation_token(),
                connection: connection.clone(),
                origin: None,
                ready: false,
                max_bytes,
            });
        };

        let (mut play, output) = saved.prepare_native(
            scene,
            resolve,
            max_bytes,
            connection,
            |rate, recipes| project.audio_worker(rate, recipes),
            |baseline| project.audio_watcher_from_checkpoint(baseline),
        )?;
        if let Err(error) = play.attach_prepared_native_output(connection.clone()) {
            drop(play.close());
            return Err(error);
        }
        Ok(PreparedNativeAudioSession {
            staged: PreparedAudioSession { play: Some(play) },
            idle_pump: None,
            output,
            scene: scene.change_token(),
            project: project.observation_token(),
            connection: connection.clone(),
            origin: None,
            ready: false,
            max_bytes,
        })
    }
    pub(crate) fn prepare(
        &self,
        scene: &SceneGraph,
        project: &AuthoringProject,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
    ) -> Result<PreparedAudioSession, EditorError> {
        self.to_json(max_bytes)?;
        if self.version != 1 {
            return Err("unsupported audio session checkpoint".into());
        }
        let play = self
            .play
            .as_ref()
            .map(|play| {
                play.prepare(
                    scene,
                    resolve,
                    max_bytes,
                    |rate, recipes| project.audio_worker(rate, recipes),
                    |baseline| project.audio_watcher_from_checkpoint(baseline),
                )
            })
            .transpose()?;
        Ok(PreparedAudioSession { play })
    }
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("audio session checkpoint byte budget".into());
        }
        Ok(json)
    }
    pub(crate) fn from_json(json: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if json.len() > max_bytes {
            return Err("audio session checkpoint byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
}
#[allow(dead_code)]
impl AudioSession {
    fn checkpoint_boundary(&self) -> Result<(), EditorError> {
        if self.device.is_some()
            || self.connection.is_some()
            || self.native_checkpoint.is_some()
            || self
                .play
                .as_ref()
                .and_then(audio_play::AudioPlay::output_connection)
                .is_some()
        {
            return Err("native audio checkpoint needs a device queue barrier".into());
        }
        if self.pending_play.is_some()
            || self.preparation.is_some()
            || self.prepared.is_some()
            || self.retired.iter().any(|worker| !worker.is_finished())
        {
            return Err("audio session transition is not quiescent".into());
        }
        Ok(())
    }
    /// Clone the already fenced connection for detached CPU preparation. The
    /// native stream remains exclusively owned by the original device worker.
    pub(crate) fn checkpoint_output_connection(
        &self,
    ) -> Result<Option<voxy_audio_device::OutputConnection>, EditorError> {
        if !self.capture_boundary()? {
            return Ok(None);
        }
        let native = self
            .native_checkpoint
            .as_ref()
            .ok_or("missing native checkpoint")?;
        if !self.checkpoint_requested
            || native.frozen.is_none()
            || native.resuming
            || native.publication_committed
        {
            return Err("native checkpoint output is not held for preparation".into());
        }
        Ok(Some(native.connection.clone()))
    }
    fn capture_boundary(&self) -> Result<bool, EditorError> {
        if let Some(mode) = self.checkpoint_startup {
            if self.pending_play.is_some()
                || self.preparation.is_some()
                || self.prepared.is_some()
                || self.retired.iter().any(|worker| !worker.is_finished())
            {
                return Err("audio session transition is not quiescent".into());
            }
            let committed = self
                .native_checkpoint
                .as_ref()
                .is_some_and(|native| native.publication_committed);
            if !committed && mode != self.output_mode {
                return Err("native restoration output policy changed during startup".into());
            }
            if !self.checkpoint_requested
                || self.device.is_none()
                || (!committed
                    && self
                        .play
                        .as_ref()
                        .and_then(audio_play::AudioPlay::output_connection)
                        .is_some())
            {
                return Err("native restoration startup ownership changed".into());
            }
            if self.native_checkpoint.is_none() {
                return Ok(false); // The original logical playback owner is unchanged.
            }
        }
        if self.device.is_none()
            && self.connection.is_none()
            && self
                .play
                .as_ref()
                .and_then(audio_play::AudioPlay::output_connection)
                .is_none()
        {
            if self.native_checkpoint.is_some() {
                return Err("native audio checkpoint owner changed".into());
            }
            self.checkpoint_boundary()?;
            return Ok(false);
        }
        let connection = self
            .connection
            .as_ref()
            .ok_or("native audio checkpoint needs a device queue barrier")?;
        let device = self
            .device
            .as_ref()
            .ok_or("native audio checkpoint needs a device queue barrier")?;
        let output = self
            .play
            .as_ref()
            .and_then(audio_play::AudioPlay::output_connection);
        if self
            .native_checkpoint
            .as_ref()
            .is_some_and(|native| !native.connection.shares_output(connection))
        {
            return Err("native audio checkpoint owner changed".into());
        }
        if !device.owns_connection(connection)
            || (!output.is_some_and(|output| output.shares_output(connection))
                && self.checkpoint_startup.is_none())
        {
            return Err("native audio checkpoint connection ownership changed".into());
        }
        if self.pending_play.is_some()
            || self.preparation.is_some()
            || self.prepared.is_some()
            || self.retired.iter().any(|worker| !worker.is_finished())
        {
            return Err("audio session transition is not quiescent".into());
        }
        Ok(true)
    }
    /// Start a provisional empty output under an existing logical source fence.
    /// The original device worker stays owned by this session through cancellation.
    pub(crate) fn begin_checkpoint_output_startup(
        &mut self,
        has_window: bool,
    ) -> Result<(), EditorError> {
        self.checkpoint_boundary()?;
        if !self.checkpoint_requested || self.checkpoint_startup.is_some() {
            return Err("native restoration startup requires a logical owned checkpoint".into());
        }
        if self.output_mode != AudioOutputMode::Native && !has_window {
            return Err("native restoration output is not enabled by the host".into());
        }
        self.device = Some(voxy_audio_device::OutputDeviceWorker::spawn(4096)?);
        self.checkpoint_startup = Some(self.output_mode);
        Ok(())
    }
    pub(crate) fn poll_checkpoint_output_startup(
        &mut self,
        rate: u32,
        max_bytes: usize,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<bool, EditorError> {
        if self.checkpoint_startup.is_none() {
            return Err("native restoration startup not requested".into());
        }
        self.capture_boundary()?;
        if self.native_checkpoint.is_none() {
            match self.poll_device().map_err(|error| error.to_string())? {
                super::DevicePoll::Pending => return Ok(false),
                super::DevicePoll::Stopped => {
                    return Err("native restoration device stopped during startup".into());
                }
                super::DevicePoll::Connected => {}
            }
            let connection = self
                .connection
                .as_ref()
                .ok_or("missing provisional output")?;
            if connection.sample_rate() != rate {
                return Err(
                    "native restoration sample rate mismatch; resampling is not permitted".into(),
                );
            }
            self.device
                .as_mut()
                .ok_or("missing provisional device")?
                .request_pause()?;
            self.native_checkpoint = Some(NativeCheckpoint {
                connection: connection.clone(),
                frozen: None,
                resuming: false,
                epoch: std::sync::Arc::new(()),
                publication_resuming: false,
                publication_committed: false,
            });
        }
        Ok(self.poll_checkpoint(identify, max_bytes)?.is_some())
    }
    pub(crate) fn checkpoint_admission(&self) -> Result<(), EditorError> {
        self.capture_boundary()?;
        if self.checkpoint_requested {
            return Err("audio session checkpoint already requested".into());
        }
        Ok(())
    }
    pub(crate) fn request_checkpoint(&mut self) -> Result<(), EditorError> {
        let native = self.capture_boundary()?;
        if self.checkpoint_requested {
            return Err("audio session checkpoint already requested".into());
        }
        if native {
            let play = self.play.as_mut().ok_or("missing native playback")?;
            play.begin_owner_checkpoint_with_output()?;
            if let Err(error) = self
                .device
                .as_mut()
                .ok_or("missing native audio device")?
                .request_pause()
            {
                // The just-acquired reload fence is still Draining: rollback has no IO wait.
                play.resume_owner_checkpoint()?;
                return Err(error.into());
            }
            self.native_checkpoint = Some(NativeCheckpoint {
                connection: self
                    .connection
                    .as_ref()
                    .ok_or("missing native audio connection")?
                    .clone(),
                frozen: None,
                resuming: false,
                epoch: std::sync::Arc::new(()),
                publication_resuming: false,
                publication_committed: false,
            });
        } else if let Some(play) = &mut self.play {
            play.begin_owner_checkpoint()?;
        }
        self.checkpoint_requested = true;
        Ok(())
    }
    pub(crate) fn poll_checkpoint(
        &mut self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<Option<AudioSessionCheckpoint>, EditorError> {
        self.capture_boundary()?;
        if !self.checkpoint_requested {
            return Err("audio session checkpoint not requested".into());
        }
        let play = if let Some(native) = &mut self.native_checkpoint {
            if native.resuming || native.publication_committed {
                return Err("native audio checkpoint is releasing or committed".into());
            }
            if !native.publication_resuming {
                match self
                    .device
                    .as_mut()
                    .ok_or("missing native device")?
                    .poll_paused()?
                {
                    None => return Ok(None),
                    Some(true) => {}
                    Some(false) => return Err("native audio device has not paused".into()),
                }
            } else if native.frozen.is_none() {
                return Err("native publication lost its queue fence".into());
            }
            if native.frozen.is_none() {
                native.frozen = Some(
                    self.connection
                        .as_ref()
                        .ok_or("missing native connection")?
                        .sender()
                        .freeze_queue()
                        .map_err(|error| error.to_string())?,
                );
            }
            if self.checkpoint_startup.is_some() {
                let Some(queue) = native
                    .frozen
                    .as_mut()
                    .ok_or("missing provisional queue fence")?
                    .checkpoint(4096)
                    .map_err(|e| e.to_string())?
                else {
                    return Ok(None);
                };
                if !queue.frames.is_empty() {
                    return Err("provisional output queue changed before publication".into());
                }
            }
            if self.checkpoint_startup.is_some()
                && self
                    .play
                    .as_ref()
                    .is_some_and(|play| play.output_connection().is_none())
            {
                let Some(saved) = self
                    .play
                    .as_mut()
                    .unwrap()
                    .poll_owner_checkpoint(identify, max_bytes)?
                else {
                    return Ok(None);
                };
                Some(saved)
            } else if let Some(play) = &mut self.play {
                let Some(saved) = play.poll_owner_checkpoint_with_output(
                    identify,
                    max_bytes,
                    native.frozen.as_mut().ok_or("missing output queue fence")?,
                )?
                else {
                    return Ok(None);
                };
                Some(saved)
            } else if self.checkpoint_startup.is_some() {
                None // A provisional empty device does not alter the source archive.
            } else {
                return Err("missing native playback".into());
            }
        } else if let Some(play) = &mut self.play {
            let Some(saved) = play.poll_owner_checkpoint(identify, max_bytes)? else {
                return Ok(None);
            };
            Some(saved)
        } else {
            None
        };
        let saved = AudioSessionCheckpoint { version: 1, play };
        saved.to_json(max_bytes)?;
        Ok(Some(saved))
    }
    pub(crate) fn resume_checkpoint(&mut self) -> Result<bool, EditorError> {
        if self.checkpoint_startup.is_some()
            && !self
                .native_checkpoint
                .as_ref()
                .is_some_and(|native| native.publication_committed)
        {
            if let Some(play) = &mut self.play
                && !play.resume_owner_checkpoint()?
            {
                return Ok(false);
            }
            self.close_audio_device();
            self.native_checkpoint = None;
            self.checkpoint_startup = None;
            self.checkpoint_requested = false;
            return Ok(true);
        }
        if self.native_checkpoint.is_some() && !self.capture_boundary()? {
            return Err("native audio checkpoint owner changed".into());
        }
        if let Some(play) = &mut self.play
            && !play.resume_owner_checkpoint()?
        {
            return Ok(false);
        }
        if let Some(native) = &mut self.native_checkpoint {
            let device = self
                .device
                .as_mut()
                .ok_or("missing native audio device during resume")?;
            if !native.resuming {
                // Cancellation can arrive before pause acknowledgement. A failed
                // native pause also needs an explicit recovery resume (unknown state).
                if matches!(device.poll_paused(), Ok(None)) {
                    return Ok(false);
                }
                device.request_resume()?;
                native.resuming = true;
                return Ok(false);
            }
            match device.poll_paused() {
                Ok(None) => return Ok(false),
                Ok(Some(false)) => {}
                Ok(Some(true)) => return Err("native audio resume not acknowledged".into()),
                Err(error) => {
                    native.resuming = false; // Allow an explicit recovery retry after a transient host error.
                    return Err(error.into());
                }
            }
            self.native_checkpoint = None; // Release PCM only after native resume ACK.
        }
        self.checkpoint_requested = false;
        self.checkpoint_startup = None;
        Ok(true)
    }
    fn validate_native_candidate(
        &mut self,
        candidate: &mut PreparedNativeAudioSession,
        scene: &SceneGraph,
        project: &AuthoringProject,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<(), EditorError> {
        let connection = self
            .connection
            .as_ref()
            .ok_or("missing native connection")?;
        candidate.validate_destination(scene, project, connection)?;
        if connection.stats().errors != 0 {
            return Err("native audio output reported a device error".into());
        }
        let native = self
            .native_checkpoint
            .as_ref()
            .ok_or("native restoration requires an owned source checkpoint")?;
        if native.resuming || native.publication_committed || native.frozen.is_none() {
            return Err("native source checkpoint is not held for publication".into());
        }
        let epoch = native.epoch.clone();
        if candidate
            .origin
            .as_ref()
            .is_some_and(|(origin, _)| !std::sync::Arc::ptr_eq(origin, &epoch))
        {
            return Err("native source checkpoint epoch changed".into());
        }
        let source = self
            .poll_checkpoint(identify, candidate.max_bytes)?
            .ok_or("native source checkpoint not ready")?
            .to_json(candidate.max_bytes)?;
        match &candidate.origin {
            Some((_, original)) if original != &source => {
                return Err("native source playback changed after staging".into());
            }
            None => candidate.origin = Some((epoch, source)),
            _ => {}
        }
        Ok(())
    }
    /// Advance native API readiness while the original PCM queue remains frozen.
    pub(crate) fn poll_native_checkpoint_ready(
        &mut self,
        candidate: &mut PreparedNativeAudioSession,
        scene: &SceneGraph,
        project: &AuthoringProject,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<bool, EditorError> {
        self.validate_native_candidate(candidate, scene, project, identify)?;
        let native = self
            .native_checkpoint
            .as_mut()
            .ok_or("missing native checkpoint")?;
        let device = self.device.as_mut().ok_or("missing native device")?;
        if !native.publication_resuming {
            device.request_resume()?;
            native.publication_resuming = true;
            return Ok(false);
        }
        match device.poll_paused() {
            Ok(None) => Ok(false),
            Ok(Some(false)) => {
                candidate.ready = true;
                Ok(true)
            }
            Ok(Some(true)) => Err("native publication resume not acknowledged".into()),
            Err(error) => {
                candidate.ready = false;
                Err(error.into())
            }
        }
    }
    /// Complete final native checks before any host owner is committed.
    pub(crate) fn native_checkpoint_publication<'a, 'b, 'c>(
        &'a mut self,
        candidate: &'b mut PreparedNativeAudioSession,
        scene: &'c SceneGraph,
        project: &'c AuthoringProject,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<Option<NativeAudioPublicationPermit<'a, 'b, 'c>>, EditorError> {
        self.validate_native_candidate(candidate, scene, project, identify)?;
        if !candidate.ready
            || self
                .device
                .as_mut()
                .ok_or("missing native device")?
                .poll_paused()?
                != Some(false)
        {
            return Err("native audio candidate has not reached publication readiness".into());
        }
        let native = self
            .native_checkpoint
            .as_mut()
            .ok_or("missing native checkpoint")?;
        let installation = if let Some(pump) = &candidate.idle_pump {
            candidate
                .output
                .stage_publication(&candidate.connection, &mut native.frozen, pump)?
        } else {
            candidate
                .staged
                .play
                .as_mut()
                .ok_or("native candidate already consumed")?
                .stage_output_publication(&mut candidate.output, &mut native.frozen)?
        };
        let Some(installation) = installation else {
            return Ok(None);
        };
        Ok(Some(NativeAudioPublicationPermit {
            session: Some(self),
            candidate,
            installation: Some(installation),
            _scene: scene,
            _project: project,
        }))
    }
    /// Audio-only compatibility: native PCM stays held for explicit resume;
    /// a logical target releases its final receipt and closes the old output.
    pub(crate) fn publish_native_checkpoint(
        &mut self,
        candidate: &mut PreparedNativeAudioSession,
        scene: &SceneGraph,
        project: &AuthoringProject,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<Option<Vec<std::thread::JoinHandle<()>>>, EditorError> {
        Ok(self
            .native_checkpoint_publication(candidate, scene, project, identify)?
            .map(|permit| permit.publish().hold()))
    }
    /// Admit the target before a coordinated, infallible owner publication.
    /// The returned borrow prevents target transitions until commit or permit drop.
    pub(crate) fn checkpoint_publication(
        &mut self,
    ) -> Result<AudioPublicationPermit<'_>, EditorError> {
        self.checkpoint_boundary()?;
        if self.checkpoint_requested {
            return Err("finish or cancel audio save before loading".into());
        }
        self.publication_permit()
    }
    /// The common coordinator keeps the original save fence held until commit.
    /// Poll native readiness without waiting; failure leaves the old owner held.
    pub(crate) fn checkpoint_publication_at_barrier(
        &mut self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<AudioPublicationPermit<'_>, EditorError> {
        self.checkpoint_boundary()?; // Any actual native output still needs its queue publication protocol.
        if self.poll_checkpoint(identify, max_bytes)?.is_none() {
            return Err("coordinated audio publication requires a completed barrier".into());
        }
        self.publication_permit()
    }
    fn publication_permit(&mut self) -> Result<AudioPublicationPermit<'_>, EditorError> {
        // Only completed handles reach this path; do not block on active IO.
        for worker in self.retired.drain(..) {
            worker.join().map_err(|panic| {
                format!(
                    "retired audio worker panicked: {}",
                    crate::panic_message(panic.as_ref())
                )
            })?;
        }
        Ok(AudioPublicationPermit { session: self })
    }
    /// Convenience path for an audio-only replacement. Whole-game loaders can
    /// instead stage all owners, acquire a permit and publish after other admission.
    pub(crate) fn load_checkpoint(
        &mut self,
        saved: &AudioSessionCheckpoint,
        scene: &SceneGraph,
        project: &AuthoringProject,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
    ) -> Result<(), EditorError> {
        self.checkpoint_boundary()?;
        if self.checkpoint_requested {
            return Err("finish or cancel audio save before loading".into());
        }
        let candidate = saved.prepare(scene, project, resolve, max_bytes)?;
        candidate.validate_output_mode(self.output_mode)?;
        self.checkpoint_publication()?.publish(candidate);
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::InputRecipe;
    use voxy_scene::Transform;
    #[test]
    fn native_idle_capture_cancel_and_load_preserve_mode_without_starting_a_device() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = AuthoringProject::new(&temp.0, &InputRecipe::Direct).unwrap();
        let scene = SceneGraph::new(1);
        let mut session = AudioSession::default();
        session.set_output_mode(AudioOutputMode::Native);
        for cancel in [true, false] {
            session.checkpoint_admission().unwrap();
            session.request_checkpoint().unwrap();
            assert!(session.checkpoint_output_connection().unwrap().is_none());
            let saved = session.poll_checkpoint(|_| None, 10000).unwrap().unwrap();
            assert!(saved.play.is_none());
            let json = saved.to_json(10000).unwrap();
            let saved = AudioSessionCheckpoint::from_json(&json, 10000).unwrap();
            if cancel {
                assert!(session.resume_checkpoint().unwrap());
            } else {
                let prepared = saved.prepare(&scene, &project, |_| None, 10000).unwrap();
                session
                    .checkpoint_publication_at_barrier(|_| None, 10000)
                    .unwrap()
                    .publish(prepared);
            }
            assert_eq!(session.output_mode, AudioOutputMode::Native);
            assert!(session.device.is_none() && session.connection.is_none());
            assert!(session.native_checkpoint.is_none() && !session.checkpoint_requested);
        }
        session.pending_play = Some(std::time::Instant::now());
        for error in [
            session.checkpoint_admission().unwrap_err(),
            session.request_checkpoint().unwrap_err(),
        ] {
            assert!(error.to_string().contains("transition is not quiescent"));
        }
        assert!(!session.checkpoint_requested);
        session.pending_play = None;
        assert!(session.join_workers().is_empty());
    }
    #[test]
    fn native_startup_requires_source_checkpoint_and_host_authorization() {
        let mut session = AudioSession::default();
        assert!(session.begin_checkpoint_output_startup(false).is_err());
        session.request_checkpoint().unwrap();
        let before = session
            .poll_checkpoint(|_| None, 10000)
            .unwrap()
            .unwrap()
            .to_json(10000)
            .unwrap();
        let error = session.begin_checkpoint_output_startup(false).unwrap_err();
        assert!(error.to_string().contains("not enabled by the host"));
        assert!(
            session.device.is_none()
                && session.connection.is_none()
                && session.checkpoint_startup.is_none()
        );
        assert_eq!(
            session
                .poll_checkpoint(|_| None, 10000)
                .unwrap()
                .unwrap()
                .to_json(10000)
                .unwrap(),
            before
        );
        assert!(session.resume_checkpoint().unwrap());
        session.set_output_mode(AudioOutputMode::Native);
        assert!(session.begin_checkpoint_output_startup(false).is_err());
        assert!(session.device.is_none());
    }
    fn capture(session: &mut AudioSession) -> AudioSessionCheckpoint {
        session.request_checkpoint().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(saved) = session
                .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
            {
                assert!(session.resume_checkpoint().unwrap());
                return saved;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    #[ignore = "physical default audio output; gain zero, no window"]
    fn physical_native_session_save_cancel_resume_and_closed_load_admission() {
        use std::time::{Duration, Instant};
        let temp = crate::project_prefab_tests::Temp::new();
        std::fs::write(temp.0.join("tone.wav"), wav()).unwrap();
        let project = AuthoringProject::new(&temp.0, &InputRecipe::Direct).unwrap();
        let mut scene = SceneGraph::new(1);
        let node = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
        scene
            .insert_component(
                node,
                voxy_gameplay::AudioSource {
                    asset: "tone.wav".into(),
                    import_settings: None,
                    bus: 0,
                    gain: 0.0,
                    looping: true,
                    spatial: false,
                    near: 1.0,
                    far: 5.0,
                },
            )
            .unwrap();
        let mut device = voxy_audio_device::OutputDeviceWorker::spawn(4096).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let connection = loop {
            if let Some(connection) = device.poll().unwrap() {
                break connection;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        println!("NATIVE SESSION rate={}", connection.sample_rate());
        let mut session = AudioSession::default();
        session.set_output_mode(AudioOutputMode::Native);
        session.set_play(
            audio_play::AudioPlay::prepare(&scene, &project, Some(connection.clone())).unwrap(),
        );
        session.connection = Some(connection.clone());
        session.device = Some(device);
        session.play_mut().unwrap().advance(&scene, 7).unwrap();
        for cancel_before_capture in [true, false, false] {
            session.checkpoint_admission().unwrap();
            session.request_checkpoint().unwrap();
            if !cancel_before_capture {
                let saved = loop {
                    if let Some(saved) = session
                        .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                        .unwrap()
                    {
                        break saved;
                    }
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                };
                let json = saved.to_json(2_000_000).unwrap();
                assert!(json.contains("\"output\""));
                let path = temp.0.join("native-audio.json");
                std::fs::write(&path, &json).unwrap();
                let decoded = AudioSessionCheckpoint::from_json(
                    &std::fs::read_to_string(path).unwrap(),
                    2_000_000,
                )
                .unwrap();
                assert_eq!(decoded.to_json(2_000_000).unwrap(), json);
                if temp.0.join("tone.wav").exists() {
                    std::fs::remove_file(temp.0.join("tone.wav")).unwrap();
                }
                assert!(
                    decoded
                        .prepare_native(&scene, &project, |_| None, 2_000_000, &connection)
                        .is_err()
                );
                let staged = decoded
                    .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &connection)
                    .unwrap();
                staged
                    .validate_destination(&scene, &project, &connection)
                    .unwrap();
                let other_project = AuthoringProject::new(&temp.0, &InputRecipe::Direct).unwrap();
                assert!(
                    staged
                        .validate_destination(&scene, &other_project, &connection)
                        .is_err()
                );
                scene
                    .set_name(
                        node,
                        format!(
                            "Changed native destination at {}",
                            session.play().unwrap().frames
                        ),
                    )
                    .unwrap();
                assert!(
                    staged
                        .validate_destination(&scene, &project, &connection)
                        .is_err()
                );
                for worker in staged.discard() {
                    worker.join().unwrap();
                }
                let staged = decoded
                    .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &connection)
                    .unwrap();
                staged
                    .validate_destination(&scene, &project, &connection)
                    .unwrap();
                for worker in staged.discard() {
                    worker.join().unwrap();
                }
                assert_eq!(
                    session
                        .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                        .unwrap()
                        .unwrap()
                        .to_json(2_000_000)
                        .unwrap(),
                    json
                );

                let prepared_logical = decoded
                    .prepare(&scene, &project, |_| Some(node), 2_000_000)
                    .unwrap();
                let mut logical = AudioSession::default();
                logical
                    .checkpoint_publication()
                    .unwrap()
                    .publish(prepared_logical);
                assert!(logical.play().unwrap().output_connection().is_none());
                let logical_saved = capture(&mut logical);
                let logical_value: serde_json::Value =
                    serde_json::from_str(&logical_saved.to_json(2_000_000).unwrap()).unwrap();
                let original_value: serde_json::Value = serde_json::from_str(&json).unwrap();
                assert_eq!(
                    logical_value["play"]["playback"],
                    original_value["play"]["playback"]
                );
                logical.close_play_audio();
                assert!(logical.join_workers().is_empty());
                assert!(
                    session
                        .checkpoint_publication_at_barrier(
                            |_| Some(ObjectId("sound".into())),
                            2_000_000
                        )
                        .is_err()
                );
                assert!(
                    session.checkpoint_requested
                        && session.native_checkpoint.as_ref().unwrap().frozen.is_some()
                );
                let held_connection = session.connection.take();
                assert!(session.resume_checkpoint().is_err());
                assert!(session.native_checkpoint.as_ref().unwrap().frozen.is_some());
                assert_eq!(
                    session.device.as_mut().unwrap().poll_paused().unwrap(),
                    Some(true)
                );
                session.connection = held_connection;
                std::thread::sleep(Duration::from_millis(20));
                assert_eq!(
                    session
                        .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                        .unwrap()
                        .unwrap()
                        .to_json(2_000_000)
                        .unwrap(),
                    json
                );
                println!(
                    "NATIVE SESSION CAPTURE bytes={} frames={}",
                    json.len(),
                    session.play().unwrap().frames
                );
                let mut stale_scene = decoded
                    .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &connection)
                    .unwrap();
                while !session
                    .poll_native_checkpoint_ready(&mut stale_scene, &scene, &project, |_| {
                        Some(ObjectId("sound".into()))
                    })
                    .unwrap()
                {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                let held_frames = session.play().unwrap().frames;
                session.play_mut().unwrap().frames += 1;
                assert!(
                    session
                        .publish_native_checkpoint(&mut stale_scene, &scene, &project, |_| Some(
                            ObjectId("sound".into())
                        ))
                        .unwrap_err()
                        .to_string()
                        .contains("playback changed")
                );
                assert_eq!(session.play().unwrap().frames, held_frames + 1);
                session.play_mut().unwrap().frames = held_frames;
                scene
                    .set_name(
                        node,
                        format!(
                            "Late native scene change {}",
                            session.play().unwrap().frames
                        ),
                    )
                    .unwrap();
                assert!(
                    session
                        .publish_native_checkpoint(&mut stale_scene, &scene, &project, |_| Some(
                            ObjectId("sound".into())
                        ))
                        .is_err()
                );
                assert_eq!(
                    session
                        .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                        .unwrap()
                        .unwrap()
                        .to_json(2_000_000)
                        .unwrap(),
                    json
                );
                for worker in stale_scene.discard() {
                    worker.join().unwrap();
                }
                let mut stale_epoch = decoded
                    .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &connection)
                    .unwrap();
                while !session
                    .poll_native_checkpoint_ready(&mut stale_epoch, &scene, &project, |_| {
                        Some(ObjectId("sound".into()))
                    })
                    .unwrap()
                {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                while !session.resume_checkpoint().unwrap() {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                session.request_checkpoint().unwrap();
                let fresh_source = loop {
                    if let Some(source) = session
                        .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                        .unwrap()
                    {
                        break source.to_json(2_000_000).unwrap();
                    }
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                };
                assert!(
                    session
                        .poll_native_checkpoint_ready(&mut stale_epoch, &scene, &project, |_| Some(
                            ObjectId("sound".into())
                        ))
                        .unwrap_err()
                        .to_string()
                        .contains("epoch")
                );
                assert_eq!(
                    session
                        .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                        .unwrap()
                        .unwrap()
                        .to_json(2_000_000)
                        .unwrap(),
                    fresh_source
                );
                for worker in stale_epoch.discard() {
                    worker.join().unwrap();
                }
                let mut candidate = decoded
                    .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &connection)
                    .unwrap();
                assert!(
                    session
                        .publish_native_checkpoint(&mut candidate, &scene, &project, |_| Some(
                            ObjectId("sound".into())
                        ))
                        .is_err()
                );
                while !session
                    .poll_native_checkpoint_ready(&mut candidate, &scene, &project, |_| {
                        Some(ObjectId("sound".into()))
                    })
                    .unwrap()
                {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                let retired = session
                    .publish_native_checkpoint(&mut candidate, &scene, &project, |_| {
                        Some(ObjectId("sound".into()))
                    })
                    .unwrap()
                    .unwrap();
                assert!(
                    session
                        .native_checkpoint
                        .as_ref()
                        .unwrap()
                        .publication_committed
                );
                assert!(session.native_checkpoint.as_ref().unwrap().frozen.is_some());
                std::thread::sleep(Duration::from_millis(20)); // Native API is running, PCM must still be held.
                let value: serde_json::Value = serde_json::from_str(&json).unwrap();
                let wire: Vec<u8> =
                    serde_json::from_value(value["play"]["playback"]["output"].clone()).unwrap();
                let expected = voxy_audio_device::OutputCheckpoint::from_bytes(
                    &wire,
                    voxy_audio_device::OutputCheckpointLimits {
                        max_bytes: 2_000_000,
                        max_queue_frames: 4096,
                        max_pump_frames: 256,
                        sample_rate: connection.sample_rate(),
                    },
                )
                .unwrap();
                let actual = session
                    .play
                    .as_ref()
                    .unwrap()
                    .capture_native_output(
                        session
                            .native_checkpoint
                            .as_mut()
                            .unwrap()
                            .frozen
                            .as_mut()
                            .unwrap(),
                        2_000_000,
                    )
                    .unwrap();
                assert_eq!(actual, expected);
                assert_eq!(
                    session.play().unwrap().frames,
                    value["play"]["playback"]["clock"]["frames"]
                        .as_u64()
                        .unwrap()
                );
                assert!(
                    session
                        .publish_native_checkpoint(&mut candidate, &scene, &project, |_| Some(
                            ObjectId("sound".into())
                        ))
                        .is_err()
                );
                for worker in retired {
                    worker.join().unwrap();
                }
                println!(
                    "NATIVE SESSION PUBLICATION queue/pump/clock exact; source epoch and late-scene guards passed"
                );
            }
            while !session.resume_checkpoint().unwrap() {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            assert!(!session.checkpoint_requested && session.native_checkpoint.is_none());
            assert_eq!(
                session.device.as_mut().unwrap().poll_paused().unwrap(),
                Some(false)
            );
            let frames = session.play().unwrap().frames;
            while session.play().unwrap().frames == frames {
                session.play_mut().unwrap().advance(&scene, 1).unwrap();
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            assert_eq!(connection.stats().errors, 0);
        }
        session.request_checkpoint().unwrap();
        let saved = loop {
            if let Some(saved) = session
                .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
            {
                break saved;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let before = saved.to_json(2_000_000).unwrap();
        let mut candidate = saved
            .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &connection)
            .unwrap();
        while !session
            .poll_native_checkpoint_ready(&mut candidate, &scene, &project, |_| {
                Some(ObjectId("sound".into()))
            })
            .unwrap()
        {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let permit = session
            .native_checkpoint_publication(&mut candidate, &scene, &project, |_| {
                Some(ObjectId("sound".into()))
            })
            .unwrap()
            .unwrap();
        drop(permit); // Simulate another owner's failed admission before any commit.
        assert_eq!(
            session
                .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
                .unwrap()
                .to_json(2_000_000)
                .unwrap(),
            before
        );
        let permit = session
            .native_checkpoint_publication(&mut candidate, &scene, &project, |_| {
                Some(ObjectId("sound".into()))
            })
            .unwrap()
            .unwrap();
        let mut committed = permit.publish();
        std::thread::sleep(Duration::from_millis(20));
        let actual = committed
            .session
            .as_ref()
            .unwrap()
            .play
            .as_ref()
            .unwrap()
            .capture_native_output(committed.frozen.as_mut().unwrap(), 2_000_000)
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&before).unwrap();
        let bytes: Vec<u8> =
            serde_json::from_value(value["play"]["playback"]["output"].clone()).unwrap();
        let expected = voxy_audio_device::OutputCheckpoint::from_bytes(
            &bytes,
            voxy_audio_device::OutputCheckpointLimits {
                max_bytes: 2_000_000,
                max_queue_frames: 4096,
                max_pump_frames: 256,
                sample_rate: connection.sample_rate(),
            },
        )
        .unwrap();
        assert_eq!(actual, expected);
        scene
            .set_name(node, "Other scene owner published while native PCM held")
            .unwrap();
        let retired = committed.finish(); // Infallible and requires no backend resume call.
        assert!(session.native_checkpoint.is_none() && !session.checkpoint_requested);
        assert_eq!(
            session.device.as_mut().unwrap().poll_paused().unwrap(),
            Some(false)
        );
        for worker in retired {
            worker.join().unwrap();
        }
        println!("NATIVE INFALLIBLE COMMIT/RELEASE PASS; permit cancellation preserved source");
        session.request_checkpoint().unwrap();
        let before = loop {
            if let Some(saved) = session
                .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
            {
                break saved.to_json(2_000_000).unwrap();
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let idle = AudioSessionCheckpoint {
            version: 1,
            play: None,
        };
        let discarded = idle
            .prepare_native(&scene, &project, |_| None, 2_000_000, &connection)
            .unwrap();
        assert!(discarded.discard().is_empty());
        assert_eq!(
            session
                .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
                .unwrap()
                .to_json(2_000_000)
                .unwrap(),
            before
        );
        let mut candidate = idle
            .prepare_native(&scene, &project, |_| None, 2_000_000, &connection)
            .unwrap();
        while !session
            .poll_native_checkpoint_ready(&mut candidate, &scene, &project, |_| {
                Some(ObjectId("sound".into()))
            })
            .unwrap()
        {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let permit = session
            .native_checkpoint_publication(&mut candidate, &scene, &project, |_| {
                Some(ObjectId("sound".into()))
            })
            .unwrap()
            .unwrap();
        drop(permit);
        assert_eq!(
            session
                .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
                .unwrap()
                .to_json(2_000_000)
                .unwrap(),
            before
        );
        let permit = session
            .native_checkpoint_publication(&mut candidate, &scene, &project, |_| {
                Some(ObjectId("sound".into()))
            })
            .unwrap()
            .unwrap();
        let mut committed = permit.publish();
        assert!(committed.session.as_ref().unwrap().play.is_none());
        assert!(committed.session.as_ref().unwrap().connection.is_some());
        assert!(
            committed
                .frozen
                .as_mut()
                .unwrap()
                .checkpoint(4096)
                .unwrap()
                .unwrap()
                .frames
                .is_empty()
        );
        assert_eq!(candidate.idle_pump.as_ref().unwrap().pending_frames(), 0);
        let retired = committed.finish();
        assert!(session.play.is_none() && session.connection.is_none());
        assert!(session.native_checkpoint.is_none() && !session.checkpoint_requested);
        for worker in retired {
            worker.join().unwrap();
        }
        assert!(session.join_workers().is_empty());
        assert!(session.checkpoint_admission().is_ok());
        assert_eq!(session.output_mode(), AudioOutputMode::Native);
        println!(
            "NATIVE ACTIVE-TO-IDLE exact cancellation, empty queue/pump and final device retirement PASS"
        );
        let native_archive = AudioSessionCheckpoint::from_json(&before, 2_000_000).unwrap();
        native_archive.validate_native_output(2_000_000).unwrap();
        let rate = native_archive.native_output_rate().unwrap();
        for wrong_rate in [true, false] {
            session.request_checkpoint().unwrap();
            session.begin_checkpoint_output_startup(false).unwrap();
            if !wrong_rate {
                session.set_output_mode(AudioOutputMode::Offline);
            }
            let error = loop {
                match session.poll_checkpoint_output_startup(
                    if wrong_rate { rate + 1 } else { rate },
                    2_000_000,
                    |_| None,
                ) {
                    Err(error) => break error,
                    Ok(false) => {}
                    Ok(true) => panic!("invalid startup unexpectedly admitted"),
                }
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            };
            assert!(error.to_string().contains(if wrong_rate {
                "sample rate mismatch"
            } else {
                "policy changed"
            }));
            assert!(session.resume_checkpoint().unwrap());
            assert!(session.play.is_none() && session.connection.is_none());
            assert!(session.checkpoint_startup.is_none() && session.native_checkpoint.is_none());
            assert!(session.join_workers().is_empty());
            if !wrong_rate {
                assert_eq!(session.output_mode(), AudioOutputMode::Offline);
            }
            session.set_output_mode(AudioOutputMode::Native);
        }
        session.request_checkpoint().unwrap();
        let idle_before = session
            .poll_checkpoint(|_| None, 2_000_000)
            .unwrap()
            .unwrap()
            .to_json(2_000_000)
            .unwrap();
        session.begin_checkpoint_output_startup(false).unwrap();
        while !session
            .poll_checkpoint_output_startup(rate, 2_000_000, |_| None)
            .unwrap()
        {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            session
                .poll_checkpoint(|_| None, 2_000_000)
                .unwrap()
                .unwrap()
                .to_json(2_000_000)
                .unwrap(),
            idle_before
        );
        let new_connection = session.checkpoint_output_connection().unwrap().unwrap();
        let mut candidate = native_archive
            .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &new_connection)
            .unwrap();
        while !session
            .poll_native_checkpoint_ready(&mut candidate, &scene, &project, |_| None)
            .unwrap()
        {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let permit = session
            .native_checkpoint_publication(&mut candidate, &scene, &project, |_| None)
            .unwrap()
            .unwrap();
        let mut committed = permit.publish();
        let actual = committed
            .session
            .as_ref()
            .unwrap()
            .play
            .as_ref()
            .unwrap()
            .capture_native_output(committed.frozen.as_mut().unwrap(), 2_000_000)
            .unwrap();
        let wire: serde_json::Value = serde_json::from_str(&before).unwrap();
        let bytes: Vec<u8> =
            serde_json::from_value(wire["play"]["playback"]["output"].clone()).unwrap();
        let expected = voxy_audio_device::OutputCheckpoint::from_bytes(
            &bytes,
            voxy_audio_device::OutputCheckpointLimits {
                max_bytes: 2_000_000,
                max_queue_frames: 4096,
                max_pump_frames: 256,
                sample_rate: rate,
            },
        )
        .unwrap();
        assert_eq!(actual, expected);
        assert!(committed.finish().is_empty());
        assert!(session.play.is_some() && session.connection.is_some());
        assert!(
            session.checkpoint_startup.is_none()
                && session.native_checkpoint.is_none()
                && !session.checkpoint_requested
        );
        assert_eq!(new_connection.stats().errors, 0);
        println!(
            "IDLE-TO-NATIVE rate/policy rejection, unchanged idle archive and exact queued PCM/pump commit PASS"
        );
        session.close_play_audio();
        session.close_audio_device();
        assert!(session.join_workers().is_empty());
        // A real offline owner has already consumed its mixed PCM, so conversion
        // must preserve mixer/clock/clip data and introduce no queued prefix.
        std::fs::write(temp.0.join("tone.wav"), wav()).unwrap();
        session.set_play(audio_play::AudioPlay::prepare(&scene, &project, None).unwrap());
        session.play_mut().unwrap().advance(&scene, 7).unwrap();
        let offline = capture(&mut session);
        assert!(offline.native_output_rate().is_none());
        assert_eq!(offline.playback_rate(), Some(rate));
        std::fs::remove_file(temp.0.join("tone.wav")).unwrap();
        let offline_wire: serde_json::Value =
            serde_json::from_str(&offline.to_json(2_000_000).unwrap()).unwrap();
        for cancel in [true, false] {
            session.request_checkpoint().unwrap();
            let source_before = loop {
                if let Some(saved) = session
                    .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                    .unwrap()
                {
                    break saved.to_json(2_000_000).unwrap();
                }
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            };
            session.begin_checkpoint_output_startup(false).unwrap();
            while !session
                .poll_checkpoint_output_startup(rate, 2_000_000, |_| Some(ObjectId("sound".into())))
                .unwrap()
            {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            assert_eq!(
                session
                    .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                    .unwrap()
                    .unwrap()
                    .to_json(2_000_000)
                    .unwrap(),
                source_before
            );
            if cancel {
                while !session.resume_checkpoint().unwrap() {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                assert!(session.join_workers().is_empty());
                assert!(session.play().unwrap().output_connection().is_none());
                let after = capture(&mut session);
                let after: serde_json::Value =
                    serde_json::from_str(&after.to_json(2_000_000).unwrap()).unwrap();
                assert_eq!(
                    after["play"]["playback"]["runtime"],
                    offline_wire["play"]["playback"]["runtime"]
                );
                assert_eq!(
                    after["play"]["playback"]["clock"],
                    offline_wire["play"]["playback"]["clock"]
                );
                continue;
            }
            let connection = session.checkpoint_output_connection().unwrap().unwrap();
            let mut candidate = offline
                .prepare_native(&scene, &project, |_| Some(node), 2_000_000, &connection)
                .unwrap();
            while !session
                .poll_native_checkpoint_ready(&mut candidate, &scene, &project, |_| {
                    Some(ObjectId("sound".into()))
                })
                .unwrap()
            {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            let permit = session
                .native_checkpoint_publication(&mut candidate, &scene, &project, |_| {
                    Some(ObjectId("sound".into()))
                })
                .unwrap()
                .unwrap();
            let mut committed = permit.publish();
            let output = committed
                .session
                .as_ref()
                .unwrap()
                .play
                .as_ref()
                .unwrap()
                .capture_native_output(committed.frozen.as_mut().unwrap(), 2_000_000)
                .unwrap();
            assert!(output.queue.frames.is_empty());
            assert_eq!(output.pump.cursor, output.pump.block.len());
            for worker in committed.finish() {
                worker.join().unwrap();
            }
        }
        session.request_checkpoint().unwrap();
        let restored = loop {
            if let Some(saved) = session
                .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
            {
                break saved;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let restored: serde_json::Value =
            serde_json::from_str(&restored.to_json(2_000_000).unwrap()).unwrap();
        for field in ["runtime", "clock", "pcm", "imported"] {
            assert_eq!(
                restored["play"]["playback"][field],
                offline_wire["play"]["playback"][field]
            );
        }
        while !session.resume_checkpoint().unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        session.close_play_audio();
        session.close_audio_device();
        assert!(session.join_workers().is_empty());
        println!(
            "OFFLINE-TO-NATIVE source-free mixer/clock/PCM exact; empty queue/pump; cancelled owner resumes and re-checkpoints PASS"
        );
        println!("NATIVE SESSION SAVE/CANCEL/RESUME PASS");
    }
    pub(crate) fn wav() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&42_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&48000_u32.to_le_bytes());
        bytes.extend_from_slice(&96000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&6_u32.to_le_bytes());
        for sample in [3000_i16, -5000, 7000] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }
    #[test]
    fn audio_session_load_publishes_candidate_and_failed_load_keeps_live_playback() {
        exercise_session_load(false);
    }
    #[test]
    fn frozen_audio_publication_retains_owner_on_discard_and_releases_old_fence_on_commit() {
        exercise_session_load(true);
    }
    fn exercise_session_load(at_barrier: bool) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "voxy-session-save-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(path.clone());
        std::fs::write(path.join("tone.wav"), wav()).unwrap();
        let project = AuthoringProject::new(&path, &InputRecipe::Direct).unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let descriptor = voxy_gameplay::AudioSource {
            asset: "tone.wav".into(),
            import_settings: None,
            bus: 0,
            gain: 0.8,
            looping: true,
            spatial: false,
            near: 1.,
            far: 5.,
        };
        scene.insert_component(owner, descriptor.clone()).unwrap();
        let mut original = AudioSession::default();
        original.set_play(audio_play::AudioPlay::prepare(&scene, &project, None).unwrap());
        original.play_mut().unwrap().advance(&scene, 7).unwrap();
        let saved = capture(&mut original);
        let json = saved.to_json(2_000_000).unwrap();
        let saved = AudioSessionCheckpoint::from_json(&json, 2_000_000).unwrap();
        let mut fresh = SceneGraph::new(2);
        fresh.spawn(None, Transform::default()).unwrap();
        let fresh_owner = fresh.spawn(None, Transform::default()).unwrap();
        fresh.insert_component(fresh_owner, descriptor).unwrap();
        let mut loaded = AudioSession::default();
        loaded.set_play(audio_play::AudioPlay::prepare(&fresh, &project, None).unwrap());
        loaded.play_mut().unwrap().advance(&fresh, 2).unwrap();
        let retained = loaded.play().unwrap().frames;
        assert!(
            loaded
                .load_checkpoint(&saved, &fresh, &project, |_| None, 2_000_000)
                .is_err()
        );
        assert_eq!(loaded.play().unwrap().frames, retained);
        loaded.play_mut().unwrap().advance(&fresh, 1).unwrap();
        assert_eq!(loaded.play().unwrap().frames, retained + 800);
        let before_stage = loaded.play().unwrap().frames;
        let candidate = saved
            .prepare(&fresh, &project, |_| Some(fresh_owner), 2_000_000)
            .unwrap();
        assert_eq!(loaded.play().unwrap().frames, before_stage);
        assert!(
            candidate
                .validate_output_mode(AudioOutputMode::Offline)
                .is_ok()
        );
        assert!(
            candidate
                .validate_output_mode(AudioOutputMode::Native)
                .unwrap_err()
                .to_string()
                .contains("owned device")
        );
        // Simulate another participant failing after audio preparation.
        for worker in candidate.discard() {
            worker.join().unwrap();
        }
        assert_eq!(loaded.play().unwrap().frames, before_stage);
        let candidate = saved
            .prepare(&fresh, &project, |_| Some(fresh_owner), 2_000_000)
            .unwrap();
        if at_barrier {
            assert!(
                loaded
                    .checkpoint_publication_at_barrier(
                        |_| Some(ObjectId("sound".into())),
                        2_000_000
                    )
                    .is_err()
            );
        }
        loaded.request_checkpoint().unwrap();
        assert!(loaded.checkpoint_publication().is_err());
        for worker in candidate.discard() {
            worker.join().unwrap();
        }
        if at_barrier {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let before = loop {
                if let Some(saved) = loaded
                    .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                    .unwrap()
                {
                    break saved.to_json(2_000_000).unwrap();
                }
                loaded.play_mut().unwrap().poll_reload(&fresh).unwrap();
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            };
            assert!(
                loaded
                    .checkpoint_publication_at_barrier(|_| Some(ObjectId("sound".into())), 1,)
                    .is_err()
            );
            assert!(loaded.checkpoint_requested);
            // Another participant can still reject after admission; dropping
            // this exclusive borrow must neither release nor replace the source.
            {
                let _permit = loaded
                    .checkpoint_publication_at_barrier(
                        |_| Some(ObjectId("sound".into())),
                        2_000_000,
                    )
                    .unwrap();
            }
            assert!(loaded.checkpoint_requested);
            assert_eq!(loaded.play().unwrap().frames, before_stage);
            assert_eq!(
                before,
                loaded
                    .poll_checkpoint(|_| Some(ObjectId("sound".into())), 2_000_000)
                    .unwrap()
                    .unwrap()
                    .to_json(2_000_000)
                    .unwrap()
            );
            let candidate = saved
                .prepare(&fresh, &project, |_| Some(fresh_owner), 2_000_000)
                .unwrap();
            loaded
                .checkpoint_publication_at_barrier(|_| Some(ObjectId("sound".into())), 2_000_000)
                .unwrap()
                .publish(candidate);
            assert!(!loaded.checkpoint_requested);
        } else {
            assert!(loaded.resume_checkpoint().unwrap());
            loaded
                .load_checkpoint(&saved, &fresh, &project, |_| Some(fresh_owner), 2_000_000)
                .unwrap();
        }
        assert!(loaded.join_workers().is_empty());
        for step in 0..120 {
            let count = [0, 1, 2, 5][step % 4];
            original.play_mut().unwrap().advance(&scene, count).unwrap();
            loaded.play_mut().unwrap().advance(&fresh, count).unwrap();
            assert_eq!(
                original.play().unwrap().frames,
                loaded.play().unwrap().frames
            );
            assert_eq!(
                original.play().unwrap().peak.to_bits(),
                loaded.play().unwrap().peak.to_bits()
            );
        }
        let a = serde_json::to_value(capture(&mut original)).unwrap();
        let b = serde_json::to_value(capture(&mut loaded)).unwrap();
        assert_eq!(a["play"]["playback"], b["play"]["playback"]);
        assert!(AudioSessionCheckpoint::from_json(&json, 1).is_err());
        original.close_play_audio();
        loaded.close_play_audio();
        assert!(original.join_workers().is_empty());
        assert!(loaded.join_workers().is_empty());
    }
}
