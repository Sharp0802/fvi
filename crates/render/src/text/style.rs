use crate::Sp;
use crate::text::FontRef;

/// A builder type of [`TextStyle`].
#[derive(Debug)]
pub struct TextStyleBuilder {
    font: FontRef,
    weight: f32,
    italic: bool,
    size: Sp,
}

impl TextStyleBuilder {
    const fn new(font: FontRef) -> Self {
        Self {
            font,
            weight: 400.0,
            italic: false,
            size: Sp(12.0),
        }
    }

    /// Sets weight as given (default: 400.0).
    #[must_use]
    pub const fn weight(mut self, value: f32) -> Self {
        self.weight = value;
        self
    }

    /// Sets italic as given (default: false).
    #[must_use]
    pub const fn italic(mut self, value: bool) -> Self {
        self.italic = value;
        self
    }

    /// Sets size as given (default: 12.0).
    #[must_use]
    pub const fn size(mut self, size: Sp) -> Self {
        self.size = size;
        self
    }

    /// Builds [`TextStyle`].
    #[must_use]
    pub const fn build(self) -> TextStyle {
        TextStyle {
            font: self.font,
            weight: self.weight.to_ne_bytes(),
            italic: self.italic,
            size: self.size.0.to_ne_bytes(),
        }
    }
}

/// A style of glyph.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TextStyle {
    font: FontRef,
    weight: [u8; 4],
    italic: bool,
    size: [u8; 4],
}

impl TextStyle {
    /// Creates a builder of [`TextStyle`].
    #[must_use]
    pub const fn builder(font: FontRef) -> TextStyleBuilder {
        TextStyleBuilder::new(font)
    }

    /// Returns font set of `self`.
    #[must_use]
    pub const fn font(&self) -> &FontRef {
        &self.font
    }

    /// Returns weight value.
    #[must_use]
    pub const fn weight(&self) -> f32 {
        f32::from_ne_bytes(self.weight)
    }

    /// Returns whether the italic is set.
    #[must_use]
    pub const fn italic(&self) -> bool {
        self.italic
    }

    /// Returns font size.
    #[must_use]
    pub const fn size(&self) -> Sp {
        Sp(f32::from_ne_bytes(self.size))
    }
}
