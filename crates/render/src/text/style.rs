use crate::text::Fonts;
use crate::{Color, Sp};

#[derive(Clone, Debug)]
pub struct Style {
    pub fonts: Fonts,
    pub color: Color,
    pub weight: u16,
    pub italic: bool,
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
