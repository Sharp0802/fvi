//! A doubly linked list implementation based on slab container.

use core::iter::FusedIterator;
use core::ops::{Index, IndexMut};
use std::vec::Vec;

#[derive(Debug, Clone)]
struct Slot<T> {
    next: usize,
    prev: usize,
    value: Option<T>,
}

/// A doubly linked list implementation based on slab container.
#[derive(Debug, Clone)]
pub struct List<T> {
    vec: Vec<Slot<T>>,
    //     empty:   free == start == end
    // non-empty: vec[start].prev == start
    //              vec[end].next == free
    free: usize,
    start: usize,
    end: usize,
}

impl<T> List<T> {
    /// Creates a new [`List`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vec: Vec::new(),
            free: 0,
            start: 0,
            end: 0,
        }
    }

    fn link(&mut self, prev: usize, next: usize) {
        if let Some(prev) = self.vec.get_mut(prev) {
            prev.next = next;
        }
        if let Some(next) = self.vec.get_mut(next) {
            next.prev = prev;
        }
    }

    /// Returns whether `self` is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.free == self.start
    }

    /// Clears `self`.
    ///
    /// Note that allocated capacity isn't affected.
    pub fn clear(&mut self) {
        self.vec.clear();
        self.free = 0;
        self.start = 0;
        self.end = 0;
    }

    /// Pops an item out from start of `self`,
    /// returning `None` if `self` is empty.
    pub fn pop(&mut self) -> Option<T> {
        self.remove(self.start)
    }

    /// Peeks an item from start of `self`,
    /// returning `None` if `self` is empty.
    #[must_use]
    pub fn peek(&self) -> Option<&T> {
        self.get(self.start)
    }

    /// Pushes given item at end of `self`,
    /// returning the index of inserted item.
    pub fn push(&mut self, value: T) -> usize {
        let id = self.free;

        if let Some(slot) = self.vec.get_mut(id) {
            debug_assert!(slot.value.is_none());
            slot.value = Some(value);
            self.free = slot.next;
        } else {
            self.free += 1;
            self.vec.push(Slot {
                prev: self.end,
                next: self.free,
                value: Some(value),
            });
        }

        self.end = id;
        id
    }

    /// Removes a specified item from `self` and returns it,
    /// returning `None` if there is no such item.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        let slot = self.vec.get_mut(index)?;
        let ret = slot.value.take()?;
        let prev = slot.prev;
        let next = slot.next;

        let is_end = self.end == index;
        let is_start = self.start == index;

        self.link(index, self.free);
        self.free = index;

        if is_end && is_start {
            // free == start == end
        } else if is_end {
            self.end = prev;
        } else if is_start {
            self.start = next;
            self.vec[next].prev = next;
            self.link(self.end, index);
        } else {
            self.link(prev, next);
            self.link(self.end, index);
        }

        Some(ret)
    }

    /// Moves a specified item to end of `self`.
    ///
    /// # Panics
    ///
    /// Panics if there is no such item.
    pub fn set_end(&mut self, index: usize) {
        let item = self.remove(index).unwrap();
        let new_index = self.push(item);
        debug_assert_eq!(new_index, index);
    }

    /// Returns a reference to specified item from `self`,
    /// returning `None` if there is no such item.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.vec.get(index)?.value.as_ref()
    }

    /// Returns a mutable reference to specified item from `self`,
    /// returning `None` if there is no such item.
    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.vec.get_mut(index)?.value.as_mut()
    }

    /// Iterates on items of `self` in insertion order.
    #[must_use]
    pub const fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<usize> for List<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<T> IndexMut<usize> for List<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: PartialEq> PartialEq for List<T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other)
    }
}

impl<T: Eq> Eq for List<T> {}

/// An iterator iterating items of [`List`] in list order.
#[derive(Debug, Clone)]
pub struct Iter<'a, T> {
    list: &'a List<T>,
    front: usize,
    back: usize,
    done: bool,
}

impl<'a, T> Iter<'a, T> {
    const fn new(list: &'a List<T>) -> Self {
        Self {
            list,
            front: list.start,
            back: list.end,
            done: list.is_empty(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.front == self.back {
            self.done = true;
        }

        let slot = &self.list.vec[self.front];
        let item = slot.value.as_ref().unwrap();
        self.front = slot.next;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done {
            (0, Some(0))
        } else {
            (1, Some(self.list.vec.len()))
        }
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.front == self.back {
            self.done = true;
        }

        let slot = &self.list.vec[self.back];
        let item = slot.value.as_ref().unwrap();
        self.back = slot.prev;
        Some(item)
    }
}

impl<T> FusedIterator for Iter<'_, T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;
    use std::vec;

    #[derive(Debug, Clone)]
    enum Op {
        Push(i32),
        Pop,
        RemoveExisting(u8),
        RemoveRaw(u8),
        SetEnd(u8),
        Clear,
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            5 => any::<i32>().prop_map(Op::Push),
            2 => Just(Op::Pop),
            3 => any::<u8>().prop_map(Op::RemoveExisting),
            2 => any::<u8>().prop_map(Op::RemoveRaw),
            2 => any::<u8>().prop_map(Op::SetEnd),
            1 => Just(Op::Clear),
        ]
    }

