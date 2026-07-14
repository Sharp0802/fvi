use core::fmt::Debug;
use core::num::NonZero;

use crate::unreachable;
use crate::view::ViewSize;

/// A logical view struct pointing a page of buffer.
#[derive(Clone, Copy)]
pub struct View {
    grp: NonZero<u32>,
    off: u16,
    size: ViewSize,
}

impl View {
    /// Creates a new [`View`].
    ///
    /// Returns `None` if any of the following conditions are met:
    ///
    /// - `size + off` logically exceeds 64KiB.
    /// - `ver` is greater than `0x3FFF_FFFF`.
    #[inline]
    #[must_use]
    pub fn new(original: bool, ver: u32, off: u16, size: ViewSize) -> Option<Self> {
        if ver > 0x3FFF_FFFF || size.checked_add(off).is_none() {
            None
        } else {
            let grp = (1 << 31) | (u32::from(original) << 30) | ver;
            let grp = NonZero::new(grp).unwrap_or_else(|| unreachable());
            Some(Self { grp, off, size })
        }
    }

    /// Returns whether the `self` points original buffer.
    #[inline]
    #[must_use]
    pub const fn is_original(&self) -> bool {
        (self.grp.get() >> 30) & 1 == 1
    }

    /// Returns the version of view,
    /// corresponding to undo buffer index.
    #[inline]
    #[must_use]
    pub const fn ver(&self) -> u32 {
        self.grp.get() & 0x3FFF_FFFF
    }

    /// Returns starting offset on the page.
    #[inline]
    #[must_use]
    pub const fn off(&self) -> u16 {
        self.off
    }

    /// Returns ending offset of view range on the page.
    #[inline]
    #[must_use]
    pub fn end(&self) -> ViewSize {
        self.size
            .checked_add(self.off)
            .unwrap_or_else(|| unreachable())
    }

    /// Returns size of view.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> ViewSize {
        self.size
    }

    /// Returns whether `self` precedes to `other`.
    ///
    /// Preceding means:
    ///
    /// - `self` points same buffer with `other`.
    /// - `self` has same version with `other`.
    /// - An ending offset of `self` is same with starting offset of `other`.
    #[inline]
    #[must_use]
    pub fn precedes(&self, other: &Self) -> bool {
        self.grp == other.grp && self.end() == other.off
    }

    /// Coarsen `self` with `other`.
    ///
    /// Returns `None` if `self` doesn't precede to `other`.
    /// See [`View::precedes`].
    #[inline]
    #[must_use]
    pub fn coarsen(&self, other: &Self) -> Option<Self> {
        if self.precedes(other) {
            Some(Self {
                grp: self.grp,
                off: self.off,
                // NOTE: s.size + s.off = o.off && o.size + o.off <= 64KiB;
                //       => o.size + s.size + s.off <= 64KiB.
                size: self.size().strict_add(other.size().get()),
            })
        } else {
            None
        }
    }

    /// Splits `self` at local offset, `off`.
    ///
    /// Returns `None` if `off` is out of bounds or
    /// the spliting itself creates empty view.
    #[inline]
    #[must_use]
    pub fn split_at(&self, off: u16) -> Option<(Self, Self)> {
        if let Some(off) = NonZero::new(off)
            && off.get() < self.size()
        {
            Some((
                Self {
                    grp: self.grp,
                    off: self.off,
                    size: off.into(),
                },
                Self {
                    grp: self.grp,
                    off: off.get() + self.off,
                    size: self.size().strict_sub(off.get()),
                },
            ))
        } else {
            None
        }
    }
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "grp is splited into buf and ver exhaustively"
)]
impl Debug for View {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("View")
            .field("is_original", &self.is_original())
            .field("ver", &self.ver())
            .field("off", &self.off)
            .field("size", &self.size)
            .finish()
    }
}
