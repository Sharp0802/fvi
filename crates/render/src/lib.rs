#![doc = include_str!("../README.md")]

mod backend;
pub mod canvas;
pub mod context;
pub mod error;
pub mod shape;

pub use canvas::{Canvas, Draw};
pub use context::{DevicePref, RenderConfig, RenderContext, RenderDevice, RenderPref, SurfacePref};
pub use error::{InitError, RenderError};
pub use shape::{Id, Location, Rect, Shape, ShapeData};

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
