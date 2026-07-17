use crate::cfg::Config;

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
    /// Converts into unscaled pixels using given context.
    #[must_use]
    fn px(&self, cfg: &Config) -> Px;
}

impl Unit for Px {
    fn px(&self, _cfg: &Config) -> Px {
        *self
    }
}

impl From<Px> for f32 {
    #[inline]
    fn from(value: Px) -> Self {
        value.0
    }
}
