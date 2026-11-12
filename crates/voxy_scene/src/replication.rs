//! Deterministic snapshot ring buffer, server reconciliation rollback, and object-level deltas
//! for authoritative multiplayer and lockstep replay (`docs/architecture.md` §6.3).
//!
//! A delta carries whole changed objects plus removed IDs. Its versioned byte
//! representation can be passed to a transport without coupling it to scene loading.

use crate::{ComponentRegistry, DocumentError, LoadedScene, ObjectId, SceneDocument, SceneObject};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, VecDeque};
use std::io::{self, Read, Write};
use voxy_core::crc32c;

/// Canonical tick-stamped scene snapshot with a deterministic `CRC-32C` state digest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickSnapshot {
    pub tick: u64,
    pub state_crc32c: u32,
    pub document: SceneDocument,
}

impl TickSnapshot {
    const WIRE_HEADER: &'static [u8] = b"VXSS\x01";

    /// Captures a canonical tick snapshot and computes its deterministic `CRC-32C` digest.
    /// # Errors
    /// Propagates [`DocumentError`] if `document` fails canonical JSON serialization.
    pub fn from_document(tick: u64, mut document: SceneDocument) -> Result<Self, DocumentError> {
        document.objects.sort_by(|a, b| a.id.cmp(&b.id));
        let canonical = document.to_json()?;
        let state_crc32c = crc32c(canonical.as_bytes());
        Ok(Self {
            tick,
            state_crc32c,
            document,
        })
    }

    /// Writes a versioned full snapshot over a length-prefixed stream.
    ///
    /// # Errors
    /// Rejects invalid snapshots, oversized payloads and I/O failures.
    pub fn write_frame(
        &self,
        writer: &mut impl Write,
        max_bytes: usize,
    ) -> Result<(), SceneDeltaStreamError> {
        self.validate_wire()?;
        let mut bytes = Self::WIRE_HEADER.to_vec();
        serde_json::to_writer(&mut bytes, self).map_err(DocumentError::from)?;
        if bytes.len() > max_bytes {
            return Err(SceneDeltaStreamError::Length);
        }
        write_payload(writer, &bytes)
    }

    /// Reads a complete full snapshot and verifies its canonical state digest.
    ///
    /// # Errors
    /// Rejects I/O failures, oversized or malformed messages and invalid state.
    pub fn read_frame(
        reader: &mut impl Read,
        max_bytes: usize,
    ) -> Result<Self, SceneDeltaStreamError> {
        let bytes = read_payload(reader, max_bytes, Self::WIRE_HEADER.len() + 1)?;
        Ok(Self::decode_payload(&bytes)?)
    }

    fn decode_payload(bytes: &[u8]) -> Result<Self, DocumentError> {
        let payload = bytes
            .strip_prefix(Self::WIRE_HEADER)
            .ok_or_else(|| DocumentError::Invalid("unknown snapshot wire version".into()))?;
        let snapshot: Self = serde_json::from_slice(payload).map_err(DocumentError::from)?;
        snapshot.validate_wire()?;
        Ok(snapshot)
    }

    fn validate_wire(&self) -> Result<(), DocumentError> {
        if self.document.version != 1
            || self
                .document
                .objects
                .iter()
                .any(|object| object.id.0.is_empty())
            || self
                .document
                .objects
                .windows(2)
                .any(|pair| pair[0].id >= pair[1].id)
        {
            return Err(DocumentError::Invalid(
                "invalid canonical scene snapshot".into(),
            ));
        }
        if crc32c(self.document.to_json()?.as_bytes()) != self.state_crc32c {
            return Err(DocumentError::Invalid("scene snapshot CRC mismatch".into()));
        }
        Ok(())
    }
}

/// Compact object-level delta between a baseline `SceneDocument` and a target `SceneDocument`
/// for bandwidth-bounded network replication.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneSnapshotDelta {
    pub base_tick: u64,
    pub target_tick: u64,
    pub target_crc32c: u32,
    pub upserted: Vec<SceneObject>,
    pub removed: Vec<ObjectId>,
}

/// Failure while transferring a framed scene update over a byte stream.
#[derive(Debug, thiserror::Error)]
pub enum SceneDeltaStreamError {
    #[error("scene update stream I/O: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("invalid scene update frame length")]
    Length,
}

fn write_payload(writer: &mut impl Write, bytes: &[u8]) -> Result<(), SceneDeltaStreamError> {
    let len = u32::try_from(bytes.len()).map_err(|_| SceneDeltaStreamError::Length)?;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(bytes)?;
    Ok(())
}

