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
#[derive(Debug)]
pub struct OutputDeviceWorker {
    result: Option<mpsc::Receiver<Result<OutputConnection, String>>>,
    stop: Option<mpsc::Sender<()>>,
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
    fn spawn_with<O: 'static>(
        open: impl FnOnce() -> Result<(OutputConnection, O), String> + Send + 'static,
    ) -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("voxy-audio-device".into())
            .spawn(move || match open() {
                Ok((connection, owner)) => {
                    if matches!(stop_rx.try_recv(), Err(mpsc::TryRecvError::Empty))
                        && ready_tx.send(Ok(connection)).is_ok()
                    {
                        let _ = stop_rx.recv();
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
                result.map(Some)
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.result = None;
                Err("audio device worker terminated before publication".into())
            }
        }
    }
    /// Requests termination without waiting. Native initialization cannot be
    /// forcibly interrupted; cancellation suppresses late connection publication.
    pub fn close(&mut self) {
        self.result = None;
        if let Some(sender) = self.stop.take() {
            let _ = sender.send(());
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
            let (tx, _) = mpsc::sync_channel(1);
            Ok((
                OutputConnection {
                    rate: 48000,
                    sender: OutputSender(tx),
                    counters: Arc::new(Counters::default()),
                },
                Owner(dropped_tx),
            ))
        })
        .unwrap();
        let owner_thread = entered_rx.recv().unwrap();
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
