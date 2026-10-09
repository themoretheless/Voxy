//! Renderer-neutral authoritative runtime contracts.

mod bootstrap;
mod frame_loop;
pub use frame_loop::{FrameLoop, FrameWork};
mod time;
pub use time::{SimulationClock, TimeDrop, TimeFrame};

use std::collections::BTreeMap;

use voxy_core::{ChunkPos, TickId, WorldEpoch};

pub use bootstrap::{
    BootstrapChunk, BootstrapError, BootstrapScene, build_bootstrap_mesh, build_bootstrap_scene,
    build_generated_scene, build_procedural_scene, invalidated_derived_chunks,
    rebuild_bootstrap_chunks,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JobClass {
    SimulationCritical,
    Visible,
    Prefetch,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompletionKey {
    pub class: JobClass,
    pub chunk: ChunkPos,
    pub ticket: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Completion<T> {
    pub world_epoch: WorldEpoch,
    pub key: CompletionKey,
    pub value: T,
}

/// Pending results are applied only at an explicit barrier and in canonical order.
#[derive(Debug)]
pub struct BarrierQueue<T> {
    world_epoch: WorldEpoch,
    pending: BTreeMap<CompletionKey, T>,
}

impl<T> BarrierQueue<T> {
    #[must_use]
    pub fn new(world_epoch: WorldEpoch) -> Self {
        Self {
            world_epoch,
            pending: BTreeMap::new(),
        }
    }

    /// Epoch to attach to newly dispatched jobs.
    #[must_use]
    pub const fn world_epoch(&self) -> WorldEpoch {
        self.world_epoch
    }

    /// Starts a new world generation and discards pending old-world results.
    /// Late completions carrying the previous epoch will be rejected by `push`.
    /// Returns `None` on epoch exhaustion, preserving both epoch and results.
    pub fn advance_epoch(&mut self) -> Option<WorldEpoch> {
        let next = self.world_epoch.checked_next()?;
        self.pending.clear();
        self.world_epoch = next;
        Some(next)
    }

    /// Returns false for stale epochs or duplicate keys.
    pub fn push(&mut self, completion: Completion<T>) -> bool {
        if completion.world_epoch != self.world_epoch {
            return false;
        }
        match self.pending.entry(completion.key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(completion.value);
                true
            }
            std::collections::btree_map::Entry::Occupied(_) => false,
        }
    }

    pub fn drain_barrier(&mut self, max_count: usize) -> Vec<(CompletionKey, T)> {
        let count = max_count.min(self.pending.len());
        (0..count)
            .filter_map(|_| self.pending.pop_first())
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionFrame {
    pub tick: TickId,
    pub held_bits: u64,
    pub pressed_bits: u64,
    pub mouse_delta: [i32; 2],
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(ticket: u64, x: i64) -> CompletionKey {
        CompletionKey {
            class: JobClass::Visible,
            chunk: ChunkPos { x, y: 0, z: 0 },
            ticket,
        }
    }

    #[test]
    fn barrier_order_is_independent_of_arrival_order() {
        let epoch = WorldEpoch::new(1).unwrap();
        let inputs = [(key(9, 2), 20), (key(3, -1), 10), (key(1, 2), 15)];
        let mut forward = BarrierQueue::new(epoch);
        let mut reverse = BarrierQueue::new(epoch);
        for &(key, value) in &inputs {
            assert!(forward.push(Completion {
                world_epoch: epoch,
                key,
                value
            }));
        }
        for &(key, value) in inputs.iter().rev() {
            assert!(reverse.push(Completion {
                world_epoch: epoch,
                key,
                value
            }));
        }
        assert_eq!(
            forward.drain_barrier(usize::MAX),
            reverse.drain_barrier(usize::MAX)
        );
    }

    #[test]
    fn stale_epoch_and_duplicate_are_rejected() {
        let epoch = WorldEpoch::new(7).unwrap();
        let mut queue = BarrierQueue::new(epoch);
        let completion = Completion {
            world_epoch: epoch,
            key: key(1, 0),
            value: 5,
        };
        assert!(queue.push(completion.clone()));
        assert!(!queue.push(completion));
        assert!(!queue.push(Completion {
            world_epoch: WorldEpoch::new(6).unwrap(),
            key: key(2, 0),
            value: 6,
        }));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn rejected_results_preserve_the_first_accepted_value() {
        let epoch = WorldEpoch::new(7).unwrap();
        let mut queue = BarrierQueue::new(epoch);
        let accepted_key = key(1, 0);
        assert!(queue.push(Completion {
            world_epoch: epoch,
            key: accepted_key,
            value: "accepted mesh",
        }));
        assert!(!queue.push(Completion {
            world_epoch: epoch,
            key: accepted_key,
            value: "duplicate mesh",
        }));
        assert!(!queue.push(Completion {
            world_epoch: WorldEpoch::new(6).unwrap(),
            key: accepted_key,
            value: "old scene mesh",
        }));
        assert_eq!(
            queue.drain_barrier(usize::MAX),
            vec![(accepted_key, "accepted mesh")]
        );
    }

    #[test]
    fn world_transition_discards_pending_and_rejects_late_jobs() {
        let old = WorldEpoch::new(7).unwrap();
        let mut queue = BarrierQueue::new(old);
        let job_key = key(1, 0);
        assert!(queue.push(Completion {
            world_epoch: old,
            key: job_key,
            value: "old mesh",
        }));
        let current = queue.advance_epoch().unwrap();
        assert_eq!(current.get(), 8);
        assert_eq!(queue.world_epoch(), current);
        assert!(queue.is_empty());
        assert!(!queue.push(Completion {
            world_epoch: old,
            key: job_key,
            value: "late mesh",
        }));
        assert!(queue.push(Completion {
            world_epoch: current,
            key: job_key,
            value: "new mesh",
        }));
        assert_eq!(queue.drain_barrier(1), vec![(job_key, "new mesh")]);
    }

    #[test]
    fn exhausted_epoch_preserves_pending_results() {
        let epoch = WorldEpoch::new(u64::MAX).unwrap();
        let mut queue = BarrierQueue::new(epoch);
        let job_key = key(1, 0);
        assert!(queue.push(Completion {
            world_epoch: epoch,
            key: job_key,
            value: 42,
        }));
        assert_eq!(queue.advance_epoch(), None);
        assert_eq!(queue.world_epoch(), epoch);
        assert_eq!(queue.drain_barrier(1), vec![(job_key, 42)]);
    }

    #[test]
    fn count_budget_leaves_remainder_for_next_barrier() {
        let epoch = WorldEpoch::new(1).unwrap();
        let mut queue = BarrierQueue::new(epoch);
        for ticket in 0..5 {
            assert!(queue.push(Completion {
                world_epoch: epoch,
                key: key(ticket, i64::try_from(ticket).unwrap()),
                value: ticket,
            }));
        }
        assert_eq!(queue.drain_barrier(2).len(), 2);
        assert_eq!(queue.len(), 3);
    }
}
