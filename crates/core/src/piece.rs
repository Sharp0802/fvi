//! A module providing types related to piece.

use crate::collections::table::Version;

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
    pub const fn build(self, add_at: Version) -> Piece {
        Piece {
            buffer: self.buffer,
            start: self.start,
            end: self.end,
            add_at,
            del_at: None,
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
    /// An editing version counter for insertion.
    pub add_at: Version,
    /// An editing version counter for removal.
    pub del_at: Option<Version>,
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

    // cannot be `pub`, since it assumes
    // current version as most recent version.
    #[must_use]
    pub(super) const fn len(&self) -> u64 {
        if self.del_at.is_some() {
            0
        } else {
            self.desc().len()
        }
    }

    /// Returns whether this [`Piece`] is visible at given version.
    #[must_use]
    pub fn is_visible_at(&self, version: Version) -> bool {
        self.add_at <= version && self.del_at.is_none_or(|t| version < t)
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
            && self.add_at == other.add_at
            && self.del_at == other.del_at
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustive() {
        let desc = PieceDesc {
            buffer: Buffer::Original,
            start: 0xCAFE,
            end: 0xCAFE,
        };

        assert!(desc.is_empty());
    }
}
