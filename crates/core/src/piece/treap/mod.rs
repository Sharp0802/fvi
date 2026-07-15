use core::cmp::Ordering;

use crate::piece::ptr::Ptr;
use crate::piece::{Node, Orphan, Slab};
use crate::unreachable;
use crate::view::{View, ViewSize};
pub use iter::Iter;
use monad::Key;

mod iter;
mod monad;

const MAX: u16 = 1024;

/// A piece table based on implicit treap,
/// able to hold up to 1024 pieces.
#[derive(Debug)]
pub struct Pieces {
    root: Ptr,
    salt: u16,
    slab: Slab,
}

impl Pieces {
    #[inline]
    #[must_use]
    pub(super) const fn salt(&self) -> u16 {
        self.salt
    }

    #[inline]
    pub(in crate::piece::treap) fn get<T: Key>(&self, at: T) -> T::Output<&Node> {
        at.get_from(&self.slab)
    }

    #[inline]
    pub(in crate::piece::treap) fn get_mut<T: Key>(&mut self, at: T) -> T::Output<&mut Node> {
        at.get_mut_from(&mut self.slab)
    }
}

impl Pieces {
    /// Creates a new piece treap with given salt constant.
    ///
    /// Salt is assumed as it's unique over treaps,
    /// Although it's not checked in this function.
    #[inline]
    #[must_use]
    pub fn new(salt: u16) -> Self {
        Self {
            root: Ptr::NIL,
            salt,
            slab: Slab::new(MAX),
        }
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
    pub fn len(&self) -> u32 {
        self.len_of_impl(self.root)
    }

    /// Returns total byte length of given node.
    ///
    /// # Panics
    ///
    /// Panics if given node is not for `self`.
    /// See [`Orphan::is_for`].
    #[inline]
    #[must_use]
    pub fn len_of(&self, orphan: &Orphan) -> u32 {
        assert!(orphan.is_for(self));
        self.len_of_impl(orphan.ptr())
    }

    /// Returns in-order traversal iterator.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self, self.root)
    }

