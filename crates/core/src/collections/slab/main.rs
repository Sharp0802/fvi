use core::ops::{Index, IndexMut};
use std::vec::Vec;

use crate::collections::slab::slot::Slot;

/// A stable vector based on slab allocation.
#[derive(Debug, Clone)]
pub struct Slab<T> {
    len: usize,
    free: usize,
    slots: Vec<Slot<T>>,
}

impl<T> Slab<T> {
    /// Creates a new [`Slab<T>`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            len: 0,
            free: 0,
            slots: Vec::new(),
        }
    }

    /// Returns the total number of slots that
    /// this [`Slab<T>`] can hold without reallocating.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.slots.capacity()
    }

    /// Returns the length of this [`Slab<T>`].
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns the emptiness of this [`Slab<T>`].
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Inserts given value into this [`Slab<T>`].
    #[must_use]
    pub fn insert(&mut self, value: T) -> usize {
        let key = self.free;

        if self.free == self.slots.len() {
            let slot = Slot::new(key + 1, value);
            self.slots.push(slot);
        } else {
            self.slots[key].write(value);
        }

        self.free = self.slots[key].next();
        self.len += 1;

        key
    }

    /// Removes an item at specified key from this [`Slab<T>`].
    #[must_use]
    pub fn remove(&mut self, key: usize) -> Option<T> {
        let val = self.slots.get_mut(key)?.take()?;
        self.slots[key].link(self.free);
        self.free = key;
        self.len -= 1;
        Some(val)
    }

    /// Returns a reference of the item at specified key.
    #[must_use]
    pub const fn get(&self, key: usize) -> Option<&T> {
        if key < self.slots.len() {
            self.slots.as_slice()[key].as_ref()
        } else {
            None
        }
    }

    /// Returns a mutable reference of the item at specified key.
    #[must_use]
    pub const fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if key < self.slots.len() {
            self.slots.as_mut_slice()[key].as_mut()
        } else {
            None
        }
    }
}

impl<T> Index<usize> for Slab<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

impl<T> IndexMut<usize> for Slab<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    #[derive(Debug, Clone, Copy)]
    enum Op {
        Insert(usize),
        Remove(usize),
    }

    type Mock = Vec<Option<usize>>;
    type Actual = Slab<usize>;

    fn apply(op: Op, mock: &mut Mock, actual: &mut Actual) -> TestCaseResult {
        match op {
            Op::Insert(val) => {
                let key = actual.insert(val);
                if key < mock.len() {
                    prop_assert!(mock[key].is_none());
                    mock[key] = Some(val);
                } else {
                    prop_assert_eq!(key, mock.len());
                    mock.push(Some(val));
                }

                prop_assert_eq!(actual.get(key), mock[key].as_ref());
            }
            Op::Remove(at) => {
                if mock.is_empty() {
                    prop_assert!(actual.is_empty());
                } else {
                    let at = at % mock.len();
                    prop_assert_eq!(actual.remove(at), mock[at].take());
                }
            }
        }

        Ok(())
    }

    fn arb_op(max_len: usize) -> impl Strategy<Value = Vec<Op>> {
        vec(
            prop_oneof![
                any::<usize>().prop_map(Op::Insert),
                any::<usize>().prop_map(Op::Remove)
            ],
            1..=max_len,
        )
    }

    proptest! {
        #[test]
        fn match_models(ops in arb_op(256)) {
            let mut mock = Mock::default();
            let mut actual = Actual::default();

            for op in ops {
                apply(op, &mut mock, &mut actual)?;
            }
        }
    }

    #[test]
    fn exhaustive() {
        let slab = Actual::new();
        assert_eq!(slab.capacity(), 0);
        assert_eq!(slab.len(), 0);
        assert!(slab.is_empty());
    }
}
