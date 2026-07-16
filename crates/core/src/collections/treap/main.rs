use crate::collections::Slab;
use crate::collections::treap::node::Node;
use crate::collections::treap::state::{State, Verdict};
use crate::collections::treap::{Iter, NIL};
use crate::math::xorshift;
use crate::piece::PieceDesc;

macro_rules! debug_assert_alive {
    ($self:ident, $at:expr) => {
        #[cfg(debug_assertions)]
        {
            if let Some(t) = $self.slab.get($at) {
                assert!(
                    t.val.del_at == u32::MAX,
                    "alive node expected; got {:?}.",
                    t
                );
            } else {
                panic!("alive node expected; got NIL.");
            }
        }
    };
}

macro_rules! debug_assert_dead_or_nil {
    ($self:ident, $at:expr) => {
        #[cfg(debug_assertions)]
        {
            if let Some(t) = $self.slab.get($at) {
                assert!(
                    t.val.del_at != u32::MAX,
                    "dead or nil node expected; got {:?}.",
                    t
                );
            }
        }
    };
}

/// A low-level piece table implemented with an implicit treap.
#[derive(Debug, Clone)]
pub struct Treap {
    state: State,
    root: usize,
    salt: usize,
    pub(super) slab: Slab<Node>,
}

// Node Accessors
impl Treap {
    fn invalidate(&mut self, mut at: usize) {
        while let Some(t) = self.slab.get_mut(at) {
            t.len = None;
            at = t.prv;
        }
    }

    #[must_use]
    fn len_of(&mut self, at: usize) -> u64 {
        let Some(&t) = self.slab.get(at) else {
            return 0;
        };

        if t.val.del_at != u32::MAX {
            return 0;
        }

        if let Some(len) = t.len {
            len
        } else {
            let lhs = self.len_of(t.lhs);
            let mid = t.val.len();
            let rhs = self.len_of(t.rhs);

            let len = lhs + mid + rhs;
            self.slab[at].len = Some(len);
            len
        }
    }

    #[must_use]
    fn pri_of(&self, at: usize) -> usize {
        xorshift(at ^ self.salt)
    }
}

// Kill
impl Treap {
    fn kill_unsafe(&mut self, at: usize) {
        let Some(t) = self.slab.remove(at) else {
            return;
        };

        self.kill_unsafe(t.lhs);
        self.kill_unsafe(t.rhs);
    }

    fn kill(&mut self, at: usize) {
        let Some(t) = self.slab.get(at) else { return };

        if let Some(prv) = self.slab.get_mut(t.prv) {
            if prv.lhs == at {
                prv.lhs = NIL;
            } else {
                prv.rhs = NIL;
            }
        }

        self.kill_unsafe(at);
    }

    fn prune(&mut self) {
        let mut cur = self.root;
        while cur != NIL {
            let next = self.next(cur);

            match self.state.verdict(&self.slab[cur].val) {
                Verdict::None => {}
                Verdict::Update => {
                    self.slab[cur].val.add_at = self.state.oldest;
                }
                Verdict::Kill => {
                    self.kill(cur);
                }
                Verdict::Revive => {
                    self.slab[cur].val.del_at = u32::MAX;
                    self.invalidate(cur);
                }
            }

            cur = next;
        }
    }
}

// Merge / Split
impl Treap {
    #[must_use]
    fn merge(&mut self, a: usize, b: usize) -> usize {
        match (self.slab.get(a), self.slab.get(b)) {
            (None, _) => b,
            (_, None) => a,
            (Some(_), Some(_)) if self.pri_of(a) > self.pri_of(b) => {
                let rhs = self.merge(self.slab[a].rhs, b);
                self.slab[a].rhs = rhs;
                self.invalidate(a);

                if let Some(rv) = self.slab.get_mut(rhs) {
                    rv.prv = a;
                }

                a
            }
            (Some(_), Some(_)) => {
                let lhs = self.merge(a, self.slab[b].lhs);
                self.slab[b].lhs = lhs;
                self.invalidate(b);

                if let Some(lv) = self.slab.get_mut(lhs) {
                    lv.prv = b;
                }

                b
            }
        }
    }

