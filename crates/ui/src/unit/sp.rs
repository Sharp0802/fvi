use crate::cfg::Context;
use crate::unit::{Dp, Px, Unit};

super::decl_unit!(
    /// A logical size, in scale-independent pixels.
    Sp
);

impl Unit for Sp {
    #[inline]
    fn px(&self, ctx: &Context) -> Px {
        let t = ctx.text_scale / 24.0;
        let t = t * t * t * t;
        let dp = self.0 * ((self.0 - 1.0) * (1.0 + t) + 1.0);
        Dp::new(dp).px(ctx)
    }

    #[inline]
    fn zero() -> Self {
        Self::new(0.0)
    }
}
