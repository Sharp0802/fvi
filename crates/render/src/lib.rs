#![doc = include_str!("../README.md")]

mod backend;
pub mod config;
pub mod context;
mod error;
mod id;
mod types;

#[path = "gfx.g.rs"]
#[allow(
    unsafe_code,
    reason = "cannot modify generated source into safe code manually"
)]
mod gfx;

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
