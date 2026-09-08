use bytemuck::{Pod, Zeroable};

pub use crate::gfx::types::*;

/// A visibility mask bit.
pub const BIT_VIS: u32 = 0x0000_0001;

/// A primitive shape data.
pub trait ShapeData: PartialEq + Zeroable + Pod {
    /// Returns whether the visibility mask is set.
    fn is_visible(&self) -> bool;

    /// Sets visibility mask.
    fn set_visibility(&mut self, visible: bool);
}

impl ShapeData for Rect {
    fn is_visible(&self) -> bool {
        self.mask & BIT_VIS != 0
    }

    fn set_visibility(&mut self, visible: bool) {
        if visible {
            self.mask |= BIT_VIS;
        } else {
            self.mask &= !BIT_VIS;
        }
    }
}
