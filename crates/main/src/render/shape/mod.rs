use bytemuck::{Pod, Zeroable};

mod rect;
mod vec;

pub use rect::Rect;

/// A visibility mask bit.
pub const BIT_VIS: u32 = 0x0000_0001;

/// A primitive shape data.
pub trait ShapeData: PartialEq + Zeroable + Pod {
    /// Returns whether the visibility mask is set.
    fn is_visible(&self) -> bool;

    /// Sets visibility mask.
    fn set_visibility(&mut self, visible: bool);
}

/// A primitive shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// A rect shape.
    Rect(Rect),
}