    /// Returns in-order traversal iterator of given node.
    ///
    /// # Panics
    ///
    /// Panics if given node is not for `self`.
    /// See [`Orphan::is_for`].
    #[inline]
    #[must_use]
    pub fn iter_of(&self, orphan: &Orphan) -> Iter<'_> {
        assert!(orphan.is_for(self));
        Iter::new(self, orphan.ptr())
    }

    /// Returns whether the piece with given size can be inserted into `self`.
    #[inline]
    #[must_use]
    pub fn can_insert(&self, size: ViewSize) -> bool {
        // NOTE: - two slots (a) for splits during insertion,
        //       - two slots (b) for splits during insert_orphan / remove.
        //       - last two slots (b) are reserved not to suprisingly panic in insert_orphan and remove.
        let Some(total) = self.len().checked_add(size.get()) else {
            return false;
        };

        total <= ViewSize::MAX.get() && self.slab.len() <= MAX - 4
    }

    /// Returns whether the given orphan can be inserted into `self`.
    ///
    /// # Panics
    ///
    /// Panics if given orphan is not for `self`.
    /// See [`Orphan::is_for`].
    #[inline]
    #[must_use]
    pub fn can_insert_orphan(&self, orphan: &Orphan) -> bool {
        assert!(orphan.is_for(self));
        let Some(total) = self.len().checked_add(self.len_of(orphan)) else {
            return false;
        };

        total <= ViewSize::MAX.get()
    }

    /// Returns whether the remove operation is available.
    ///
    /// Note that remove operation also causes allocation on slab slot,
    /// Since [`Pieces::remove`] splits piece in byte offset, not piece index.
    #[inline]
    #[must_use]
    pub const fn can_remove(&self) -> bool {
        self.slab.len() <= MAX - 2
    }

    /// Inserts a piece desc at given byte offset.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Given offset is not in this treap.
    /// - Cannot insert given view. See [`Pieces::can_insert`].
    pub fn insert(&mut self, off: u16, view: View) {
        assert!(u32::from(off) <= self.len());
        assert!(self.can_insert(view.size()));

        let (lhs, rhs) = self.split(self.root, off);
        let mid = self.new_leaf(Ptr::NIL, view);
        debug_assert!(!mid.is_nil());

        let tmp = self.concat(lhs, mid);
        self.root = self.concat(tmp, rhs);
    }

    /// Inserts an orphan node at given byte offset.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions are met:
    ///
    /// - Given node is not for this treap. See [`Orphan::is_for`].
    /// - Given offset is not in this treap.
    /// - Cannot insert given node. See [`Pieces::can_insert_orphan`].
    #[expect(
        clippy::needless_pass_by_value,
        reason = "orphan must be consumed not to cause double-free"
    )]
    pub fn insert_orphan(&mut self, off: u16, orphan: Orphan) {
        assert!(orphan.is_for(self));

        if orphan.ptr().is_nil() {
            return;
        }

        assert!(self.can_insert_orphan(&orphan));
        assert!(u32::from(off) <= self.len());

        let (lhs, rhs) = self.split(self.root, off);
        let tmp = self.concat(lhs, orphan.ptr());
        self.root = self.concat(tmp, rhs);
    }

    /// Removes a range from the treap, returning orphaned node.
    ///
    /// Returns `None` if the requested range is not fully included in the treap.
    ///
    /// # Panics
    ///
    /// Panics if the operation is not available. See [`Pieces::can_remove`].
    pub fn remove(&mut self, off: u16, len: ViewSize) -> Option<Orphan> {
        assert!(self.can_remove());

        let off32 = u32::from(off);
        let end = off32 + len.get();

        match end.cmp(&self.len()) {
            Ordering::Greater => None,
            Ordering::Equal => {
                let (rst, rhs) = self.split(self.root, off);
                self.root = rst;
                Some(Orphan::new(self.salt, rhs))
            }
            Ordering::Less => {
                #[expect(clippy::cast_possible_truncation, reason = "end < PageSize::MAX")]
                let (rst, rhs) = self.split(self.root, end as u16);
                let (lhs, mid) = self.split(rst, off);
                self.root = self.concat(lhs, rhs);
                Some(Orphan::new(self.salt, mid))
            }
        }
    }

    /// Removes an orphan node.
    ///
    /// # Panics
    ///
    /// Panics if given node is not for `self`.
    /// See [`Orphan::is_for`].
    #[expect(
        clippy::needless_pass_by_value,
        reason = "orphan must be consumed not to cause double-free"
    )]
    pub fn kill(&mut self, orphan: Orphan) {
        assert!(orphan.is_for(self));
        self.kill_impl(orphan.ptr());
    }
}

#[inline]
#[must_use]
const fn pri_of(mut v: u16, salt: u16) -> u16 {
    v ^= salt;

    v ^= v >> 7;
    v = v.wrapping_mul(0x9E37);
    v ^= v >> 9;
    v = v.wrapping_mul(0xBB67);
    v ^= v >> 8;
    v
}

impl Pieces {
    #[must_use]
    fn len_of_impl(&self, at: Ptr) -> u32 {
        self.get(at).map_or(0, |(t, _)| t.size.get())
    }

    fn update(&mut self, at: Ptr) {
        let Some((v, key)) = self.get(at) else { return };

        let sum = v
            .desc
            .size()
            .strict_add(self.len_of_impl(v.lhs))
            .strict_add(self.len_of_impl(v.rhs));

        self.get_mut(key).size = sum;
    }

    #[must_use]
    const fn new_leaf(&mut self, prv: Ptr, desc: View) -> Ptr {
        self.slab.insert(Node {
            size: desc.size(),
            prv,
            lhs: Ptr::NIL,
            rhs: Ptr::NIL,
            desc,
        })
    }

    fn kill_impl(&mut self, t: Ptr) {
        let Some((v, _)) = self.get(t) else { return };
        let (lhs, rhs) = (v.lhs, v.rhs);
        self.kill_impl(lhs);
        self.kill_impl(rhs);
        _ = self.slab.remove(t);
    }

    #[must_use]
    fn split(&mut self, at: Ptr, pos: u16) -> (Ptr, Ptr) {
        let Some((v, key)) = self.get(at) else {
            return (Ptr::NIL, Ptr::NIL);
        };

        let l_len = self.len_of_impl(v.lhs);
        let d_len = v.desc.size();

        match u32::from(pos).checked_sub(l_len) {
            None | Some(0) => {
                let (a, b) = self.split(v.lhs, pos);
                self.get_mut(key).lhs = b;
                self.update(at);

                if let Some((av, _)) = self.get_mut(a) {
                    av.prv = Ptr::NIL;
                }

                if let Some((bv, _)) = self.get_mut(b) {
                    bv.prv = at;
                }

                (a, at)
            }
            Some(mid) if d_len > mid => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "(pos > l_len) where (pos: u16)"
                )]
                let Some((l_desc, r_desc)) = v.desc.split_at(mid as u16) else {
                    unreachable();
                };

