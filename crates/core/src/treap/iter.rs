use core::marker::PhantomData;
use core::ptr::NonNull;
use core::{iter::FusedIterator, ops::Deref};

use crate::piece::Piece;
use crate::treap::{NIL, Treap};

/// An in-order iterator over pieces in [`Treap`].
pub struct Iter<'a> {
    raw: &'a mut Treap,
    cur: usize,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(super) const fn new(raw: &'a mut Treap, root: usize) -> Self {
        let cur = raw.leftmost(root);
        Self { raw, cur }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.cur;
        if cur == NIL {
            return None;
        }

        self.cur = self.raw.next(cur);

        #[expect(unsafe_code, reason = "bounded by 'a")]
        Some(unsafe { Item::<'a>::new(self.raw, cur) })
    }
}

impl FusedIterator for Iter<'_> {}

/// Killable wrapper of an iterated piece.
#[derive(Debug)]
pub struct Item<'a> {
    treap: NonNull<Treap>,
    at: usize,
    _dummy: PhantomData<&'a mut Treap>,
}

impl Item<'_> {
    #[must_use]
    #[expect(unsafe_code, reason = "cannot assume treap has valid lifetime")]
    const unsafe fn new(treap: &mut Treap, at: usize) -> Self {
        Self {
            treap: NonNull::from_mut(treap),
            at,
            _dummy: PhantomData,
        }
    }

    /// Remove item permanently from treap.
    pub fn kill(mut self) {
        #[expect(unsafe_code, reason = "bounded by 'a")]
        let treap = unsafe { self.treap.as_mut() };
        treap.kill(self.at);
    }
}

impl Deref for Item<'_> {
    type Target = Piece;

    fn deref(&self) -> &Self::Target {
        #[expect(unsafe_code, reason = "bounded by 'a")]
        let treap = unsafe { self.treap.as_ref() };
        &treap.slab[self.at].val
    }
}

impl From<Item<'_>> for Piece {
    fn from(value: Item<'_>) -> Self {
        *value
    }
}