    #[must_use]
    fn concat(&mut self, a: usize, b: usize) -> usize {
        let Some(a_rhs) = self.slab.get(self.rightmost(a)) else {
            return b;
        };

        let Some(b_lhs) = self.slab.get(self.leftmost(b)) else {
            return a;
        };

        if let Some(coarsen) = a_rhs.val.coarsen(&b_lhs.val) {
            let (b_first, b_rest) = self.pop_leftmost(b);

            self.slab[a].val = coarsen;
            self.invalidate(a);

            let rem = self.slab.remove(b_first);
            debug_assert!(rem.is_some());

            self.merge(a, b_rest)
        } else {
            self.merge(a, b)
        }
    }

    fn reset_prv(&mut self, at: usize, prv: usize) {
        if let Some(t) = self.slab.get_mut(at) {
            t.prv = prv;
        }
    }

    #[must_use]
    fn split_unsafe(&mut self, root: usize, pos: u64) -> (usize, usize) {
        let Some(&root_v) = self.slab.get(root) else {
            return (NIL, NIL);
        };

        debug_assert!(root_v.val.del_at != u32::MAX, "cannot split deleted node");

        // must be at latest version!
        // or node length will be mismatched.
        let lhs_len = self.len_of(root_v.lhs);
        let mid_len = root_v.val.len();

        if pos <= lhs_len {
            // pos is on lhs
            if lhs_len == 0 {
                debug_assert_dead_or_nil!(self, root_v.lhs);
                (NIL, root)
            } else if pos == lhs_len {
                debug_assert_alive!(self, root_v.lhs);
                self.slab[root].lhs = NIL;
                self.slab[root_v.lhs].prv = NIL;
                self.invalidate(root);

                (root_v.lhs, root)
            } else {
                debug_assert_alive!(self, root_v.lhs);
                let (a, b) = self.split_unsafe(root_v.lhs, pos);
                self.slab[root].lhs = b;
                self.invalidate(root);

                self.reset_prv(a, NIL);
                self.reset_prv(b, root);

                (a, root)
            }
        } else if pos <= lhs_len + mid_len {
            // pos is on mid
            let mid_pos = pos - lhs_len;

            if mid_pos != mid_len {
                let (mid_l, mid_r) = root_v.val.split_at(mid_pos);
                self.slab[root].val = mid_r;

                let mid_lhs = self.slab.insert(mid_l.into());
                let new_lhs = self.merge(root_v.lhs, mid_lhs);

                (new_lhs, root)
            } else if self.len_of(root_v.rhs) == 0 {
                debug_assert_dead_or_nil!(self, root_v.rhs);
                (root, NIL)
            } else {
                self.slab[root].rhs = NIL;
                self.slab[root_v.rhs].prv = NIL;
                self.invalidate(root);

                (root, root_v.rhs)
            }
        } else {
            // pos is on rhs
            let rhs_pos = pos - lhs_len - mid_len;
            let rhs_len = self.len_of(root_v.rhs);

            if rhs_pos > rhs_len {
                panic!("offset out of bounds");
            } else if rhs_pos == rhs_len {
                (root, NIL)
            } else {
                let (a, b) = self.split_unsafe(root_v.rhs, pos);
                self.slab[root].rhs = a;
                self.invalidate(root);

                self.reset_prv(a, root);
                self.reset_prv(b, NIL);

                (root, b)
            }
        }
    }
}

