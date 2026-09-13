use crate::context::TextureRef;
use crate::{Dp, Frame, raw};

/// A drawing session that replaces the contents of a [`Frame`].
#[derive(Debug)]
pub struct Canvas<'a> {
    frame: &'a mut Frame,
    cursor: usize,
}

/// A rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectDescriptor {
    /// Position of the top-left corner.
    pub pos: [Dp; 2],
    /// Width and height.
    pub size: [Dp; 2],
    /// Fill color.
    pub color: [f32; 4],
    /// A texture sampled across its entire extent.
    pub texture: Option<TextureRef>,
    /// Fill corner radius.
    pub radius: Dp,
    /// Outer corner radius of the border.
    pub border_radius: Dp,
    /// Inward border width.
    pub border_stroke: Dp,
    /// Border color.
    pub border_color: [f32; 4],
}

impl Default for RectDescriptor {
    fn default() -> Self {
        Self {
            pos: [Dp(0.0); 2],
            size: [Dp(0.0); 2],
            color: [1.0; 4],
            texture: None,
            radius: Dp(0.0),
            border_radius: Dp(0.0),
            border_stroke: Dp(0.0),
            border_color: [0.0; 4],
        }
    }
}

impl<'a> Canvas<'a> {
    pub(crate) const fn new(frame: &'a mut Frame) -> Self {
        Self { frame, cursor: 0 }
    }

    /// Records a rectangle.
    pub fn rect(&mut self, desc: &RectDescriptor) {
        self.frame.write_rect(
            self.cursor,
            raw::Rect {
                pos: desc.pos.map(f32::from),
                size: desc.size.map(f32::from),
                color: desc.color,
                tex: desc.texture.unwrap_or(TextureRef::WHITE).index(),
                radius: desc.radius.0,
                border_radius: desc.border_radius.0,
                border_stroke: desc.border_stroke.0,
                border_color: desc.border_color,
            },
        );
        self.cursor += 1;
    }
}

impl Drop for Canvas<'_> {
    fn drop(&mut self) {
        self.frame.truncate_rects(self.cursor);
    }
}
