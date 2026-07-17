#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod cfg;
mod macros;
pub mod unit;

use macros::impl_op;
