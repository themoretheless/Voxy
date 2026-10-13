//! Priority-queued asynchronous asset and LOD streaming (Optimizations #81-85).

use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::{AssetCatalog, AssetError, AssetId, AssetTicket};

/// Coarse priority class for asynchronous asset and LOD streaming.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StreamingPriority {
    Prefetch = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Camera-distance LOD threshold policy for selecting mesh/texture detail levels
/// before queuing streaming requests.
#[derive(Clone, Debug, PartialEq)]
pub struct LodStreamingPolicy {
    /// Monotonically increasing camera distance thresholds (in world units).
    /// Distance `<= thresholds[i]` selects LOD `i`; beyond all thresholds selects `thresholds.len()`.
    thresholds: Vec<f32>,
}

impl LodStreamingPolicy {
    /// Creates a validated camera-distance LOD streaming policy.
    ///
    /// # Errors
    /// Rejects non-finite, negative, or non-increasing distance thresholds.
    pub fn new(thresholds: Vec<f32>) -> Result<Self, &'static str> {
        let mut prev = 0.0_f32;
        for &d in &thresholds {
            if !d.is_finite() || d < prev {
                return Err("LOD distance thresholds must be finite, non-negative, and monotonic");
            }
            prev = d;
        }
        Ok(Self { thresholds })
    }

    /// Selects the target LOD index for a given camera distance.
    #[must_use]
    pub fn select_lod(&self, camera_distance: f32) -> usize {
        if !camera_distance.is_finite() || camera_distance <= 0.0 {
            return 0;
        }
        self.thresholds
            .partition_point(|&threshold| threshold < camera_distance)
    }

    #[must_use]
    pub fn thresholds(&self) -> &[f32] {
        &self.thresholds
    }
}

/// A prioritized streaming request ordered by [`StreamingPriority`] first, then
/// closer camera distance, and finally earlier sequence number (FIFO tie-breaker).
#[derive(Clone, Debug)]
pub struct StreamingRequest {
    pub ticket: AssetTicket,
    pub priority: StreamingPriority,
    pub distance_millimetres: u64,
    pub sequence: u64,
}

impl PartialEq for StreamingRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.distance_millimetres == other.distance_millimetres
            && self.sequence == other.sequence
    }
}

impl Eq for StreamingRequest {}

impl PartialOrd for StreamingRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StreamingRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.distance_millimetres.cmp(&self.distance_millimetres))
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

/// Asynchronous asset streamer combining a priority queue with non-blocking worker
/// completion channels connected to an [`AssetCatalog`].
#[derive(Debug)]
pub struct AsyncAssetStreamer<T> {
    queue: BinaryHeap<StreamingRequest>,
    queued_ids: BTreeSet<AssetId>,
    tx: Sender<(AssetTicket, Result<T, String>)>,
    rx: Receiver<(AssetTicket, Result<T, String>)>,
    next_sequence: u64,
}

impl<T: Send + 'static> Default for AsyncAssetStreamer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + 'static> AsyncAssetStreamer<T> {
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            queue: BinaryHeap::new(),
            queued_ids: BTreeSet::new(),
            tx,
            rx,
            next_sequence: 0,
        }
    }

    /// Enqueues an asset for priority streaming if it is not already queued or ready.
    ///
    /// # Errors
    /// Propagates [`AssetCatalog::request`] errors when capacity or ID validation fails.
    pub fn enqueue(
        &mut self,
        catalog: &mut AssetCatalog<T>,
        asset: AssetId,
        priority: StreamingPriority,
        camera_distance: f32,
    ) -> Result<bool, AssetError> {
        if self.queued_ids.contains(&asset) {
            return Ok(false);
        }
        if matches!(catalog.status(&asset), Some(crate::AssetStatus::Ready)) {
            return Ok(false);
        }
        let ticket = catalog.request(asset.clone())?;
        let distance_millimetres = if camera_distance.is_finite() && camera_distance >= 0.0 {
            (camera_distance * 1000.0).min(u64::MAX as f32) as u64
        } else {
            u64::MAX
        };
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.queued_ids.insert(asset);
        self.queue.push(StreamingRequest {
            ticket,
            priority,
            distance_millimetres,
            sequence,
        });
        Ok(true)
    }

    #[must_use]
    pub fn queued_len(&self) -> usize {
        self.queue.len()
    }

    /// Pops up to `max_jobs` highest-priority streaming requests and spawns background
    /// loader tasks that send their results back without blocking the caller thread.
    pub fn dispatch_background<F>(&mut self, max_jobs: usize, loader: F) -> usize
    where
        F: Fn(&AssetId) -> Result<T, String> + Send + Sync + 'static,
    {
        let loader = std::sync::Arc::new(loader);
        let mut dispatched = 0;
        while dispatched < max_jobs {
            let Some(request) = self.queue.pop() else {
                break;
            };
            self.queued_ids.remove(request.ticket.asset());
            let tx = self.tx.clone();
            let loader = std::sync::Arc::clone(&loader);
            std::thread::spawn(move || {
                let outcome = loader(request.ticket.asset());
                let _ = tx.send((request.ticket, outcome));
            });
            dispatched += 1;
        }
        dispatched
    }

    /// Drains up to `max_completions` finished background loads and publishes them
    /// into `catalog` on the owner thread without blocking.
    pub fn poll_completed(
        &mut self,
        catalog: &mut AssetCatalog<T>,
        max_completions: usize,
    ) -> usize {
        let mut completed = 0;
        while completed < max_completions {
            let Ok((ticket, outcome)) = self.rx.try_recv() else {
                break;
            };
            let _ = catalog.complete(&ticket, outcome);
            completed += 1;
        }
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod_streaming_policy_selects_expected_level_by_distance() {
        let policy = LodStreamingPolicy::new(vec![15.0, 40.0, 100.0]).unwrap();
        assert_eq!(policy.select_lod(0.0), 0);
        assert_eq!(policy.select_lod(15.0), 0);
        assert_eq!(policy.select_lod(15.1), 1);
        assert_eq!(policy.select_lod(40.0), 1);
        assert_eq!(policy.select_lod(75.0), 2);
        assert_eq!(policy.select_lod(150.0), 3);
    }

    #[test]
    fn priority_queue_orders_by_priority_then_closest_camera_distance() {
        let mut catalog = AssetCatalog::<String>::new(16, 16).unwrap();
        let mut streamer = AsyncAssetStreamer::<String>::new();

        streamer
            .enqueue(
                &mut catalog,
                AssetId("far_normal".into()),
                StreamingPriority::Normal,
                80.0,
            )
            .unwrap();
        streamer
            .enqueue(
                &mut catalog,
                AssetId("near_normal".into()),
                StreamingPriority::Normal,
                12.0,
            )
            .unwrap();
        streamer
            .enqueue(
                &mut catalog,
                AssetId("critical_mesh".into()),
                StreamingPriority::Critical,
                50.0,
            )
            .unwrap();

        assert_eq!(streamer.queued_len(), 3);
        let first = streamer.queue.pop().unwrap();
        let second = streamer.queue.pop().unwrap();
        let third = streamer.queue.pop().unwrap();
        assert_eq!(first.ticket.asset().0, "critical_mesh");
        assert_eq!(second.ticket.asset().0, "near_normal");
        assert_eq!(third.ticket.asset().0, "far_normal");
    }
}
