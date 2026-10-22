use core::fmt;

/// Monotonic identifier of an authoritative simulation tick.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TickId(u64);

impl TickId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl fmt::Debug for TickId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("TickId").field(&self.0).finish()
    }
}

/// Identifies one open/reset lifetime of a world inside the process.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorldEpoch(u64);

impl WorldEpoch {
    /// Epoch zero is deliberately invalid so default/uninitialized values cannot cross boundaries.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Self::new(value),
            None => None,
        }
    }
}

impl fmt::Debug for WorldEpoch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("WorldEpoch").field(&self.0).finish()
    }
}

/// Non-max `u32` handle with niche optimization so `Option<NonMaxU32>` is 4 bytes.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NonMaxU32(core::num::NonZeroU32);

impl NonMaxU32 {
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match core::num::NonZeroU32::new(value ^ u32::MAX) {
            Some(nz) => Some(Self(nz)),
            None => None,
        }
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get() ^ u32::MAX
    }
}

impl fmt::Debug for NonMaxU32 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("NonMaxU32")
            .field(&self.get())
            .finish()
    }
}

/// Non-max `u64` handle with niche optimization so `Option<NonMaxU64>` is 8 bytes.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NonMaxU64(core::num::NonZeroU64);

impl NonMaxU64 {
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match core::num::NonZeroU64::new(value ^ u64::MAX) {
            Some(nz) => Some(Self(nz)),
            None => None,
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get() ^ u64::MAX
    }
}

impl fmt::Debug for NonMaxU64 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("NonMaxU64")
            .field(&self.get())
            .finish()
    }
}

/// Generational handle (`u32` slot index + non-zero `u32` generation) guarding against ABA reuse.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GenerationalId {
    index: u32,
    generation: core::num::NonZeroU32,
}

impl GenerationalId {
    #[must_use]
    pub const fn new(index: u32, generation: u32) -> Option<Self> {
        match core::num::NonZeroU32::new(generation) {
            Some(generation) => Some(Self { index, generation }),
            None => None,
        }
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation.get()
    }

    #[must_use]
    pub const fn to_u64(self) -> u64 {
        ((self.generation.get() as u64) << 32) | (self.index as u64)
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_u64(packed: u64) -> Option<Self> {
        Self::new(packed as u32, (packed >> 32) as u32)
    }
}

impl fmt::Debug for GenerationalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GenerationalId")
            .field("index", &self.index)
            .field("generation", &self.generation.get())
            .finish()
    }
}

/// Generational arena (`SlotMap`) providing $O(1)$ insertion, lookup, and ABA-safe removal.
#[derive(Clone, Debug)]
pub struct SlotMap<T> {
    slots: Vec<Slot<T>>,
    free_head: Option<u32>,
    len: usize,
}

#[derive(Clone, Debug)]
struct Slot<T> {
    generation: u32,
    state: SlotState<T>,
}

#[derive(Clone, Debug)]
enum SlotState<T> {
    Occupied(T),
    Free { next_free: Option<u32> },
}

impl<T> Default for SlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SlotMap<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_head: None,
            len: 0,
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Inserts a value and returns its unique [`GenerationalId`].
    pub fn insert(&mut self, value: T) -> Option<GenerationalId> {
        if let Some(index) = self.free_head {
            let slot = self.slots.get_mut(index as usize)?;
            let SlotState::Free { next_free } = slot.state else {
                return None;
            };
            self.free_head = next_free;
            slot.state = SlotState::Occupied(value);
            self.len += 1;
            GenerationalId::new(index, slot.generation)
        } else {
            let index = u32::try_from(self.slots.len()).ok()?;
            self.slots.push(Slot {
                generation: 1,
                state: SlotState::Occupied(value),
            });
            self.len += 1;
            GenerationalId::new(index, 1)
        }
    }

    #[must_use]
    pub fn get(&self, id: GenerationalId) -> Option<&T> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation != id.generation.get() {
            return None;
        }
        match &slot.state {
            SlotState::Occupied(value) => Some(value),
            SlotState::Free { .. } => None,
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, id: GenerationalId) -> Option<&mut T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation.get() {
            return None;
        }
        match &mut slot.state {
            SlotState::Occupied(value) => Some(value),
            SlotState::Free { .. } => None,
        }
    }

    pub fn remove(&mut self, id: GenerationalId) -> Option<T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation.get() {
            return None;
        }
        let previous = core::mem::replace(
            &mut slot.state,
            SlotState::Free {
                next_free: self.free_head,
            },
        );
        let SlotState::Occupied(value) = previous else {
            slot.state = previous;
            return None;
        };
        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.free_head = Some(id.index);
        self.len -= 1;
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_max_niche_matches_primitive_size() {
        assert_eq!(size_of::<Option<NonMaxU32>>(), size_of::<u32>());
        assert_eq!(size_of::<Option<NonMaxU64>>(), size_of::<u64>());
        assert_eq!(size_of::<Option<GenerationalId>>(), size_of::<u64>());
        assert!(NonMaxU32::new(u32::MAX).is_none());
        assert_eq!(NonMaxU32::new(0).unwrap().get(), 0);
        assert_eq!(NonMaxU32::new(42).unwrap().get(), 42);
    }

    #[test]
    fn slot_map_rejects_stale_generations_after_remove_and_reuse() {
        let mut map = SlotMap::new();
        let a = map.insert("alpha").unwrap();
        let b = map.insert("beta").unwrap();
        assert_eq!(map.get(a), Some(&"alpha"));
        assert_eq!(map.remove(a), Some("alpha"));
        assert_eq!(map.get(a), None);
        let c = map.insert("gamma").unwrap();
        assert_eq!(c.index(), a.index());
        assert_ne!(c.generation(), a.generation());
        assert_eq!(map.get(a), None);
        assert_eq!(map.get(c), Some(&"gamma"));
        assert_eq!(map.get(b), Some(&"beta"));
    }
}
