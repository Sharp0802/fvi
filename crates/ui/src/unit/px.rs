use crate::cfg::Context;

super::decl_unit!(
    /// A physical size, in unscaled pixels.
    Px
);

impl Px {
    /// A constant of 0px.
    pub const ZERO: Self = Self::new(0.0);
}

/// A trait to convert units into unscaled pixels.
pub trait Unit {
    /// Returns a size equivalents to 0px.
    #[must_use]
    fn zero() -> Self;

    /// Converts into unscaled pixels using given context.
    #[must_use]
    fn px(&self, ctx: &Context) -> Px;
}

impl Unit for Px {
    fn zero() -> Self {
        Self::ZERO
    }

    fn px(&self, _ctx: &Context) -> Px {
        *self
    }
}

impl From<Px> for f32 {
    #[inline]
    fn from(value: Px) -> Self {
        value.0
    }
}