// Queries
impl Treap {
    #[must_use]
    const fn leftmost(&self, mut at: usize) -> usize {
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
    fn pop_leftmost(&mut self, root: usize) -> (usize, usize) {
        let lhs = self.leftmost(root);

        // NOTE: (lhs = NIL) iff (at = NIL)
        let Some(lhs_v) = self.slab.get_mut(lhs) else {
            return (NIL, NIL);
        };

        let rhs = core::mem::replace(&mut lhs_v.rhs, NIL);
        let prv = core::mem::replace(&mut lhs_v.prv, NIL);
        self.invalidate(lhs);

        if let Some(rv) = self.slab.get_mut(rhs) {
            rv.prv = prv;
        }

        if let Some(pv) = self.slab.get_mut(prv) {
            pv.lhs = rhs;
            self.invalidate(prv);
        }

        if lhs == root { (lhs, rhs) } else { (lhs, root) }
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

// Publics
impl Treap {
    /// Creates a new [`Treap`].
    ///
    /// # Panics
    ///
    /// Panics if given version is reserved version value (`u32::MAX`).
    #[must_use]
    pub const fn new(salt: usize, version: u32) -> Self {
        Self {
            state: State::new(version),
            root: NIL,
            salt,
            slab: Slab::new(),
        }
    }

    /// Returns an iterator over pieces visible for given version.
    #[must_use]
    pub const fn iter(&self, version: u32) -> Iter {
        Iter::new(self, self.leftmost(self.root), version)
    }

    /// Inserts given descriptor, at specified offset,
    /// versioning as given version value.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Given version is reserved version value (`u32::MAX`).
    /// - Given descriptor has invalid range (`start < end`).
    /// - Given offset is out of bounds.
    ///   0 and the total byte length of visible nodes are considered as in bounds.
    pub fn insert(&mut self, off: u64, desc: PieceDesc, version: u32) {
        assert!(version != u32::MAX, "invalid version constant");
        assert!(desc.start < desc.end, "invalid descriptor range");

        if self.state.invalidate(version) {
            self.prune();
        }

        assert!(off <= self.len_of(self.root));

        let (lhs, rhs) = self.split_unsafe(self.root, off);
        let mid = self.slab.insert(desc.build(version).into());
        let tmp = self.concat(lhs, mid);
        self.root = self.concat(tmp, rhs);
    }

    /// Marks given range as removed from this [`Treap`],
    /// versioned as given version value.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Given version is reserved version value (`u32::MAX`).
    /// - Given range is invalid (`start < end`).
    /// - Given ending offset is out of bounds.
    ///   0 and the total byte length of visible nodes are considered as in bounds.
    pub fn remove(&mut self, start: u64, end: u64, version: u32) {
        assert!(version != u32::MAX, "invalid version constant");
        assert!(start <= end, "invalid removal range");

        if start == end {
            return;
        }

        if self.state.invalidate(version) {
            self.prune();
        }

        let len = self.len_of(self.root);
        if end == len {
            if start == 0 {
                self.slab[self.root].val.del_at = version;
            } else {
                let (rest, del) = self.split_unsafe(self.root, start);
                self.slab[del].val.del_at = version;
                self.root = self.merge(rest, del);
            }
        } else if end < len {
            if start == 0 {
                let (del, rest) = self.split_unsafe(self.root, end);
                self.slab[del].val.del_at = version;
                self.root = self.merge(del, rest);
            } else {
                let (rest, rhs) = self.split_unsafe(self.root, end);
                let (lhs, mid) = self.split_unsafe(rest, end);
                self.slab[mid].val.del_at = version;

                let root = self.merge(lhs, mid);
                let root = self.merge(root, rhs);
                self.root = root;
            }
        } else {
            panic!("offset out of bounds");
        }
    }

    /// Splits this [`Treap`] at given offset,
    /// versioned by given version value.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Given version is reserved version value (`u32::MAX`).
    /// - Given offset is out of bounds.
    ///   0 and the total byte length of visible nodes are considered as in bounds.
    #[must_use]
    pub fn split_off(&mut self, off: u64, version: u32) -> Self {
        assert!(version != u32::MAX, "invalid version constant");

        if self.state.invalidate(version) {
            self.prune();
        }

        let len = self.len_of(self.root);
        if off == 0 {
            core::mem::replace(self, Self::new(xorshift(self.salt), version))
        } else if off == len {
            Self::new(xorshift(self.salt), version)
        } else if off < len {
            let (lhs, rhs) = self.split_unsafe(self.root, off);
            let mut cloned = self.clone();

            self.root = lhs;
            self.reset_prv(lhs, NIL);
            self.kill(rhs);

            cloned.root = rhs;
            cloned.reset_prv(rhs, NIL);
            cloned.kill(lhs);

            cloned
        } else {
            panic!("offset out of bounds");
        }
    }
}
