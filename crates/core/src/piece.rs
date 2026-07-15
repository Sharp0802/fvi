//! A module providing types related to piece.

/// An enumeration representing type of buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Buffer {
    /// An original buffer.
    Original,
    /// An append buffer.
    Append,
}

/// A descriptor struct of the [`Piece`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PieceDesc {
    /// A referenced buffer.
    pub buffer: Buffer,
    /// An inclusive starting offset on buffer, in bytes.
    pub start: u64,
    /// An exclusive ending offset on buffer, in bytes.
    pub end: u64,
}

impl PieceDesc {
    /// Builds a [`Piece`] from this [`PieceDesc`] using specified states.
    #[must_use]
    pub const fn build(self, deleted: bool, version: u32) -> Piece {
        Piece {
            buffer: self.buffer,
            start: self.start,
            end: self.end,
            deleted,
            version,
        }
    }

    /// Returns the length of this [`PieceDesc`], in bytes.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.end - self.start
    }

    /// Returns the emptiness of this [`PieceDesc`].
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.end == self.start
    }
}

/// A piece node stored in treap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    /// A referenced buffer.
    pub buffer: Buffer,
    /// An inclusive starting offset on buffer, in bytes.
    pub start: u64,
    /// An exclusive ending offset on buffer, in bytes.
    pub end: u64,
    /// Whether the this [`Piece`] is in deleted state.
    pub deleted: bool,
    /// An editing version counter.
    pub version: u32,
}

impl Piece {
    /// Returns a [`PieceDesc`] corresponding to this [`Piece`].
    #[must_use]
    pub const fn desc(&self) -> PieceDesc {
        PieceDesc {
            buffer: self.buffer,
            start: self.start,
            end: self.end,
        }
    }

    /// Returns the length of this [`Piece`], in bytes.
    #[must_use]
    #[expect(
        clippy::len_without_is_empty,
        reason = r#"
            semantic of this len() is not appropriate for
            is_empty() since it varies by field `deleted`
        "#
    )]
    pub const fn len(&self) -> u64 {
        if self.deleted { 0 } else { self.desc().len() }
    }

    /// Splits [`Piece`] at given relative offset.
    ///
    /// Note that this function doesn't check
    /// whether the given offset is in piece bounds.
    #[must_use]
    pub const fn split_at(&self, at: u64) -> (Self, Self) {
        (
            {
                let mut this = *self;
                this.end = this.start + at;
                this
            },
            {
                let mut this = *self;
                this.start += at;
                this
            },
        )
    }

    /// Returns whether `self` precedes to `other`.
    #[must_use]
    pub fn precede(&self, other: &Self) -> bool {
        let tmp = self.end == other.start;
        tmp && self.buffer == other.buffer
            && self.deleted == other.deleted
            && self.version == other.version
    }

    /// Coarsen given two [`Piece`],
    /// returning `None` if `self` doesn't precede to `other`.
    #[must_use]
    pub fn coarsen(&self, other: &Self) -> Option<Self> {
        if self.precede(other) {
            let mut this = *self;
            this.end = other.end;
            Some(this)
        } else {
            None
        }
    }
}

impl From<Piece> for PieceDesc {
    fn from(value: Piece) -> Self {
        value.desc()
    }
}
