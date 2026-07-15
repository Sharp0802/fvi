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
                    prop_assert!(mock[val].is_none());
                    mock[val] = Some(val);
                } else {
                    prop_assert_eq!(key, mock.len());
                    mock.push(Some(val));
                }
            }
            Op::Remove(at) => {
                if !mock.is_empty() {
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
            let mut mock = Mock::new();
            let mut actual = Actual::new();

            for op in ops {
                apply(op, &mut mock, &mut actual)?;
            }
        }
    }
}
