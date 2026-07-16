use core::iter::FusedIterator;

use crate::collections::treap::Treap;
use crate::piece::Piece;

/// An iterator over visible pieces in [`Treap`].
#[derive(Debug)]
pub struct Iter<'a> {
    treap: &'a Treap,
    version: u32,
    cur: usize,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(super) const fn new(treap: &'a Treap, leftmost: usize, version: u32) -> Self {
        Self {
            treap,
            version,
            cur: leftmost,
        }
    }
}

impl Iterator for Iter<'_> {
    type Item = Piece;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let piece = self.treap.slab.get(self.cur)?.val;
            self.cur = self.treap.next(self.cur);
            if piece.add_at <= self.version && self.version < piece.del_at {
                return Some(piece);
            }
        }
    }
}

impl FusedIterator for Iter<'_> {}
