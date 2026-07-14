use core::fmt::Debug;
use core::num::NonZero;

use crate::unreachable;
use crate::view::ViewSize;

/// A logical view struct pointing a page of buffer.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct View {
    grp: NonZero<u16>,
    off: u16,
    size: ViewSize,
}

const _: () = const {
    assert!(size_of::<View>() == size_of::<Option<View>>());
};

impl View {
    /// Creates a new [`View`].
    ///
    /// Returns `None` if any of the following conditions are met:
    ///
    /// - `size + off` logically exceeds 64KiB.
    /// - `buf` is greater than `0x07`.
    /// - `ver` is greater than `0x1FFF`.
    #[inline]
    #[must_use]
    pub fn new(buf: NonZero<u8>, ver: u16, off: u16, size: ViewSize) -> Option<Self> {
        if buf.get() > 0x07 || ver > 0x1FFF || size.checked_add(off).is_none() {
            None
        } else {
            let grp =
                NonZero::new(u16::from(buf.get()) << 13 | ver).unwrap_or_else(|| unreachable());
            Some(Self { grp, off, size })
        }
    }

    /// Returns corresponding buffer index.
    #[inline]
    #[must_use]
    pub const fn buf(&self) -> u8 {
        (self.grp.get() >> 13) as u8
    }

    /// Returns the version of view,
    /// corresponding to undo buffer index.
    #[inline]
    #[must_use]
    pub const fn ver(&self) -> u16 {
        self.grp.get() & 0x1FFF
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
            .field("buf", &self.buf())
            .field("ver", &self.ver())
            .field("off", &self.off)
            .field("size", &self.size)
            .finish()
    }
}
