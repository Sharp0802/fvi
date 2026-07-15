#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod cfg;
pub mod io;
mod macros;
pub mod text;
pub mod unit;

use macros::impl_op;
