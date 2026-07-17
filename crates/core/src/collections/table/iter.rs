use core::iter::FusedIterator;

use crate::collections::table::{Table, Version};
use crate::piece::Piece;

/// An iterator over visible pieces in [`Table`].
#[derive(Debug)]
pub struct Iter<'a> {
    treap: &'a Table,
    version: Version,
    cur: usize,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(super) fn new(treap: &'a Table, leftmost: usize, version: Version) -> Self {
        debug_assert!(
            treap
                .slab
                .get(leftmost)
                .is_none_or(|t| t.val.is_visible_at(version))
        );

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
            self.cur = self.treap.next_visible(self.cur, self.version);
            if piece.add_at <= self.version && piece.del_at.is_none_or(|t| self.version < t) {
                return Some(piece);
            }
        }
    }
}

impl FusedIterator for Iter<'_> {}
