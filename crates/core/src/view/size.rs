use core::cmp::Ordering;
use core::fmt::{Debug, Display};
use core::num::NonZero;

use crate::unreachable;

/// A size type of page view, able to hold 1B to 64KiB.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ViewSize(u16);

impl ViewSize {
    /// The minimum possible size of a view, equivalent to 1B.
    pub const MIN: Self = Self(0);

    /// The maximum possible size of a view, equivalent to 64KiB.
    pub const MAX: Self = Self(u16::MAX);

    /// Creates a new [`ViewSize`],
    /// returning `None` if `size` is zero or `size` exceeds 64KiB.
    #[inline]
    #[must_use]
    pub fn new<T>(size: T) -> Option<Self>
    where
        T: TryInto<Self, Error = Error>,
    {
        size.try_into().ok()
    }

    /// Gets byte size as [`NonZero`] type.
    #[inline]
    #[must_use]
    pub fn get_nonzero(&self) -> NonZero<u32> {
        NonZero::new(u32::from(self.0) + 1).unwrap_or_else(|| unreachable())
    }

    /// Gets byte size as zeroable type.
    #[inline]
    #[must_use]
    pub fn get(&self) -> u32 {
        self.get_nonzero().get()
    }

    /// Strict addition. Computes `self + val`,
    /// panicking if overflow occurred.
    ///
    /// # Panics
    ///
    /// This function will always panic on overflow,
    /// regardless of whether overflow checks are enabled.
    #[inline]
    #[must_use]
    pub fn strict_add<T>(&self, val: T) -> Self
    where
        T: TryInto<Self, Error = Error>,
    {
        self.checked_add(val)
            .expect("ViweSize::strict_add overflows")
    }

    /// Strict substraction. Computes `self - val`,
    /// panicking if underflow occurred.
    ///
    /// # Panics
    ///
    /// This function will always panic on underflow,
    /// regardless of whether overflow checks are enabled.
    #[inline]
    #[must_use]
    pub fn strict_sub<T>(&self, val: T) -> Self
    where
        T: TryInto<Self, Error = Error>,
    {
        self.checked_sub(val)
            .expect("ViewSize::strict_sub underflows")
    }

    /// Checked addition. Computes `self + val`,
    /// returning `None` if overflow occurred.
    #[inline]
    #[must_use]
    pub fn checked_add<T>(&self, val: T) -> Option<Self>
    where
        T: TryInto<Self, Error = Error>,
    {
        let val = match val.try_into() {
            Ok(val) => val,
            Err(Error::TooSmall) => return Some(*self),
            Err(Error::TooBig) => return None,
        };

        Self::new(self.get() + val.get())
    }

    /// Checked substraction. Computes `self - val`,
    /// returning `None` if underflow occurred.
    #[inline]
    #[must_use]
    pub fn checked_sub<T>(&self, val: T) -> Option<Self>
    where
        T: TryInto<Self, Error = Error>,
    {
        let val = match val.try_into() {
            Ok(val) if val < *self => val,
            Ok(_) | Err(Error::TooBig) => return None,
            Err(Error::TooSmall) => return Some(*self),
        };

        Some(Self::new(self.get() - val.get()).unwrap_or_else(|| unreachable()))
    }
}

impl Debug for ViewSize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let size = self.get();
        let kib_int = size >> 10;
        let kib_frac = (size & 0x3FFF) * 10 / 1024;
        write!(f, "ViewSize(\"{kib_int}.{kib_frac:.1}KiB ({size}B)\")")
    }
}

/// An error type representing errors during integer to [`ViewSize`] conversions.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Given integer is zero.
    TooSmall,
    /// Given integer is bigger than 64KiB (65536).
    TooBig,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let str = match self {
            Self::TooSmall => "size cannot be zero",
            Self::TooBig => "size cannot be greater than 64KiB",
        };

        f.write_str(str)
    }
}

impl core::error::Error for Error {}

macro_rules! impl_for {
    ($($ty:ty),+ $(,)?) => { $(
        impl TryFrom<$ty> for ViewSize {
            type Error = Error;

            #[inline]
            fn try_from(value: $ty) -> Result<Self, Self::Error> {
                if value <= 0 {
                    Err(Error::TooSmall)
                } else if value > ViewSize::MAX {
                    Err(Error::TooBig)
                } else {
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "unreachable if truncation is possible"
                    )]
                    #[allow(
                        clippy::cast_lossless,
                        reason = "will conflict with other types"
                    )]
                    Ok(Self((value - 1) as u16))
                }
            }
        }

        impl PartialEq<$ty> for ViewSize {
            #[inline]
            fn eq(&self, other: &$ty) -> bool {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "each cases are unreachable if it can cause truncation"
                )]
                if const { <$ty>::BITS > 32 } {
                    self.get() as $ty == *other
                } else {
                    #[allow(
                        clippy::cast_lossless,
                        reason = "will conflict with other types"
                    )]
                    (self.get() == *other as u32)
                }
            }
        }

        impl PartialEq<ViewSize> for $ty {
            #[inline]
            fn eq(&self, other: &ViewSize) -> bool {
                other.eq(self)
            }
        }

        impl PartialOrd<$ty> for ViewSize {
            #[inline]
            fn partial_cmp(&self, other: &$ty) -> Option<Ordering> {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "each cases are unreachable if it can cause truncation"
                )]
                if const { <$ty>::BITS > 32 } {
                    Some((self.get() as $ty).cmp(other))
                } else {
                    #[allow(
                        clippy::cast_lossless,
                        reason = "will conflict with other types"
                    )]
                    Some(self.get().cmp(&(*other as u32)))
                }
            }
        }

        impl PartialOrd<ViewSize> for $ty {
            #[inline]
            fn partial_cmp(&self, other: &ViewSize) -> Option<Ordering> {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "each cases are unreachable if it can cause truncation"
                )]
                if const { <$ty>::BITS > 32 } {
                    Some(self.cmp(&(other.get() as $ty)))
                } else {
                    #[allow(
                        clippy::cast_lossless,
                        reason = "will conflict with other types"
                    )]
                    Some((*self as u32).cmp(&other.get()))
                }
            }
        }
    )+ };
}

impl_for!(u8, u16, u32, u64, usize);

impl From<NonZero<u16>> for ViewSize {
    #[inline]
    fn from(value: NonZero<u16>) -> Self {
        Self(value.get() - 1)
    }
}

impl PartialEq for ViewSize {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ViewSize {}

impl PartialOrd for ViewSize {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ViewSize {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