fn read_payload(
    reader: &mut impl Read,
    max_bytes: usize,
    min_len: usize,
) -> Result<Vec<u8>, SceneDeltaStreamError> {
    let mut prefix = [0; 4];
    reader.read_exact(&mut prefix)?;
    let len = u32::from_be_bytes(prefix) as usize;
    if len < min_len || len > max_bytes {
        return Err(SceneDeltaStreamError::Length);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|_| SceneDeltaStreamError::Length)?;
    bytes.resize(len, 0);
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

impl SceneSnapshotDelta {
    const WIRE_HEADER: &'static [u8] = b"VXSD\x01";

    /// Encodes a versioned, uncompressed delta.
    ///
    /// # Errors
    /// Rejects invalid deltas, serialization failures and messages larger than
    /// `max_bytes`, including the header.
    pub fn encode(&self, max_bytes: usize) -> Result<Vec<u8>, DocumentError> {
        self.validate_wire()?;
        let mut bytes = Self::WIRE_HEADER.to_vec();
        serde_json::to_writer(&mut bytes, self)?;
        if bytes.len() > max_bytes {
            return Err(DocumentError::Invalid(
                "scene delta exceeds byte budget".into(),
            ));
        }
        Ok(bytes)
    }

    /// Decodes a complete delta message within the caller's byte budget.
    ///
    /// # Errors
    /// Rejects oversized messages, unknown versions, malformed payloads and
    /// invalid tick or object changes. Apply the delta to verify its state digest.
    pub fn decode(bytes: &[u8], max_bytes: usize) -> Result<Self, DocumentError> {
        if bytes.len() > max_bytes {
            return Err(DocumentError::Invalid(
                "scene delta exceeds byte budget".into(),
            ));
        }
        let payload = bytes
            .strip_prefix(Self::WIRE_HEADER)
            .ok_or_else(|| DocumentError::Invalid("unknown scene delta wire version".into()))?;
        let delta: Self = serde_json::from_slice(payload)?;
        delta.validate_wire()?;
        Ok(delta)
    }

    /// Writes a big-endian u32 payload length followed by the encoded delta.
    ///
    /// # Errors
    /// Rejects invalid deltas, payloads larger than `max_bytes` and I/O failures.
    /// An I/O failure can leave a partial frame in the stream.
    pub fn write_frame(
        &self,
        writer: &mut impl Write,
        max_bytes: usize,
    ) -> Result<(), SceneDeltaStreamError> {
        let bytes = self.encode(max_bytes)?;
        write_payload(writer, &bytes)
    }

    /// Reads one delta from a stream, bounding payload size before allocation.
    ///
    /// # Errors
    /// Rejects invalid lengths, invalid deltas and I/O failures. An invalid
    /// length or partial read requires the caller to discard the stream.
    pub fn read_frame(
        reader: &mut impl Read,
        max_bytes: usize,
    ) -> Result<Self, SceneDeltaStreamError> {
        let bytes = read_payload(reader, max_bytes, Self::WIRE_HEADER.len() + 1)?;
        Ok(Self::decode(&bytes, max_bytes)?)
    }

    fn validate_wire(&self) -> Result<(), DocumentError> {
        if self.target_tick <= self.base_tick {
            return Err(DocumentError::Invalid(
                "scene delta ticks must increase".into(),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        if self
            .upserted
            .iter()
            .map(|object| &object.id)
            .chain(&self.removed)
            .any(|id| !seen.insert(id))
        {
            return Err(DocumentError::Invalid(
                "scene delta repeats an object ID".into(),
            ));
        }
        for object in &self.upserted {
            object.transform().matrix()?;
        }
        Ok(())
    }

    /// Computes the minimal object-level delta from `base` to `target`.
    #[must_use]
    pub fn diff(base: &TickSnapshot, target: &TickSnapshot) -> Self {
        let base_map: BTreeMap<&ObjectId, &SceneObject> =
            base.document.objects.iter().map(|o| (&o.id, o)).collect();
        let target_map: BTreeMap<&ObjectId, &SceneObject> =
            target.document.objects.iter().map(|o| (&o.id, o)).collect();

        let mut upserted = Vec::new();
        for (id, obj) in &target_map {
            if base_map.get(id) != Some(obj) {
                upserted.push((*obj).clone());
            }
        }
        let mut removed = Vec::new();
        for id in base_map.keys() {
            if !target_map.contains_key(id) {
                removed.push((*id).clone());
            }
        }

        Self {
            base_tick: base.tick,
            target_tick: target.tick,
            target_crc32c: target.state_crc32c,
            upserted,
            removed,
        }
    }

    /// Applies this delta to `base` and verifies the resulting `CRC-32C` state digest.
    /// # Errors
    /// Rejects base tick mismatches, invalid transforms, or post-apply checksum divergence.
    pub fn apply(&self, base: &TickSnapshot) -> Result<TickSnapshot, DocumentError> {
        if base.tick != self.base_tick {
            return Err(DocumentError::Invalid(format!(
                "delta base tick mismatch: expected {}, got {}",
                self.base_tick, base.tick
            )));
        }
        let mut objects: BTreeMap<ObjectId, SceneObject> = base
            .document
            .objects
            .iter()
            .cloned()
            .map(|o| (o.id.clone(), o))
            .collect();
        for id in &self.removed {
            objects.remove(id);
        }
        for obj in &self.upserted {
            objects.insert(obj.id.clone(), obj.clone());
        }
        let doc = SceneDocument {
            version: base.document.version,
            objects: objects.into_values().collect(),
        };
        let reconstructed = TickSnapshot::from_document(self.target_tick, doc)?;
        if reconstructed.state_crc32c != self.target_crc32c {
            return Err(DocumentError::Invalid(format!(
                "delta state CRC mismatch at tick {}: expected {:#010x}, got {:#010x}",
                self.target_tick, self.target_crc32c, reconstructed.state_crc32c
            )));
        }
        Ok(reconstructed)
    }
}

/// Bounded ring buffer of historical [`TickSnapshot`]s for client-side prediction rollback
/// and server state reconciliation.
#[derive(Debug)]
pub struct SnapshotRollbackBuffer {
    max_history: usize,
    history: VecDeque<TickSnapshot>,
}

/// Update selected from authoritative history for a client's confirmed baseline.
#[derive(Debug)]
pub enum SnapshotUpdate<'a> {
    /// The client already has the latest authoritative state.
    UpToDate,
    /// The client can reconstruct the latest state from a retained baseline.
    Delta(SceneSnapshotDelta),
    /// The baseline is unknown or divergent, so the client needs a full state.
    Full(Cow<'a, TickSnapshot>),
}

/// Authoritative scene update and the input acknowledgement captured with it.
#[derive(Debug)]
pub struct PredictionUpdate<'a> {
    pub applied_sequence: u64,
    pub applied_tick: u64,
    pub state_tick: u64,
    pub update: SnapshotUpdate<'a>,
}

impl PredictionUpdate<'_> {
    pub const HEADER: [u8; 5] = *b"VXSP\x02";

    /// Requests an authoritative update with its input acknowledgement.
    ///
    /// # Errors
    /// Propagates stream write failures.
    pub fn write_request(baseline: SnapshotBaseline, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(&Self::HEADER)?;
        baseline.write_request(writer)
    }

    /// Writes the acknowledgement and associated scene update.
    ///
    /// # Errors
    /// Rejects inconsistent ticks, invalid or oversized updates and I/O failures.
    pub fn write_frame(
        &self,
        writer: &mut impl Write,
        max_bytes: usize,
    ) -> Result<(), SceneDeltaStreamError> {
        self.validate_ticks()?;
        writer.write_all(&Self::HEADER)?;
        writer.write_all(&self.applied_sequence.to_be_bytes())?;
        writer.write_all(&self.applied_tick.to_be_bytes())?;
        writer.write_all(&self.state_tick.to_be_bytes())?;
        self.update.write_frame(writer, max_bytes)
    }

    fn validate_ticks(&self) -> Result<(), DocumentError> {
        let payload_tick = match &self.update {
            SnapshotUpdate::Full(snapshot) => snapshot.tick,
            SnapshotUpdate::Delta(delta) => delta.target_tick,
            SnapshotUpdate::UpToDate => self.state_tick,
        };
        if payload_tick != self.state_tick
            || self.applied_tick > self.state_tick
            || (self.applied_sequence == 0) != (self.applied_tick == 0)
        {
            return Err(DocumentError::Invalid(
                "inconsistent prediction update ticks".into(),
            ));
        }
        Ok(())
    }
}

impl PredictionUpdate<'static> {
    /// Reads and validates a prediction reply.
    ///
    /// # Errors
    /// Rejects malformed replies, inconsistent ticks and I/O failures.
    pub fn read_frame(
        reader: &mut impl Read,
        max_bytes: usize,
    ) -> Result<Self, SceneDeltaStreamError> {
        let mut header = [0; 5];
        reader.read_exact(&mut header)?;
        if header != Self::HEADER {
            return Err(
                DocumentError::Invalid("unknown prediction update wire version".into()).into(),
            );
        }
        let mut read_tick = || {
            let mut bytes = [0; 8];
            reader
                .read_exact(&mut bytes)
                .map(|()| u64::from_be_bytes(bytes))
        };
        let reply = Self {
            applied_sequence: read_tick()?,
            applied_tick: read_tick()?,
            state_tick: read_tick()?,
            update: SnapshotUpdate::read_frame(reader, max_bytes)?,
        };
        reply.validate_ticks()?;
        Ok(reply)
    }
}

/// Client-side owner of the last confirmed authoritative snapshot.
#[derive(Debug)]
pub struct PredictionClient {
    history: SnapshotRollbackBuffer,
    confirmed: Option<(u64, u32)>,
    applied: (u64, u64),
}

impl Default for PredictionClient {
    fn default() -> Self {
        Self {
            history: SnapshotRollbackBuffer::new(1),
            confirmed: None,
            applied: (0, 0),
        }
    }
}

impl PredictionClient {
    /// Last applied input sequence and its authoritative tick.
    #[must_use]
    pub const fn applied_input(&self) -> (u64, u64) {
        self.applied
    }

    /// Rebuilds a predicted scene from the confirmed snapshot and replays
    /// pending commands through the game's deterministic scene adapter.
    ///
    /// # Errors
    /// Rejects missing baselines, unordered or missing commands, replay budgets
    /// and scene or adapter failures. The confirmed snapshot remains unchanged.
    pub fn replay_pending<T>(
        &self,
        registry: &ComponentRegistry,
        capacity: usize,
        commands: &[(u64, T)],
        max_commands: usize,
        mut replay: impl FnMut(&mut LoadedScene, &T) -> Result<(), DocumentError>,
    ) -> Result<LoadedScene, DocumentError> {
        let snapshot = self
            .confirmed
            .and_then(|(tick, _)| self.history.get(tick))
            .ok_or_else(|| DocumentError::Invalid("no confirmed snapshot to replay".into()))?;
        if commands.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
            return Err(DocumentError::Invalid(
                "prediction commands are not ordered".into(),
            ));
        }
        let pending =
            &commands[commands.partition_point(|(sequence, _)| *sequence <= self.applied.0)..];
        if pending.len() > max_commands {
            return Err(DocumentError::Invalid(
                "prediction replay exceeds command budget".into(),
            ));
        }
        let mut previous = self.applied.0;
        for (sequence, _) in pending {
            if previous.checked_add(1) != Some(*sequence) {
                return Err(DocumentError::Invalid(
                    "prediction replay has a command gap".into(),
                ));
            }
            previous = *sequence;
        }
        let mut scene = snapshot.document.load(registry, capacity)?;
        for (_, command) in pending {
            replay(&mut scene, command)?;
        }
        Ok(scene)
    }

    /// Requests an update and stages the received scene. Returns `None` when
    /// the server confirms the existing scene.
    ///
    /// # Errors
    /// On stream, validation or scene load errors, confirmed state is preserved.
    pub fn synchronize(
        &mut self,
        stream: &mut (impl Read + Write),
        registry: &ComponentRegistry,
        capacity: usize,
        max_bytes: usize,
    ) -> Result<Option<LoadedScene>, SceneDeltaStreamError> {
        let baseline = self
            .confirmed
            .map_or(SnapshotBaseline::Full, |(tick, state_crc32c)| {
                SnapshotBaseline::Confirmed { tick, state_crc32c }
            });
        PredictionUpdate::write_request(baseline, stream)?;
        stream.flush()?;
        let reply = PredictionUpdate::read_frame(stream, max_bytes)?;
        Ok(self.apply_reply(registry, capacity, reply)?)
    }

    fn apply_reply(
        &mut self,
        registry: &ComponentRegistry,
        capacity: usize,
        reply: PredictionUpdate<'_>,
    ) -> Result<Option<LoadedScene>, DocumentError> {
        let applied = (reply.applied_sequence, reply.applied_tick);
        if applied.0 < self.applied.0
            || (applied.0 == self.applied.0 && applied.1 != self.applied.1)
            || (applied.0 > self.applied.0 && applied.1 <= self.applied.1)
            || self
                .confirmed
                .is_some_and(|(tick, _)| reply.state_tick < tick)
        {
            return Err(DocumentError::Invalid("stale prediction reply".into()));
        }
        let confirmed = match &reply.update {
            SnapshotUpdate::UpToDate => {
                self.confirmed.filter(|(tick, _)| *tick == reply.state_tick)
            }
            SnapshotUpdate::Full(snapshot) => Some((snapshot.tick, snapshot.state_crc32c)),
            SnapshotUpdate::Delta(delta)
                if self
                    .confirmed
                    .is_some_and(|(tick, _)| tick == delta.base_tick) =>
            {
                Some((delta.target_tick, delta.target_crc32c))
            }
            SnapshotUpdate::Delta(_) => None,
        };
        let Some(confirmed) = confirmed else {
            return Err(DocumentError::Invalid(
                "prediction reply does not match the confirmed baseline".into(),
            ));
        };
        if self
            .confirmed
            .is_some_and(|(tick, crc)| tick == confirmed.0 && crc != confirmed.1)
        {
            return Err(DocumentError::Invalid(
                "authoritative state changed within a confirmed tick".into(),
            ));
        }
        let scene = self
            .history
            .apply_authoritative_update(registry, capacity, reply.update)?;
        self.confirmed = Some(confirmed);
        self.applied = applied;
        Ok(scene)
    }
}

