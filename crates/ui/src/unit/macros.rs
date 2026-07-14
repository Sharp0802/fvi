macro_rules! decl_unit {
    ($(#[$meta:meta])* $ty:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy)]
        #[repr(transparent)]
        pub struct $ty(f32);

        impl $ty {
            /// A minimum value of the size.
            pub const MIN: Self = Self::new(f32::MIN);

            /// A maximum value of the size.
            pub const MAX: Self = Self::new(f32::MAX);

            #[inline]
            #[must_use]
            pub(crate) const fn new(value: f32) -> Self {
                Self(value)
            }
        }

        crate::impl_op!($ty::cmp(&self, rhs) {
            self.0.total_cmp(&rhs.0)
        });

        crate::impl_op!($ty::add(self, rhs) {
            Self::new(self.0 + rhs.0)
        });

        crate::impl_op!($ty::sub(self, rhs) {
            Self::new(self.0 - rhs.0)
        });

        crate::impl_op!($ty::mul::<f32>(self, rhs) {
            Self::new(self.0 * rhs)
        });

        crate::impl_op!($ty::div::<Self>(self, rhs) -> f32 {
            self.0 / rhs.0
        });

        crate::impl_op!($ty::rem(self, rhs) {
            Self::new(self.0 % rhs.0)
        });
    };
}

pub(super) use decl_unit;
