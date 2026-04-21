#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Generation(u32);

impl Generation {
    pub fn new() -> Self {
        Generation(0)
    }
    pub fn increment(self) -> Self {
        Generation(self.0 + 1)
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

impl<T> Default for Gen<T> {
    fn default() -> Self {
        Gen::new(usize::MAX, Generation::new())
    }
}

impl<T> Clone for Gen<T> {
    fn clone(&self) -> Self {
        Gen {
            index: self.index,
            generation: self.generation,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> Copy for Gen<T> {}

pub struct Arena<T> {
    items: Vec<Option<T>>,
    generations: Vec<Generation>,
    free_list: Vec<usize>,
    next_generation: Generation,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Arena {
            items: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
            next_generation: Generation::new(),
        }
    }

    pub fn alloc(&mut self, value: T) -> Gen<T> {
        let index = if let Some(free_idx) = self.free_list.pop() {
            self.items[free_idx] = Some(value);
            self.generations[free_idx] = self.next_generation;
            free_idx
        } else {
            let idx = self.items.len();
            self.items.push(Some(value));
            self.generations.push(self.next_generation);
            idx
        };

        let gen = self.next_generation;
        self.next_generation = self.next_generation.increment();

        Gen::new(index, gen)
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
        self.items.len() - self.free_list.len()
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
        let key = self.next_key;
        self.next_key = self.next_key.wrapping_add(1);

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
}
