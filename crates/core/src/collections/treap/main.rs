use core::cmp::Ordering;

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
    const fn pri_of(&self, at: usize) -> usize {
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

    const fn reset_prv(&mut self, at: usize, prv: usize) {
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
                self.invalidate(root);

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
                    let (a, b) = self.split_unsafe(root_v.rhs, pos);
                    self.slab[root].rhs = a;
                    self.invalidate(root);

                    self.reset_prv(a, root);
                    self.reset_prv(b, NIL);

                    (root, b)
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
    pub const fn iter(&self, version: u32) -> Iter<'_> {
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

        match end.cmp(&self.len_of(self.root)) {
            Ordering::Equal if start == 0 => {
                self.slab[self.root].val.del_at = version;
            }
            Ordering::Equal => {
                let (rest, del) = self.split_unsafe(self.root, start);
                self.slab[del].val.del_at = version;
                self.root = self.merge(rest, del);
            }
            Ordering::Less if start == 0 => {
                let (del, rest) = self.split_unsafe(self.root, end);
                self.slab[del].val.del_at = version;
                self.root = self.merge(del, rest);
            }
            Ordering::Less => {
                let (rest, rhs) = self.split_unsafe(self.root, end);
                let (lhs, mid) = self.split_unsafe(rest, end);
                self.slab[mid].val.del_at = version;

                let root = self.merge(lhs, mid);
                let root = self.merge(root, rhs);
                self.root = root;
            }
            Ordering::Greater => {
                panic!("offset out of bounds");
            }
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

#[cfg(test)]
mod tests {
    use proptest::collection::vec;
    use proptest::prelude::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::vec::Vec;

    use super::*;
    use crate::piece::{Buffer, Piece, PieceDesc};

    const MAX_VERSION: u8 = 7;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Token {
        buffer: Buffer,
        offset: u64,
    }

    #[derive(Debug, Clone)]
    struct Model {
        latest: u32,
        snapshots: BTreeMap<u32, Vec<Token>>,
    }

    impl Model {
        fn new(version: u32) -> Self {
            let mut snapshots = BTreeMap::new();
            snapshots.insert(version, Vec::new());
            Self {
                latest: version,
                snapshots,
            }
        }

        fn at(&self, version: u32) -> Vec<Token> {
            self.snapshots
                .range(..=version)
                .next_back()
                .map(|(_, value)| value.clone())
                .unwrap_or_default()
        }

        fn branch(&mut self, version: u32) -> Vec<Token> {
            if version < self.latest {
                self.snapshots.retain(|&at, _| at <= version);
            }

            self.latest = version;
            self.at(version)
        }

        fn commit(&mut self, version: u32, value: Vec<Token>) {
            self.latest = version;
            self.snapshots.insert(version, value);
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum Op {
        Insert {
            raw_offset: u16,
            buffer: Buffer,
            start: u16,
            len: u8,
            version: u8,
        },
        Remove {
            raw_start: u16,
            raw_end: u16,
            version: u8,
        },
    }

    fn arb_op() -> impl Strategy<Value = Op> {
        prop_oneof![
            (
                any::<u16>(),
                any::<bool>(),
                any::<u16>(),
                1_u8..=8,
                0_u8..=MAX_VERSION,
            )
                .prop_map(|(raw_offset, append, start, len, version)| Op::Insert {
                    raw_offset,
                    buffer: if append {
                        Buffer::Append
                    } else {
                        Buffer::Original
                    },
                    start,
                    len,
                    version,
                }),
            (any::<u16>(), any::<u16>(), 0_u8..=MAX_VERSION).prop_map(
                |(raw_start, raw_end, version)| Op::Remove {
                    raw_start,
                    raw_end,
                    version,
                }
            ),
        ]
    }

    fn desc_tokens(desc: PieceDesc) -> Vec<Token> {
        (desc.start..desc.end)
            .map(|offset| Token {
                buffer: desc.buffer,
                offset,
            })
            .collect()
    }

    fn piece_tokens(piece: Piece) -> Vec<Token> {
        desc_tokens(piece.desc())
    }

    fn bounded_pieces(treap: &Treap, version: u32, expected_tokens: usize) -> Vec<Piece> {
        treap
            .iter(version)
            .take(expected_tokens.saturating_add(1))
            .collect()
    }

    fn assert_matches(treap: &Treap, version: u32, expected: &[Token]) {
        let pieces = bounded_pieces(treap, version, expected.len());
        assert!(
            pieces.len() <= expected.len(),
            "iterator yielded too many pieces for {expected:?}: {pieces:?}"
        );

        for piece in &pieces {
            assert!(piece.start < piece.end, "empty piece yielded: {piece:?}");
            assert!(
                piece.add_at <= version && version < piece.del_at,
                "invisible piece yielded at version {version}: {piece:?}"
            );
        }

        let actual: Vec<_> = pieces.into_iter().flat_map(piece_tokens).collect();
        assert_eq!(actual, expected, "content mismatch at version {version}");
    }

    fn walk(
        treap: &Treap,
        at: usize,
        parent: usize,
        seen: &mut BTreeSet<usize>,
        inorder: &mut Vec<usize>,
    ) -> u64 {
        let Some(&node) = treap.slab.get(at) else {
            assert_eq!(at, NIL, "tree refers to a vacant slab slot");
            return 0;
        };

        assert!(seen.insert(at), "node {at} is reachable more than once");
        assert_eq!(node.prv, parent, "incorrect parent link at node {at}");
        assert!(
            node.val.start < node.val.end,
            "node {at} contains an empty piece"
        );

        for child in [node.lhs, node.rhs] {
            if child != NIL {
                assert!(
                    treap.pri_of(at) > treap.pri_of(child),
                    "heap order is broken between {at} and {child}"
                );
            }
        }

        let lhs_len = walk(treap, node.lhs, at, seen, inorder);
        inorder.push(at);
        let rhs_len = walk(treap, node.rhs, at, seen, inorder);
        let expected_len = if node.val.del_at == u32::MAX {
            lhs_len + node.val.desc().len() + rhs_len
        } else {
            0
        };

        if node.val.del_at == u32::MAX
            && let Some(cached_len) = node.len
        {
            assert_eq!(cached_len, expected_len, "stale cached length at node {at}");
        }

        expected_len
    }

    fn assert_invariants(treap: &mut Treap) {
        let mut seen = BTreeSet::new();
        let mut inorder = Vec::new();
        let root = treap.root;
        let expected_len = walk(treap, root, NIL, &mut seen, &mut inorder);

        assert_eq!(
            seen.len(),
            treap.slab.len(),
            "the slab contains unreachable nodes"
        );
        assert_eq!(root == NIL, treap.slab.is_empty());

        let mut linked = Vec::new();
        let mut cur = treap.leftmost(root);
        while cur != NIL {
            assert!(
                linked.len() < treap.slab.len(),
                "linked traversal contains a cycle"
            );
            linked.push(cur);
            cur = treap.next(cur);
        }
        assert_eq!(linked, inorder, "linked and structural traversal differ");

        assert_eq!(treap.len_of(root), expected_len);
        let mut checked = BTreeSet::new();
        let mut checked_inorder = Vec::new();
        assert_eq!(
            walk(treap, root, NIL, &mut checked, &mut checked_inorder),
            expected_len
        );
    }

    fn apply(op: Op, model: &mut Model, treap: &mut Treap) {
        match op {
            Op::Insert {
                raw_offset,
                buffer,
                start,
                len,
                version,
            } => {
                let version = u32::from(version);
                let mut document = model.branch(version);
                let offset = usize::from(raw_offset) % document.len().saturating_add(1);
                let start = u64::from(start);
                let desc = PieceDesc {
                    buffer,
                    start,
                    end: start + u64::from(len),
                };

                treap.insert(offset as u64, desc, version);
                document.splice(offset..offset, desc_tokens(desc));
                model.commit(version, document);
            }
            Op::Remove {
                raw_start,
                raw_end,
                version,
            } => {
                let version = u32::from(version);
                let mut document = model.branch(version);
                let bound = document.len().saturating_add(1);
                let lhs = usize::from(raw_start) % bound;
                let rhs = usize::from(raw_end) % bound;
                let (start, end) = if lhs <= rhs { (lhs, rhs) } else { (rhs, lhs) };

                treap.remove(start as u64, end as u64, version);
                document.drain(start..end);
                model.commit(version, document);
            }
        }
    }

    fn assert_all_versions(treap: &Treap, model: &Model) {
        for version in 0..=u32::from(MAX_VERSION) + 1 {
            assert_matches(treap, version, &model.at(version));
        }
    }

    proptest! {
        #[test]
        fn matches_snapshot_model(salt in any::<usize>(), ops in vec(arb_op(), 1..=64)) {
            let mut treap = Treap::new(salt, 0);
            let mut model = Model::new(0);

            for op in ops {
                apply(op, &mut model, &mut treap);
                assert_invariants(&mut treap);
                assert_all_versions(&treap, &model);
            }
        }

        #[test]
        fn split_conserves_content(
            salt in any::<usize>(),
            ops in vec(arb_op(), 0..=32),
            raw_offset in any::<u16>(),
            split_version in 0_u8..=MAX_VERSION,
        ) {
            let mut treap = Treap::new(salt, 0);
            let mut model = Model::new(0);

            for op in ops {
                apply(op, &mut model, &mut treap);
            }

            let split_version = u32::from(split_version);
            let document = model.branch(split_version);
            let offset = usize::from(raw_offset) % document.len().saturating_add(1);
            let mut right = treap.split_off(offset as u64, split_version);
            let (expected_left, expected_right) = document.split_at(offset);

            assert_invariants(&mut treap);
            assert_invariants(&mut right);
            assert_matches(&treap, split_version, expected_left);
            assert_matches(&right, split_version, expected_right);

            let mut reconstructed = Vec::from(expected_left);
            reconstructed.extend_from_slice(expected_right);
            prop_assert_eq!(reconstructed.as_slice(), document.as_slice());

            let next_version = split_version + 1;
            let left_desc = PieceDesc {
                buffer: Buffer::Append,
                start: 10_000,
                end: 10_002,
            };
            let right_desc = PieceDesc {
                buffer: Buffer::Original,
                start: 20_000,
                end: 20_001,
            };
            treap.insert(expected_left.len() as u64, left_desc, next_version);
            right.insert(0, right_desc, next_version);

            let mut edited_left = Vec::from(expected_left);
            edited_left.extend(desc_tokens(left_desc));
            let mut edited_right = desc_tokens(right_desc);
            edited_right.extend_from_slice(expected_right);

            assert_invariants(&mut treap);
            assert_invariants(&mut right);
            assert_matches(&treap, next_version, &edited_left);
            assert_matches(&right, next_version, &edited_right);
        }
    }

    #[test]
    fn iterator_advances_and_terminates() {
        let desc = PieceDesc {
            buffer: Buffer::Original,
            start: 0,
            end: 3,
        };
        let mut treap = Treap::new(0, 0);
        treap.insert(0, desc, 0);

        let pieces = bounded_pieces(&treap, 0, 3);
        assert_eq!(pieces, std::vec![desc.build(0)]);
    }

    #[test]
    fn empty_and_boundary_operations() {
        let mut treap = Treap::new(1, 0);
        treap.remove(0, 0, 0);
        assert_matches(&treap, 0, &[]);

        let mut other = treap.split_off(0, 0);
        assert_matches(&treap, 0, &[]);
        assert_matches(&other, 0, &[]);
        assert_invariants(&mut treap);
        assert_invariants(&mut other);

        let original = PieceDesc {
            buffer: Buffer::Original,
            start: 0,
            end: 6,
        };
        let prefix = PieceDesc {
            buffer: Buffer::Append,
            start: 100,
            end: 101,
        };
        let middle = PieceDesc {
            buffer: Buffer::Append,
            start: 101,
            end: 102,
        };
        let suffix = PieceDesc {
            buffer: Buffer::Append,
            start: 102,
            end: 103,
        };
        treap.insert(0, original, 0);
        treap.insert(0, prefix, 1);
        treap.insert(4, middle, 2);
        treap.insert(8, suffix, 3);

        let mut expected = desc_tokens(prefix);
        expected.extend(desc_tokens(PieceDesc {
            buffer: Buffer::Original,
            start: 0,
            end: 3,
        }));
        expected.extend(desc_tokens(middle));
        expected.extend(desc_tokens(PieceDesc {
            buffer: Buffer::Original,
            start: 3,
            end: 6,
        }));
        expected.extend(desc_tokens(suffix));
        assert_matches(&treap, 3, &expected);
        assert_invariants(&mut treap);
    }

    #[test]
    fn removals_preserve_history() {
        let original = PieceDesc {
            buffer: Buffer::Original,
            start: 0,
            end: 8,
        };
        let mut treap = Treap::new(2, 0);
        treap.insert(0, original, 0);
        treap.remove(0, 2, 1);
        treap.remove(2, 4, 2);
        treap.remove(2, 4, 3);
        treap.remove(0, 2, 4);

        assert_matches(&treap, 0, &desc_tokens(original));
        assert_matches(
            &treap,
            1,
            &desc_tokens(PieceDesc {
                buffer: Buffer::Original,
                start: 2,
                end: 8,
            }),
        );

        let mut version_two = desc_tokens(PieceDesc {
            buffer: Buffer::Original,
            start: 2,
            end: 4,
        });
        version_two.extend(desc_tokens(PieceDesc {
            buffer: Buffer::Original,
            start: 6,
            end: 8,
        }));
        assert_matches(&treap, 2, &version_two);
        assert_matches(
            &treap,
            3,
            &desc_tokens(PieceDesc {
                buffer: Buffer::Original,
                start: 2,
                end: 4,
            }),
        );
        assert_matches(&treap, 4, &[]);
        assert_invariants(&mut treap);
    }

    #[test]
    fn coarsens_only_compatible_neighbors() {
        let first = PieceDesc {
            buffer: Buffer::Original,
            start: 0,
            end: 2,
        };
        let second = PieceDesc {
            buffer: Buffer::Original,
            start: 2,
            end: 4,
        };
        let incompatible = PieceDesc {
            buffer: Buffer::Append,
            start: 0,
            end: 2,
        };
        let mut treap = Treap::new(3, 0);
        treap.insert(0, first, 0);
        treap.insert(2, second, 0);
        treap.insert(4, incompatible, 0);

        let pieces = bounded_pieces(&treap, 0, 6);
        assert_eq!(
            pieces,
            std::vec![
                PieceDesc {
                    buffer: Buffer::Original,
                    start: 0,
                    end: 4,
                }
                .build(0),
                incompatible.build(0),
            ]
        );
        assert_invariants(&mut treap);
    }

    #[test]
    fn split_at_boundaries_and_middle() {
        let original = PieceDesc {
            buffer: Buffer::Original,
            start: 0,
            end: 6,
        };
        let expected = desc_tokens(original);
        let mut whole = Treap::new(4, 0);
        whole.insert(0, original, 0);

        let mut left_empty = whole.clone();
        let mut right_whole = left_empty.split_off(0, 0);
        assert_matches(&left_empty, 0, &[]);
        assert_matches(&right_whole, 0, &expected);
        assert_invariants(&mut left_empty);
        assert_invariants(&mut right_whole);

        let mut left_whole = whole.clone();
        let mut right_empty = left_whole.split_off(6, 0);
        assert_matches(&left_whole, 0, &expected);
        assert_matches(&right_empty, 0, &[]);
        assert_invariants(&mut left_whole);
        assert_invariants(&mut right_empty);

        let mut left = whole;
        let mut right = left.split_off(3, 0);
        assert_matches(&left, 0, &expected[..3]);
        assert_matches(&right, 0, &expected[3..]);
        assert_invariants(&mut left);
        assert_invariants(&mut right);
    }

    fn descriptor() -> PieceDesc {
        PieceDesc {
            buffer: Buffer::Original,
            start: 0,
            end: 1,
        }
    }

    #[test]
    #[should_panic(expected = "invalid version constant")]
    fn new_rejects_reserved_version() {
        _ = Treap::new(0, u32::MAX);
    }

    #[test]
    #[should_panic(expected = "invalid version constant")]
    fn insert_rejects_reserved_version() {
        Treap::new(0, 0).insert(0, descriptor(), u32::MAX);
    }

    #[test]
    #[should_panic(expected = "invalid descriptor range")]
    fn insert_rejects_empty_descriptor() {
        Treap::new(0, 0).insert(
            0,
            PieceDesc {
                buffer: Buffer::Original,
                start: 1,
                end: 1,
            },
            0,
        );
    }

    #[test]
    #[should_panic(expected = "invalid descriptor range")]
    fn insert_rejects_reversed_descriptor() {
        Treap::new(0, 0).insert(
            0,
            PieceDesc {
                buffer: Buffer::Original,
                start: 2,
                end: 1,
            },
            0,
        );
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn insert_rejects_out_of_bounds_offset() {
        Treap::new(0, 0).insert(1, descriptor(), 0);
    }

    #[test]
    #[should_panic(expected = "invalid version constant")]
    fn remove_rejects_reserved_version() {
        Treap::new(0, 0).remove(0, 0, u32::MAX);
    }

    #[test]
    #[should_panic(expected = "invalid removal range")]
    fn remove_rejects_reversed_range() {
        Treap::new(0, 0).remove(1, 0, 0);
    }

    #[test]
    #[should_panic(expected = "offset out of bounds")]
    fn remove_rejects_out_of_bounds_end() {
        Treap::new(0, 0).remove(0, 1, 0);
    }

    #[test]
    #[should_panic(expected = "offset out of bounds")]
    fn remove_rejects_empty_out_of_bounds_range() {
        Treap::new(0, 0).remove(1, 1, 0);
    }

    #[test]
    #[should_panic(expected = "invalid version constant")]
    fn split_rejects_reserved_version() {
        _ = Treap::new(0, 0).split_off(0, u32::MAX);
    }

    #[test]
    #[should_panic(expected = "offset out of bounds")]
    fn split_rejects_out_of_bounds_offset() {
        _ = Treap::new(0, 0).split_off(1, 0);
    }
}
