use core::cmp::Ordering;

use crate::collections::Slab;
use crate::collections::treap::node::Node;
use crate::collections::treap::{Iter, NIL};
use crate::math::xorshift;
use crate::piece::{Piece, PieceDesc};

/// A low-level piece table implemented with an implicit treap.
#[derive(Debug, Clone)]
pub struct Treap {
    root: usize,
    salt: usize,
    pub(super) slab: Slab<Node>,
}

impl Treap {
    /// Creates a new piece treap with given salt constant.
    #[inline]
    #[must_use]
    pub const fn new(salt: usize) -> Self {
        Self {
            root: NIL,
            salt,
            slab: Slab::new(),
        }
    }

    /// Returns the total number of slots that
    /// this [`Treap`] can hold without reallocating.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.slab.capacity()
    }

    /// Returns the total number of nodes,
    /// including deleted ones.
    #[inline]
    #[must_use]
    pub const fn capacity_used(&self) -> usize {
        self.slab.len()
    }

    /// Returns true if total byte length is zero;
    /// Otherwise, returns false.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns total byte length.
    #[inline]
    #[must_use]
    pub fn len(&self) -> u64 {
        self.len_of(self.root)
    }

    /// Returns in-order traversal iterator for specified version.
    #[inline]
    #[must_use]
    pub const fn iter(&self, version: u32) -> Iter<'_> {
        Iter::new(self, self.root, version)
    }

    /// Inserts a piece desc at given byte offset.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Specified version is [`u32::MAX`].
    /// - Given offset is not in this [`Treap`].
    pub fn insert(&mut self, off: u64, desc: PieceDesc, version: u32) {
        assert!(version != u32::MAX);
        assert!(off <= self.len());

        let (lhs, rhs) = self.split(self.root, off);
        let mid = self.new_leaf(NIL, desc.build(version));
        let tmp = self.concat(lhs, mid);
        self.root = self.concat(tmp, rhs);
    }

    /// Removes a range from the treap, returning index of removed node.
    ///
    /// # Panics
    ///
    /// Panics if specified version is [`u32::MAX`].
    pub fn remove(&mut self, start: u64, end: u64, version: u32) -> Option<usize> {
        assert!(version != u32::MAX);

        match end.cmp(&self.len()) {
            Ordering::Greater => None,
            Ordering::Equal => {
                let (rst, rhs) = self.split(self.root, start);
                self.slab[rhs].val.del_version = version;
                self.root = self.merge(rst, rhs);
                Some(rhs)
            }
            Ordering::Less => {
                let (rst, rhs) = self.split(self.root, end);
                let (lhs, mid) = self.split(rst, start);
                self.slab[mid].val.del_version = version;

                let root = self.merge(lhs, mid);
                let root = self.merge(root, rhs);
                self.root = root;

                Some(mid)
            }
        }
    }

    /// Splits this [`Treap`] at given byte offset.
    ///
    /// # Panics
    ///
    /// Panics if given offset is not in this [`Treap`].
    #[must_use]
    pub fn split_off(&mut self, off: u64) -> Self {
        assert!(off <= self.len());

        if off == 0 {
            core::mem::replace(self, Self::new(xorshift(self.salt)))
        } else if off == self.len() {
            Self::new(xorshift(self.salt))
        } else {
            let (lhs, rhs) = self.split(self.root, off);
            let mut cloned = self.clone();
            self.root = lhs;
            self.kill(rhs);
            cloned.root = rhs;
            cloned.kill(lhs);
            cloned
        }
    }

    /// Prunes nodes that is invalid in given version range.
    pub fn prune(&mut self, min: u32, max: u32) {
        let mut cur = self.root;
        while cur != NIL {
            let next = self.next(cur);

            let node = self.slab[cur];
            if node.val.del_version < min || max < node.val.add_version {
                self.kill(cur);
            } else if max < node.val.del_version {
                self.slab[cur].val.del_version = u32::MAX;
                self.propagate(cur);
            } else if node.val.add_version < min {
                self.slab[cur].val.add_version = min;
            }

            cur = next;
        }
    }
}

impl Treap {
    #[must_use]
    const fn pri_of(&self, at: usize) -> usize {
        xorshift(self.salt ^ at)
    }

    #[must_use]
    fn len_of(&self, at: usize) -> u64 {
        self.slab.get(at).map_or(0, |t| t.len)
    }

    fn update(&mut self, at: usize) {
        let Some(v) = self.slab.get(at) else { return };
        if v.val.del_version == u32::MAX {
            let len = self.len_of(v.lhs) + v.val.len() + self.len_of(v.rhs);
            self.slab[at].len = len;
        } else {
            self.slab[at].len = 0;
        }
    }

    pub(crate) fn propagate(&mut self, mut at: usize) {
        while let Some(v) = self.slab.get(at) {
            let prv = v.prv;
            self.update(at);
            at = prv;
        }
    }

    fn kill(&mut self, t: usize) {
        let Some(v) = self.slab.get(t) else { return };

        if let Some(pv) = self.slab.get_mut(v.prv) {
            if pv.lhs == t {
                pv.lhs = NIL;
            } else {
                pv.rhs = NIL;
            }

            self.propagate(t);
        }

        self.kill_impl(t);
    }

