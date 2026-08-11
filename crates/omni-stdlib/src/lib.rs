#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Generation(u64);

impl Generation {
    pub fn new() -> Self {
        Generation(0)
    }
    pub fn increment(self) -> Self {
        Generation(self.0.saturating_add(1))
    }
}

#[derive(Debug)]
pub struct Gen<T> {
    index: usize,
    generation: Generation,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Gen<T> {
    pub fn new(index: usize, generation: Generation) -> Self {
        Gen {
            index,
            generation,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn generation(&self) -> Generation {
        self.generation
    }

    pub fn is_valid(&self, current_generation: Generation) -> bool {
        self.generation == current_generation
    }
}

impl<T> Clone for Gen<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Gen<T> {}

pub struct Arena<T> {
    items: Vec<Option<T>>,
    generations: Vec<Generation>,
    free_list: Vec<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Arena {
            items: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn alloc(&mut self, value: T) -> Gen<T> {
        while let Some(free_idx) = self.free_list.pop() {
            let current = self.generations[free_idx];
            if current.0 == u64::MAX {
                continue;
            }
            let generation = current.increment();
            self.items[free_idx] = Some(value);
            self.generations[free_idx] = generation;
            return Gen::new(free_idx, generation);
        }

        let index = self.items.len();
        let generation = Generation::new();
        self.items.push(Some(value));
        self.generations.push(generation);
        Gen::new(index, generation)
    }

    pub fn get(&self, gen: &Gen<T>) -> Option<&T> {
        if gen.index >= self.items.len() {
            return None;
        }
        if self.generations[gen.index] != gen.generation {
            return None;
        }
        self.items[gen.index].as_ref()
    }

    pub fn get_mut(&mut self, gen: &Gen<T>) -> Option<&mut T> {
        if gen.index >= self.items.len() {
            return None;
        }
        if self.generations[gen.index] != gen.generation {
            return None;
        }
        self.items[gen.index].as_mut()
    }

    pub fn contains(&self, gen: &Gen<T>) -> bool {
        self.get(gen).is_some()
    }

    pub fn len(&self) -> usize {
        self.items.iter().filter(|item| item.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn release(&mut self, gen: Gen<T>) -> bool {
        if gen.index >= self.items.len() {
            return false;
        }
        if self.generations[gen.index] != gen.generation {
            return false;
        }
        if self.items[gen.index].is_none() {
            return false;
        }

        self.items[gen.index] = None;
        self.free_list.push(gen.index);
        true
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SlotMap<T> {
    keys: Vec<u64>,
    values: Vec<Option<T>>,
    next_key: u64,
}

impl<T> SlotMap<T> {
    pub fn new() -> Self {
        SlotMap {
            keys: Vec::new(),
            values: Vec::new(),
            next_key: 1,
        }
    }

    pub fn insert(&mut self, value: T) -> u64 {
        let mut key = self.next_key.max(1);
        while self.keys.contains(&key) {
            key = key.checked_add(1).unwrap_or(1);
        }
        self.next_key = key.checked_add(1).unwrap_or(1);

        self.keys.push(key);
        self.values.push(Some(value));

        key
    }

    pub fn get(&self, key: u64) -> Option<&T> {
        for (i, &k) in self.keys.iter().enumerate() {
            if k == key {
                return self.values[i].as_ref();
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: u64) -> Option<&mut T> {
        for (i, &k) in self.keys.iter().enumerate() {
            if k == key {
                return self.values[i].as_mut();
            }
        }
        None
    }

    pub fn remove(&mut self, key: u64) -> Option<T> {
        for (i, &k) in self.keys.iter().enumerate() {
            if k == key {
                self.keys.remove(i);
                return self.values.remove(i);
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<T> Default for SlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocation failure for the v0.1.4 bootstrap scalar-cell allocator contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAllocError {
    CapacityOverflow,
    InvalidShrink,
}

/// Owned storage for a contiguous sequence of Omni's current eight-byte scalar cells.
///
/// This is bootstrap runtime infrastructure, not the final Edition-1 allocator ABI. It
/// intentionally exposes cells rather than raw byte pointers so v0.1.4 collections can
/// establish growth/capacity rules without introducing source-level raw-pointer or
/// provenance semantics before the v0.2.0 ownership milestone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellAllocation {
    cells: Vec<u64>,
}

impl CellAllocation {
    pub fn capacity_cells(&self) -> usize {
        self.cells.len()
    }

    pub fn as_slice(&self) -> &[u64] {
        &self.cells
    }

    pub fn as_mut_slice(&mut self) -> &mut [u64] {
        &mut self.cells
    }
}

/// Allocator interface used by the v0.1.4 scalar-cell collection foundation.
///
/// Implementations must preserve all existing cells when growing or shrinking to a
/// capacity that still contains them. `deallocate` consumes the allocation so an
/// allocation cannot be released twice through this interface.
pub trait CellAllocator {
    fn allocate(&self, cells: usize) -> Result<CellAllocation, CellAllocError>;

    fn grow(
        &self,
        allocation: &mut CellAllocation,
        new_capacity_cells: usize,
    ) -> Result<(), CellAllocError>;

    fn shrink(
        &self,
        allocation: &mut CellAllocation,
        new_capacity_cells: usize,
    ) -> Result<(), CellAllocError>;

    fn deallocate(&self, allocation: CellAllocation);
}

/// Safe host-backed allocator used by the Rust bootstrap runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct BootstrapCellAllocator;

impl CellAllocator for BootstrapCellAllocator {
    fn allocate(&self, cells: usize) -> Result<CellAllocation, CellAllocError> {
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(cells)
            .map_err(|_| CellAllocError::CapacityOverflow)?;
        storage.resize(cells, 0);
        Ok(CellAllocation { cells: storage })
    }

    fn grow(
        &self,
        allocation: &mut CellAllocation,
        new_capacity_cells: usize,
    ) -> Result<(), CellAllocError> {
        if new_capacity_cells < allocation.cells.len() {
            return Err(CellAllocError::InvalidShrink);
        }
        let additional = new_capacity_cells - allocation.cells.len();
        allocation
            .cells
            .try_reserve_exact(additional)
            .map_err(|_| CellAllocError::CapacityOverflow)?;
        allocation.cells.resize(new_capacity_cells, 0);
        Ok(())
    }

    fn shrink(
        &self,
        allocation: &mut CellAllocation,
        new_capacity_cells: usize,
    ) -> Result<(), CellAllocError> {
        if new_capacity_cells > allocation.cells.len() {
            return Err(CellAllocError::InvalidShrink);
        }
        allocation.cells.truncate(new_capacity_cells);
        allocation.cells.shrink_to_fit();
        Ok(())
    }

    fn deallocate(&self, allocation: CellAllocation) {
        drop(allocation);
    }
}

/// Allocator-backed dynamic collection for the qualified eight-byte scalar-cell runtime
/// representation. This deliberately stores raw scalar cells, not arbitrary `T`: generic
/// ownership/drop semantics are a v0.2.0+ concern.
pub struct OmniCellVector<A: CellAllocator = BootstrapCellAllocator> {
    allocator: A,
    allocation: CellAllocation,
    len: usize,
}

impl OmniCellVector<BootstrapCellAllocator> {
    pub fn new() -> Self {
        Self::with_allocator(BootstrapCellAllocator)
            .expect("zero-capacity bootstrap allocation must succeed")
    }
}

impl Default for OmniCellVector<BootstrapCellAllocator> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: CellAllocator> OmniCellVector<A> {
    pub fn with_allocator(allocator: A) -> Result<Self, CellAllocError> {
        let allocation = allocator.allocate(0)?;
        Ok(Self {
            allocator,
            allocation,
            len: 0,
        })
    }

    pub fn with_capacity(allocator: A, capacity_cells: usize) -> Result<Self, CellAllocError> {
        let allocation = allocator.allocate(capacity_cells)?;
        Ok(Self {
            allocator,
            allocation,
            len: 0,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.allocation.capacity_cells()
    }

    pub fn get(&self, index: usize) -> Option<u64> {
        (index < self.len).then(|| self.allocation.cells[index])
    }

    pub fn set(&mut self, index: usize, value: u64) -> bool {
        if index >= self.len {
            return false;
        }
        self.allocation.cells[index] = value;
        true
    }

    pub fn push(&mut self, value: u64) -> Result<(), CellAllocError> {
        if self.len == self.capacity() {
            self.reserve(1)?;
        }
        self.allocation.cells[self.len] = value;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<u64> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let value = self.allocation.cells[self.len];
        self.allocation.cells[self.len] = 0;
        Some(value)
    }

    pub fn reserve(&mut self, additional: usize) -> Result<(), CellAllocError> {
        let required = self
            .len
            .checked_add(additional)
            .ok_or(CellAllocError::CapacityOverflow)?;
        if required <= self.capacity() {
            return Ok(());
        }
        let doubled = self.capacity().max(1).checked_mul(2);
        let new_capacity = doubled
            .map(|candidate| candidate.max(required))
            .unwrap_or(required);
        self.allocator.grow(&mut self.allocation, new_capacity)
    }

    pub fn shrink_to_fit(&mut self) -> Result<(), CellAllocError> {
        if self.capacity() == self.len {
            return Ok(());
        }
        self.allocator.shrink(&mut self.allocation, self.len)
    }

    pub fn into_allocation(mut self) -> CellAllocation
    where
        A: Default,
    {
        self.len = 0;
        std::mem::replace(&mut self.allocation, CellAllocation { cells: Vec::new() })
    }
}

/// Minimal runtime wrapper for a dynamic vector used by the bootstrap runtime.
pub struct OmniVector<T>(pub Vec<T>);

impl<T> OmniVector<T> {
    pub fn new() -> Self {
        OmniVector(Vec::new())
    }
    pub fn push(&mut self, v: T) {
        self.0.push(v);
    }
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T> Default for OmniVector<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal runtime wrapper for a hashmap used by the bootstrap runtime.
pub struct OmniHashMap<K, V>(pub std::collections::HashMap<K, V>);

impl<K: std::cmp::Eq + std::hash::Hash, V> OmniHashMap<K, V> {
    pub fn new() -> Self {
        OmniHashMap(std::collections::HashMap::new())
    }
    pub fn insert(&mut self, k: K, v: V) {
        self.0.insert(k, v);
    }
    pub fn get(&self, k: &K) -> Option<&V> {
        self.0.get(k)
    }
    pub fn remove(&mut self, k: &K) -> Option<V> {
        self.0.remove(k)
    }
}

impl<K: std::cmp::Eq + std::hash::Hash, V> Default for OmniHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_alloc_and_get() {
        let mut arena: Arena<i32> = Arena::new();
        let gen = arena.alloc(42);
        assert_eq!(arena.get(&gen), Some(&42));
    }

    #[test]
    fn test_arena_reuse_after_release() {
        let mut arena: Arena<i32> = Arena::new();
        let gen1 = arena.alloc(1);
        let index = gen1.index();
        arena.release(gen1);

        let gen2 = arena.alloc(2);
        assert_eq!(gen2.index(), index);
        assert!(gen2.generation() != gen1.generation());
    }

    #[test]
    fn test_arena_double_release_is_rejected() {
        let mut arena: Arena<i32> = Arena::new();
        let gen = arena.alloc(42);
        assert!(arena.release(gen));
        assert!(!arena.release(gen));
        assert_eq!(arena.len(), 0);

        let reused = arena.alloc(7);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.get(&reused), Some(&7));
    }

    #[test]
    fn test_gen_validation() {
        let mut arena: Arena<String> = Arena::new();
        let _gen = arena.alloc("hello".to_string());
        arena.alloc("world".to_string());
        let gen = arena.alloc("test".to_string());
        arena.release(gen);

        assert!(arena.get(&gen).is_none());
    }

    #[test]
    fn test_slot_map() {
        let mut map: SlotMap<String> = SlotMap::new();
        let key1 = map.insert("value1".to_string());
        let key2 = map.insert("value2".to_string());

        assert_eq!(map.get(key1), Some(&"value1".to_string()));
        assert_eq!(map.get(key2), Some(&"value2".to_string()));

        assert_eq!(map.remove(key1), Some("value1".to_string()));
        assert!(map.get(key1).is_none());
    }

    #[test]
    fn test_omni_vector_and_hashmap() {
        let mut v = OmniVector::new();
        v.push(1);
        v.push(2);
        assert_eq!(v.len(), 2);
        assert_eq!(v.pop(), Some(2));

        let mut m: OmniHashMap<String, i32> = OmniHashMap::new();
        m.insert("a".to_string(), 10);
        assert_eq!(m.get(&"a".to_string()), Some(&10));
        assert_eq!(m.remove(&"a".to_string()), Some(10));
    }

    #[test]
    fn cell_allocator_grow_shrink_preserves_prefix() {
        let allocator = BootstrapCellAllocator;
        let mut block = allocator.allocate(2).expect("allocate");
        block.as_mut_slice().copy_from_slice(&[40, 2]);
        allocator.grow(&mut block, 5).expect("grow");
        assert_eq!(&block.as_slice()[..2], &[40, 2]);
        assert_eq!(&block.as_slice()[2..], &[0, 0, 0]);
        allocator.shrink(&mut block, 2).expect("shrink");
        assert_eq!(block.as_slice(), &[40, 2]);
        allocator.deallocate(block);
    }

    #[test]
    fn cell_vector_uses_checked_capacity_growth_and_bounds() {
        let mut vector = OmniCellVector::new();
        assert_eq!(vector.capacity(), 0);
        vector.push(40).expect("push 40");
        vector.push(2).expect("push 2");
        assert_eq!(vector.len(), 2);
        assert!(vector.capacity() >= 2);
        assert_eq!(vector.get(0), Some(40));
        assert_eq!(vector.get(1), Some(2));
        assert_eq!(vector.get(2), None);
        assert!(!vector.set(2, 99));
        assert!(vector.set(1, 3));
        assert_eq!(vector.pop(), Some(3));
        assert_eq!(vector.pop(), Some(40));
        assert_eq!(vector.pop(), None);
    }

    #[test]
    fn cell_vector_reserve_and_shrink_are_deterministic() {
        let mut vector = OmniCellVector::new();
        vector.reserve(3).expect("reserve");
        assert!(vector.capacity() >= 3);
        vector.push(7).expect("push");
        vector.push(42).expect("push");
        vector.shrink_to_fit().expect("shrink");
        assert_eq!(vector.capacity(), 2);
        assert_eq!(vector.get(0), Some(7));
        assert_eq!(vector.get(1), Some(42));
    }

    #[derive(Clone, Copy)]
    struct FailingGrowAllocator;

    impl CellAllocator for FailingGrowAllocator {
        fn allocate(&self, cells: usize) -> Result<CellAllocation, CellAllocError> {
            Ok(CellAllocation {
                cells: vec![0; cells],
            })
        }

        fn grow(
            &self,
            _allocation: &mut CellAllocation,
            _new_capacity_cells: usize,
        ) -> Result<(), CellAllocError> {
            Err(CellAllocError::CapacityOverflow)
        }

        fn shrink(
            &self,
            allocation: &mut CellAllocation,
            new_capacity_cells: usize,
        ) -> Result<(), CellAllocError> {
            if new_capacity_cells > allocation.cells.len() {
                return Err(CellAllocError::InvalidShrink);
            }
            allocation.cells.truncate(new_capacity_cells);
            Ok(())
        }

        fn deallocate(&self, allocation: CellAllocation) {
            drop(allocation);
        }
    }

    #[test]
    fn failed_collection_growth_preserves_existing_state() {
        let mut vector = OmniCellVector::with_capacity(FailingGrowAllocator, 1).expect("allocate");
        vector.push(42).expect("initial push");
        assert_eq!(vector.push(7), Err(CellAllocError::CapacityOverflow));
        assert_eq!(vector.len(), 1);
        assert_eq!(vector.capacity(), 1);
        assert_eq!(vector.get(0), Some(42));
    }
}
