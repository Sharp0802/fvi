use crate::cfg::Context;
use crate::unit::{Px, Unit};

super::decl_unit!(
    /// A logical size, in density-independent pixels.
    Dp
);

impl Unit for Dp {
    #[inline]
    fn px(&self, ctx: &Context) -> Px {
        Px::new(self.0 * (ctx.dpi / 160.0))
    }

    #[inline]
    fn zero() -> Self {
        Self::new(0.0)
    }
}
