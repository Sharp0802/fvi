use core::num::NonZero;

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

impl From<Version> for u32 {
    fn from(value: Version) -> Self {
        value.0.get().wrapping_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustive() {
        assert!(Version::new(u32::MAX).is_none());
    }
}
