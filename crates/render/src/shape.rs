use bytemuck::{Pod, Zeroable};
use std::fmt::Debug;

use crate::gfx::ShaderEntry;
pub use crate::gfx::types::*;

pub const BIT_VIS: u32 = 0x0000_0001;

pub trait Shape: Debug + Zeroable + Pod {
    const CULL: ShaderEntry;
    const DRAW: ShaderEntry;

    fn is_visible(&self) -> bool;

    fn set_visibility(&mut self, visible: bool);
}

impl Shape for Rect {
    const CULL: ShaderEntry = ShaderEntry::RectCull;
    const DRAW: ShaderEntry = ShaderEntry::RectDraw;

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
