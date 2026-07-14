#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod piece;
pub mod sorted;
pub mod view;

#[inline]
#[track_caller]
#[doc(hidden)]
pub const fn unreachable() -> ! {
    if cfg!(debug_assertions) {
        panic!("broken invariants");
    } else {
        #[expect(unsafe_code, reason = "it's intended usage")]
        unsafe {
            core::hint::unreachable_unchecked()
        }
    }
}