    fn assert_consistent(list: &List<i32>, model: &[(usize, i32)]) -> TestCaseResult {
        // Public behavior.
        prop_assert_eq!(list.is_empty(), model.is_empty());
        prop_assert_eq!(list.peek().copied(), model.first().map(|(_, value)| *value));
        let expected = model.iter().map(|(_, value)| *value).collect::<Vec<_>>();
        prop_assert_eq!(list.iter().copied().collect::<Vec<_>>(), expected);

        let expected_rev = model
            .iter()
            .rev()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>();

        prop_assert_eq!(list.iter().rev().copied().collect::<Vec<_>>(), expected_rev);

        // Every model index points at the expected value.
        for &(index, value) in model {
            prop_assert_eq!(list.get(index), Some(&value));
        }

        // Every occupied slab slot occurs in the model, and vice versa.
        for (index, slot) in list.vec.iter().enumerate() {
            let expected = model
                .iter()
                .find(|(id, _)| *id == index)
                .map(|(_, value)| value);

            prop_assert_eq!(slot.value.as_ref(), expected);
        }

        // Active-list invariants.
        if model.is_empty() {
            prop_assert_eq!(list.free, list.start);
            prop_assert_eq!(list.start, list.end);
        } else {
            prop_assert_eq!(list.start, model[0].0);
            prop_assert_eq!(list.end, model.last().unwrap().0);
            prop_assert_eq!(list.vec[list.start].prev, list.start);
            prop_assert_eq!(list.vec[list.end].next, list.free);

            for pair in model.windows(2) {
                let prev = pair[0].0;
                let next = pair[1].0;

                prop_assert_eq!(list.vec[prev].next, next);
                prop_assert_eq!(list.vec[next].prev, prev);
            }
        }

        // The free chain must:
        //
        // - contain only vacant slots;
        // - contain every vacant slot exactly once;
        // - terminate at vec.len();
        let mut seen = vec![false; list.vec.len()];
        let mut current = list.free;

        while current < list.vec.len() {
            prop_assert!(!seen[current], "cycle in free list at slot {current}");

            seen[current] = true;

            prop_assert!(
                list.vec[current].value.is_none(),
                "occupied slot {current} occurs in free list",
            );

            current = list.vec[current].next;
        }

        prop_assert_eq!(current, list.vec.len(), "free list has invalid terminator");

        for (index, slot) in list.vec.iter().enumerate() {
            if slot.value.is_none() {
                prop_assert!(seen[index], "vacant slot {index} is missing from free list");
            } else {
                prop_assert!(!seen[index], "occupied slot {index} occurs in free list");
            }
        }

        Ok(())
    }

    proptest! {
        #[test]
        fn test_operations(
            operations in prop::collection::vec(op(), 0..500),
        ) {
            let mut list = List::new();
            let mut model: Vec<(usize, i32)> = Vec::new();

            assert_consistent(&list, &model)?;

            for operation in operations {
                match operation {
                    Op::Push(value) => {
                        let index = list.push(value);
                        prop_assert!(model.iter().all(|(id, _)| *id != index));
                        model.push((index, value));
                    }

                    Op::Pop => {
                        let expected = if model.is_empty() {
                            None
                        } else {
                            Some(model.remove(0).1)
                        };

                        prop_assert_eq!(list.pop(), expected);
                    }

                    Op::RemoveExisting(selector) => {
                        if !model.is_empty() {
                            let position = selector as usize % model.len();
                            let (index, value) = model.remove(position);

                            prop_assert_eq!(
                                list.remove(index),
                                Some(value),
                            );
                        }
                    }

                    Op::RemoveRaw(index) => {
                        let index = index as usize;
                        let expected = model
                            .iter()
                            .position(|(id, _)| *id == index)
                            .map(|position| model.remove(position).1);

                        prop_assert_eq!(
                            list.remove(index),
                            expected,
                        );
                    }

                    Op::SetEnd(selector) => {
                        if !model.is_empty() {
                            let position = selector as usize % model.len();
                            let item = model.remove(position);

                            list.set_end(item.0);
                            model.push(item);
                        }
                    }

                    Op::Clear => {
                        list.clear();
                        model.clear();
                    }
                }

                assert_consistent(&list, &model)?;
            }
        }
    }

    proptest! {
        #[test]
        fn test_double_ended_iter(
            values in prop::collection::vec(any::<i32>(), 0..100),
            directions in prop::collection::vec(any::<bool>(), 0..200),
        ) {
            use std::collections::VecDeque;

            let mut list = List::new();

            for &value in &values {
                list.push(value);
            }

            let mut expected: VecDeque<_> = values.into_iter().collect();
            let mut iter = list.iter();

            for from_front in directions {
                let actual = if from_front {
                    iter.next().copied()
                } else {
                    iter.next_back().copied()
                };

                let wanted = if from_front {
                    expected.pop_front()
                } else {
                    expected.pop_back()
                };

                prop_assert_eq!(actual, wanted);

                let (lower, upper) = iter.size_hint();

                prop_assert!(lower <= expected.len());

                if let Some(upper) = upper {
                    prop_assert!(expected.len() <= upper);
                }
            }

            while let Some(expected) = expected.pop_front() {
                prop_assert_eq!(iter.next(), Some(&expected));
            }

            // Iterator is fused.
            prop_assert_eq!(iter.next(), None);
            prop_assert_eq!(iter.next(), None);
            prop_assert_eq!(iter.next_back(), None);
            prop_assert_eq!(iter.next_back(), None);
            prop_assert_eq!(iter.size_hint(), (0, Some(0)));
        }
    }

    proptest! {
        #[test]
        fn test_eq(
            a in prop::collection::vec(any::<i32>(), 0..100),
            b in prop::collection::vec(any::<i32>(), 0..100),
        ) {
            let mut list_a = List::new();
            let mut list_b = List::new();

            for &value in &a {
                list_a.push(value);
            }

            for &value in &b {
                list_b.push(value);
            }

            prop_assert_eq!(list_a == list_b, a == b);
        }
    }
}
