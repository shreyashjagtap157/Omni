//! Generational references and arena allocator for linear type support.
//!
//! `GenRef<T>` is a safe pointer that combines a generational index with
//! a raw pointer. The generation is incremented every time the slot is
//! reused, so any stale `GenRef` will detect a generation mismatch.
//!
//! `Arena<T>` is a simple arena allocator that stores values of type `T`
//! in a `Vec` and returns `GenRef<T>` handles.

use std::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// A generational reference to a value inside an `Arena<T>`.
///
/// The `gen` field acts as a "version" for the slot at index `idx`.
/// When the arena reclaims a slot (e.g., when a linear value is dropped),
/// it increments the generation so that any stale `GenRef` becomes invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenRef<T> {
    pub gen: u64,
    pub idx: usize,
    _marker: PhantomData<T>,
}

impl<T> GenRef<T> {
    /// Create a new generational reference.
    pub fn new(idx: usize, gen: u64) -> Self {
        GenRef {
            idx,
            gen,
            _marker: PhantomData,
        }
    }

    /// Get the raw pointer (unsafe, for internal arena use).
    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self, arena: &Arena<T>) -> Option<NonNull<T>> {
        arena.get_slot(self.idx).and_then(|slot| {
            if slot.gen.get() == self.gen {
                Some(slot.ptr)
            } else {
                None // stale reference
            }
        })
    }
}

/// An arena slot that tracks both the value pointer and its current generation.
struct ArenaSlot<T> {
    ptr: NonNull<T>,
    gen: Cell<u64>,
    live: Cell<bool>,
}

/// A simple arena allocator that returns `GenRef<T>` handles.
///
/// When a value is removed (dropped), the slot is marked dead and the
/// generation is incremented so that stale `GenRef` handles become invalid.
pub struct Arena<T> {
    slots: Vec<ArenaSlot<T>>,
    _marker: PhantomData<T>,
}

impl<T> Arena<T> {
    /// Create a new empty arena.
    pub fn new() -> Self {
        Arena {
            slots: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Allocate a value in the arena and return a `GenRef` handle.
    ///
    /// This will reuse a dead slot if one is available, otherwise push a new slot.
    pub fn alloc(&mut self, value: T) -> GenRef<T> {
        // Try to find a dead slot to reuse.
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if !slot.live.get() {
                // Reuse this slot: update pointer, bump generation, mark live.
                unsafe {
                    std::ptr::write(slot.ptr.as_ptr(), value);
                }
                let new_gen = slot.gen.get() + 1;
                slot.gen.set(new_gen);
                slot.live.set(true);
                return GenRef::new(idx, new_gen);
            }
        }

        // No dead slot; allocate a new one.
        let boxed = Box::new(value);
        let ptr = NonNull::new(Box::into_raw(boxed)).expect("Box::into_raw returned null");
        let idx = self.slots.len();
        self.slots.push(ArenaSlot {
            ptr,
            gen: Cell::new(0),
            live: Cell::new(true),
        });
        GenRef::new(idx, 0)
    }

    /// Get a mutable reference to the value behind a `GenRef`.
    ///
    /// Returns `None` if the reference is stale (wrong generation) or the
    /// slot is dead.
    pub fn get_mut(&mut self, r: GenRef<T>) -> Option<&mut T> {
        self.slots.get(r.idx).and_then(|slot| {
            if slot.live.get() && slot.gen.get() == r.gen {
                unsafe { Some(slot.ptr.as_ptr().as_mut().unwrap()) }
            } else {
                None
            }
        })
    }

    /// Get an immutable reference to the value behind a `GenRef`.
    pub fn get(&self, r: GenRef<T>) -> Option<&T> {
        self.slots.get(r.idx).and_then(|slot| {
            if slot.live.get() && slot.gen.get() == r.gen {
                unsafe { Some(slot.ptr.as_ref()) }
            } else {
                None
            }
        })
    }

    /// Remove (drop) a value from the arena using its `GenRef`.
    ///
    /// Returns `true` if the value was successfully removed, `false` if the
    /// reference was stale.
    ///
    /// After removal, the slot is marked dead and its generation is bumped,
    /// invalidating any existing `GenRef` to this slot.
    pub fn remove(&mut self, r: GenRef<T>) -> bool {
        if let Some(slot) = self.slots.get(r.idx) {
            if slot.live.get() && slot.gen.get() == r.gen {
                unsafe {
                    std::ptr::drop_in_place(slot.ptr.as_ptr());
                }
                slot.live.set(false);
                // Bump generation to invalidate stale refs.
                slot.gen.set(slot.gen.get() + 1);
                return true;
            }
        }
        false
    }

    /// Check if a `GenRef` is still valid (live and correct generation).
    pub fn is_valid(&self, r: GenRef<T>) -> bool {
        self.slots
            .get(r.idx)
            .map(|slot| slot.live.get() && slot.gen.get() == r.gen)
            .unwrap_or(false)
    }

    /// Get the number of live (allocated) values in the arena.
    pub fn live_count(&self) -> usize {
        self.slots.iter().filter(|s| s.live.get()).count()
    }

    /// Iterate over all live values.
    pub fn iter(&self) -> ArenaIter<'_, T> {
        ArenaIter {
            arena: self,
            idx: 0,
        }
    }

