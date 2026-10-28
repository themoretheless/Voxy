//! One worker owns native device initialization, stream lifetime and termination.
use crate::{Counters, OutputDevice, OutputSender, OutputStats};
use std::sync::{Arc, mpsc};
#[derive(Clone, Debug)]
pub struct OutputConnection {
    rate: u32,
    sender: OutputSender,
    counters: Arc<Counters>,
}
impl OutputConnection {
    #[must_use]
    pub fn shares_output(&self, other: &Self) -> bool {
        self.rate == other.rate && Arc::ptr_eq(&self.sender.0, &other.sender.0)
    }

    #[cfg(test)]
    pub(crate) fn for_test_queue(rate: u32, sender: OutputSender) -> Self {
        Self {
            rate,
            sender,
            counters: Arc::new(Counters::default()),
        }
    }

    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }
    #[must_use]
    pub fn sender(&self) -> &OutputSender {
        &self.sender
    }
    #[must_use]
    pub fn stats(&self) -> OutputStats {
        use std::sync::atomic::Ordering;
        OutputStats {
            supplied: self.counters.supplied.load(Ordering::Relaxed),
            missing: self.counters.missing.load(Ordering::Relaxed),
            errors: self.counters.errors.load(Ordering::Relaxed),
        }
    }
}
trait DeviceOwner {
    fn pause(&mut self) -> Result<(), String>;
    fn resume(&mut self) -> Result<(), String>;
}
impl DeviceOwner for OutputDevice {
    fn pause(&mut self) -> Result<(), String> {
        OutputDevice::pause(self).map_err(|error| error.to_string())
    }
    fn resume(&mut self) -> Result<(), String> {
        self.start().map_err(|error| error.to_string())
    }
}
enum Command {
    Stop,
    SetPaused {
        paused: bool,
        reply: mpsc::SyncSender<Result<bool, String>>,
    },
}
#[derive(Debug)]
struct Transition {
    paused: bool,
    reply: mpsc::Receiver<Result<bool, String>>,
}
#[derive(Debug)]
pub struct OutputDeviceWorker {
    result: Option<mpsc::Receiver<Result<OutputConnection, String>>>,
    stop: Option<mpsc::Sender<Command>>,
    connected: bool,
    published: Option<OutputConnection>,
    paused: Option<bool>,
    transition: Option<Transition>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl OutputDeviceWorker {
    /// Spawns initialization without waiting for native APIs. The native stream
    /// is created, started and destroyed exclusively on this worker.
    /// # Errors
    /// Rejects zero capacity and thread creation failure.
    pub fn spawn(capacity: usize) -> Result<Self, String> {
        if capacity == 0 {
            return Err("zero output capacity".into());
        }
        Self::spawn_with(move || {
            let (device, sender) = OutputDevice::open(capacity).map_err(|e| e.to_string())?;
            device.start().map_err(|e| e.to_string())?;
            let connection = OutputConnection {
                rate: device.sample_rate(),
                sender,
                counters: Arc::clone(&device.counters),
            };
            Ok((connection, device))
        })
    }
    fn spawn_with<O: DeviceOwner + 'static>(
        open: impl FnOnce() -> Result<(OutputConnection, O), String> + Send + 'static,
    ) -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("voxy-audio-device".into())
            .spawn(move || match open() {
                Ok((connection, mut owner)) => {
                    if matches!(stop_rx.try_recv(), Err(mpsc::TryRecvError::Empty))
                        && ready_tx.send(Ok(connection)).is_ok()
                    {
                        while let Ok(command) = stop_rx.recv() {
                            match command {
                                Command::Stop => break,
                                Command::SetPaused { paused, reply } => {
                                    let result = if paused {
                                        owner.pause()
                                    } else {
                                        owner.resume()
                                    };
                                    let _ = reply.send(result.map(|()| paused));
                                }
                            }
                        }
                    }
                    drop(owner);
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            result: Some(ready_rx),
            stop: Some(stop_tx),
            connected: false,
            published: None,
            paused: Some(false),
            transition: None,
            thread: Some(thread),
        })
    }
    /// Returns a connection once ready; never waits. The connection contains no
    /// native stream and may be used by the scene/mixer owner.
    /// # Errors
    /// Returns native initialization failure or unexpected worker termination.
    pub fn poll(&mut self) -> Result<Option<OutputConnection>, String> {
        let Some(receiver) = &self.result else {
            return Ok(None);
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.result = None;
                self.connected = result.is_ok();
                self.published = result.as_ref().ok().cloned();
                result.map(Some)
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.result = None;
                Err("audio device worker terminated before publication".into())
            }
        }
    }
    #[must_use]
    pub fn owns_connection(&self, connection: &OutputConnection) -> bool {
        self.connected
            && !self.is_finished()
            && self
                .published
                .as_ref()
                .is_some_and(|published| published.shares_output(connection))
    }
    /// Queues a native pause without calling platform APIs on the caller thread.
    /// # Errors
    /// Rejects unpublished/closed devices and an opposite in-flight transition.
    pub fn request_pause(&mut self) -> Result<(), String> {
        self.request_paused(true)
    }
    /// Queues native playback resumption without waiting for the platform.
    /// # Errors
    /// Rejects unpublished/closed devices and an opposite in-flight transition.
    pub fn request_resume(&mut self) -> Result<(), String> {
        self.request_paused(false)
    }
    fn request_paused(&mut self, paused: bool) -> Result<(), String> {
        if !self.connected || self.stop.is_none() {
            return Err("audio device is not connected".into());
        }
        if let Some(transition) = &self.transition {
            return if transition.paused == paused {
                Ok(())
            } else {
                Err("audio device transition already pending".into())
            };
        }
        if self.paused == Some(paused) {
            return Ok(());
        }
        let (reply, receiver) = mpsc::sync_channel(1);
        if self
            .stop
            .as_ref()
            .ok_or("audio device is closed")?
            .send(Command::SetPaused { paused, reply })
            .is_err()
        {
            self.connected = false;
            self.paused = None;
            return Err("audio device worker terminated".into());
        }
        self.transition = Some(Transition {
            paused,
            reply: receiver,
        });
        Ok(())
    }
    /// Polls the acknowledged stream state; `None` means the native API is pending.
    /// After a failed transition the state is unknown until a new command succeeds.
    /// A completed pause is a device control acknowledgement, not a PCM snapshot.
    /// # Errors
    /// Reports platform transition failure or unexpected worker termination.
    pub fn poll_paused(&mut self) -> Result<Option<bool>, String> {
        if self.is_finished() {
            self.connected = false;
            self.paused = None;
            self.transition = None;
        }
        if !self.connected {
            return Err("audio device is not connected".into());
        }
        let Some(transition) = &self.transition else {
            return self
                .paused
                .map(Some)
                .ok_or_else(|| "audio device state is unknown after transition failure".into());
        };
        match transition.reply.try_recv() {
            Ok(result) => {
                self.transition = None;
                match result {
                    Ok(paused) => {
                        self.paused = Some(paused);
                        Ok(Some(paused))
                    }
                    Err(error) => {
                        self.paused = None;
                        Err(error)
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.transition = None;
                self.paused = None;
                self.connected = false;
                Err("audio device worker terminated during transition".into())
            }
        }
    }
    /// Requests termination without waiting. Native initialization cannot be
    /// forcibly interrupted; cancellation suppresses late connection publication.
    pub fn close(&mut self) {
        self.result = None;
        self.connected = false;
        self.published = None;
        self.transition = None;
        if let Some(sender) = self.stop.take() {
            let _ = sender.send(Command::Stop);
        }
    }
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.thread
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished)
    }
    /// Closes then joins. Intended for final shutdown, not the UI update path.
    /// # Errors
    /// Reports native-worker panic; completion may wait for native initialization.
    pub fn join(&mut self) -> Result<(), String> {
        self.close();
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|payload| {
                payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| {
                        payload
                            .downcast_ref::<&str>()
                            .map(|text| (*text).to_owned())
                    })
                    .unwrap_or_else(|| "audio device worker panicked".into())
            })?;
        }
        Ok(())
    }
}
impl Drop for OutputDeviceWorker {
    fn drop(&mut self) {
        if let Err(error) = self.join() {
            eprintln!("audio device shutdown: {error}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Owner(mpsc::Sender<std::thread::ThreadId>);
    impl DeviceOwner for Owner {
        fn pause(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn resume(&mut self) -> Result<(), String> {
            Ok(())
        }
    }
    impl DeviceOwner for () {
        fn pause(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn resume(&mut self) -> Result<(), String> {
            Ok(())
        }
    }
    impl Drop for Owner {
        fn drop(&mut self) {
            let _ = self.0.send(std::thread::current().id());
        }
    }
    #[test]
    fn blocked_initialization_poll_and_close_never_wait_and_late_owner_is_destroyed() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let mut worker = OutputDeviceWorker::spawn_with(move || {
            entered_tx.send(std::thread::current().id()).unwrap();
            release_rx.recv().unwrap();
            let (sender, _) = crate::queue::output_queue(1).unwrap();
            Ok((
                OutputConnection {
                    rate: 48000,
                    sender,
                    counters: Arc::new(Counters::default()),
                },
                Owner(dropped_tx),
            ))
        })
        .unwrap();
        let owner_thread = entered_rx.recv().unwrap();
        assert!(worker.request_pause().is_err());
        assert!(worker.request_resume().is_err());
        assert!(worker.poll_paused().is_err());
        assert!(worker.poll().unwrap().is_none());
        worker.close();
        assert!(!worker.is_finished());
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        assert_eq!(dropped_rx.recv().unwrap(), owner_thread);
        assert!(worker.poll().unwrap().is_none());
        worker.join().unwrap();
    }
    #[test]
    fn initialization_failure_is_explicit_and_shutdown_idempotent() {
        let mut worker = OutputDeviceWorker::spawn_with::<()>(|| Err("no device".into())).unwrap();
        while !worker.is_finished() {
            std::thread::yield_now();
        }
        assert_eq!(worker.poll().unwrap_err(), "no device");
        worker.join().unwrap();
        worker.join().unwrap();
    }
}

#[cfg(test)]
mod transition_tests {
    use super::*;
    use std::time::{Duration, Instant};
    type Observed = (bool, std::thread::ThreadId);
    struct ControlledOwner {
        entered: mpsc::Sender<Observed>,
        release: mpsc::Receiver<Result<(), String>>,
        dropped: mpsc::Sender<std::thread::ThreadId>,
    }
    impl ControlledOwner {
        fn transition(&self, paused: bool) -> Result<(), String> {
            self.entered
                .send((paused, std::thread::current().id()))
                .unwrap();
            self.release.recv_timeout(Duration::from_secs(10)).unwrap()
        }
    }
    impl DeviceOwner for ControlledOwner {
        fn pause(&mut self) -> Result<(), String> {
            self.transition(true)
        }
        fn resume(&mut self) -> Result<(), String> {
            self.transition(false)
        }
    }
    impl Drop for ControlledOwner {
        fn drop(&mut self) {
            let _ = self.dropped.send(std::thread::current().id());
        }
    }
    struct Harness {
        worker: OutputDeviceWorker,
        entered: mpsc::Receiver<Observed>,
        release: mpsc::Sender<Result<(), String>>,
        dropped: mpsc::Receiver<std::thread::ThreadId>,
    }
    fn harness() -> Harness {
        let (entered, observed) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let (dropped, destroyed) = mpsc::channel();
        let mut worker = OutputDeviceWorker::spawn_with(move || {
            let (sender, _) = crate::queue::output_queue(1).unwrap();
            Ok((
                OutputConnection {
                    rate: 48000,
                    sender,
                    counters: Arc::new(Counters::default()),
                },
                ControlledOwner {
                    entered,
                    release: released,
                    dropped,
                },
            ))
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while worker.poll().unwrap().is_none() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        Harness {
            worker,
            entered: observed,
            release,
            dropped: destroyed,
        }
    }
    fn acknowledgement(worker: &mut OutputDeviceWorker) -> Result<bool, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(paused) = worker.poll_paused()? {
                return Ok(paused);
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn blocked_native_pause_and_resume_never_block_caller_or_acknowledge_early() {
        let mut h = harness();
        assert_eq!(h.worker.poll_paused().unwrap(), Some(false));
        h.worker.request_pause().unwrap();
        let (paused, owner_thread) = h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(paused);
        assert_ne!(owner_thread, std::thread::current().id());
        assert_eq!(h.worker.poll_paused().unwrap(), None);
        h.worker.request_pause().unwrap();
        assert!(h.worker.request_resume().is_err());
        assert!(matches!(
            h.entered.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        h.release.send(Ok(())).unwrap();
        assert!(acknowledgement(&mut h.worker).unwrap());
        h.worker.request_resume().unwrap();
        assert_eq!(
            h.entered.recv_timeout(Duration::from_secs(5)).unwrap(),
            (false, owner_thread)
        );
        assert_eq!(h.worker.poll_paused().unwrap(), None);
        h.release.send(Ok(())).unwrap();
        assert!(!acknowledgement(&mut h.worker).unwrap());
        h.worker.join().unwrap();
        assert_eq!(
            h.dropped.recv_timeout(Duration::from_secs(5)).unwrap(),
            owner_thread
        );
    }
    #[test]
    fn failed_pause_does_not_report_old_state_and_resume_reaches_native_owner() {
        let mut h = harness();
        h.worker.request_pause().unwrap();
        let (_, owner_thread) = h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
        h.release.send(Err("host pause rejected".into())).unwrap();
        assert_eq!(
            acknowledgement(&mut h.worker).unwrap_err(),
            "host pause rejected"
        );
        assert!(h.worker.poll_paused().unwrap_err().contains("unknown"));
        // The last acknowledged state was running, but native failure makes it
        // unsafe to elide the explicit recovery resume.
        h.worker.request_resume().unwrap();
        assert_eq!(
            h.entered.recv_timeout(Duration::from_secs(5)).unwrap(),
            (false, owner_thread)
        );
        h.release.send(Ok(())).unwrap();
        assert!(!acknowledgement(&mut h.worker).unwrap());
        h.worker.join().unwrap();
    }
    #[test]
    fn failed_resume_requires_explicit_recovery_instead_of_claiming_paused() {
        let mut h = harness();
        h.worker.request_pause().unwrap();
        h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
        h.release.send(Ok(())).unwrap();
        assert!(acknowledgement(&mut h.worker).unwrap());
        h.worker.request_resume().unwrap();
        let (_, owner_thread) = h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
        h.release.send(Err("host resume rejected".into())).unwrap();
        assert_eq!(
            acknowledgement(&mut h.worker).unwrap_err(),
            "host resume rejected"
        );
        assert!(h.worker.poll_paused().is_err());
        h.worker.request_pause().unwrap();
        assert_eq!(
            h.entered.recv_timeout(Duration::from_secs(5)).unwrap(),
            (true, owner_thread)
        );
        h.release.send(Ok(())).unwrap();
        assert!(acknowledgement(&mut h.worker).unwrap());
        h.worker.join().unwrap();
    }
    #[test]
    fn native_owner_panic_cannot_publish_a_successful_transition() {
        let mut h = harness();
        h.worker.request_pause().unwrap();
        let (_, owner_thread) = h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(h.release); // The controlled native API panics on a lost release channel.
        assert!(acknowledgement(&mut h.worker).is_err());
        assert!(h.worker.poll_paused().is_err());
        assert!(h.worker.request_resume().is_err());
        assert!(h.worker.join().is_err());
        assert_eq!(
            h.dropped.recv_timeout(Duration::from_secs(5)).unwrap(),
            owner_thread
        );
        h.worker.join().unwrap();
    }
    #[test]
    fn close_during_blocked_pause_suppresses_late_ack_and_destroys_on_owner_thread() {
        let mut h = harness();
        h.worker.request_pause().unwrap();
        let (_, owner_thread) = h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
        h.worker.close();
        assert!(!h.worker.is_finished());
        assert!(h.worker.poll_paused().is_err());
        assert!(h.worker.request_resume().is_err());
        h.release.send(Ok(())).unwrap();
        h.worker.join().unwrap();
        assert_eq!(
            h.dropped.recv_timeout(Duration::from_secs(5)).unwrap(),
            owner_thread
        );
    }
}

#[cfg(test)]
mod native_checkpoint_tests {
    use super::*;
    #[test]
    #[ignore = "physical default output device; only zero PCM, no window"]
    fn physical_device_pause_freeze_capture_and_resume_zero_pcm() {
        use std::time::{Duration, Instant};
        let mut worker = OutputDeviceWorker::spawn(4096).unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        let connection = loop {
            if let Some(connection) = worker.poll().unwrap() {
                break connection;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        println!(
            "NATIVE AUDIO CONNECTED rate={} stats={:?}",
            connection.sample_rate(),
            connection.stats()
        );
        worker.request_pause().unwrap();
        while worker.poll_paused().unwrap() != Some(true) {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            connection
                .sender()
                .submit_frames(&[[0.0; 2]; 128])
                .submitted,
            128
        );
        let mut frozen = connection.sender().freeze_queue().unwrap();
        let pump = crate::PcmPump::new(256).unwrap();
        let limits = crate::OutputCheckpointLimits {
            max_bytes: 65536,
            max_queue_frames: 4096,
            max_pump_frames: 256,
            sample_rate: connection.sample_rate(),
        };
        let saved = loop {
            if let Some(saved) =
                crate::OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits).unwrap()
            {
                break saved;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert_eq!(saved.queue.frames, vec![[0; 2]; 128]);
        let wire = saved.to_bytes(limits).unwrap();
        assert_eq!(
            crate::OutputCheckpoint::from_bytes(&wire, limits).unwrap(),
            saved
        );
        worker.request_resume().unwrap();
        while worker.poll_paused().unwrap() != Some(false) {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        // Resuming the native API must not consume PCM before queue admission.
        assert_eq!(
            crate::OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits)
                .unwrap()
                .unwrap(),
            saved
        );
        drop(frozen);
        assert_eq!(connection.stats().errors, 0);
        worker.join().unwrap();
        println!("NATIVE AUDIO PAUSE/QUEUE/RESUME PASS");
    }
}
