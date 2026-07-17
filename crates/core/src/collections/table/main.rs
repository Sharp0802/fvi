use core::cmp::Ordering;

use crate::collections::Slab;
use crate::collections::table::node::Node;
use crate::collections::table::state::{State, Verdict};
use crate::collections::table::{Context, Iter, NIL, Version};
use crate::math::xorshift;
use crate::piece::PieceDesc;
use crate::util::unreachable;

macro_rules! debug_assert_dead_or_nil {
    ($self:ident, $at:expr) => {
        #[cfg(debug_assertions)]
        {
            if let Some(t) = $self.slab.get($at) {
                assert!(
                    t.val.del_at.is_some(),
                    "dead or nil node expected; got {:?}.",
                    t
                );
            }
        }
    };
}

/// A piece table implemented with an implicit treap.
#[derive(Debug, Clone)]
pub struct Table {
    state: State,
    root: usize,
    salt: usize,
    pub(super) slab: Slab<Node>,
}

// Node Accessors
impl Table {
    const fn invalidate(&mut self, mut at: usize) {
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

        if let Some(len) = t.len {
            len
        } else {
            let lhs = self.len_of(t.lhs);
            let mid = t.val.desc().len();
            let rhs = self.len_of(t.rhs);

            let len = lhs + mid + rhs;
            self.slab[at].len = Some(len);
            len
        }
    }

    #[must_use]
    const fn pri_of(&self, at: usize) -> usize {
        xorshift(at ^ self.salt)
    }

    fn mark_removed(&mut self, at: usize, version: Version) {
        let Some(t) = self.slab.get(at).copied() else {
            return;
        };

        self.mark_removed(t.lhs, version);
        self.mark_removed(t.rhs, version);

        let node = &mut self.slab[at];
        if node.val.del_at.is_none() {
            node.val.del_at = Some(version);
        }
        node.len = None;
    }

    const fn reset_prv(&mut self, at: usize, prv: usize) {
        if let Some(t) = self.slab.get_mut(at) {
            t.prv = prv;
        }
    }
}

// Kill
impl Table {
    fn kill_all_unsafe(&mut self, at: usize) {
        let Some(t) = self.slab.remove(at) else {
            return;
        };

        self.kill_all_unsafe(t.lhs);
        self.kill_all_unsafe(t.rhs);
    }

    fn kill_all(&mut self, at: usize) {
        let Some(t) = self.slab.get(at) else { return };
        let prv_i = t.prv;

        if let Some(prv) = self.slab.get_mut(prv_i) {
            if prv.lhs == at {
                prv.lhs = NIL;
            } else {
                debug_assert_eq!(prv.rhs, at);
                prv.rhs = NIL;
            }
            self.invalidate(prv_i);
        }

        self.kill_all_unsafe(at);
    }

    fn kill(&mut self, at: usize) {
        let Some(&node) = self.slab.get(at) else {
            return;
        };

        let replace = self.merge(node.lhs, node.rhs);
        self.reset_prv(replace, node.prv);

        if let Some(prv) = self.slab.get_mut(node.prv) {
            if prv.lhs == at {
                prv.lhs = replace;
            } else {
                debug_assert_eq!(prv.rhs, at);
                prv.rhs = replace;
            }
            self.invalidate(node.prv);
        } else {
            debug_assert_eq!(self.root, at);
            self.root = replace;
        }

        let del = self.slab.remove(at);
        debug_assert!(del.is_some());
    }

    fn prune(&mut self) {
        let mut cur = self.leftmost(self.root);
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
                    self.slab[cur].val.del_at = None;
                }
            }

            cur = next;
        }
    }
}

// Merge / Split
impl Table {
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
        let a_rhs_i = self.rightmost(a);
        let Some(a_rhs) = self.slab.get(a_rhs_i) else {
            return b;
        };

        let Some(b_lhs) = self.slab.get(self.leftmost(b)) else {
            return a;
        };

