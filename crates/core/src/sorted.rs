//! A module providing sorted collections.

use std::vec::Vec;

use crate::view::ViewSize;

#[must_use]
const fn find(vec: &[u16], val: u16) -> usize {
    let i = vec.len() / 2;

    let (l, r) = vec.split_at(i);

    let lv = match l.last().copied() {
        Some(lv) => lv,
        None => 0,
    };
    let rv = match r.first().copied() {
        Some(rv) => rv,
        None => 0,
    };

    match (lv, rv) {
        (lv, _) if val < lv => find(l, val),
        (_, rv) if rv < val => find(r, val) + i,
        _ => i,
    }
}

/// A sorted vector storage.
#[derive(Debug)]
pub struct SortedVec {
    vec: Vec<u16>,
}

impl SortedVec {
    /// Creates a new [`SortedVec`] with given capacity.
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: ViewSize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity.get() as usize),
        }
    }

    /// Returns whether the `self` is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of items in `self`.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.vec.len()
    }

    /// Inserts new item to `self`.
    ///
    /// # Panics
    ///
    /// Panics if given value is not greater than past value.
    #[inline]
    pub fn push(&mut self, val: u16) {
        assert!(self.vec.len() < ViewSize::MAX);
        assert!(self.vec.last().is_none_or(|&old| old < val));
        self.vec.push(val);
    }

    #[inline]
    #[must_use]
    const fn find(&self, val: u16) -> usize {
        find(self.vec.as_slice(), val)
    }

    /// Counts number of items in inclusive range `beg..=end`.
    #[inline]
    #[must_use]
    pub const fn count(&self, beg: u16, end: u16) -> usize {
        self.find(end) - self.find(beg)
    }
}
