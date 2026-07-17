use core::num::NonZero;
use core::ops::{Deref, DerefMut};

/// A version constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Version(NonZero<u32>);

impl Version {
    /// Creates a new [`Version`], returning `None` if `value` is [`u32::MAX`].
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        if let Some(val) = NonZero::new(value.wrapping_add(1)) {
            Some(Self(val))
        } else {
            None
        }
    }
}

impl Deref for Version {
    type Target = NonZero<u32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Version {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Version> for u32 {
    fn from(value: Version) -> Self {
        value.0.get().wrapping_sub(1)
    }
}
