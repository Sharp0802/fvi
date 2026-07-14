use core::iter::FusedIterator;

use crate::piece::{Pieces, Ptr};
use crate::view::View;

/// An in-order iterator over pieces in [`Pieces`].
pub struct Iter<'a> {
    raw: &'a Pieces,
    cur: Ptr,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(super) fn new(raw: &'a Pieces, root: Ptr) -> Self {
        let cur = raw.leftmost(root);
        Self { raw, cur }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a View;

    fn next(&mut self) -> Option<&'a View> {
        let cur = self.cur;
        let (node, _) = self.raw.get(cur)?;

        let ret = &node.desc;

        if node.rhs.is_nil() {
            let mut x = cur;
            let mut p = node.prv;

            while let Some((parent, _)) = self.raw.get(p) {
                if parent.lhs == x {
                    break;
                }

                x = p;
                p = parent.prv;
            }

            self.cur = p;
        } else {
            self.cur = self.raw.leftmost(node.rhs);
        }

        Some(ret)
    }
}

impl FusedIterator for Iter<'_> {}
