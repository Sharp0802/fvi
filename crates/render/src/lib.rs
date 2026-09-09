#![doc = include_str!("../README.md")]

pub mod backend;
mod context;
mod error;
mod id;
mod shape;

#[path = "gfx.g.rs"]
mod gfx;

pub use error::*;
pub use shape::*;

macro_rules! label {
    ($fmt:literal $(, $arg:expr)*) => {
        Some(&format!(
            concat!("fvi:{}.", $fmt),
            ::std::any::type_name::<Self>(),
            $($arg,)*
        ))
    };
}

use label;
