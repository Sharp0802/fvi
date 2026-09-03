//! A module for rendering features.

mod cx;
mod error;
mod id;
mod id_map;
mod layer;
pub mod shape;

pub use cx::*;
pub use error::*;
pub use id::*;
pub use id_map::*;

pub use shape::Shape;

#[derive(Debug)]
pub struct Canvas {
    
}

pub fn draw(canvas: &Canvas, id: Id, shape: Shape) {
    
}
