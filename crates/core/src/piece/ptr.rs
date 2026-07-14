#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Ptr(u16);

impl Ptr {
    pub const NIL: Self = Self(u16::MAX);

    #[inline]
    #[must_use]
    pub const fn new(pos: u16) -> Self {
        Self(pos)
    }

    #[inline]
    #[must_use]
    pub const fn is_nil(self) -> bool {
        self.0 == Self::NIL.0
    }

    #[inline]
    #[must_use]
    pub const fn get(self) -> Option<u16> {
        if self.is_nil() { None } else { Some(self.0) }
    }

    #[inline]
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl PartialEq for Ptr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Ptr {}
