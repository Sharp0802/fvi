use core::num::NonZero;

use crate::collections::table::Version;
use crate::util::unreachable;

/// An editing context
#[derive(Debug, Clone)]
pub struct Context {
    /// A maximum size of undo stack.
    pub undo_max_len: NonZero<u32>,
    /// Current editing version.
    pub version: Version,
}

impl Context {
    /// Returns the possible oldest version constant.
    #[must_use]
    pub fn possible_oldest(&self) -> Version {
        let raw = self.version.get().saturating_sub(self.undo_max_len.get());
        Version::new(raw).unwrap_or_else(|| unreachable())
    }
}
