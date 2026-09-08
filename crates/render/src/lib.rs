#![doc = include_str!("../README.md")]

mod backend;
mod canvas;
mod context;
mod error;
mod id;
mod shape;

#[path = "gfx.g.rs"]
mod gfx;

pub use canvas::*;
pub use context::*;
pub use error::*;
pub use id::*;
pub use shape::*;

macro_rules! label {
    ($name:literal) => {
        Some(&format!(
            "fvi:{}.{}",
            ::std::any::type_name::<Self>(),
            $name,
        ))
    };
}

use label;
