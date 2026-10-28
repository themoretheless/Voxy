//! Bounded PCM storage with a quiescent checkpoint gate. No callback allocation or mutex.
use crate::{OutputSender, SubmitError};
use crossbeam_queue::ArrayQueue;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
const FROZEN: usize = 1 << (usize::BITS - 1);
#[derive(Debug)]
pub(crate) struct Queue {
    frames: ArrayQueue<[f32; 2]>,
    operations: AtomicUsize,
    connected: AtomicBool,
}
struct Operation<'a>(&'a AtomicUsize);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}
impl Queue {
    pub(crate) fn capacity(&self) -> usize {
        self.frames.capacity()
    }
    fn enter(&self) -> Option<Operation<'_>> {
        self.operations
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                (state < FROZEN - 1).then(|| state + 1)
            })
            .ok()
            .map(|_| Operation(&self.operations))
    }
    pub(crate) fn submit_frames(&self, frames: &[[f32; 2]]) -> crate::PumpReport {
        let mut report = crate::PumpReport {
            submitted: 0,
            stop: None,
        };
        if frames.is_empty() {
            return report;
        }
        if frames[0]
            .iter()
            .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
        {
            report.stop = Some(SubmitError::InvalidFrame);
            return report;
        }
        if !self.connected.load(Ordering::Acquire) {
            report.stop = Some(SubmitError::Closed);
            return report;
        }
        let Some(_operation) = self.enter() else {
            report.stop = Some(SubmitError::Full);
            return report;
        };
        for &frame in frames {
            let error = if frame
                .iter()
                .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
            {
                Some(SubmitError::InvalidFrame)
            } else if !self.connected.load(Ordering::Acquire) {
                Some(SubmitError::Closed)
            } else {
                self.frames.push(frame).err().map(|_| SubmitError::Full)
            };
            if let Some(error) = error {
                report.stop = Some(error);
                break;
            }
            report.submitted += 1;
        }
        report
    }
    pub(crate) fn submit(&self, frame: [f32; 2]) -> Result<(), SubmitError> {
        if !self.connected.load(Ordering::Acquire) {
            return Err(SubmitError::Closed);
        }
        let _operation = self.enter().ok_or(SubmitError::Full)?;
        if !self.connected.load(Ordering::Acquire) {
            return Err(SubmitError::Closed);
        }
        self.frames.push(frame).map_err(|_| SubmitError::Full)
    }
}
#[derive(Debug)]
pub(crate) struct OutputReceiver(Arc<Queue>);
impl OutputReceiver {
    pub(crate) fn render(&self, output: &mut [f32]) -> (u64, u64) {
        let Some(_operation) = self.0.enter() else {
            // Intentional checkpoint silence is not an underrun. Native owners
            // must also pause the stream before treating this as a device fence.
            output.fill(0.0);
            return (0, 0);
        };
        let mut supplied = 0;
        let mut missing = 0;
        for frame in output.chunks_exact_mut(2) {
            if let Some(sample) = self.0.frames.pop() {
                frame.copy_from_slice(&sample);
                supplied += 1;
            } else {
                frame.fill(0.0);
                missing += 1;
            }
        }
        (supplied, missing)
    }
    #[cfg(test)]
    pub(crate) fn recv(&self) -> Result<[f32; 2], &'static str> {
        let _operation = self.0.enter().ok_or("queue frozen")?;
        self.0.frames.pop().ok_or("queue empty")
    }
}
impl Drop for OutputReceiver {
    fn drop(&mut self) {
        self.0.connected.store(false, Ordering::Release);
    }
}
pub(crate) fn output_queue(
    capacity: usize,
) -> Result<(OutputSender, OutputReceiver), QueueCheckpointError> {
    if capacity == 0 {
        return Err(QueueCheckpointError::Capacity);
    }
    let queue = Arc::new(Queue {
        frames: ArrayQueue::new(capacity),
        operations: AtomicUsize::new(0),
        connected: AtomicBool::new(true),
    });
    Ok((OutputSender(queue.clone()), OutputReceiver(queue)))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueCheckpointError {
    Closed,
    AlreadyFrozen,
    NotFrozen,
    Capacity,
    InvalidFrame,
}
impl std::fmt::Display for QueueCheckpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PCM queue checkpoint: {self:?}")
    }
}
impl std::error::Error for QueueCheckpointError {}
/// Exact queued application PCM only; host buffers and playback clocks are separate owners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputQueueCheckpoint {
    pub capacity: usize,
    pub frames: Vec<[u32; 2]>,
}
#[derive(Clone, Debug)]
pub struct PreparedOutputQueue {
    capacity: usize,
    frames: Vec<[f32; 2]>,
}
impl PreparedOutputQueue {
    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }
}
impl OutputQueueCheckpoint {
    /// # Errors
    /// Rejects zero/over-budget capacity, excess frames and invalid normalized PCM.
    pub fn prepare(&self, max_frames: usize) -> Result<PreparedOutputQueue, QueueCheckpointError> {
        if self.capacity == 0 || self.capacity > max_frames || self.frames.len() > self.capacity {
            return Err(QueueCheckpointError::Capacity);
        }
        let frames: Vec<_> = self
            .frames
            .iter()
            .map(|frame| frame.map(f32::from_bits))
            .collect();
        if frames
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
        {
            return Err(QueueCheckpointError::InvalidFrame);
        }
        Ok(PreparedOutputQueue {
            capacity: self.capacity,
            frames,
        })
    }
}
/// Unique queue freeze. Dropping it resumes the same queue, including on cancellation.
#[derive(Debug)]
pub struct FrozenOutputQueue(Arc<Queue>);
impl OutputSender {
    /// Stops new submissions/callback consumption without waiting for in-flight operations.
    /// Keep the token and poll its checkpoint. This does not pause the hardware stream.
    /// # Errors
    /// Rejects disconnected queues and a concurrent freeze owner.
    pub fn freeze_queue(&self) -> Result<FrozenOutputQueue, QueueCheckpointError> {
        if !self.0.connected.load(Ordering::Acquire) {
            return Err(QueueCheckpointError::Closed);
        }
        if self.0.operations.fetch_or(FROZEN, Ordering::AcqRel) & FROZEN != 0 {
            return Err(QueueCheckpointError::AlreadyFrozen);
        }
        Ok(FrozenOutputQueue(self.0.clone()))
    }
}
impl FrozenOutputQueue {
    // The owned publication token retains this unique freeze after quiescence.
    pub(crate) fn install_ready(&mut self, candidate: &PreparedOutputQueue) {
        while self.0.frames.pop().is_some() {}
        for &frame in &candidate.frames {
            self.0
                .frames
                .push(frame)
                .expect("validated frozen queue capacity");
        }
    }
    pub(crate) fn belongs_to(&self, sender: &OutputSender) -> bool {
        Arc::ptr_eq(&self.0, &sender.0)
    }

