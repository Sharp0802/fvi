use core::iter::FusedIterator;

use crate::collections::treap::Treap;
use crate::piece::Piece;

/// An in-order iterator over visible pieces in [`Treap`].
#[derive(Debug)]
pub struct Iter<'a> {
    treap: &'a Treap,
    version: u32,
    cur: usize,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(super) const fn new(treap: &'a Treap, root: usize, version: u32) -> Self {
        let cur = treap.leftmost(root);
        Self {
            treap,
            version,
            cur,
        }
    }
}

impl Iterator for Iter<'_> {
    type Item = Piece;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let piece = self.treap.slab.get(self.cur)?.val;
            let add_at = piece.add_version;
            let del_at = piece.del_version;

            if add_at <= self.version && self.version < del_at {
                return Some(piece);
            }

            self.cur = self.treap.next(self.cur);
        }
    }
}

impl FusedIterator for Iter<'_> {}

/// An in-order iterator over all pieces in [`Treap`].
#[derive(Debug)]
pub struct IterAll<'a> {
    treap: &'a Treap,
    cur: usize,
}

impl<'a> IterAll<'a> {
    #[inline]
    pub(super) const fn new(treap: &'a Treap, root: usize) -> Self {
        let cur = treap.leftmost(root);
        Self { treap, cur }
    }
}

impl Iterator for IterAll<'_> {
    type Item = Piece;

    fn next(&mut self) -> Option<Self::Item> {
        let piece = self.treap.slab.get(self.cur)?.val;
        self.cur = self.treap.next(self.cur);
        Some(piece)
    }
}

impl FusedIterator for IterAll<'_> {}
