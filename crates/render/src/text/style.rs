use crate::text::Fonts;
use crate::{Color, Sp};

/// A style of text.
#[derive(Clone, Debug)]
pub struct Style {
    /// A set of fonts.
    pub fonts: Fonts,
    /// An overlay color.
    pub color: Color,
    /// Font weight.
    pub weight: u16,
    /// Whether to set italic.
    pub italic: bool,
    /// Font size.
    pub size: Sp,
}

impl PartialEq for Style {
    fn eq(&self, other: &Self) -> bool {
        self.fonts == other.fonts
            && self.color.to_bits() == other.color.to_bits()
            && self.weight == other.weight
            && self.italic == other.italic
            && self.size.0.to_bits() == other.size.0.to_bits()
    }
}

impl Eq for Style {}