        if let Some(coarsen) = a_rhs.val.coarsen(&b_lhs.val) {
            let (b_first, b_rest) = self.pop_leftmost(b);

            self.slab[a_rhs_i].val = coarsen;
            self.invalidate(a_rhs_i);

            let rem = self.slab.remove(b_first);
            debug_assert!(rem.is_some());

            self.merge(a, b_rest)
        } else {
            self.merge(a, b)
        }
    }

    #[must_use]
    fn split_unsafe(&mut self, root: usize, pos: u64) -> (usize, usize) {
        let Some(&root_v) = self.slab.get(root) else {
            return (NIL, NIL);
        };

        debug_assert!(root_v.val.del_at.is_none(), "cannot split removed node");

        // must be at latest version!
        // or node length will be mismatched.
        let lhs_len = self.len_of(root_v.lhs);
        let mid_len = root_v.val.desc().len();

        if pos <= lhs_len {
            // pos is on lhs
            if lhs_len == 0 {
                debug_assert_dead_or_nil!(self, root_v.lhs);
                (NIL, root)
            } else if pos == lhs_len {
                self.slab[root].lhs = NIL;
                self.slab[root_v.lhs].prv = NIL;
                self.invalidate(root);

                (root_v.lhs, root)
            } else {
                let (a, b) = self.split_unsafe(root_v.lhs, pos);
                self.slab[root].lhs = NIL;
                self.invalidate(root);

                self.reset_prv(a, NIL);
                self.reset_prv(b, NIL);
                let rhs = self.merge(b, root);
                //self.reset_prv(rhs, NIL);

                (a, rhs)
            }
        } else if pos <= lhs_len + mid_len {
            // pos is on mid
            let mid_pos = pos - lhs_len;

            if mid_pos != mid_len {
                let (mid_l, mid_r) = root_v.val.split_at(mid_pos);
                self.slab[root].lhs = NIL;
                self.slab[root].val = mid_r;
                self.invalidate(root);

                self.reset_prv(root_v.lhs, NIL);
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

            match rhs_pos.cmp(&rhs_len) {
                Ordering::Less => {
                    let (a, b) = self.split_unsafe(root_v.rhs, rhs_pos);
                    self.slab[root].rhs = NIL;
                    self.invalidate(root);

                    self.reset_prv(a, NIL);
                    self.reset_prv(b, NIL);
                    let lhs = self.merge(root, a);

                    (lhs, b)
                }
                Ordering::Equal => (root, NIL),
                Ordering::Greater => {
                    panic!("offset out of bounds");
                }
            }
        }
    }
}

