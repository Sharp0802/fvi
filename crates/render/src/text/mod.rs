//! A module for text system.

mod atlas;
mod cache;
mod error;
mod font;
mod image;
mod layout;
mod raster;
mod shape;
mod style;
mod unit;

use atlas::*;
use cache::*;
pub use error::*;
pub use font::*;
use image::*;
pub use layout::*;
use raster::*;
pub use style::*;
use unit::*;

use std::ops::Range;
use swash::scale::image::Image as SwashImage;

#[derive(Debug)]
pub(crate) struct GlyphContext {
    cache: GlyphCache,
    fonts: FontMap,
}

impl GlyphContext {
    pub fn new() -> Self {
        Self {
            cache: GlyphCache::new(),
            fonts: FontMap::new(),
        }
    }

    pub fn update(&mut self) {
        self.cache.update();
    }
}

/// A text element.
#[derive(Clone, Debug)]
pub struct Text {
    string: String,
    spans: Vec<Span>,
}

#[derive(Clone, Debug)]
struct Span {
    range: Range<usize>,
    style: Style,
}

impl Text {
    /// Creates a new [`Text`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            string: String::new(),
            spans: Vec::new(),
        }
    }

    /// Appends text with the given style, merging adjacent equal styles.
    pub fn push(&mut self, string: &str, style: &Style) {
        if string.is_empty() {
            return;
        }

        let start = self.string.len();
        self.string.push_str(string);

        if let Some(span) = self.spans.last_mut()
            && span.style == *style
        {
            span.range.end = self.string.len();
        } else {
            self.spans.push(Span {
                range: start..self.string.len(),
                style: style.clone(),
            });
        }
    }

    /// Removes all text and styles, retaining allocated storage.
    pub fn clear(&mut self) {
        self.string.clear();
        self.spans.clear();
    }

    /// Measures and positions left-to-right text in density independent pixels.
    /// Unsupported characters use a replacement or missing-glyph symbol.
    ///
    /// # Errors
    /// Returns an error for invalid dimensions, unavailable fonts, bidi text, or
    /// style changes inside a shaping cluster.
    pub fn layout(&self, fonts: &FontMap, desc: &LayoutDescriptor) -> Result<Layout, LayoutError> {
        layout::layout(self, fonts, desc)
    }
}
