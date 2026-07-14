use crate::piece::{Pieces, ptr::Ptr};

/// An orphaned node from [`Pieces`].
///
/// It's required to call [`Pieces::kill`] or [`Pieces::insert_orphan`]
/// to avoid memory leaks because it doesn't release its slot
/// in [`Pieces`] automatically.
#[must_use]
#[derive(Debug)]
pub struct Orphan {
    salt: u16,
    slot: Ptr,
}

impl Orphan {
    #[inline]
    pub(super) const fn new(salt: u16, slot: Ptr) -> Self {
        Self { salt, slot }
    }

    #[inline]
    #[must_use]
    pub(super) const fn ptr(&self) -> Ptr {
        self.slot
    }

    /// Returns whether the `self` is allocated from `owner`.
    ///
    /// Note that it just checks baked salt of both `self` and `owner`,
    /// so duplicated salt can cause invalid behaviour.
    #[inline]
    #[must_use]
    pub const fn is_for(&self, owner: &Pieces) -> bool {
        self.salt == owner.salt()
    }
}
