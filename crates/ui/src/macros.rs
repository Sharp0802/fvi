macro_rules! impl_op {
    ($Self:tt::cmp(&$self:ident, $arg:ident) $blk:block) => {
        pastey::paste! {
            impl core::cmp::Ord for $Self {
                #[inline]
                fn cmp(&$self, $arg: &$Self) -> core::cmp::Ordering $blk
            }

            impl core::cmp::PartialOrd for $Self {
                #[inline]
                fn partial_cmp(&self, $arg: &$Self) -> Option<core::cmp::Ordering> {
                    Some(self.cmp($arg))
                }
            }

            impl core::cmp::Eq for $Self {}

            impl core::cmp::PartialEq for $Self {
                #[inline]
                fn eq(&self, $arg: &$Self) -> bool {
                    self.cmp($arg) == core::cmp::Ordering::Equal
                }
            }
        }
    };

    ($Self:tt::$op:ident::<$rhs:ty>($self:ident, $arg:ident) -> $ret:ty $blk:block) => {
        pastey::paste! {
            impl core::ops::[< $op:camel >]<$rhs> for $Self {
                type Output = $ret;

                #[inline]
                fn [< $op:snake >]($self, $arg: $rhs) -> Self::Output $blk
            }
        }
    };

    ($Self:tt::$op:ident($self:ident, $arg:ident) $blk:block) => {
        pastey::paste! {
            impl core::ops::[< $op:camel >] for $Self {
                type Output = $Self;

                #[inline]
                fn [< $op:snake >]($self, $arg: $Self) -> Self::Output $blk
            }

            impl core::ops::[< $op:camel Assign >] for $Self {
                #[inline]
                fn [< $op:snake _assign >](&mut self, rhs: $Self) {
                    use core::ops::[< $op:camel >];
                    *self = self.[< $op:snake >](rhs);
                }
            }
        }
    };

    ($Self:tt::$op:ident::<$rhs:ty>($self:ident, $arg:ident) $blk:block) => {
        pastey::paste! {
            impl core::ops::[< $op:camel >]<$rhs> for $Self {
                type Output = $Self;

                #[inline]
                fn [< $op:snake >]($self, $arg: $rhs) -> Self::Output $blk
            }

            impl core::ops::[< $op:camel >]<$Self> for $rhs {
                type Output = $Self;

                #[inline]
                fn [< $op:snake >](self, rhs: $Self) -> Self::Output {
                    rhs.[< $op:snake >](self)
                }
            }

            impl core::ops::[< $op:camel Assign >]<$rhs> for $Self {
                #[inline]
                fn [< $op:snake _assign >](&mut self, rhs: $rhs) {
                    use core::ops::[< $op:camel >];
                    *self = self.[< $op:snake >](rhs);
                }
            }
        }
    };
}

pub(crate) use impl_op;
