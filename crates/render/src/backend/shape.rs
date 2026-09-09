use bytemuck::{Pod, Zeroable};
use std::fmt::Debug;

use crate::gfx::ShaderEntry;
use crate::types::Rect;

pub(crate) const BIT_VIS: u32 = 0x0000_0001;

pub trait Shape: Debug + Zeroable + Pod {
    const CULL: ShaderEntry;
    const VERTEX: ShaderEntry;
    const FRAGMENT: ShaderEntry;
    const MAX_AGE: u32;

    fn is_visible(&self) -> bool;

    fn set_visibility(&mut self, visible: bool);
}

impl Shape for Rect {
    const CULL: ShaderEntry = ShaderEntry::RectCull;
    const VERTEX: ShaderEntry = ShaderEntry::RectDraw;
    const FRAGMENT: ShaderEntry = ShaderEntry::RectDraw;
    const MAX_AGE: u32 = 120;

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
