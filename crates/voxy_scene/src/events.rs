//! Bounded multi-reader events with explicit lag reporting.
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_CHANNEL: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug)]
pub struct EventCursor {
    channel: u64,
    next: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventError {
    ZeroCapacity,
    ForeignCursor,
    SequenceExhausted,
    ChannelExhausted,
}
impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event error: {self:?}")
    }
}
impl std::error::Error for EventError {}
#[derive(Debug)]
pub struct EventChannel<T> {
    id: u64,
    next: u64,
    capacity: usize,
    events: VecDeque<T>,
}
#[derive(Debug)]
pub struct EventRead<'a, T> {
    /// Number of events evicted/cleared before this consumer observed them.
    pub missed: u64,
    pub events: Vec<&'a T>,
}
impl<T> EventChannel<T> {
    /// # Errors
    /// Rejects zero capacity and exhausted channel identities.
    pub fn new(capacity: usize) -> Result<Self, EventError> {
        if capacity == 0 {
            return Err(EventError::ZeroCapacity);
        }
        let id = NEXT_CHANNEL
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| EventError::ChannelExhausted)?;
        Ok(Self {
            id,
            next: 0,
            capacity,
            events: VecDeque::new(),
        })
    }
    /// Starts at the oldest retained event or at the next event when replay=false.
    #[must_use]
    pub fn subscribe(&self, replay: bool) -> EventCursor {
        EventCursor {
            channel: self.id,
            next: if replay { self.oldest() } else { self.next },
        }
    }
    fn oldest(&self) -> u64 {
        self.next - self.events.len() as u64
    }
    /// Appends an event and returns an evicted value if the bounded buffer was full.
    /// # Errors
    /// Sequence exhaustion leaves the buffer unchanged.
    pub fn emit(&mut self, event: T) -> Result<Option<T>, EventError> {
        let next = self
            .next
            .checked_add(1)
            .ok_or(EventError::SequenceExhausted)?;
        let evicted = if self.events.len() == self.capacity {
            self.events.pop_front()
        } else {
            None
        };
        self.events.push_back(event);
        self.next = next;
        Ok(evicted)
    }
    /// Returns unread retained events and advances only this consumer's cursor.
    /// The borrow prevents emission while references are being processed.
    /// # Errors
    /// Rejects cursors from another channel without advancing them.
    pub fn read<'a>(&'a self, cursor: &mut EventCursor) -> Result<EventRead<'a, T>, EventError> {
        if cursor.channel != self.id {
            return Err(EventError::ForeignCursor);
        }
        let oldest = self.oldest();
        let missed = oldest.saturating_sub(cursor.next);
        let start = usize::try_from(cursor.next.max(oldest) - oldest)
            .map_err(|_| EventError::SequenceExhausted)?;
        let events = self.events.iter().skip(start).collect();
        cursor.next = self.next;
        Ok(EventRead { missed, events })
    }
    /// Clears retained data without resetting sequence numbers. Lagging consumers
    /// learn how many events were lost on their next read.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consumers_are_independent_and_lag_is_explicit() {
        let mut channel = EventChannel::new(2).unwrap();
        let mut a = channel.subscribe(false);
        let mut b = channel.subscribe(false);
        assert_eq!(channel.emit(1), Ok(None));
        assert_eq!(channel.read(&mut a).unwrap().events, vec![&1]);
        channel.emit(2).unwrap();
        assert_eq!(channel.emit(3), Ok(Some(1)));
        let read = channel.read(&mut b).unwrap();
        assert_eq!(read.missed, 1);
        assert_eq!(read.events, vec![&2, &3]);
        let read = channel.read(&mut a).unwrap();
        assert_eq!(read.missed, 0);
        assert_eq!(read.events, vec![&2, &3]);
        assert!(channel.read(&mut a).unwrap().events.is_empty());
    }
    #[test]
    fn replay_clear_foreign_cursor_and_exhaustion() {
        let mut channel = EventChannel::new(2).unwrap();
        channel.emit(7).unwrap();
        let mut replay = channel.subscribe(true);
        let mut fresh = channel.subscribe(false);
        assert!(channel.read(&mut fresh).unwrap().events.is_empty());
        channel.clear();
        assert_eq!(channel.read(&mut replay).unwrap().missed, 1);
        let foreign = EventChannel::<u32>::new(1).unwrap();
        assert!(matches!(
            foreign.read(&mut replay),
            Err(EventError::ForeignCursor)
        ));
        channel.next = u64::MAX;
        assert_eq!(channel.emit(9), Err(EventError::SequenceExhausted));
        assert!(channel.events.is_empty());
    }
}