/// Client request for an update from authoritative snapshot history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotBaseline {
    Full,
    Confirmed { tick: u64, state_crc32c: u32 },
}

impl SnapshotBaseline {
    const WIRE_HEADER: [u8; 5] = *b"VXSB\x01";

    /// Writes a fixed-size baseline request.
    ///
    /// # Errors
    /// An I/O failure may leave a partial request in the stream.
    pub fn write_request(self, writer: &mut impl Write) -> io::Result<()> {
        let mut bytes = [0; 18];
        bytes[..5].copy_from_slice(&Self::WIRE_HEADER);
        if let Self::Confirmed { tick, state_crc32c } = self {
            bytes[5] = 1;
            bytes[6..14].copy_from_slice(&tick.to_be_bytes());
            bytes[14..].copy_from_slice(&state_crc32c.to_be_bytes());
        }
        writer.write_all(&bytes)
    }

    /// Reads a complete baseline request and validates its version and fields.
    ///
    /// # Errors
    /// Rejects invalid or incomplete requests and propagates I/O failures.
    pub fn read_request(reader: &mut impl Read) -> io::Result<Self> {
        let mut bytes = [0; 18];
        reader.read_exact(&mut bytes)?;
        match bytes {
            [header @ .., kind, a, b, c, d, e, f, g, h, i, j, k, l]
                if header == Self::WIRE_HEADER =>
            {
                let tick = u64::from_be_bytes([a, b, c, d, e, f, g, h]);
                let state_crc32c = u32::from_be_bytes([i, j, k, l]);
                match kind {
                    0 if tick == 0 && state_crc32c == 0 => return Ok(Self::Full),
                    1 => return Ok(Self::Confirmed { tick, state_crc32c }),
                    _ => {}
                }
            }
            _ => {}
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid scene baseline request",
        ))
    }
}

