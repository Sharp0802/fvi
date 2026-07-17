use crate::cfg::Config;
use crate::unit::{Dp, Px, Unit};

super::decl_unit!(
    /// A logical size, in scale-independent pixels.
    Sp
);

impl Unit for Sp {
    #[inline]
    fn px(&self, cfg: &Config) -> Px {
        let t = cfg.text_scale / 24.0;
        let t = t * t * t * t;
        let dp = self.0 * ((self.0 - 1.0) * (1.0 + t) + 1.0);
        Dp::new(dp).px(cfg)
    }
}
