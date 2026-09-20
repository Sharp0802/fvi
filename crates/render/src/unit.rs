use bytemuck::{Pod, Zeroable};

macro_rules! impl_ops {
    ($name:ident add) => {
        impl std::ops::Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0.algebraic_add(rhs.0))
            }
        }

        impl std::ops::AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }
    };

    ($name:ident sub) => {
        impl std::ops::Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0.algebraic_sub(rhs.0))
            }
        }

        impl std::ops::SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }
    };

    ($name:ident scale) => {
        impl std::ops::Mul<f32> for $name {
            type Output = Self;
            fn mul(self, rhs: f32) -> Self::Output {
                Self(self.0.algebraic_mul(rhs))
            }
        }

        impl std::ops::MulAssign<f32> for $name {
            fn mul_assign(&mut self, rhs: f32) {
                *self = *self * rhs;
            }
        }

        impl std::ops::Div<f32> for $name {
            type Output = Self;
            fn div(self, rhs: f32) -> Self::Output {
                Self(self.0.algebraic_div(rhs))
            }
        }

        impl std::ops::DivAssign<f32> for $name {
            fn div_assign(&mut self, rhs: f32) {
                *self = *self / rhs;
            }
        }
    };

    ($name:ident rem) => {
        impl std::ops::Rem for $name {
            type Output = Self;
            fn rem(self, rhs: Self) -> Self::Output {
                Self(self.0.algebraic_rem(rhs.0))
            }
        }
    };

    ($(#[$meta:meta])* $name:ident $($op:ident)*) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Zeroable, Pod)]
        #[repr(transparent)]
        pub struct $name(pub f32);

        impl From<$name> for f32 {
            fn from(value: $name) -> f32 {
                value.0
            }
        }

        $(impl_ops!($name $op);)*
    };
}

impl_ops!(
    /// Density independent pixels.
    ///
    /// Semantically equals to `dp` of Android and `pt` of iOS.
    /// Instead of directly using DPI,
    /// DPI is inferred from scale factor of the window.
    ///
    /// Calculation of [`Dp`] is not deterministic.
    Dp
    add
    sub
    scale
    rem
);

impl Dp {
    /// Converts [`Dp`] into physical pixels using given scale factor.
    #[must_use]
    pub fn to_px(self, scale: f32) -> f32 {
        scale.algebraic_mul(96.0 / 160.0).algebraic_mul(self.0)
    }

    /// Creates [`Dp`] from physical pixels and scale factor.
    #[must_use]
    pub fn from_px(px: f32, scale: f32) -> Self {
        Self(px.algebraic_mul(160.0 / 96.0).algebraic_div(scale))
    }
}

impl_ops!(
    /// Scale independent pixels.
    ///
    /// Semantically equals to `sp` of Android.
    /// Instead of directly using DPI,
    /// DPI is inferred from scale factor of the window.
    ///
    /// Calculation of [`Sp`] is not deterministic.
    Sp
    add
    sub
    scale
    rem
);

impl Sp {
    /// Converts [`Sp`] into [`Dp`] using given font scale.
    #[rustfmt::skip]
    #[must_use]
    pub fn to_dp(self, scale: f32) -> Dp {
        const R160: f32 = 1.0 / 6.0;
        const R230: f32 = 2.0 / 3.0;
        const R435: f32 = 4.0 / 35.0;

        const E10: [f32; 8] = [0.0, 12.0, 14.0, 18.0, 20.0, 24.0, 30.0,100.0];
        const E15: [f32; 8] = [0.5, -1.5,  1.5,  0.5, -1.0,-R160, R230,  0.0];
        const E20: [f32; 8] = [0.5, -1.5,  1.5,  0.5, -1.0,  0.0,-R435, R435];

        let sp = self.0.abs();

        let [mut d15, mut d20] = [0.0; 2];
        for i in 0..8 {
            let h = sp.algebraic_sub(E10[i]).max(0.0);
            d15 = h.algebraic_mul(E15[i]).algebraic_add(d15);
            d20 = h.algebraic_mul(E20[i]).algebraic_add(d20);
        }

        let a = scale.algebraic_sub(1.0).algebraic_mul(2.0).clamp(0.0, 1.0).algebraic_mul(d15);
        let b = scale.algebraic_sub(1.5).algebraic_mul(2.0).clamp(0.0, 1.0).algebraic_mul(d20);
        let dp = sp.algebraic_add(a).algebraic_add(b);

        Dp(dp.copysign(self.0))
    }
}
