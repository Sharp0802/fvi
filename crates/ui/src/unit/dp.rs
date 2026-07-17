use crate::cfg::Config;
use crate::unit::{Px, Unit};

super::decl_unit!(
    /// A logical size, in density-independent pixels.
    Dp
);

impl Unit for Dp {
    #[inline]
    fn px(&self, cfg: &Config) -> Px {
        Px::new(self.0 * (cfg.dpi / 160.0))
    }
}
