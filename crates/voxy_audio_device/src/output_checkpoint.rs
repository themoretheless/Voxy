//! Bounded exact transport of queued application PCM and the original pump suffix.
//! Native host buffers, mixer state and hardware clocks remain outer participants.
use crate::{
    FrozenOutputQueue, OutputConnection, OutputQueueCheckpoint, PcmPump, PcmPumpCheckpoint,
    PreparedOutputQueue,
};
const HEADER: usize = 80;
const MAGIC: &[u8; 8] = b"VOXYOUT1";
#[derive(Clone, Copy, Debug)]
pub struct OutputCheckpointLimits {
    pub max_bytes: usize,
    pub max_queue_frames: usize,
    pub max_pump_frames: usize,
    pub sample_rate: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputCheckpoint {
    pub sample_rate: u32,
    pub queue: OutputQueueCheckpoint,
    pub pump: PcmPumpCheckpoint,
}
#[derive(Debug)]
pub struct PreparedOutputCheckpoint {
    limits: OutputCheckpointLimits,
    queue: PreparedOutputQueue,
    pump: Option<PcmPump>,
}
/// Device-free consumer of the already mixed queue and pump suffix. The mixer
/// cursor is ahead of these samples; consumption must not advance it again.
#[derive(Debug)]
pub struct OfflineOutput {
    sample_rate: u32,
    capacity: usize,
    queue: std::collections::VecDeque<[u32; 2]>,
    pump: PcmPumpCheckpoint,
}
impl OfflineOutput {
    #[must_use]
    pub fn checkpoint(&self) -> OutputCheckpoint {
        OutputCheckpoint {
            sample_rate: self.sample_rate,
            queue: OutputQueueCheckpoint {
                capacity: self.capacity,
                frames: self.queue.iter().copied().collect(),
            },
            pump: self.pump.clone(),
        }
    }
    #[must_use]
    pub fn remaining_frames(&self) -> usize {
        self.queue.len() + self.pump.block.len() - self.pump.cursor
    }
    /// Consume a prefix for a sink that intentionally discards rendered PCM.
    #[must_use]
    pub fn discard_frames(&mut self, count: usize) -> usize {
        let queued = count.min(self.queue.len());
        self.queue.drain(..queued);
        let suffix = (count - queued).min(self.pump.block.len() - self.pump.cursor);
        self.pump.cursor += suffix;
        queued + suffix
    }
    /// Copy the available prefix without arithmetic or synthesis. Returns the
    /// copied count; callers render any remaining destination with their mixer.
    pub fn render_prefix(&mut self, destination: &mut [[f32; 2]]) -> usize {
        let mut copied = 0;
        for target in destination {
            let frame = if let Some(frame) = self.queue.pop_front() {
                frame
            } else if self.pump.cursor < self.pump.block.len() {
                let frame = self.pump.block[self.pump.cursor];
                self.pump.cursor += 1;
                frame
            } else {
                break;
            };
            *target = frame.map(f32::from_bits);
            copied += 1;
        }
        copied
    }
}
/// Owns the admitted freeze and all allocations needed for an infallible logical commit.
#[derive(Debug)]
pub struct ReadyOutputPublication {
    limits: OutputCheckpointLimits,
    queue: PreparedOutputQueue,
    pump: PcmPump,
    old: OutputCheckpoint,
    frozen: FrozenOutputQueue,
}
impl ReadyOutputPublication {
    #[must_use]
    pub fn cancel(self) -> (FrozenOutputQueue, PreparedOutputCheckpoint) {
        (
            self.frozen,
            PreparedOutputCheckpoint {
                limits: self.limits,
                queue: self.queue,
                pump: Some(self.pump),
            },
        )
    }
    /// No native API or admission check remains. Caller must retain its target owner.
    #[must_use]
    pub fn publish(mut self, target: &mut PcmPump) -> (FrozenOutputQueue, OutputCheckpoint) {
        self.frozen.install_ready(&self.queue);
        *target = self.pump;
        (self.frozen, self.old)
    }
}
type Error = String;
fn invalid() -> Error {
    "invalid or over-budget PCM output checkpoint".into()
}
impl OutputCheckpoint {
    /// Prepare an exact bounded consumer without creating a device or callback.
    /// # Errors
    /// Rejects malformed samples, capacities, cursors, rates and caller limits.
    pub fn prepare_offline(&self, limits: OutputCheckpointLimits) -> Result<OfflineOutput, Error> {
        self.size(limits)?;
        Ok(OfflineOutput {
            sample_rate: self.sample_rate,
            capacity: self.queue.capacity,
            queue: self.queue.frames.clone().into(),
            pump: self.pump.clone(),
        })
    }
    fn size(&self, limits: OutputCheckpointLimits) -> Result<usize, Error> {
        if self.sample_rate == 0
            || self.sample_rate != limits.sample_rate
            || self.queue.capacity == 0
            || self.queue.capacity > limits.max_queue_frames
            || self.queue.frames.len() > self.queue.capacity
            || self.pump.block.is_empty()
            || self.pump.block.len() > limits.max_pump_frames
            || self.pump.cursor > self.pump.block.len()
        {
            return Err(invalid());
        }
        let size = self
            .queue
            .frames
            .len()
            .checked_add(self.pump.block.len())
            .and_then(|count| count.checked_mul(8))
            .and_then(|bytes| bytes.checked_add(HEADER))
            .ok_or_else(invalid)?;
        if size > limits.max_bytes {
            return Err(invalid());
        }
        for &bits in self.queue.frames.iter().chain(&self.pump.block).flatten() {
            let value = f32::from_bits(bits);
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err(invalid());
            }
        }
        Ok(size)
    }
    /// Captures the actual connection queue and pump, polling in-flight queue activity.
    /// # Errors
    /// Rejects a foreign freeze, rate mismatch, closed queue and caller admission limits.
    pub fn capture(
        connection: &OutputConnection,
        frozen: &mut FrozenOutputQueue,
        pump: &PcmPump,
        limits: OutputCheckpointLimits,
    ) -> Result<Option<Self>, Error> {
        if !frozen.belongs_to(connection.sender()) || connection.sample_rate() != limits.sample_rate
        {
            return Err(invalid());
        }
        let Some(queue) = frozen
            .checkpoint(limits.max_queue_frames)
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        let record = Self {
            sample_rate: connection.sample_rate(),
            queue,
            pump: pump.checkpoint(),
        };
        record.size(limits)?;
        Ok(Some(record))
    }
    /// # Errors
    /// Rejects invalid PCM, cursor, rate, capacity and aggregate byte limits before encoding.
    pub fn to_bytes(&self, limits: OutputCheckpointLimits) -> Result<Vec<u8>, Error> {
        let size = self.size(limits)?;
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.sample_rate.to_le_bytes());
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        for value in [
            self.queue.capacity,
            self.queue.frames.len(),
            self.pump.block.len(),
            self.pump.cursor,
        ] {
            bytes.extend_from_slice(&u64::try_from(value).map_err(|_| invalid())?.to_le_bytes());
        }
        bytes.extend_from_slice(&[0; 32]);
        for frame in self.queue.frames.iter().chain(&self.pump.block) {
            for bits in frame {
                bytes.extend_from_slice(&bits.to_le_bytes());
            }
        }
        let mut hash = blake3::Hasher::new();
        hash.update(&bytes[..48]);
        hash.update(&bytes[HEADER..]);
        bytes[48..HEADER].copy_from_slice(hash.finalize().as_bytes());
        Ok(bytes)
    }
    /// # Errors
    /// Checks header, aggregate sizes and checksum before allocating decoded frame arrays.
    pub fn from_bytes(bytes: &[u8], limits: OutputCheckpointLimits) -> Result<Self, Error> {
        if bytes.len() < HEADER || bytes.len() > limits.max_bytes || &bytes[..8] != MAGIC {
            return Err(invalid());
        }
        let rate = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| invalid())?);
        let channels = u32::from_le_bytes(bytes[12..16].try_into().map_err(|_| invalid())?);
        if rate == 0 || rate != limits.sample_rate || channels != 2 {
            return Err(invalid());
        }
        let number = |offset| -> Result<usize, Error> {
            usize::try_from(u64::from_le_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .map_err(|_| invalid())?,
            ))
            .map_err(|_| invalid())
        };
        let capacity = number(16)?;
        let queued = number(24)?;
        let block = number(32)?;
        let cursor = number(40)?;
        if capacity == 0
            || capacity > limits.max_queue_frames
            || queued > capacity
            || block == 0
            || block > limits.max_pump_frames
            || cursor > block
        {
            return Err(invalid());
        }
        let expected = queued
            .checked_add(block)
            .and_then(|n| n.checked_mul(8))
            .and_then(|n| n.checked_add(HEADER))
            .ok_or_else(invalid)?;
        if expected != bytes.len() {
            return Err(invalid());
        }
        let mut hash = blake3::Hasher::new();
        hash.update(&bytes[..48]);
        hash.update(&bytes[HEADER..]);
        if hash.finalize().as_bytes() != &bytes[48..HEADER] {
            return Err("PCM output checkpoint checksum".into());
        }
        let frames = |slice: &[u8]| -> Result<Vec<[u32; 2]>, Error> {
            slice
                .chunks_exact(8)
                .map(|frame| {
                    Ok([
                        u32::from_le_bytes(frame[..4].try_into().map_err(|_| invalid())?),
                        u32::from_le_bytes(frame[4..].try_into().map_err(|_| invalid())?),
                    ])
                })
                .collect()
        };
        let split = HEADER + queued * 8;
        let record = Self {
            sample_rate: rate,
            queue: OutputQueueCheckpoint {
                capacity,
                frames: frames(&bytes[HEADER..split])?,
            },
            pump: PcmPumpCheckpoint {
                block: frames(&bytes[split..])?,
                cursor,
            },
        };
        record.size(limits)?;
        Ok(record)
    }
    /// # Errors
    /// Validates every participant before producing a detached publication candidate.
    pub fn prepare(
        &self,
        limits: OutputCheckpointLimits,
    ) -> Result<PreparedOutputCheckpoint, Error> {
        self.size(limits)?;
        Ok(PreparedOutputCheckpoint {
            limits,
            queue: self
                .queue
                .prepare(limits.max_queue_frames)
                .map_err(|e| e.to_string())?,
            pump: Some(
                self.pump
                    .prepare(limits.max_pump_frames)
                    .map_err(|e| e.to_string())?,
            ),
        })
    }
}
impl PreparedOutputCheckpoint {
    /// Completes validation/allocation, moving the unique freeze only on readiness.
    /// Errors/pending leave both the candidate and freeze slot untouched.
    /// # Errors
    /// Rejects incompatible owners/limits and an already consumed candidate.
    pub fn stage_publication(
        &mut self,
        connection: &OutputConnection,
        frozen: &mut Option<FrozenOutputQueue>,
        target: &PcmPump,
    ) -> Result<Option<ReadyOutputPublication>, Error> {
        self.validate_connection(connection)?;
        let gate = frozen.as_mut().ok_or_else(invalid)?;
        if !gate.belongs_to(connection.sender())
            || target.block_frames() > self.limits.max_pump_frames
        {
            return Err(invalid());
        }
        let Some(queue) = gate
            .checkpoint(self.limits.max_queue_frames)
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        let old = OutputCheckpoint {
            sample_rate: connection.sample_rate(),
            queue,
            pump: target.checkpoint(),
        };
        old.size(self.limits)?;
        let queue = self.queue.clone();
        Ok(Some(ReadyOutputPublication {
            limits: self.limits,
            queue,
            pump: self.pump.take().expect("admitted prepared pump"),
            old,
            frozen: frozen.take().expect("admitted unique freeze"),
        }))
    }
    /// Validates a future destination without touching its queue or native API.
    /// # Errors
    /// Rejects consumed candidates, incompatible native rate and queue capacity.
    pub fn validate_connection(&self, connection: &OutputConnection) -> Result<(), Error> {
        if self.pump.is_none()
            || connection.sample_rate() != self.limits.sample_rate
            || connection.sender().capacity() != self.queue.capacity()
        {
            return Err(invalid());
        }
        Ok(())
    }