// Queries
impl Table {
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
    fn leftmost_visible(&self, mut at: usize, version: Version) -> usize {
        let Some(mut curr) = self.slab.get(at) else {
            return NIL;
        };

        if !curr.val.is_visible_at(version) {
            return NIL;
        }

        while let Some(v) = self.slab.get(curr.lhs)
            && v.val.is_visible_at(version)
        {
            at = curr.lhs;
            curr = v;
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
    const fn pop_leftmost(&mut self, root: usize) -> (usize, usize) {
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
    const fn next(&self, cur: usize) -> usize {
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

    #[must_use]
    pub(super) fn next_visible(&self, cur: usize, version: Version) -> usize {
        let Some(node) = self.slab.get(cur) else {
            return NIL;
        };

        if node.rhs != NIL {
            let next = self.leftmost_visible(node.rhs, version);
            if next != NIL {
                return next;
            }
        }

        let mut x = cur;
        let mut p = node.prv;

        while let Some(prv) = self.slab.get(p) {
            if prv.lhs == x {
                break;
            }

            x = p;
            p = prv.prv;
        }

        p
    }
}

// Publics
impl Table {
    /// Creates a new [`Table`].
    ///
    /// # Panics
    ///
    /// Panics if given version is reserved version value (`u32::MAX`).
    #[must_use]
    pub const fn new(salt: usize, cx: &Context) -> Self {
        Self {
            state: State::new(cx),
            root: NIL,
            salt,
            slab: Slab::new(),
        }
    }

    /// Returns an iterator over pieces visible for given version.
    #[must_use]
    pub fn iter(&self, version: Version) -> Iter<'_> {
        Iter::new(self, self.leftmost_visible(self.root, version), version)
    }

    /// Updates this [`Table`] to follow state of given context (`cx`).
    ///
    /// It may attept to prune nodes by iterating in `O(n)`,
    /// if given context (`cx`) requires rewind some of retained history
    /// (that means the context has older version than latest version of this [`Table`]).
    ///
    /// It may clear this [`Table`] that specifying a version older than retained history.
    /// Note that the allocated capacity isn't affected by clearing anyway.
    pub fn update(&mut self, cx: &Context) {
        if self.state.update(cx) {
            self.prune();
        }
    }

    /// Inserts given descriptor at specified offset to this [`Table`].
    ///
    /// Rewinding the version can cause iterating all nodes.
    /// See [`Table::update()`].
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Given descriptor has invalid range (`start > end`).
    /// - Given offset is out of bounds.
    ///   0 and the total byte length of visible nodes are considered as in bounds.
    pub fn insert(&mut self, cx: &Context, off: u64, desc: PieceDesc) {
        assert!(desc.start <= desc.end, "invalid descriptor range");
        self.update(cx);
        assert!(off <= self.len_of(self.root));

        if desc.start == desc.end {
            return;
        }

        let (lhs, rhs) = self.split_unsafe(self.root, off);
        let mid = self.slab.insert(desc.build(cx.version).into());
        let tmp = self.concat(lhs, mid);
        self.root = self.concat(tmp, rhs);
    }

    /// Marks given range as removed from this [`Table`].
    ///
    /// Rewinding the version can cause iterating all nodes.
    /// See [`Table::update()`].
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Given range is invalid (`start > end`).
    /// - Given ending offset is out of bounds.
    ///   0 and the total byte length of visible nodes are considered as in bounds.
    pub fn remove(&mut self, cx: &Context, start: u64, end: u64) {
        assert!(start <= end, "invalid removal range");
        self.update(cx);
        let len = self.len_of(self.root);
        assert!(end <= len, "offset out of bounds");

        if start == end {
            return;
        }

        match end.cmp(&self.len_of(self.root)) {
            Ordering::Equal if start == 0 => {
                self.mark_removed(self.root, cx.version);
            }
            Ordering::Equal => {
                let (rest, del) = self.split_unsafe(self.root, start);
                self.mark_removed(del, cx.version);
                self.root = self.merge(rest, del);
            }
            Ordering::Less if start == 0 => {
                let (del, rest) = self.split_unsafe(self.root, end);
                self.mark_removed(del, cx.version);
                self.root = self.merge(del, rest);
            }
            Ordering::Less => {
                let (rest, rhs) = self.split_unsafe(self.root, end);
                let (lhs, mid) = self.split_unsafe(rest, start);
                self.mark_removed(mid, cx.version);

                let root = self.merge(lhs, mid);
                let root = self.merge(root, rhs);
                self.root = root;
            }
            Ordering::Greater => {
                unreachable();
            }
        }
    }

    /// Splits this [`Table`] at given offset.
    ///
    /// Rewinding the version can cause iterating all nodes.
    /// See [`Table::update()`].
    ///
    /// # Panics
    ///
    /// Panics if the given offset is out of bounds.
    /// 0 and the total byte length of visible nodes are considered as in bounds.
    #[must_use]
    pub fn split_off(&mut self, cx: &Context, off: u64) -> Self {
        self.update(cx);

        let len = self.len_of(self.root);
        if off == 0 {
            core::mem::replace(self, Self::new(xorshift(self.salt), cx))
        } else if off == len {
            Self::new(xorshift(self.salt), cx)
        } else if off < len {
            let (lhs, rhs) = self.split_unsafe(self.root, off);
            let mut cloned = self.clone();

            self.root = lhs;
            self.reset_prv(lhs, NIL);
            self.kill_all(rhs);

            cloned.root = rhs;
            cloned.reset_prv(rhs, NIL);
            cloned.kill_all(lhs);

            cloned
        } else {
            panic!("offset out of bounds");
        }
    }
}
