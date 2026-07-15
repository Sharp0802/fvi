#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc as std;

pub mod alloc;
pub mod math;
pub mod piece;
pub mod treap;

/// A prelude module.
pub mod prelude {
    pub use super::alloc::Slab;
    pub use super::piece::{Buffer, Piece, PieceDesc};
    pub use super::treap::Treap;
}

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