                let rhs_old = v.rhs;
                if let Some((rv, _)) = self.get_mut(rhs_old) {
                    rv.prv = Ptr::NIL;
                }

                let rhs_new = self.new_leaf(Ptr::NIL, r_desc);
                debug_assert!(!rhs_new.is_nil(), "insertion may breaks invariants");
                let rhs_new = self.merge(rhs_new, rhs_old);

                let node = self.get_mut(key);
                node.desc = l_desc;
                node.prv = Ptr::NIL;
                node.rhs = Ptr::NIL;

                self.update(at);

                (at, rhs_new)
            }
            Some(mid) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "(pos >= l_len + d_len) where (pos: u16)"
                )]
                let next_pos = (mid - d_len.get()) as u16;

                let (a, b) = self.split(v.rhs, next_pos);
                self.get_mut(key).rhs = a;
                self.update(at);

                if let Some((av, _)) = self.get_mut(a) {
                    av.prv = at;
                }

                if let Some((bv, _)) = self.get_mut(b) {
                    bv.prv = Ptr::NIL;
                }

                (at, b)
            }
        }
    }

    #[must_use]
    fn merge(&mut self, a: Ptr, b: Ptr) -> Ptr {
        match (self.get(a), self.get(b)) {
            (None, _) => b,
            (_, None) => a,
            (Some((_, ak)), Some((_, _)))
                if pri_of(a.raw(), self.salt) > pri_of(b.raw(), self.salt) =>
            {
                let rhs = self.merge(self.get(ak).rhs, b);
                self.get_mut(ak).rhs = rhs;
                self.update(a);

                if let Some((rv, _)) = self.get_mut(rhs) {
                    rv.prv = a;
                }

                a
            }
            (Some(_), Some((_, bk))) => {
                let lhs = self.merge(a, self.get(bk).lhs);
                self.get_mut(bk).lhs = lhs;
                self.update(b);

                if let Some((lv, _)) = self.get_mut(lhs) {
                    lv.prv = b;
                }

                b
            }
        }
    }

    fn propagate(&mut self, mut at: Ptr) {
        while let Some((v, _)) = self.get(at) {
            let prv = v.prv;
            self.update(at);
            at = prv;
        }
    }

    #[must_use]
    pub(super) fn leftmost(&self, mut at: Ptr) -> Ptr {
        while let Some((v, _)) = self.get(at) {
            if v.lhs.is_nil() {
                break;
            }

            at = v.lhs;
        }

        at
    }

    #[must_use]
    fn rightmost(&self, mut at: Ptr) -> Ptr {
        while let Some((v, _)) = self.get(at) {
            if v.rhs.is_nil() {
                break;
            }

            at = v.rhs;
        }

        at
    }

    #[must_use]
    fn pop_leftmost(&mut self, at: Ptr) -> (Ptr, Ptr) {
        let lhs = self.leftmost(at);

        // NOTE: (lhs = NIL) iff (at = NIL)
        let Some((v, _)) = self.get_mut(lhs) else {
            return (Ptr::NIL, Ptr::NIL);
        };

        let rhs = core::mem::replace(&mut v.rhs, Ptr::NIL);
        let prv = core::mem::replace(&mut v.prv, Ptr::NIL);
        self.update(lhs);

        if let Some((rv, _)) = self.get_mut(rhs) {
            rv.prv = prv;
        }

        if let Some((pv, _)) = self.get_mut(prv) {
            pv.lhs = rhs;
            self.propagate(prv);
        }

        if lhs == at { (lhs, rhs) } else { (lhs, at) }
    }

    #[must_use]
    fn concat(&mut self, a: Ptr, b: Ptr) -> Ptr {
        let Some((arv, ark)) = self.get(self.rightmost(a)) else {
            return b;
        };

        let Some((blv, _)) = self.get(self.leftmost(b)) else {
            return a;
        };

        if let Some(coarsen) = arv.desc.coarsen(&blv.desc) {
            let (b_first, b_rest) = self.pop_leftmost(b);

            self.get_mut(ark).desc = coarsen;
            self.propagate(*ark);

            let rem = self.slab.remove(b_first);
            debug_assert!(rem.is_some());

            self.merge(a, b_rest)
        } else {
            self.merge(a, b)
        }
    }
}

