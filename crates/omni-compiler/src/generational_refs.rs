//! Generational references and a safe arena used by bootstrap linear-type support.
//!
//! Handles carry a slot index plus a monotonically increasing generation. A
//! removed slot increments its generation before it can be reused, so stale
//! handles cannot become valid again. Saturated slots are retired permanently
//! rather than wrapping their generation.

use std::marker::PhantomData;
use std::ptr::NonNull;

/// A generational reference to a value inside an `Arena<T>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenRef<T> {
    pub gen: u64,
    pub idx: usize,
    _marker: PhantomData<T>,
}

impl<T> GenRef<T> {
    pub fn new(idx: usize, gen: u64) -> Self {
        Self {
            idx,
            gen,
            _marker: PhantomData,
        }
    }

    /// Return the current stable heap address when this handle is valid.
    ///
    /// The pointer is an observation only; safe code cannot dereference it
    /// without going back through the arena's checked accessors.
    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self, arena: &Arena<T>) -> Option<NonNull<T>> {
        arena
            .slots
            .get(self.idx)
            .filter(|slot| slot.gen == self.gen)
            .and_then(|slot| slot.value.as_deref())
            .map(NonNull::from)
    }
}

struct ArenaSlot<T> {
    value: Option<Box<T>>,
    gen: u64,
}

/// A safe generational arena.
///
/// Values themselves live in `Box<T>`, so growth of the slot vector never
/// moves a live value. No raw allocation ownership is retained after removal,
/// avoiding the leak/double-drop hazards of the historical bootstrap arena.
pub struct Arena<T> {
    slots: Vec<ArenaSlot<T>>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    /// Allocate a value, reusing a retired slot only when its generation can
    /// still distinguish it from every previously issued handle for that slot.
    pub fn alloc(&mut self, value: T) -> GenRef<T> {
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.value.is_none() && slot.gen < u64::MAX {
                slot.value = Some(Box::new(value));
                return GenRef::new(idx, slot.gen);
            }
        }

        let idx = self.slots.len();
        self.slots.push(ArenaSlot {
            value: Some(Box::new(value)),
            gen: 0,
        });
        GenRef::new(idx, 0)
    }

    pub fn get(&self, r: GenRef<T>) -> Option<&T> {
        self.slots
            .get(r.idx)
            .filter(|slot| slot.gen == r.gen)
            .and_then(|slot| slot.value.as_deref())
    }

    pub fn get_mut(&mut self, r: GenRef<T>) -> Option<&mut T> {
        self.slots
            .get_mut(r.idx)
            .filter(|slot| slot.gen == r.gen)
            .and_then(|slot| slot.value.as_deref_mut())
    }

    /// Remove a live value. The generation advances exactly once at removal,
    /// invalidating every handle previously issued for the slot.
    pub fn remove(&mut self, r: GenRef<T>) -> bool {
        let Some(slot) = self.slots.get_mut(r.idx) else {
            return false;
        };
        if slot.gen != r.gen || slot.value.is_none() {
            return false;
        }

        slot.value = None;
        slot.gen = slot.gen.saturating_add(1);
        true
    }

    pub fn is_valid(&self, r: GenRef<T>) -> bool {
        self.get(r).is_some()
    }

    pub fn live_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.value.is_some())
            .count()
    }

    pub fn iter(&self) -> ArenaIter<'_, T> {
        ArenaIter {
            arena: self,
            idx: 0,
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ArenaIter<'a, T> {
    arena: &'a Arena<T>,
    idx: usize,
}

impl<'a, T> Iterator for ArenaIter<'a, T> {
    type Item = (GenRef<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < self.arena.slots.len() {
            let idx = self.idx;
            self.idx += 1;
            let slot = &self.arena.slots[idx];
            if let Some(value) = slot.value.as_deref() {
                return Some((GenRef::new(idx, slot.gen), value));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genref_basic_alloc_and_get() {
        let mut arena: Arena<i32> = Arena::new();
        let r = arena.alloc(42);
        assert_eq!(arena.get(r), Some(&42));
        assert_eq!(arena.live_count(), 1);
    }

    #[test]
    fn genref_mutate_through_ref() {
        let mut arena: Arena<i32> = Arena::new();
        let r = arena.alloc(10);
        if let Some(v) = arena.get_mut(r) {
            *v = 20;
        } else {
            panic!("fresh handle must be valid");
        }
        assert_eq!(arena.get(r), Some(&20));
    }

    #[test]
    fn genref_remove_invalidates_and_double_remove_fails() {
        let mut arena: Arena<i32> = Arena::new();
        let r = arena.alloc(99);
        assert!(arena.remove(r));
        assert!(!arena.remove(r));
        assert!(!arena.is_valid(r));
        assert_eq!(arena.live_count(), 0);
    }

    #[test]
    fn stale_handle_never_aliases_reused_slot() {
        let mut arena: Arena<i32> = Arena::new();
        let old = arena.alloc(1);
        assert!(arena.remove(old));
        let new = arena.alloc(2);
        assert_eq!(old.idx, new.idx);
        assert_ne!(old.gen, new.gen);
        assert_eq!(arena.get(old), None);
        assert_eq!(arena.get(new), Some(&2));
    }

    #[test]
    fn genref_iter_skips_removed_values() {
        let mut arena: Arena<i32> = Arena::new();
        let _r1 = arena.alloc(1);
        let r2 = arena.alloc(2);
        let _r3 = arena.alloc(3);
        assert!(arena.remove(r2));
        let values: Vec<i32> = arena.iter().map(|(_, v)| *v).collect();
        assert_eq!(values, vec![1, 3]);
    }

    #[test]
    fn pointer_observation_tracks_validity() {
        let mut arena: Arena<i32> = Arena::new();
        let r = arena.alloc(7);
        assert!(r.as_ptr(&arena).is_some());
        assert!(arena.remove(r));
        assert!(r.as_ptr(&arena).is_none());
    }
}