impl SnapshotUpdate<'_> {
    const UP_TO_DATE: &'static [u8] = b"VXSA\x01";

    /// Sends a self-describing reply to a client's baseline confirmation.
    ///
    /// # Errors
    /// Rejects invalid state, oversized payloads and I/O failures.
    pub fn write_frame(
        &self,
        writer: &mut impl Write,
        max_bytes: usize,
    ) -> Result<(), SceneDeltaStreamError> {
        match self {
            Self::Full(snapshot) => snapshot.write_frame(writer, max_bytes),
            Self::Delta(delta) => delta.write_frame(writer, max_bytes),
            Self::UpToDate if Self::UP_TO_DATE.len() <= max_bytes => {
                write_payload(writer, Self::UP_TO_DATE)
            }
            Self::UpToDate => Err(SceneDeltaStreamError::Length),
        }
    }
}

impl SnapshotUpdate<'static> {
    /// Reads a reply, dispatching by the versioned payload header.
    ///
    /// # Errors
    /// Rejects invalid lengths, unknown message types, invalid payloads and I/O failures.
    pub fn read_frame(
        reader: &mut impl Read,
        max_bytes: usize,
    ) -> Result<Self, SceneDeltaStreamError> {
        let bytes = read_payload(reader, max_bytes, Self::UP_TO_DATE.len())?;
        if bytes == Self::UP_TO_DATE {
            return Ok(Self::UpToDate);
        }
        if bytes.starts_with(TickSnapshot::WIRE_HEADER) {
            return Ok(Self::Full(Cow::Owned(TickSnapshot::decode_payload(
                &bytes,
            )?)));
        }
        if bytes.starts_with(SceneSnapshotDelta::WIRE_HEADER) {
            return Ok(Self::Delta(SceneSnapshotDelta::decode(&bytes, max_bytes)?));
        }
        Err(DocumentError::Invalid("unknown scene update wire type or version".into()).into())
    }
}

