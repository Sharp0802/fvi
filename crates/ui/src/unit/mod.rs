//! A module providing several unit types.

mod dp;
mod macros;
mod px;
mod sp;

pub use dp::*;
pub use px::*;
pub use sp::*;

use macros::decl_unit;

use crate::cfg::Config;
use crate::impl_op;

/// Creates new logical size, in [`Dp`].
#[inline]
#[must_use]
pub const fn dp(v: f32) -> Vp {
    Vp {
        dp: Dp::new(v),
        sp: Sp::new(0.0),
    }
}

/// Creates new logical size, in [`Sp`].
#[inline]
#[must_use]
pub const fn sp(v: f32) -> Vp {
    Vp {
        dp: Dp::new(0.0),
        sp: Sp::new(v),
    }
}

/// A composite logical size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vp {
    dp: Dp,
    sp: Sp,
}

impl Vp {
    /// A minimum value of the size.
    pub const MIN: Self = Self {
        dp: Dp::MIN,
        sp: Sp::MIN,
    };

    /// A maximum value of the size.
    pub const MAX: Self = Self {
        dp: Dp::MAX,
        sp: Sp::MAX,
    };
}

impl Unit for Vp {
    #[inline]
    fn px(&self, cfg: &Config) -> Px {
        self.dp.px(cfg) + self.sp.px(cfg)
    }
}

impl_op!(Vp::add(self, rhs) {
    Self {
        dp: self.dp + rhs.dp,
        sp: self.sp + rhs.sp,
    }
});

impl_op!(Vp::sub(self, rhs) {
    Self {
        dp: self.dp - rhs.dp,
        sp: self.sp - rhs.sp,
    }
});

impl_op!(Vp::mul::<f32>(self, rhs) {
    Self {
        dp: self.dp * rhs,
        sp: self.sp * rhs,
    }
});
