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

#[cfg(test)]
mod tests {
    #![expect(
        clippy::should_panic_without_expect,
        reason = "panic messages are not part of public contract"
    )]

    use core::num::NonZero;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::test_runner::{TestCaseError, TestCaseResult};
    use std::collections::BTreeMap;
    use std::vec::Vec;

    use super::*;
    use crate::piece::{Buffer, Piece};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Atom {
        buffer: Buffer,
        offset: u64,
    }

    #[derive(Debug, Clone)]
    struct Model {
        history: BTreeMap<u32, Vec<Atom>>,
        current: u32,
        oldest: u32,
        query_floor: u32,
        undo_max_len: u32,
    }

    #[derive(Debug, Clone, Copy)]
    enum Op {
        Insert { off: u16, append: bool, len: u8 },
        Remove { a: u16, b: u16 },
        SwitchVersion { version: u8 },
        Split { off: u16, keep_right: bool },
    }

    fn version(raw: u32) -> Version {
        Version::new(raw).expect("test versions are not reserved")
    }

    fn context(raw: u32, undo_max_len: u32) -> Context {
        Context {
            undo_max_len: NonZero::new(undo_max_len).expect("undo length is nonzero"),
            version: version(raw),
        }
    }

    const fn desc(buffer: Buffer, start: u64, end: u64) -> PieceDesc {
        PieceDesc { buffer, start, end }
    }

    fn piece(buffer: Buffer, start: u64, end: u64, add_at: u32, del_at: Option<u32>) -> Piece {
        Piece {
            buffer,
            start,
            end,
            add_at: version(add_at),
            del_at: del_at.map(version),
        }
    }

    fn atoms(buffer: Buffer, start: u64, end: u64) -> Vec<Atom> {
        (start..end).map(|offset| Atom { buffer, offset }).collect()
    }

    fn flatten(table: &Table, at: Version) -> Vec<Atom> {
        table
            .iter(at)
            .flat_map(|piece| {
                assert!(piece.start < piece.end, "empty piece returned by iterator");
                assert!(
                    piece.is_visible_at(at),
                    "invisible piece returned by iterator"
                );
                (piece.start..piece.end).map(move |offset| Atom {
                    buffer: piece.buffer,
                    offset,
                })
            })
            .collect()
    }

    fn pieces(table: &Table, at: u32) -> Vec<Piece> {
        table.iter(version(at)).collect()
    }

    fn assert_view(table: &Table, at: u32, expected: &[Atom]) {
        assert_eq!(flatten(table, version(at)), expected);
    }

    fn audit_node(
        table: &Table,
        at: usize,
        parent: usize,
        seen: &mut [bool],
    ) -> Result<(u64, usize), TestCaseError> {
        if at == NIL {
            return Ok((0, 0));
        }

        prop_assert!(at < seen.len(), "node key exceeds slab capacity");
        prop_assert!(!seen[at], "cycle or duplicate child at node {at}");
        seen[at] = true;

        prop_assert!(
            table.slab.get(at).is_some(),
            "reachable node must be occupied"
        );
        let node = table.slab.get(at).expect("reachable node must be occupied");
        prop_assert_eq!(node.prv, parent, "incorrect parent pointer at node {}", at);
        prop_assert!(node.val.start < node.val.end, "empty node at {at}");
        if let Some(del_at) = node.val.del_at {
            prop_assert!(node.val.add_at <= del_at, "piece deleted before insertion");
        }

        for child in [node.lhs, node.rhs] {
            if child != NIL {
                prop_assert!(
                    table.pri_of(at) >= table.pri_of(child),
                    "treap priority violation between {at} and {child}"
                );
            }
        }

        let (lhs_len, lhs_count) = audit_node(table, node.lhs, at, seen)?;
        let (rhs_len, rhs_count) = audit_node(table, node.rhs, at, seen)?;
        let mid_len = if node.val.is_visible_at(table.state.latest) {
            node.val.desc().len()
        } else {
            0
        };
        let expected_len = lhs_len
            .checked_add(mid_len)
            .and_then(|len| len.checked_add(rhs_len));
        prop_assert!(expected_len.is_some(), "generated table length fits u64");
        let expected_len = expected_len.expect("length checked above");

        if let Some(cached) = node.len {
            prop_assert_eq!(cached, expected_len, "stale length cache at node {}", at);
        }

        Ok((expected_len, lhs_count + rhs_count + 1))
    }

    fn audit(table: &Table) -> TestCaseResult {
        if table.root == NIL {
            prop_assert_eq!(table.slab.len(), 0, "empty root leaked slab nodes");
            return Ok(());
        }

        prop_assert_eq!(
            table.slab[table.root].prv,
            NIL,
            "root must not have a parent"
        );

        let mut seen = std::vec![false; table.slab.capacity()];
        let (visible_len, reachable) = audit_node(table, table.root, NIL, &mut seen)?;
        prop_assert_eq!(
            reachable,
            table.slab.len(),
            "unreachable occupied slab node"
        );

        let iter_len = flatten(table, table.state.latest).len() as u64;
        prop_assert_eq!(
            visible_len,
            iter_len,
            "tree and iterator disagree on length"
        );

        let mut cloned = table.clone();
        let root = cloned.root;
        prop_assert_eq!(
            cloned.len_of(root),
            visible_len,
            "computed root length is not the visible length"
        );
        Ok(())
    }

    fn assert_audit(table: &Table) {
        if let Err(error) = audit(table) {
            panic!("table invariant audit failed: {error}");
        }
    }

    impl Model {
        fn new(undo_max_len: u32) -> Self {
            Self {
                history: BTreeMap::from([(0, Vec::new())]),
                current: 0,
                oldest: 0,
                query_floor: 0,
                undo_max_len,
            }
        }

        const fn possible_oldest(&self, at: u32) -> u32 {
            (at + 1).saturating_sub(self.undo_max_len)
        }

        fn touch(&mut self) {
            self.oldest = self.oldest.max(self.possible_oldest(self.current));
        }

        fn switch_to(&mut self, next: u32) {
            self.oldest = self.oldest.max(self.possible_oldest(next));

            if next < self.current {
                let view = if next < self.oldest {
                    Vec::new()
                } else {
                    self.history
                        .get(&next)
                        .expect("generated rewind targets retained history")
                        .clone()
                };
                self.history.retain(|&at, _| at <= next);
                self.history.insert(next, view);
            } else if next > self.current {
                let view = self
                    .history
                    .get(&self.current)
                    .expect("current model snapshot exists")
                    .clone();
                for at in (self.current + 1)..=next {
                    self.history.insert(at, view.clone());
                }
            }

            self.current = next;
        }

        fn current_view(&self) -> &Vec<Atom> {
            self.history
                .get(&self.current)
                .expect("current model snapshot exists")
        }

        fn current_view_mut(&mut self) -> &mut Vec<Atom> {
            self.history
                .get_mut(&self.current)
                .expect("current model snapshot exists")
        }

        fn insert(&mut self, off: usize, inserted: &[Atom]) {
            self.touch();
            self.current_view_mut()
                .splice(off..off, inserted.iter().copied());
        }

        fn remove(&mut self, start: usize, end: usize) {
            self.touch();
            self.current_view_mut().drain(start..end);
        }

        fn split(&mut self, off: usize, keep_right: bool) -> (Vec<Atom>, Vec<Atom>) {
            self.touch();
            let view = self.current_view();
            let lhs = view[..off].to_vec();
            let rhs = view[off..].to_vec();
            let retained = if keep_right { rhs.clone() } else { lhs.clone() };

            self.history.clear();
            self.history.insert(self.current, retained);
            self.query_floor = self.current;

            (lhs, rhs)
        }

        fn minimum_query_version(&self) -> u32 {
            self.oldest.max(self.query_floor)
        }
    }

    fn assert_model(table: &Table, model: &Model) -> TestCaseResult {
        for (&at, expected) in &model.history {
            if at >= model.minimum_query_version() {
                let actual = flatten(table, version(at));
                prop_assert_eq!(actual.as_slice(), expected.as_slice());
            }
        }
        Ok(())
    }

    fn arb_op() -> impl Strategy<Value = Op> {
        prop_oneof![
            5 => (any::<u16>(), any::<bool>(), 0u8..=8).prop_map(|(off, append, len)| {
                Op::Insert { off, append, len }
            }),
            4 => (any::<u16>(), any::<u16>()).prop_map(|(a, b)| Op::Remove { a, b }),
            3 => (0u8..=16).prop_map(|version| Op::SwitchVersion { version }),
            2 => (any::<u16>(), any::<bool>())
                .prop_map(|(off, keep_right)| Op::Split { off, keep_right }),
        ]
    }

    #[test]
    fn test_no_op() {
        let cx = context(0, 4);
        let mut table = Table::new(0, &cx);

        let mut iter = table.iter(cx.version);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);

        table.insert(&cx, 0, desc(Buffer::Original, 4, 4));
        table.remove(&cx, 0, 0);
        assert_view(&table, 0, &[]);
        assert_audit(&table);
    }

    #[test]
    fn test_insert() {
        let cx = context(0, 4);
        let mut table = Table::new(0xCAFE, &cx);

        table.insert(&cx, 0, desc(Buffer::Original, 0, 8));
        table.insert(&cx, 4, desc(Buffer::Append, 100, 104));

        let mut expected = atoms(Buffer::Original, 0, 4);
        expected.extend(atoms(Buffer::Append, 100, 104));
        expected.extend(atoms(Buffer::Original, 4, 8));
        assert_view(&table, 0, &expected);
        assert_eq!(
            pieces(&table, 0),
            std::vec![
                piece(Buffer::Original, 0, 4, 0, None),
                piece(Buffer::Append, 100, 104, 0, None),
                piece(Buffer::Original, 4, 8, 0, None),
            ]
        );
        assert_audit(&table);

        let mut coarsened = Table::new(1, &cx);
        coarsened.insert(&cx, 0, desc(Buffer::Original, 0, 4));
        coarsened.insert(&cx, 4, desc(Buffer::Original, 4, 8));
        coarsened.insert(&cx, 0, desc(Buffer::Append, 50, 52));
        assert_eq!(
            pieces(&coarsened, 0),
            std::vec![
                piece(Buffer::Append, 50, 52, 0, None),
                piece(Buffer::Original, 0, 8, 0, None),
            ]
        );
        assert_audit(&coarsened);

        let mut next_cx = cx.clone();
        let mut versioned = Table::new(2, &next_cx);
        versioned.insert(&next_cx, 0, desc(Buffer::Original, 0, 4));
        next_cx.version = version(1);
        versioned.insert(&next_cx, 4, desc(Buffer::Original, 4, 8));
        assert_eq!(
            pieces(&versioned, 1),
            std::vec![
                piece(Buffer::Original, 0, 4, 0, None),
                piece(Buffer::Original, 4, 8, 1, None),
            ]
        );
        assert_audit(&versioned);
    }

    fn assert_removal_view(start: u64, end: u64, expected: &[Atom]) {
        let mut cx = context(0, 4);
        let mut table = Table::new(0xBEEF, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 10));
        cx.version = version(1);
        table.remove(&cx, start, end);
        assert_view(&table, 0, &atoms(Buffer::Original, 0, 10));
        assert_view(&table, 1, expected);
    }

    #[test]
    fn removal_offsets() {
        // at prefix
        assert_removal_view(0, 3, &atoms(Buffer::Original, 3, 10));

        // at middle
        let mut expected = atoms(Buffer::Original, 0, 3);
        expected.extend(atoms(Buffer::Original, 7, 10));
        assert_removal_view(3, 7, &expected);

        // at suffix
        assert_removal_view(7, 10, &atoms(Buffer::Original, 0, 7));

        // all
        assert_removal_view(0, 10, &[]);
    }

    #[test]
    fn removal_lifetimes() {
        let mut cx = context(0, 4);
        let mut table = Table::new(0xBEEF, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 10));
        cx.version = version(1);
        table.remove(&cx, 3, 7);

        assert_eq!(
            pieces(&table, 0),
            std::vec![
                piece(Buffer::Original, 0, 3, 0, None),
                piece(Buffer::Original, 3, 7, 0, Some(1)),
                piece(Buffer::Original, 7, 10, 0, None),
            ]
        );
        assert_eq!(
            pieces(&table, 1),
            std::vec![
                piece(Buffer::Original, 0, 3, 0, None),
                piece(Buffer::Original, 7, 10, 0, None),
            ]
        );
    }

    #[test]
    fn visible_len_of_removed() {
        let mut cx = context(0, 2);
        let mut table = Table::new(0, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 1));
        cx.version = version(1);
        table.remove(&cx, 0, 1);
        assert_audit(&table);
    }

    #[test]
    fn postremoval_edit_offsets() {
        let mut cx = context(0, 4);
        let mut table = Table::new(7, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 10));

        cx.version = version(1);
        table.remove(&cx, 3, 7);
        table.insert(&cx, 3, desc(Buffer::Append, 100, 102));

        let mut expected = atoms(Buffer::Original, 0, 3);
        expected.extend(atoms(Buffer::Append, 100, 102));
        expected.extend(atoms(Buffer::Original, 7, 10));
        assert_view(&table, 1, &expected);
        assert_audit(&table);
    }

    #[test]
    fn iter_skip_futures() {
        let mut cx = context(0, 4);
        let mut table = Table::new(0, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 2));
        cx.version = version(1);
        table.insert(&cx, 2, desc(Buffer::Append, 10, 11));

        assert_view(&table, 0, &atoms(Buffer::Original, 0, 2));
    }

    #[test]
    fn rewind_and_branch() {
        let mut cx = context(0, 8);
        let mut table = Table::new(0x1234, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 6));

        cx.version = version(1);
        table.remove(&cx, 2, 4);

        cx.version = version(2);
        table.insert(&cx, 4, desc(Buffer::Append, 100, 102));

        cx.version = version(0);
        table.update(&cx);
        assert_view(&table, 0, &atoms(Buffer::Original, 0, 6));
        table.insert(&cx, 1, desc(Buffer::Append, 200, 201));

        let mut branched = atoms(Buffer::Original, 0, 1);
        branched.extend(atoms(Buffer::Append, 200, 201));
        branched.extend(atoms(Buffer::Original, 1, 6));
        assert_view(&table, 0, &branched);

        cx.version = version(2);
        table.update(&cx);
        assert_view(&table, 2, &branched);
        assert_audit(&table);
    }

    #[test]
    fn undo_prune() {
        let mut cx = context(0, 2);
        let mut table = Table::new(9, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 4));

        cx.version = version(1);
        table.remove(&cx, 1, 2);
        cx.version = version(2);
        table.update(&cx);
        cx.version = version(3);
        table.insert(&cx, 0, desc(Buffer::Append, 20, 21));

        cx.version = version(2);
        table.update(&cx);
        let mut at_two = atoms(Buffer::Original, 0, 1);
        at_two.extend(atoms(Buffer::Original, 2, 4));
        assert_view(&table, 2, &at_two);

        cx.version = version(1);
        table.update(&cx);
        assert_view(&table, 1, &[]);

        cx.undo_max_len = NonZero::new(8).expect("nonzero");
        cx.version = version(0);
        table.update(&cx);
        assert_view(&table, 0, &[]);
        assert_audit(&table);
    }

    #[test]
    fn split_preserve_old_views() {
        let mut cx = context(0, 8);
        let mut table = Table::new(0xDEAD, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 8));
        cx.version = version(1);
        table.insert(&cx, 8, desc(Buffer::Append, 100, 102));

        let right = table.split_off(&cx, 3);
        assert_view(&table, 0, &atoms(Buffer::Original, 0, 3));
        assert_view(&table, 1, &atoms(Buffer::Original, 0, 3));

        assert_view(&right, 0, &atoms(Buffer::Original, 3, 8));
        let mut right_at_one = atoms(Buffer::Original, 3, 8);
        right_at_one.extend(atoms(Buffer::Append, 100, 102));
        assert_view(&right, 1, &right_at_one);

        assert_audit(&table);
        assert_audit(&right);
    }

    #[test]
    fn split_independent_results() {
        let mut cx = context(0, 8);
        let mut left = Table::new(0xDEAD, &cx);
        left.insert(&cx, 0, desc(Buffer::Original, 0, 8));
        let mut right = left.split_off(&cx, 3);

        cx.version = version(1);
        left.insert(&cx, 0, desc(Buffer::Append, 200, 201));
        assert_view(&right, 1, &atoms(Buffer::Original, 3, 8));

        right.insert(&cx, 5, desc(Buffer::Append, 300, 301));
        let mut expected_left = atoms(Buffer::Append, 200, 201);
        expected_left.extend(atoms(Buffer::Original, 0, 3));
        assert_view(&left, 1, &expected_left);

        assert_audit(&left);
        assert_audit(&right);
    }

    #[test]
    fn split_bounds_for_salts() {
        for &salt in &[0, 1, 0xCAFE, usize::MAX] {
            let cx = context(0, 4);
            let mut whole = Table::new(salt, &cx);
            whole.insert(&cx, 0, desc(Buffer::Original, 0, 6));

            let returned = whole.split_off(&cx, 0);
            assert_view(&whole, 0, &[]);
            assert_view(&returned, 0, &atoms(Buffer::Original, 0, 6));
            assert_audit(&whole);
            assert_audit(&returned);

            let mut kept = returned;
            let empty = kept.split_off(&cx, 6);
            assert_view(&kept, 0, &atoms(Buffer::Original, 0, 6));
            assert_view(&empty, 0, &[]);
            assert_audit(&kept);
            assert_audit(&empty);

            let mut boundary = Table::new(salt, &cx);
            boundary.insert(&cx, 0, desc(Buffer::Original, 0, 3));
            boundary.insert(&cx, 3, desc(Buffer::Append, 20, 22));
            let right = boundary.split_off(&cx, 3);
            assert_view(&boundary, 0, &atoms(Buffer::Original, 0, 3));
            assert_view(&right, 0, &atoms(Buffer::Append, 20, 22));
            assert_audit(&boundary);
            assert_audit(&right);
        }
    }

    #[test]
    fn split_tombstone_offsets() {
        let mut cx = context(0, 4);
        let mut table = Table::new(0, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 0, 6));
        cx.version = version(1);
        table.remove(&cx, 2, 4);

        let right = table.split_off(&cx, 3);
        let mut lhs = atoms(Buffer::Original, 0, 2);
        lhs.extend(atoms(Buffer::Original, 4, 5));
        assert_view(&table, 1, &lhs);
        assert_view(&right, 1, &atoms(Buffer::Original, 5, 6));
        assert_audit(&table);
        assert_audit(&right);
    }

    #[test]
    #[should_panic]
    fn insert_invalid_desc() {
        let cx = context(0, 1);
        let mut table = Table::new(0, &cx);
        table.insert(&cx, 0, desc(Buffer::Original, 2, 1));
    }

    #[test]
    #[should_panic]
    fn insert_out_of_bounds() {
        let cx = context(0, 1);
        let mut table = Table::new(0, &cx);
        table.insert(&cx, 1, desc(Buffer::Original, 0, 1));
    }

    #[test]
    #[should_panic]
    fn remove_invalid_range() {
        let cx = context(0, 1);
        let mut table = Table::new(0, &cx);
        table.remove(&cx, 1, 0);
    }

    #[test]
    #[should_panic]
    fn remove_out_of_bounds() {
        let cx = context(0, 1);
        let mut table = Table::new(0, &cx);
        table.remove(&cx, 0, 1);
    }

    #[test]
    #[should_panic]
    fn split_out_of_bounds() {
        let cx = context(0, 1);
        let mut table = Table::new(0, &cx);
        _ = table.split_off(&cx, 1);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn match_models(
            salt in any::<usize>(),
            undo_max_len in 1u32..=8,
            ops in vec(arb_op(), 1..=128),
        ) {
            let mut cx = context(0, undo_max_len);
            let mut table = Table::new(salt, &cx);
            let mut model = Model::new(undo_max_len);

            for (step, op) in ops.into_iter().enumerate() {
                match op {
                    Op::Insert { off, append, len } => {
                        let total = model.current_view().len();
                        let off = usize::from(off) % (total + 1);
                        let buffer = if append { Buffer::Append } else { Buffer::Original };
                        let start = (step as u64) * 16;
                        let end = start + u64::from(len);
                        let inserted = atoms(buffer, start, end);

                        table.insert(&cx, off as u64, desc(buffer, start, end));
                        model.insert(off, &inserted);
                    }
                    Op::Remove { a, b } => {
                        let total = model.current_view().len();
                        let a = usize::from(a) % (total + 1);
                        let b = usize::from(b) % (total + 1);
                        let start = a.min(b);
                        let end = a.max(b);

                        table.remove(&cx, start as u64, end as u64);
                        model.remove(start, end);
                    }
                    Op::SwitchVersion { version: raw } => {
                        let floor = model.minimum_query_version();
                        let target = floor + u32::from(raw) % (17 - floor);
                        cx.version = version(target);
                        table.update(&cx);
                        model.switch_to(target);
                    }
                    Op::Split { off, keep_right } => {
                        let total = model.current_view().len();
                        let off = usize::from(off) % (total + 1);
                        let mut rhs = table.split_off(&cx, off as u64);
                        let (expected_lhs, expected_rhs) = model.split(off, keep_right);

                        prop_assert_eq!(flatten(&table, cx.version), expected_lhs);
                        prop_assert_eq!(flatten(&rhs, cx.version), expected_rhs);
                        audit(&table)?;
                        audit(&rhs)?;

                        if keep_right {
                            core::mem::swap(&mut table, &mut rhs);
                        }
                    }
                }

                assert_model(&table, &model)?;
                audit(&table)?;
            }
        }
    }
}
