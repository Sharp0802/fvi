//! A module for text system.

mod atlas;
mod cache;
mod error;
mod font;
mod image;
mod raster;
mod shape;
mod style;
mod unit;

use atlas::*;
use cache::*;
pub use error::*;
pub use font::*;
use image::*;
use raster::*;
pub use style::*;
use unit::*;

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
}

impl Text {
    /// Creates a new [`Text`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            string: String::new(),
        }
    }
}