    #[must_use]
    fn new_leaf(&mut self, prv: usize, value: Piece) -> usize {
        let mut node: Node = value.into();
        node.prv = prv;
        self.slab.insert(node)
    }

    fn kill_impl(&mut self, t: usize) {
        let Some(v) = self.slab.get(t) else { return };
        let (lhs, rhs) = (v.lhs, v.rhs);
        self.kill_impl(lhs);
        self.kill_impl(rhs);
        _ = self.slab.remove(t);
    }

    #[must_use]
    fn split(&mut self, at: usize, pos: u64) -> (usize, usize) {
        let Some(v) = self.slab.get(at) else {
            return (NIL, NIL);
        };

        let l_len = self.len_of(v.lhs);
        let d_len = v.val.len();

        match pos.checked_sub(l_len) {
            None | Some(0) => {
                let (a, b) = self.split(v.lhs, pos);
                self.slab[at].lhs = b;
                self.update(at);

                if let Some(av) = self.slab.get_mut(a) {
                    av.prv = NIL;
                }

                if let Some(bv) = self.slab.get_mut(b) {
                    bv.prv = at;
                }

                (a, at)
            }
            Some(mid) if d_len > mid => {
                let (l_desc, r_desc) = v.val.split_at(mid);

                let rhs_old = v.rhs;
                if let Some(rv) = self.slab.get_mut(rhs_old) {
                    rv.prv = NIL;
                }

                let rhs_new = self.new_leaf(NIL, r_desc);
                let rhs_new = self.merge(rhs_new, rhs_old);

                let node = &mut self.slab[at];
                node.val = l_desc;
                node.prv = NIL;
                node.rhs = NIL;

                self.update(at);

                (at, rhs_new)
            }
            Some(mid) => {
                let next_pos = mid - d_len;

                let (a, b) = self.split(v.rhs, next_pos);
                self.slab[at].rhs = a;
                self.update(at);

                if let Some(av) = self.slab.get_mut(a) {
                    av.prv = at;
                }

                if let Some(bv) = self.slab.get_mut(b) {
                    bv.prv = NIL;
                }

                (at, b)
            }
        }
    }

    #[must_use]
    fn merge(&mut self, a: usize, b: usize) -> usize {
        match (self.slab.get(a), self.slab.get(b)) {
            (None, _) => b,
            (_, None) => a,
            (Some(_), Some(_)) if self.pri_of(a) > self.pri_of(b) => {
                let rhs = self.merge(self.slab[a].rhs, b);
                self.slab[a].rhs = rhs;
                self.update(a);

                if let Some(rv) = self.slab.get_mut(rhs) {
                    rv.prv = a;
                }

                a
            }
            (Some(_), Some(_)) => {
                let lhs = self.merge(a, self.slab[b].lhs);
                self.slab[b].lhs = lhs;
                self.update(b);

                if let Some(lv) = self.slab.get_mut(lhs) {
                    lv.prv = b;
                }

                b
            }
        }
    }

    #[must_use]
    pub(super) const fn leftmost(&self, mut at: usize) -> usize {
        while let Some(v) = self.slab.get(at) {
            if v.lhs == NIL {
                break;
            }

            at = v.lhs;
        }

        at
    }

    #[must_use]
    const fn rightmost(&self, mut at: usize) -> usize {
        while let Some(v) = self.slab.get(at) {
            if v.rhs == NIL {
                break;
            }

            at = v.rhs;
        }

        at
    }

    #[must_use]
    fn pop_leftmost(&mut self, at: usize) -> (usize, usize) {
        let lhs = self.leftmost(at);

        // NOTE: (lhs = NIL) iff (at = NIL)
        let Some(v) = self.slab.get_mut(lhs) else {
            return (NIL, NIL);
        };

        let rhs = core::mem::replace(&mut v.rhs, NIL);
        let prv = core::mem::replace(&mut v.prv, NIL);
        self.update(lhs);

        if let Some(rv) = self.slab.get_mut(rhs) {
            rv.prv = prv;
        }

        if let Some(pv) = self.slab.get_mut(prv) {
            pv.lhs = rhs;
            self.propagate(prv);
        }

        if lhs == at { (lhs, rhs) } else { (lhs, at) }
    }

    #[must_use]
    fn concat(&mut self, a: usize, b: usize) -> usize {
        let Some(arv) = self.slab.get(self.rightmost(a)) else {
            return b;
        };

        let Some(blv) = self.slab.get(self.leftmost(b)) else {
            return a;
        };

        if let Some(coarsen) = arv.val.coarsen(&blv.val) {
            let (b_first, b_rest) = self.pop_leftmost(b);

            self.slab[a].val = coarsen;
            self.propagate(a);

            let rem = self.slab.remove(b_first);
            debug_assert!(rem.is_some());

            self.merge(a, b_rest)
        } else {
            self.merge(a, b)
        }
    }

    #[must_use]
    pub(super) const fn next(&self, cur: usize) -> usize {
        let Some(node) = self.slab.get(cur) else {
            return NIL;
        };

        if node.rhs == NIL {
            let mut x = cur;
            let mut p = node.prv;

            while let Some(parent) = self.slab.get(p) {
                if parent.lhs == x {
                    break;
                }

                x = p;
                p = parent.prv;
            }

            p
        } else {
            self.leftmost(node.rhs)
        }
    }
}
