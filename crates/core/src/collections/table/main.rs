use crate::collections::Treap;
use crate::collections::treap::Iter;
use crate::piece::PieceDesc;

/// An editing context.
#[derive(Debug, Clone)]
pub struct Context {
    /// A version of current edit.
    pub version: u32,
}

/// A piece table based on [`Treap`].
#[derive(Debug, Clone)]
pub struct Table {
    min_version: u32,
    max_version: u32,
    treap: Treap,
}

impl Table {
    /// Inserts given descriptor at specified offset with given context.
    ///
    /// If version of context is mismatched with latest version of this [`Table`],
    /// It'll be discarded that edits whose version is later than context version.
    ///
    /// # Panics
    ///
    /// Panics if given offset is not in this [`Table`].
    pub fn insert(&mut self, cx: &Context, off: u64, desc: PieceDesc) {
        self.update(cx.version);
        self.treap.insert(off, desc, cx.version);
    }

    /// Removes specified range with given context.
    ///
    /// If version of context is mismatched with latest version of this [`Table`],
    /// It'll be discarded that edits whose version is later than context version.
    pub fn remove(&mut self, cx: &Context, start: u64, end: u64) {
        self.update(cx.version);
        self.treap.remove(start, end, cx.version);
    }

    /// Returns iterator of piece desc for given context.
    #[must_use]
    pub const fn iter<'a>(&'a self, cx: &Context) -> Iter<'a> {
        self.treap.iter(cx.version)
    }
}

impl Table {
    fn update(&mut self, current: u32) {
        self.min_version = self.min_version.min(current);
        let old_max = core::mem::replace(&mut self.max_version, current);
        if current < old_max {
            self.treap.prune(self.min_version, self.max_version);
        }
    }
}

impl Table {
    /// Creates a new empty [`Table`].
    #[must_use]
    pub const fn new(salt: usize, cx: &Context) -> Self {
        Self {
            min_version: cx.version,
            max_version: cx.version,
            treap: Treap::new(salt),
        }
    }

    /// Creates a new [`Table`] from given descriptor.
    #[must_use]
    pub fn load(salt: usize, cx: &Context, desc: PieceDesc) -> Self {
        let mut treap = Treap::new(salt);
        treap.insert(0, desc, cx.version);

        Self {
            min_version: cx.version,
            max_version: cx.version,
            treap,
        }
    }
}