    fn ready(&self) -> Result<bool, QueueCheckpointError> {
        if !self.0.connected.load(Ordering::Acquire) {
            return Err(QueueCheckpointError::Closed);
        }
        let state = self.0.operations.load(Ordering::Acquire);
        if state & FROZEN == 0 {
            return Err(QueueCheckpointError::NotFrozen);
        }
        Ok(state == FROZEN)
    }
    /// # Errors
    /// Rejects closed queues and admission budgets. `None` leaves an in-flight operation running.
    pub fn checkpoint(
        &mut self,
        max_frames: usize,
    ) -> Result<Option<OutputQueueCheckpoint>, QueueCheckpointError> {
        if self.0.frames.capacity() > max_frames {
            return Err(QueueCheckpointError::Capacity);
        }
        if !self.ready()? {
            return Ok(None);
        }
        // Only this token can access storage now. A complete rotation retains
        // original queue order without replacing its allocation or its owner.
        let count = self.0.frames.len();
        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            let frame = self.0.frames.pop().expect("quiescent queue length");
            frames.push(frame.map(f32::to_bits));
            self.0.frames.push(frame).expect("quiescent queue capacity");
        }
        Ok(Some(OutputQueueCheckpoint {
            capacity: self.0.frames.capacity(),
            frames,
        }))
    }
    /// Publishes validated queued PCM into this original frozen queue, returning
    /// its exact old contents for an outer rollback. No callback can observe a partial update.
    /// # Errors
    /// Rejects disconnected/mismatched queues. `None` means the old operation has not quiesced.
    pub fn replace(
        &mut self,
        candidate: &PreparedOutputQueue,
    ) -> Result<Option<OutputQueueCheckpoint>, QueueCheckpointError> {
        if candidate.capacity != self.0.frames.capacity() {
            return Err(QueueCheckpointError::Capacity);
        }
        let Some(old) = self.checkpoint(candidate.capacity)? else {
            return Ok(None);
        };
        while self.0.frames.pop().is_some() {}
        for &frame in &candidate.frames {
            self.0.frames.push(frame).expect("validated queue capacity");
        }
        Ok(Some(old))
    }
}
impl Drop for FrozenOutputQueue {
    fn drop(&mut self) {
        self.0.operations.fetch_and(!FROZEN, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PcmPump;
    use std::time::{Duration, Instant};
    #[test]
    fn capture_replace_and_rollback_keep_exact_pcm_order_and_original_sender() {
        let (sender, receiver) = output_queue(4).unwrap();
        let frames = [[-0.0_f32, f32::from_bits(1)], [-1.0, 1.0], [0.125, -0.25]];
        for frame in frames {
            sender.submit(frame).unwrap();
        }
        let mut frozen = sender.freeze_queue().unwrap();
        assert!(matches!(
            sender.freeze_queue(),
            Err(QueueCheckpointError::AlreadyFrozen)
        ));
        assert_eq!(sender.submit([0.5; 2]), Err(SubmitError::Full));
        let mut output = [1.0; 4];
        assert_eq!(receiver.render(&mut output), (0, 0));
        assert_eq!(output, [0.0; 4]);
        let saved = frozen.checkpoint(4).unwrap().unwrap();
        assert_eq!(saved.frames, frames.map(|frame| frame.map(f32::to_bits)));
        assert_eq!(frozen.checkpoint(4).unwrap().unwrap(), saved);
        let replacement = OutputQueueCheckpoint {
            capacity: 4,
            frames: vec![[0.75_f32.to_bits(); 2]],
        };
        let old = frozen
            .replace(&replacement.prepare(4).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(old, saved);
        assert_eq!(frozen.checkpoint(4).unwrap().unwrap(), replacement);
        frozen.replace(&old.prepare(4).unwrap()).unwrap().unwrap();
        drop(frozen);
        for expected in frames {
            assert_eq!(
                receiver.recv().unwrap().map(f32::to_bits),
                expected.map(f32::to_bits)
            );
        }
        sender.submit([0.5; 2]).unwrap();
        assert_eq!(receiver.recv().unwrap(), [0.5; 2]);
    }
    #[test]
    fn in_flight_operation_defers_capture_and_cancel_does_not_discard_its_frame() {
        let (sender, receiver) = output_queue(2).unwrap();
        let operation = sender.0.enter().unwrap();
        let mut frozen = sender.freeze_queue().unwrap();
        assert!(frozen.checkpoint(2).unwrap().is_none());
        sender.0.frames.push([0.25; 2]).unwrap(); // Already admitted operation.
        assert!(frozen.checkpoint(2).unwrap().is_none());
        drop(frozen); // Cancel before the old operation has finished.
        let mut next = sender.freeze_queue().unwrap();
        assert!(next.checkpoint(2).unwrap().is_none());
        drop(operation);
        assert_eq!(
            next.checkpoint(2).unwrap().unwrap().frames,
            vec![[0.25_f32.to_bits(); 2]]
        );
        drop(next);
        assert_eq!(receiver.recv().unwrap(), [0.25; 2]);
    }
    #[test]
    fn malformed_or_mismatched_candidate_and_budget_failure_preserve_source_queue() {
        let (sender, receiver) = output_queue(2).unwrap();
        sender.submit([0.5; 2]).unwrap();
        let mut frozen = sender.freeze_queue().unwrap();
        let before = frozen.checkpoint(2).unwrap().unwrap();
        assert_eq!(frozen.checkpoint(1), Err(QueueCheckpointError::Capacity));
        for bad in [
            OutputQueueCheckpoint {
                capacity: 0,
                frames: Vec::new(),
            },
            OutputQueueCheckpoint {
                capacity: 3,
                frames: Vec::new(),
            },
            OutputQueueCheckpoint {
                capacity: 1,
                frames: vec![[0; 2]; 2],
            },
            OutputQueueCheckpoint {
                capacity: 2,
                frames: vec![[f32::NAN.to_bits(); 2]],
            },
            OutputQueueCheckpoint {
                capacity: 2,
                frames: vec![[1.5_f32.to_bits(); 2]],
            },
        ] {
            assert!(bad.prepare(2).is_err());
        }
        let wrong = OutputQueueCheckpoint {
            capacity: 1,
            frames: vec![[0; 2]],
        }
        .prepare(2)
        .unwrap();
        assert_eq!(frozen.replace(&wrong), Err(QueueCheckpointError::Capacity));
        assert_eq!(frozen.checkpoint(2).unwrap().unwrap(), before);
        drop(receiver);
        assert_eq!(frozen.checkpoint(2), Err(QueueCheckpointError::Closed));
        drop(frozen);
        assert_eq!(sender.submit([0.0; 2]), Err(SubmitError::Closed));
    }
    #[test]
    fn batch_validation_and_backpressure_match_single_frame_prefixes() {
        for frames in [
            vec![],
            vec![[0.25; 2]],
            vec![[0.25; 2], [0.5; 2], [0.75; 2]],
            vec![[f32::NAN; 2]],
            vec![[0.25; 2], [f32::INFINITY; 2]],
        ] {
            for state in 0..3 {
                let (old_sender, old_receiver) = output_queue(2).unwrap();
                let (new_sender, new_receiver) = output_queue(2).unwrap();
                let receivers = (old_receiver, new_receiver);
                let frozen = if state == 1 {
                    Some((
                        old_sender.freeze_queue().unwrap(),
                        new_sender.freeze_queue().unwrap(),
                    ))
                } else {
                    None
                };
                if state == 2 {
                    drop(receivers);
                }
                let mut expected = crate::PumpReport {
                    submitted: 0,
                    stop: None,
                };
                for &frame in &frames {
                    match old_sender.submit(frame) {
                        Ok(()) => expected.submitted += 1,
                        Err(error) => {
                            expected.stop = Some(error);
                            break;
                        }
                    }
                }
                assert_eq!(new_sender.submit_frames(&frames), expected);
                drop(frozen);
            }
        }
    }
    #[test]
    fn queued_pcm_and_unsent_pump_suffix_continue_720_callbacks_without_duplication() {
        let (source, source_receiver) = output_queue(5).unwrap();
        let (target, target_receiver) = output_queue(5).unwrap();
        let mut original = PcmPump::new(7).unwrap();
        let mut rendered = 0_u32;
        let render = |block: &mut [[f32; 2]], next: &mut u32| {
            for frame in block {
                *frame = [f32::from_bits(0x3e00_0000 + *next), -0.0];
                *next += 1;
            }
        };
        original.pump(&source, 100, |block| render(block, &mut rendered));
        let mut prefix = [0.0; 4];
        assert_eq!(source_receiver.render(&mut prefix), (2, 0));
        let mut frozen = source.freeze_queue().unwrap();
        let queue = frozen.checkpoint(5).unwrap().unwrap();
        let mut restored = original.checkpoint().prepare(7).unwrap();
        let mut restored_rendered = rendered;
        let mut target_fence = target.freeze_queue().unwrap();
        target_fence
            .replace(&queue.prepare(5).unwrap())
            .unwrap()
            .unwrap();
        drop(target_fence);
        drop(frozen);
        for index in 0..720 {
            let budget = 1 + index % 13;
            assert_eq!(
                original.pump(&source, budget, |b| render(b, &mut rendered)),
                restored.pump(&target, budget, |b| render(b, &mut restored_rendered))
            );
            let mut a = [0.0_f32; 8];
            let mut b = [0.0_f32; 8];
            assert_eq!(
                source_receiver.render(&mut a),
                target_receiver.render(&mut b)
            );
            assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
            assert_eq!(original.checkpoint(), restored.checkpoint());
            assert_eq!(rendered, restored_rendered);
        }
    }
    #[test]
    fn concurrent_producer_and_callback_preserve_sequence_through_repeated_freezes() {
        let (sender, receiver) = output_queue(64).unwrap();
        let producer = sender.clone();
        let producer = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            for index in 0..20000_u32 {
                let frame = [f32::from_bits(0x3e00_0000 + index), -0.0];
                loop {
                    match producer.submit(frame) {
                        Ok(()) => break,
                        Err(SubmitError::Full) => {
                            assert!(Instant::now() < deadline, "producer stalled");
                            std::thread::yield_now();
                        }
                        error => panic!("unexpected submission result {error:?}"),
                    }
                }
            }
        });
        let consumer = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            for index in 0..20000_u32 {
                loop {
                    // Exercise the exact callback path, including checkpoint silence.
                    let mut frame = [0.0; 2];
                    if receiver.render(&mut frame).0 == 1 {
                        assert_eq!(
                            frame.map(f32::to_bits),
                            [0x3e00_0000 + index, (-0.0_f32).to_bits()]
                        );
                        break;
                    }
                    assert!(Instant::now() < deadline, "consumer stalled");
                    std::thread::yield_now();
                }
            }
            // Keep the receiver alive until the last producer and freeze have finished.
            receiver
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        for _ in 0..100 {
            let mut frozen = sender.freeze_queue().unwrap();
            while frozen.checkpoint(64).unwrap().is_none() {
                assert!(Instant::now() < deadline, "freeze stalled");
                std::thread::yield_now();
            }
            let before = frozen.checkpoint(64).unwrap().unwrap();
            let old = frozen
                .replace(&before.prepare(64).unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(old, before);
            drop(frozen);
        }
        producer.join().unwrap();
        let _receiver = consumer.join().unwrap();
    }
}

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::{hint::black_box, time::Instant};
    fn reference(blocks: usize) -> u128 {
        let (sender, receiver) = std::sync::mpsc::sync_channel(4096);
        let mut output = [0.0; 256];
        let start = Instant::now();
        for _ in 0..blocks {
            for _ in 0..128 {
                let frame = black_box([0.25_f32, -0.5]);
                assert!(
                    frame
                        .iter()
                        .all(|value| value.is_finite() && (-1.0..=1.0).contains(value))
                );
                sender.try_send(frame).unwrap();
            }
            for frame in output.chunks_exact_mut(2) {
                frame.copy_from_slice(&receiver.try_recv().unwrap());
            }
            black_box(&output);
        }
        start.elapsed().as_nanos()
    }
    fn checkpointable(blocks: usize) -> u128 {
        let (sender, receiver) = output_queue(4096).unwrap();
        let mut output = [0.0; 256];
        let start = Instant::now();
        for _ in 0..blocks {
            for _ in 0..128 {
                sender.submit(black_box([0.25_f32, -0.5])).unwrap();
            }
            assert_eq!(receiver.render(&mut output), (128, 0));
            black_box(&output);
        }
        start.elapsed().as_nanos()
    }
    fn batched(blocks: usize) -> u128 {
        let (sender, receiver) = output_queue(4096).unwrap();
        let input = [[0.25_f32, -0.5]; 128];
        let mut output = [0.0; 256];
        let start = Instant::now();
        for _ in 0..blocks {
            assert_eq!(
                sender.submit_frames(black_box(&input)),
                crate::PumpReport {
                    submitted: 128,
                    stop: None
                }
            );
            assert_eq!(receiver.render(&mut output), (128, 0));
            black_box(&output);
        }
        start.elapsed().as_nanos()
    }
    #[test]
    #[ignore = "controlled CPU microbenchmark, no audio device or frame-rate qualification"]
    fn paired_former_channel_and_checkpointable_queue() {
        let blocks = 8192;
        reference(64);
        checkpointable(64);
        for pair in 0..9 {
            let (old, new, batch) = match pair % 3 {
                0 => (reference(blocks), checkpointable(blocks), batched(blocks)),
                1 => {
                    let batch = batched(blocks);
                    let old = reference(blocks);
                    (old, checkpointable(blocks), batch)
                }
                _ => {
                    let new = checkpointable(blocks);
                    let batch = batched(blocks);
                    (reference(blocks), new, batch)
                }
            };
            println!(
                "PCM_QUEUE_PAIR pair={pair} frames={} reference_ns={old} checkpointable_ns={new} batched_ns={batch}",
                blocks * 128
            );
        }
    }
}
