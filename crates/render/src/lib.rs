#![doc = include_str!("../README.md")]

mod backend;
mod canvas;
mod context;
mod error;
mod id;

#[path = "gfx.g.rs"]
mod gfx;

pub use canvas::*;
pub use error::*;
pub use id::Id;

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