    /// Get the slot at an index (internal use only).
    #[allow(dead_code)]
    fn get_slot(&self, idx: usize) -> Option<&ArenaSlot<T>> {
        self.slots.get(idx)
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Arena<T> {
    fn drop(&mut self) {
        // Reconstruct boxes from all live slots and drop them.
        // This properly deallocates the memory.
        for slot in &self.slots {
            if slot.live.get() {
                unsafe {
                    // Reconstruct the Box and drop it (which drops T and frees memory).
                    let _ = Box::from_raw(slot.ptr.as_ptr());
                }
            }
        }
    }
}

/// Iterator over live values in an arena.
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
            if slot.live.get() {
                let r = GenRef::new(idx, slot.gen.get());
                unsafe {
                    return Some((r, slot.ptr.as_ref()));
                }
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
        {
            let v = arena.get_mut(r).unwrap();
            *v = 20;
        }
        assert_eq!(arena.get(r), Some(&20));
    }

    #[test]
    fn genref_remove_invalidates() {
        let mut arena: Arena<i32> = Arena::new();
        let r = arena.alloc(99);
        assert!(arena.is_valid(r));
        assert!(arena.remove(r));
        assert!(!arena.is_valid(r));
        assert_eq!(arena.get(r), None);
        assert_eq!(arena.live_count(), 0);
    }

    #[test]
    fn genref_stale_after_remove() {
        let mut arena: Arena<i32> = Arena::new();
        let r = arena.alloc(1);
        arena.remove(r);
        // Trying to get with the same GenRef should fail.
        assert_eq!(arena.get(r), None);
        // Allocating again should reuse the slot with a new generation.
        let r2 = arena.alloc(2);
        assert_ne!(r.gen, r2.gen); // generation should differ
    }

    #[test]
    fn genref_iter() {
        let mut arena: Arena<i32> = Arena::new();
        let _r1 = arena.alloc(1);
        let r2 = arena.alloc(2);
        let _r3 = arena.alloc(3);
        arena.remove(r2); // Remove middle one

        let values: Vec<i32> = arena.iter().map(|(_, v)| *v).collect();
        assert_eq!(values, vec![1, 3]); // Only live values
    }

    #[test]
    fn genref_reuse_slot() {
        let mut arena: Arena<i32> = Arena::new();
        let r1 = arena.alloc(100);
        arena.remove(r1);
        let r2 = arena.alloc(200);
        // Should reuse slot 0 with bumped generation.
        assert_eq!(r2.idx, 0);
        assert!(r2.gen > 0);
        assert_eq!(arena.get(r2), Some(&200));
    }

    #[test]
    fn arena_drop_cleans_up() {
        // This test ensures Drop is properly implemented.
        let mut arena: Arena<String> = Arena::new();
        let _r1 = arena.alloc("hello".to_string());
        let _r2 = arena.alloc("world".to_string());
        // Arena will drop all live values when it goes out of scope.
    }
}
