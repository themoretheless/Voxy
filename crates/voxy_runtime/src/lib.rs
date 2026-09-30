//! Renderer-neutral authoritative runtime contracts.

mod bootstrap;
mod time;
pub use time::{SimulationClock, TimeFrame};

use std::collections::BTreeMap;

use voxy_core::{ChunkPos, TickId, WorldEpoch};

pub use bootstrap::{
    BootstrapChunk, BootstrapError, BootstrapScene, build_bootstrap_mesh, build_bootstrap_scene,
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

    /// Returns false for stale epochs or duplicate keys.
    pub fn push(&mut self, completion: Completion<T>) -> bool {
        completion.world_epoch == self.world_epoch
            && self
                .pending
                .insert(completion.key, completion.value)
                .is_none()
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