impl<'a> IntoIterator for &'a Pieces {
    type Item = &'a View;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::num::NonZero;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;
    use std::vec::Vec;

    #[derive(Debug, Clone, Copy)]
    enum Op {
        Insert { off: u16, view: Key },
        Remove { off: u16, len: u16 },
        InsertOrphan { orphan: u16, off: u16 },
        Kill { orphan: u16 },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Key {
        buf: bool,
        ver: u32,
        off: u16,
        size: u32,
    }

    #[derive(Debug)]
    struct StoredOrphan {
        actual: Orphan,
        model: Vec<Key>,
    }

    impl Key {
        #[must_use]
        fn to_view(self) -> View {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "generated test sizes are smaller than u16::MAX"
            )]
            let size = NonZero::new(self.size as u16)
                .map(ViewSize::from)
                .expect("nonzero size expected");
            View::new(self.buf, self.ver, self.off, size).expect("size overflows")
        }
    }

    impl From<View> for Key {
        fn from(value: View) -> Self {
            Self {
                buf: value.is_original(),
                ver: value.ver(),
                off: value.off(),
                size: value.size().get(),
            }
        }
    }

    fn arb_key() -> impl Strategy<Value = Key> {
        (0..=1, 0u32..=0x1FFF_FFFF, 1u32..=32).prop_map(|(buf, ver, size)| Key {
            buf: buf == 1,
            ver,
            off: 0,
            size,
        })
    }

    fn arb_op() -> impl Strategy<Value = Op> {
        prop_oneof![
            5 => (any::<u16>(), arb_key()).prop_map(|(off, view)| Op::Insert { off, view }),
            3 => (any::<u16>(), any::<u16>()).prop_map(|(off, len)| Op::Remove { off, len }),
            2 => (any::<u16>(), any::<u16>()).prop_map(|(orphan, off)| Op::InsertOrphan { orphan, off }),
            1 => any::<u16>().prop_map(|orphan| Op::Kill { orphan }),
        ]
    }

    #[must_use]
    fn model_len(model: &[Key]) -> u32 {
        model.iter().map(|v| v.size).sum()
    }

    #[must_use]
    fn actual_vec(actual: &Pieces) -> Vec<Key> {
        actual.iter().copied().map(Key::from).collect()
    }

    fn normalize(model: &mut Vec<Key>) {
        let mut ret = Vec::new();

        for next in model.drain(..) {
            let Some(last) = ret.last_mut() else {
                ret.push(next);
                continue;
            };

            if last.buf == next.buf
                && last.ver == next.ver
                && u32::from(last.off) + last.size == u32::from(next.off)
                && last.size + next.size <= ViewSize::MAX.get()
            {
                last.size += next.size;
            } else {
                ret.push(next);
            }
        }

        *model = ret;
    }

    fn push_split(dst: &mut Vec<Key>, key: Key, at: u32) {
        debug_assert!(at <= key.size);

        if at == 0 || at == key.size {
            dst.push(key);
        } else {
            dst.push(Key { size: at, ..key });
            #[expect(
                clippy::cast_possible_truncation,
                reason = "model keys are generated inside u16 page bounds"
            )]
            dst.push(Key {
                off: key.off + at as u16,
                size: key.size - at,
                ..key
            });
        }
    }

    #[must_use]
    fn split_model(model: &[Key], at: u32) -> (Vec<Key>, Vec<Key>) {
        let mut lhs = Vec::new();
        let mut rhs = Vec::new();
        let mut pos = 0;

        for &key in model {
            let end = pos + key.size;

            if end <= at {
                lhs.push(key);
            } else if at <= pos {
                rhs.push(key);
            } else {
                let mid = at - pos;
                push_split(&mut lhs, key, mid);
                if let Some(last) = lhs.pop() {
                    if last.size == key.size {
                        lhs.push(last);
                    } else {
                        rhs.push(last);
                    }
                }
            }

            pos = end;
        }

        (lhs, rhs)
    }

    fn insert_model(model: &mut Vec<Key>, off: u32, key: Key) {
        let (mut lhs, rhs) = split_model(model, off);
        lhs.push(key);
        lhs.extend(rhs);
        normalize(&mut lhs);
        *model = lhs;
    }

    #[must_use]
    fn remove_model(model: &mut Vec<Key>, off: u32, len: u32) -> Vec<Key> {
        let (lhs, rhs) = split_model(model, off);
        let (mut mid, tail) = split_model(&rhs, len);
        let mut next = lhs;
        next.extend(tail);
        normalize(&mut next);
        normalize(&mut mid);
        *model = next;
        mid
    }

    fn assert_model(
        actual: &Pieces,
        model: &[Key],
        orphans: &[Option<StoredOrphan>],
    ) -> TestCaseResult {
        prop_assert_eq!(actual.len(), model_len(model));
        prop_assert_eq!(actual.is_empty(), model.is_empty());
        prop_assert_eq!(actual_vec(actual), model);

        for stored in orphans.iter().flatten() {
            prop_assert_eq!(actual.len_of(&stored.actual), model_len(&stored.model));
            let orphan_actual: Vec<_> = actual
                .iter_of(&stored.actual)
                .copied()
                .map(Key::from)
                .collect();
            prop_assert_eq!(orphan_actual.as_slice(), stored.model.as_slice());
        }

        Ok(())
    }

    proptest! {
        #[test]
        fn match_models(ops in vec(arb_op(), 1..256)) {
            let mut actual = Pieces::new(0xCAFE);
            let mut model = Vec::new();
            let mut orphans: Vec<Option<StoredOrphan>> = Vec::new();

            for op in ops {
                match op {
                    Op::Insert { off, view } => {
                        let len = model_len(&model);
                        let off = if len == 0 { 0 } else { u32::from(off) % (len + 1) };

                        if let Ok(off16) = u16::try_from(off)
                            && len + view.size <= ViewSize::MAX.get()
                            && actual.can_insert(view.to_view().size())
                        {
                            actual.insert(off16, view.to_view());
                            insert_model(&mut model, off, view);
                        }
                    }
                    Op::Remove { off, len } => {
                        let total = model_len(&model);
                        if total == 0 {
                            assert_model(&actual, &model, &orphans)?;
                            continue;
                        }

                        let off = u32::from(off) % total;
                        let len = u32::from(len) % (total - off) + 1;

                        if let Ok(off16) = u16::try_from(off)
                            && len <= ViewSize::MAX.get()
                        {
                            #[expect(
                                clippy::cast_possible_truncation,
                                reason = "generated total length stays below u16::MAX"
                            )]
                            let len_size = NonZero::new(len as u16).map(ViewSize::from).expect("size overflows");
                            let orphan = actual.remove(off16, len_size);
                            prop_assert!(orphan.is_some());
                            let model_orphan = remove_model(&mut model, off, len);
                            orphans.push(Some(StoredOrphan {
                                actual: orphan.expect("empty slot removed"),
                                model: model_orphan,
                            }));
                        }
                    }
                    Op::InsertOrphan { orphan, off } => {
                        if orphans.is_empty() {
                            assert_model(&actual, &model, &orphans)?;
                            continue;
                        }

                        let index = usize::from(orphan) % orphans.len();
                        let Some(stored) = orphans[index].take() else {
                            assert_model(&actual, &model, &orphans)?;
                            continue;
                        };

                        let len = model_len(&model);
                        let off = if len == 0 { 0 } else { u32::from(off) % (len + 1) };

                        if let Ok(off16) = u16::try_from(off)
                            && len + model_len(&stored.model) <= ViewSize::MAX.get()
                            && actual.can_insert_orphan(&stored.actual)
                        {
                            actual.insert_orphan(off16, stored.actual);
                            let (mut lhs, rhs) = split_model(&model, off);
                            lhs.extend(stored.model);
                            lhs.extend(rhs);
                            normalize(&mut lhs);
                            model = lhs;
                        } else {
                            orphans[index] = Some(stored);
                        }
                    }
                    Op::Kill { orphan } => {
                        if !orphans.is_empty() {
                            let index = usize::from(orphan) % orphans.len();
                            if let Some(stored) = orphans[index].take() {
                                actual.kill(stored.actual);
                            }
                        }
                    }
                }

                assert_model(&actual, &model, &orphans)?;
            }
        }
    }
}
