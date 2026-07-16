#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc as std;

pub mod collections;
pub mod math;
pub mod piece;

#[inline]
#[track_caller]
#[doc(hidden)]
pub const fn unreachable() -> ! {
    #[cfg(debug_assertions)]
    panic!("broken invariants");
    #[cfg(not(debug_assertions))]
    #[expect(unsafe_code, reason = "it's intended usage")]
    unsafe {
        core::hint::unreachable_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic = "broken invariants"]
    fn exhaustive() {
        #[cfg(debug_assertions)]
        unreachable();
        #[cfg(not(debug_assertions))]
        // avoids undefined behaviour
        panic!("broken invariants");
    }
}
