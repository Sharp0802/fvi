use crate::alloc::NIL;
use crate::alloc::slot::Slot;
use crate::alloc::std::vec::Vec;

/// A slab container.
#[derive(Debug, Clone)]
pub struct Slab<T> {
    free: usize,
    slots: Vec<Slot<T>>,
}

const _: () = const {
    assert!(size_of::<Slab<u32>>() == 32);
    assert!(size_of::<Slab<u64>>() == 32);
};

impl<T> Slab<T> {
    /// Creates a new [`Slab<T>`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            free: NIL,
            slots: Vec::new(),
        }
    }

    /// Returns the length of this [`Slab<T>`].
    #[must_use]
    pub const fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns the emptiness of this [`Slab<T>`].
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Inserts given value into this [`Slab<T>`].
    #[must_use]
    pub fn insert(&mut self, value: T) -> usize {
        if self.free == NIL {
            let key = self.len();
            let mut slot = Slot::new();
            slot.write(value);
            self.slots.push(slot);
            key
        } else {
            let key = self.free;
            self.free = self.slots[key].next();
            self.slots[key].write(value);
            key
        }
    }

    /// Removes an item at specified key from this [`Slab<T>`].
    #[must_use]
    pub fn remove(&mut self, key: usize) -> Option<T> {
        let val = self.slots[key].take()?;
        self.slots[key].link(self.free);
        self.free = key;
        Some(val)
    }

    /// Returns a reference of the item at specified key.
    #[must_use]
    pub const fn get(&self, key: usize) -> Option<&T> {
        self.slots.as_slice()[key].as_ref()
    }

    /// Returns a mutable reference of the item at specified key.
    #[must_use]
    pub const fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        self.slots.as_mut_slice()[key].as_mut()
    }
}