impl SnapshotRollbackBuffer {
    #[must_use]
    pub fn new(max_history: usize) -> Self {
        let cap = max_history.max(1);
        Self {
            max_history: cap,
            history: VecDeque::with_capacity(cap),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Records a new snapshot in strictly increasing tick order, evicting the oldest entry if full.
    /// # Errors
    /// Rejects non-monotonic ticks.
    pub fn push(&mut self, snapshot: TickSnapshot) -> Result<(), DocumentError> {
        if let Some(last) = self.history.back()
            && snapshot.tick <= last.tick
        {
            return Err(DocumentError::Invalid(
                "snapshot ticks must increase strictly".into(),
            ));
        }
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(snapshot);
        Ok(())
    }

    /// Looks up the snapshot recorded at `tick`.
    #[must_use]
    pub fn get(&self, tick: u64) -> Option<&TickSnapshot> {
        self.history.iter().find(|s| s.tick == tick)
    }

    /// Selects an update from the latest authoritative state.
    ///
    /// Returns `None` if no authoritative snapshot has been recorded. Clients
    /// without a baseline can pass `None` to request a full snapshot.
    #[must_use]
    pub fn update_for_confirmed(
        &self,
        confirmed: Option<(u64, u32)>,
    ) -> Option<SnapshotUpdate<'_>> {
        let latest = self.history.back()?;
        let base = confirmed
            .and_then(|(tick, crc)| self.get(tick).filter(|base| base.state_crc32c == crc));
        Some(match base {
            Some(base) if base.tick == latest.tick => SnapshotUpdate::UpToDate,
            Some(base) if base.document.version == latest.document.version => {
                SnapshotUpdate::Delta(SceneSnapshotDelta::diff(base, latest))
            }
            _ => SnapshotUpdate::Full(Cow::Borrowed(latest)),
        })
    }

    /// Reads one request and writes the corresponding authoritative update.
    ///
    /// # Errors
    /// Rejects invalid requests, empty history, oversized replies and stream
    /// failures. The caller owns connection timeouts and recovery.
    pub fn serve_update(
        &self,
        stream: &mut (impl Read + Write),
        max_bytes: usize,
    ) -> Result<(), SceneDeltaStreamError> {
        let confirmed = match SnapshotBaseline::read_request(stream)? {
            SnapshotBaseline::Full => None,
            SnapshotBaseline::Confirmed { tick, state_crc32c } => Some((tick, state_crc32c)),
        };
        self.update_for_confirmed(confirmed)
            .ok_or_else(|| DocumentError::Invalid("authoritative history is empty".into()))?
            .write_frame(stream, max_bytes)
    }

    /// Verifies whether the locally predicted state at `server_tick` matches `authoritative_crc32c`.
    /// If it diverges, truncates all speculative history after `server_tick` and returns `true`
    /// (indicating that the caller must rollback and replay from the authoritative snapshot).
    #[must_use]
    pub fn reconcile_or_truncate(&mut self, server_tick: u64, authoritative_crc32c: u32) -> bool {
        let Some(pos) = self.history.iter().position(|s| s.tick == server_tick) else {
            self.history.clear();
            return true;
        };
        if self.history[pos].state_crc32c == authoritative_crc32c {
            // Prune confirmed older history prior to `pos` to free memory:
            drop(self.history.drain(..pos));
            false
        } else {
            self.history.clear();
            true
        }
    }

    /// Rolls back to the snapshot recorded at `delta.base_tick`, applies the authoritative
    /// `delta`, and loads the result into a freshly staged scene.
    ///
    /// On success the history keeps only the authoritative snapshot (everything older is
    /// confirmed, everything newer was speculative), and the caller replaces its world with the
    /// returned [`LoadedScene`] before replaying local input from `delta.target_tick`.
    /// On failure neither the history nor any existing world changes.
    /// # Errors
    /// Rejects a base tick that is no longer (or not yet) in the history, a delta whose result
    /// does not match its `CRC-32C`, and a resulting document that fails [`SceneDocument::load`].
    pub fn rollback_and_apply_authoritative(
        &mut self,
        registry: &ComponentRegistry,
        capacity: usize,
        delta: &SceneSnapshotDelta,
    ) -> Result<LoadedScene, DocumentError> {
        let base = self.get(delta.base_tick).ok_or_else(|| {
            DocumentError::Invalid(format!(
                "rollback base tick {} is not in the snapshot history",
                delta.base_tick
            ))
        })?;
        let authoritative = delta.apply(base)?;
        self.install_authoritative(registry, capacity, authoritative)
    }

    /// Stages the scene described by an authoritative reply to the current request.
    ///
    /// On success the caller can replace its world with the returned scene
    /// before replaying local input. An up-to-date reply returns `None` and
    /// leaves the scene and history intact.
    ///
    /// # Errors
    /// Rejects invalid state and failed scene loads without changing history.
    pub fn apply_authoritative_update(
        &mut self,
        registry: &ComponentRegistry,
        capacity: usize,
        update: SnapshotUpdate<'_>,
    ) -> Result<Option<LoadedScene>, DocumentError> {
        match update {
            SnapshotUpdate::UpToDate => Ok(None),
            SnapshotUpdate::Delta(delta) => self
                .rollback_and_apply_authoritative(registry, capacity, &delta)
                .map(Some),
            SnapshotUpdate::Full(snapshot) => {
                snapshot.validate_wire()?;
                self.install_authoritative(registry, capacity, snapshot.into_owned())
                    .map(Some)
            }
        }
    }

    fn install_authoritative(
        &mut self,
        registry: &ComponentRegistry,
        capacity: usize,
        authoritative: TickSnapshot,
    ) -> Result<LoadedScene, DocumentError> {
        let loaded = authoritative.document.load(registry, capacity)?;
        self.history.clear();
        self.history.push_back(authoritative);
        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc(x: f32) -> SceneDocument {
        SceneDocument {
            version: 1,
            objects: vec![
                SceneObject {
                    id: ObjectId("static_floor".into()),
                    parent: None,
                    name: "Floor".into(),
                    active: true,
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                    components: BTreeMap::new(),
                },
                SceneObject {
                    id: ObjectId("player".into()),
                    parent: None,
                    name: "Player".into(),
                    active: true,
                    translation: [x, 1.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                    components: BTreeMap::new(),
                },
            ],
        }
    }

    #[test]
    fn prediction_client_installs_tcp_updates_and_rejects_a_stale_reply() {
        use std::net::{TcpListener, TcpStream};
        use std::time::Duration;

        let first = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let next = TickSnapshot::from_document(12, sample_doc(4.0)).unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut connection = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let timeout = Some(Duration::from_secs(2));
        connection.set_read_timeout(timeout).unwrap();
        server.set_read_timeout(timeout).unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let exchanges = [
                    (
                        SnapshotBaseline::Full,
                        (0, 0, 10),
                        SnapshotUpdate::Full(Cow::Borrowed(&first)),
                    ),
                    (
                        SnapshotBaseline::Confirmed {
                            tick: 10,
                            state_crc32c: first.state_crc32c,
                        },
                        (2, 11, 12),
                        SnapshotUpdate::Delta(SceneSnapshotDelta::diff(&first, &next)),
                    ),
                    (
                        SnapshotBaseline::Confirmed {
                            tick: 12,
                            state_crc32c: next.state_crc32c,
                        },
                        (0, 0, 10),
                        SnapshotUpdate::Full(Cow::Borrowed(&first)),
                    ),
                ];
                for (baseline, (applied_sequence, applied_tick, state_tick), update) in exchanges {
                    let mut header = [0; 5];
                    server.read_exact(&mut header).unwrap();
                    assert_eq!(header, PredictionUpdate::HEADER);
                    assert_eq!(
                        SnapshotBaseline::read_request(&mut server).unwrap(),
                        baseline
                    );
                    PredictionUpdate {
                        applied_sequence,
                        applied_tick,
                        state_tick,
                        update,
                    }
                    .write_frame(&mut server, 4096)
                    .unwrap();
                }
            });
            let mut client = PredictionClient::default();
            let registry = ComponentRegistry::default();
            assert!(
                client
                    .synchronize(&mut connection, &registry, 8, 4096)
                    .unwrap()
                    .is_some()
            );
            let scene = client
                .synchronize(&mut connection, &registry, 8, 4096)
                .unwrap()
                .unwrap();
            let player = scene.resolve(&ObjectId("player".into())).unwrap();
            assert_eq!(scene.graph.local(player).unwrap().translation.x, 4.0);
            assert!(
                client
                    .synchronize(&mut connection, &registry, 8, 4096)
                    .is_err()
            );
            assert_eq!(client.applied_input(), (2, 11));
            assert_eq!(client.history.get(12), Some(&next));
            let predicted = client
                .replay_pending(
                    &registry,
                    8,
                    &[(1, 100.), (2, 200.), (3, 1.), (4, 2.)],
                    2,
                    |scene, movement| {
                        let player = scene.resolve(&ObjectId("player".into())).unwrap();
                        let mut transform = scene.graph.local(player)?;
                        transform.translation.x += movement;
                        scene.graph.set_local(player, transform)?;
                        Ok(())
                    },
                )
                .unwrap();
            let player = predicted.resolve(&ObjectId("player".into())).unwrap();
            assert_eq!(predicted.graph.local(player).unwrap().translation.x, 7.);
            assert_eq!(client.history.get(12), Some(&next));
            assert!(
                client
                    .replay_pending(&registry, 8, &[(4, ())], 1, |_, ()| Ok(()))
                    .is_err()
            );
            assert!(
                client
                    .replay_pending(&registry, 8, &[(3, ())], 0, |_, ()| Ok(()))
                    .is_err()
            );
        });
    }

    #[test]
    fn prediction_reply_rejects_acknowledgements_outside_the_snapshot_tick() {
        let snapshot = TickSnapshot::from_document(12, sample_doc(4.0)).unwrap();
        let mut reply = PredictionUpdate {
            applied_sequence: 3,
            applied_tick: 11,
            state_tick: 12,
            update: SnapshotUpdate::Full(Cow::Borrowed(&snapshot)),
        };
        let mut bytes = Vec::new();
        reply.write_frame(&mut bytes, 4096).unwrap();
        let decoded = PredictionUpdate::read_frame(&mut bytes.as_slice(), 4096).unwrap();
        assert_eq!(
            (
                decoded.applied_sequence,
                decoded.applied_tick,
                decoded.state_tick
            ),
            (3, 11, 12)
        );
        assert!(
            matches!(decoded.update, SnapshotUpdate::Full(value) if value.as_ref() == &snapshot)
        );
        bytes[13..21].copy_from_slice(&13_u64.to_be_bytes());
        assert!(PredictionUpdate::read_frame(&mut bytes.as_slice(), 4096).is_err());
        reply.state_tick += 1;
        bytes.clear();
        assert!(reply.write_frame(&mut bytes, 4096).is_err());
        assert!(bytes.is_empty());
    }

    #[test]
    fn update_selection_requires_a_retained_matching_baseline() {
        let first = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let second = TickSnapshot::from_document(11, sample_doc(2.5)).unwrap();
        let latest = TickSnapshot::from_document(12, sample_doc(3.0)).unwrap();
        let mut history = SnapshotRollbackBuffer::new(2);
        assert!(history.update_for_confirmed(None).is_none());
        for snapshot in [&first, &second, &latest] {
            history.push(snapshot.clone()).unwrap();
        }
        assert!(matches!(
            history.update_for_confirmed(Some((latest.tick, latest.state_crc32c))),
            Some(SnapshotUpdate::UpToDate)
        ));
        let Some(SnapshotUpdate::Delta(delta)) =
            history.update_for_confirmed(Some((second.tick, second.state_crc32c)))
        else {
            panic!("matching retained baseline must use a delta");
        };
        assert_eq!(delta.apply(&second).unwrap(), latest);
        for baseline in [
            None,
            Some((first.tick, first.state_crc32c)),
            Some((second.tick, second.state_crc32c ^ 1)),
            Some((13, latest.state_crc32c)),
        ] {
            assert!(matches!(
                history.update_for_confirmed(baseline),
                Some(SnapshotUpdate::Full(snapshot)) if snapshot.as_ref() == &latest
            ));
        }
    }

    #[test]
    fn delta_stream_transfers_two_authoritative_ticks_over_tcp() {
        use std::net::{TcpListener, TcpStream};
        use std::time::Duration;

        let base = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let next = TickSnapshot::from_document(11, sample_doc(2.5)).unwrap();
        let last = TickSnapshot::from_document(12, sample_doc(3.0)).unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        server
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut history = SnapshotRollbackBuffer::new(2);
        history.push(base.clone()).unwrap();
        SnapshotBaseline::Full.write_request(&mut client).unwrap();
        history.serve_update(&mut server, 4096).unwrap();
        for (previous, target) in [(&base, &next), (&next, &last)] {
            history.push(target.clone()).unwrap();
            SnapshotBaseline::Confirmed {
                tick: previous.tick,
                state_crc32c: previous.state_crc32c,
            }
            .write_request(&mut client)
            .unwrap();
            history.serve_update(&mut server, 4096).unwrap();
        }
        SnapshotBaseline::Confirmed {
            tick: last.tick,
            state_crc32c: last.state_crc32c,
        }
        .write_request(&mut client)
        .unwrap();
        history.serve_update(&mut server, 4096).unwrap();
        let SnapshotUpdate::Full(received) = SnapshotUpdate::read_frame(&mut client, 4096).unwrap()
        else {
            panic!("an unavailable baseline must use a full snapshot");
        };
        assert_eq!(received.as_ref(), &base);
        let mut applied = received.into_owned();
        for target in [next, last.clone()] {
            let SnapshotUpdate::Delta(delta) =
                SnapshotUpdate::read_frame(&mut client, 4096).unwrap()
            else {
                panic!("a retained baseline must use a delta");
            };
            applied = delta.apply(&applied).unwrap();
            assert_eq!(applied, target);
        }
        assert!(matches!(
            SnapshotUpdate::read_frame(&mut client, 4096).unwrap(),
            SnapshotUpdate::UpToDate
        ));
        drop((client, server));
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        SnapshotBaseline::Confirmed {
            tick: base.tick,
            state_crc32c: base.state_crc32c,
        }
        .write_request(&mut client)
        .unwrap();
        history.serve_update(&mut server, 4096).unwrap();
        assert!(matches!(
            SnapshotUpdate::read_frame(&mut client, 4096).unwrap(),
            SnapshotUpdate::Full(snapshot) if snapshot.as_ref() == &last
        ));

        let mut invalid_snapshot = last.clone();
        invalid_snapshot.state_crc32c ^= 1;
        let mut invalid_payload = TickSnapshot::WIRE_HEADER.to_vec();
        serde_json::to_writer(&mut invalid_payload, &invalid_snapshot).unwrap();
        let mut invalid_frame = Vec::new();
        write_payload(&mut invalid_frame, &invalid_payload).unwrap();
        assert!(matches!(
            TickSnapshot::read_frame(&mut invalid_frame.as_slice(), 4096),
            Err(SceneDeltaStreamError::Document(_))
        ));

        let mut oversized = u32::MAX.to_be_bytes().as_slice().to_vec();
        oversized.push(0);
        let mut stream = oversized.as_slice();
        assert!(matches!(
            SceneSnapshotDelta::read_frame(&mut stream, 4096),
            Err(SceneDeltaStreamError::Length)
        ));
        assert_eq!(stream, &[0]);
        assert!(
            matches!(SceneSnapshotDelta::read_frame(&mut &oversized[..1], 4096), Err(SceneDeltaStreamError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof)
        );
    }

    #[test]
    fn delta_wire_roundtrip_checks_version_budget_and_object_changes() {
        let base = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let target = TickSnapshot::from_document(11, sample_doc(2.5)).unwrap();
        let mut delta = SceneSnapshotDelta::diff(&base, &target);
        let mut bytes = delta.encode(4096).unwrap();
        let limit = bytes.len();
        assert_eq!(
            SceneSnapshotDelta::decode(&bytes, limit)
                .unwrap()
                .apply(&base)
                .unwrap(),
            target
        );
        assert!(delta.encode(limit - 1).is_err());
        assert!(SceneSnapshotDelta::decode(&bytes, limit - 1).is_err());
        bytes[4] += 1;
        assert!(SceneSnapshotDelta::decode(&bytes, limit).is_err());
        delta.removed.push(delta.upserted[0].id.clone());
        let mut malformed = SceneSnapshotDelta::WIRE_HEADER.to_vec();
        serde_json::to_writer(&mut malformed, &delta).unwrap();
        assert!(SceneSnapshotDelta::decode(&malformed, 4096).is_err());
        assert!(delta.encode(4096).is_err());
        delta.removed.clear();
        delta.target_tick = delta.base_tick;
        assert!(delta.encode(4096).is_err());
        delta.target_tick += 1;
        delta.upserted[0].translation[0] = f32::NAN;
        assert!(delta.encode(4096).is_err());
    }

    #[test]
    fn snapshot_delta_and_rollback_buffer_reconcile_authoritative_ticks() {
        let snap1 = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let snap2 = TickSnapshot::from_document(11, sample_doc(2.5)).unwrap();

        let delta = SceneSnapshotDelta::diff(&snap1, &snap2);
        assert_eq!(delta.upserted.len(), 1);
        assert_eq!(delta.upserted[0].id, ObjectId("player".into()));
        assert!(delta.removed.is_empty());

        let applied = delta.apply(&snap1).unwrap();
        assert_eq!(applied, snap2);

        let mut buffer = SnapshotRollbackBuffer::new(4);
        buffer.push(snap1.clone()).unwrap();
        buffer.push(snap2.clone()).unwrap();
        assert!(!buffer.reconcile_or_truncate(10, snap1.state_crc32c));
        assert_eq!(buffer.len(), 2);

        // Divergent server checksum triggers rollback truncation:
        assert!(buffer.reconcile_or_truncate(11, snap2.state_crc32c ^ 0xFF));
        assert!(buffer.is_empty());
    }

    #[test]
    fn rollback_applies_authoritative_delta_and_drops_speculative_history() {
        let registry = ComponentRegistry::default();
        let confirmed = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let predicted = TickSnapshot::from_document(11, sample_doc(2.5)).unwrap();
        let server = TickSnapshot::from_document(11, sample_doc(4.0)).unwrap();
        let delta = SceneSnapshotDelta::diff(&confirmed, &server);

        let mut buffer = SnapshotRollbackBuffer::new(4);
        buffer.push(confirmed).unwrap();
        buffer.push(predicted).unwrap();

        let loaded = buffer
            .rollback_and_apply_authoritative(&registry, 8, &delta)
            .unwrap();
        let player = loaded.resolve(&ObjectId("player".into())).unwrap();
        assert_eq!(loaded.graph.local(player).unwrap().translation.x, 4.0);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.get(11), Some(&server));
        assert!(buffer.get(10).is_none());
    }

    #[test]
    fn full_reply_recovers_scene_after_reconnect_and_keeps_history_on_failure() {
        let registry = ComponentRegistry::default();
        let old = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let current = TickSnapshot::from_document(12, sample_doc(4.0)).unwrap();
        let mut server = SnapshotRollbackBuffer::new(1);
        server.push(current.clone()).unwrap();
        let mut bytes = Vec::new();
        server
            .update_for_confirmed(Some((old.tick, old.state_crc32c)))
            .unwrap()
            .write_frame(&mut bytes, 4096)
            .unwrap();
        let mut client = SnapshotRollbackBuffer::new(4);
        client.push(old.clone()).unwrap();
        let decode = || SnapshotUpdate::read_frame(&mut bytes.as_slice(), 4096).unwrap();
        assert!(
            client
                .apply_authoritative_update(&registry, 1, decode())
                .is_err()
        );
        assert_eq!(client.get(old.tick), Some(&old));
        let loaded = client
            .apply_authoritative_update(&registry, 8, decode())
            .unwrap()
            .unwrap();
        let player = loaded.resolve(&ObjectId("player".into())).unwrap();
        assert_eq!(loaded.graph.local(player).unwrap().translation.x, 4.0);
        assert_eq!(client.len(), 1);
        assert_eq!(client.get(current.tick), Some(&current));
        assert!(
            client
                .apply_authoritative_update(&registry, 8, SnapshotUpdate::UpToDate)
                .unwrap()
                .is_none()
        );
        assert_eq!(client.get(current.tick), Some(&current));
    }

    #[test]
    fn rollback_rejects_unknown_base_and_corrupt_delta_without_touching_history() {
        let registry = ComponentRegistry::default();
        let confirmed = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let server = TickSnapshot::from_document(11, sample_doc(4.0)).unwrap();
        let mut buffer = SnapshotRollbackBuffer::new(4);
        buffer.push(confirmed.clone()).unwrap();

        let mut stale = SceneSnapshotDelta::diff(&confirmed, &server);
        stale.base_tick = 7;
        assert!(
            buffer
                .rollback_and_apply_authoritative(&registry, 8, &stale)
                .is_err()
        );

        let mut corrupt = SceneSnapshotDelta::diff(&confirmed, &server);
        corrupt.target_crc32c ^= 1;
        assert!(
            buffer
                .rollback_and_apply_authoritative(&registry, 8, &corrupt)
                .is_err()
        );

        // Capacity too small for the authoritative document: the staged load fails.
        let valid = SceneSnapshotDelta::diff(&confirmed, &server);
        assert!(
            buffer
                .rollback_and_apply_authoritative(&registry, 1, &valid)
                .is_err()
        );
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.get(10), Some(&confirmed));
    }
}
