//! A module providing configuration things.

/// Layout measuring context.
#[derive(Debug)]
pub struct Context {
    /// DPI of surface.
    pub dpi: f32,
    /// General UI scale factor.
    pub ui_scale: f32,
    /// Text scale factor.
    pub text_scale: f32,
    /// Unscaled width of surface, in pixels.
    pub width: u32,
    /// Unscaled height of surface, in pixels.
    pub height: u32,
}