    /// Replaces queue and pump together while retaining exact rollback data.
    /// Native device pause/publication admission remains the enclosing owner's responsibility.
    /// # Errors
    /// Rejects foreign/closed queues, rate mismatch, consumed candidates and inadmissible old state.
    /// `None` retains the candidate and means the queue has not quiesced yet.
    pub fn publish(
        &mut self,
        connection: &OutputConnection,
        frozen: &mut FrozenOutputQueue,
        target: &mut PcmPump,
    ) -> Result<Option<OutputCheckpoint>, Error> {
        if self.pump.is_none()
            || target.block_frames() > self.limits.max_pump_frames
            || connection.sample_rate() != self.limits.sample_rate
            || !frozen.belongs_to(connection.sender())
        {
            return Err(invalid());
        }
        let Some(old_queue) = frozen
            .checkpoint(self.limits.max_queue_frames)
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        let old = OutputCheckpoint {
            sample_rate: connection.sample_rate(),
            queue: old_queue,
            pump: target.checkpoint(),
        };
        old.size(self.limits)?;
        if frozen
            .replace(&self.queue)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Ok(None);
        }
        *target = self.pump.take().expect("validated prepared output pump");
        Ok(Some(old))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> OutputCheckpointLimits {
        OutputCheckpointLimits {
            max_bytes: 4096,
            max_queue_frames: 16,
            max_pump_frames: 16,
            sample_rate: 44101,
        }
    }
    fn connection(capacity: usize) -> (OutputConnection, crate::queue::OutputReceiver) {
        let (sender, receiver) = crate::queue::output_queue(capacity).unwrap();
        (OutputConnection::for_test_queue(44101, sender), receiver)
    }
    fn render(block: &mut [[f32; 2]], clock: &mut u32) {
        for frame in block {
            *frame = [f32::from_bits(0x3e00_0000 + *clock), -0.0];
            *clock += 1;
        }
    }
    #[test]
    fn offline_prefix_preserves_queue_suffix_bits_and_roundtrips_partial_consumption() {
        let record = OutputCheckpoint {
            sample_rate: 44101,
            queue: OutputQueueCheckpoint {
                capacity: 5,
                frames: vec![[0x8000_0000, 1], [0x3f00_0000, 0xbf00_0000]],
            },
            pump: PcmPumpCheckpoint {
                block: vec![[0, 0], [2, 0x8000_0000], [3, 0]],
                cursor: 1,
            },
        };
        let mut offline = record.prepare_offline(limits()).unwrap();
        assert_eq!(offline.checkpoint(), record);
        let mut first = [[1.; 2]; 1];
        assert_eq!(offline.render_prefix(&mut first), 1);
        assert_eq!(first[0].map(f32::to_bits), record.queue.frames[0]);
        let partial = offline.checkpoint();
        let bytes = partial.to_bytes(limits()).unwrap();
        let partial = OutputCheckpoint::from_bytes(&bytes, limits()).unwrap();
        let mut restored = partial.prepare_offline(limits()).unwrap();
        let mut tail = [[1.; 2]; 5];
        assert_eq!(restored.render_prefix(&mut tail), 3);
        assert_eq!(
            tail[..3]
                .iter()
                .map(|f| f.map(f32::to_bits))
                .collect::<Vec<_>>(),
            vec![
                record.queue.frames[1],
                record.pump.block[1],
                record.pump.block[2]
            ]
        );
        assert_eq!(tail[3..], [[1.; 2]; 2]);
        assert_eq!(restored.remaining_frames(), 0);
        assert_eq!(restored.discard_frames(usize::MAX), 0);
        assert_eq!(offline.discard_frames(2), 2);
        assert_eq!(offline.remaining_frames(), 1);
        assert_eq!(offline.checkpoint().pump.cursor, 2);
    }
    #[test]
    fn binary_output_roundtrip_publishes_queue_and_suffix_and_continues_720_callbacks() {
        let (source, source_rx) = connection(5);
        let (target, target_rx) = connection(5);
        let mut original = PcmPump::new(7).unwrap();
        let mut target_pump = PcmPump::new(3).unwrap();
        let mut source_clock = 0_u32;
        original.pump(source.sender(), 100, |b| render(b, &mut source_clock));
        let mut source_fence = source.sender().freeze_queue().unwrap();
        let record = OutputCheckpoint::capture(&source, &mut source_fence, &original, limits())
            .unwrap()
            .unwrap();
        let bytes = record.to_bytes(limits()).unwrap();
        let decoded = OutputCheckpoint::from_bytes(&bytes, limits()).unwrap();
        assert_eq!(record, decoded);
        let mut staged = decoded.prepare(limits()).unwrap();
        let mut target_fence = target.sender().freeze_queue().unwrap();
        let old = staged
            .publish(&target, &mut target_fence, &mut target_pump)
            .unwrap()
            .unwrap();
        assert!(old.queue.frames.is_empty());
        assert_eq!(old.pump.block.len(), 3);
        assert!(
            staged
                .publish(&target, &mut target_fence, &mut target_pump)
                .is_err()
        );
        let mut restored_clock = source_clock;
        drop(source_fence);
        drop(target_fence);
        for frame in 0..720 {
            let budget = frame % 17;
            assert_eq!(
                original.pump(source.sender(), budget, |b| render(b, &mut source_clock)),
                target_pump.pump(target.sender(), budget, |b| render(b, &mut restored_clock))
            );
            let mut a = [0.0_f32; 8];
            let mut b = a;
            assert_eq!(source_rx.render(&mut a), target_rx.render(&mut b));
            assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
            assert_eq!(original.checkpoint(), target_pump.checkpoint());
            assert_eq!(source_clock, restored_clock);
        }
    }
    #[test]
    fn owned_publication_cancel_preserves_source_and_commit_keeps_pcm_gated() {
        let (connection, receiver) = connection(5);
        connection
            .sender()
            .submit([-0.0, f32::from_bits(1)])
            .unwrap();
        let mut pump = PcmPump::new(3).unwrap();
        let mut gate = Some(connection.sender().freeze_queue().unwrap());
        let original =
            OutputCheckpoint::capture(&connection, gate.as_mut().unwrap(), &pump, limits())
                .unwrap()
                .unwrap();
        let restored = OutputCheckpoint {
            sample_rate: 44101,
            queue: OutputQueueCheckpoint {
                capacity: 5,
                frames: vec![[0.75_f32.to_bits(), (-0.0_f32).to_bits()]],
            },
            pump: PcmPumpCheckpoint {
                block: vec![[0.25_f32.to_bits(), 1]; 4],
                cursor: 2,
            },
        };
        let mut candidate = restored.prepare(limits()).unwrap();
        let ready = candidate
            .stage_publication(&connection, &mut gate, &pump)
            .unwrap()
            .unwrap();
        assert!(gate.is_none());
        let mut output = [1.0; 2];
        assert_eq!(receiver.render(&mut output), (0, 0));
        let (frozen, restored_candidate) = ready.cancel();
        gate = Some(frozen);
        candidate = restored_candidate;
        assert_eq!(
            OutputCheckpoint::capture(&connection, gate.as_mut().unwrap(), &pump, limits())
                .unwrap()
                .unwrap(),
            original
        );
        let ready = candidate
            .stage_publication(&connection, &mut gate, &pump)
            .unwrap()
            .unwrap();
        let (mut frozen, old) = ready.publish(&mut pump);
        assert_eq!(old, original);
        assert_eq!(
            OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
                .unwrap()
                .unwrap(),
            restored
        );
        assert_eq!(receiver.render(&mut output), (0, 0));
        drop(frozen);
        assert_eq!(
            receiver.recv().unwrap().map(f32::to_bits),
            restored.queue.frames[0]
        );
    }
    #[test]
    fn destination_metadata_admission_does_not_mutate_queue_and_rejects_consumed_candidate() {
        let (connection, receiver) = connection(5);
        connection
            .sender()
            .submit([-0.0, f32::from_bits(1)])
            .unwrap();
        let mut pump = PcmPump::new(3).unwrap();
        let mut frozen = connection.sender().freeze_queue().unwrap();
        let before = OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
            .unwrap()
            .unwrap();
        let mut prepared = before.prepare(limits()).unwrap();
        prepared.validate_connection(&connection).unwrap();
        let (wrong_rate, _rx) = crate::queue::output_queue(5).unwrap();
        let wrong_rate = OutputConnection::for_test_queue(48000, wrong_rate);
        assert!(prepared.validate_connection(&wrong_rate).is_err());
        let (wrong_capacity, _rx2) = crate::queue::output_queue(6).unwrap();
        let wrong_capacity = OutputConnection::for_test_queue(44101, wrong_capacity);
        assert!(prepared.validate_connection(&wrong_capacity).is_err());
        assert_eq!(
            OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
                .unwrap()
                .unwrap(),
            before
        );
        prepared
            .publish(&connection, &mut frozen, &mut pump)
            .unwrap()
            .unwrap();
        assert!(prepared.validate_connection(&connection).is_err());
        drop(frozen);
        assert_eq!(
            receiver.recv().unwrap().map(f32::to_bits),
            before.queue.frames[0]
        );
    }
    #[test]
    fn correctly_checksummed_semantic_corruption_is_rejected_before_publication() {
        let (connection, _receiver) = connection(5);
        let pump = PcmPump::new(3).unwrap();
        let mut frozen = connection.sender().freeze_queue().unwrap();
        let before = OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
            .unwrap()
            .unwrap();
        let bytes = before.to_bytes(limits()).unwrap();
        for (offset, value) in [(16, u64::MAX), (24, u64::MAX), (32, u64::MAX), (40, 4)] {
            let mut bad = bytes.clone();
            bad[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            rehash(&mut bad);
            assert!(OutputCheckpoint::from_bytes(&bad, limits()).is_err());
        }
        for bits in [
            f32::NAN.to_bits(),
            f32::INFINITY.to_bits(),
            1.5_f32.to_bits(),
        ] {
            let mut bad = bytes.clone();
            bad[HEADER..HEADER + 4].copy_from_slice(&bits.to_le_bytes());
            rehash(&mut bad);
            assert!(OutputCheckpoint::from_bytes(&bad, limits()).is_err());
        }
        assert_eq!(
            OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
                .unwrap()
                .unwrap(),
            before
        );
    }
    fn rehash(bytes: &mut [u8]) {
        let mut hash = blake3::Hasher::new();
        hash.update(&bytes[..48]);
        hash.update(&bytes[HEADER..]);
        bytes[48..HEADER].copy_from_slice(hash.finalize().as_bytes());
    }
    #[test]
    fn invalid_wire_and_foreign_or_over_budget_publication_leave_live_output_intact() {
        let (connection, _receiver) = connection(5);
        connection
            .sender()
            .submit([-0.0, f32::from_bits(1)])
            .unwrap();
        let mut pump = PcmPump::new(3).unwrap();
        let mut frozen = connection.sender().freeze_queue().unwrap();
        let before = OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
            .unwrap()
            .unwrap();
        let bytes = before.to_bytes(limits()).unwrap();
        for length in 0..bytes.len() {
            assert!(OutputCheckpoint::from_bytes(&bytes[..length], limits()).is_err());
        }
        for index in 0..bytes.len() {
            let mut corrupt = bytes.clone();
            corrupt[index] ^= 1;
            assert!(OutputCheckpoint::from_bytes(&corrupt, limits()).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(OutputCheckpoint::from_bytes(&trailing, limits()).is_err());
        assert!(
            OutputCheckpoint::from_bytes(
                &bytes,
                OutputCheckpointLimits {
                    sample_rate: 48000,
                    ..limits()
                }
            )
            .is_err()
        );
        assert!(
            OutputCheckpoint::from_bytes(
                &bytes,
                OutputCheckpointLimits {
                    max_bytes: bytes.len() - 1,
                    ..limits()
                }
            )
            .is_err()
        );
        let (foreign, _foreign_rx) = self::connection(5);
        let mut foreign_fence = foreign.sender().freeze_queue().unwrap();
        assert!(
            OutputCheckpoint::capture(&connection, &mut foreign_fence, &pump, limits()).is_err()
        );
        let mut prepared = before.prepare(limits()).unwrap();
        assert!(prepared.publish(&foreign, &mut frozen, &mut pump).is_err());
        assert_eq!(
            OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
                .unwrap()
                .unwrap(),
            before
        );
        let tight = OutputCheckpointLimits {
            max_pump_frames: 2,
            ..limits()
        };
        let candidate = OutputCheckpoint {
            pump: PcmPumpCheckpoint {
                block: vec![[0; 2]],
                cursor: 0,
            },
            ..before.clone()
        };
        let mut prepared = candidate.prepare(tight).unwrap();
        assert!(
            prepared
                .publish(&connection, &mut frozen, &mut pump)
                .is_err()
        );
        assert_eq!(
            OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
                .unwrap()
                .unwrap(),
            before
        );
        let mut good = before.prepare(limits()).unwrap();
        let old = good
            .publish(&connection, &mut frozen, &mut pump)
            .unwrap()
            .unwrap();
        let mut rollback = old.prepare(limits()).unwrap();
        rollback
            .publish(&connection, &mut frozen, &mut pump)
            .unwrap()
            .unwrap();
        assert_eq!(
            OutputCheckpoint::capture(&connection, &mut frozen, &pump, limits())
                .unwrap()
                .unwrap(),
            before
        );
    }
}
