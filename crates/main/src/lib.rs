#![doc = include_str!("../README.md")]

pub mod app;
mod layout;
mod pref;

pub use app::*;
pub use fvi_core as core;
pub use fvi_render as render;
pub use pref::*;
