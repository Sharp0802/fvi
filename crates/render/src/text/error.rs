use std::error::Error;
use std::fmt::Display;

use crate::context::InsertionError;

/// An error during text layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// A dimension is negative or non-finite, or the font scale is not positive.
    InvalidDimensions,
    /// The input exceeds the shaping engine's byte-offset limit.
    TextTooLong,
    /// Text requires bidirectional resolution, which is not supported yet.
    UnsupportedBidi,
    /// A style changes inside a shaping cluster at this byte offset.
    StyleInsideCluster(usize),
    /// None of the style's fonts are available for this original byte range.
    MissingFont(std::ops::Range<usize>),
}

impl Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions => write!(f, "invalid text layout dimensions"),
            Self::TextTooLong => write!(f, "text exceeds the shaping byte-offset limit"),
            Self::UnsupportedBidi => write!(f, "bidirectional text layout is not supported"),
            Self::StyleInsideCluster(offset) => {
                write!(f, "style changes inside a cluster at byte {offset}")
            }
            Self::MissingFont(range) => {
                write!(f, "no usable font for bytes {}..{}", range.start, range.end)
            }
        }
    }
}

impl Error for LayoutError {}

/// An error during rasterization.
#[derive(Debug)]
pub struct RasterizationError;

impl Display for RasterizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("failed to rasterize glyph")
    }
}

/// An error from atlas.
#[derive(Debug)]
pub enum AtlasError {
    /// Coudln't initialize atlas.
    Init(InsertionError),
    /// Texture is too big to be inserted into atlas.
    TooBig,
}

impl From<InsertionError> for AtlasError {
    fn from(value: InsertionError) -> Self {
        Self::Init(value)
    }
}

impl Display for AtlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init(e) => write!(f, "coudln't init atlas: {e}"),
            Self::TooBig => write!(f, "texture is too big to be in atlas"),
        }
    }
}

impl Error for AtlasError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            Self::Init(e) => Some(e),
            Self::TooBig => None,
        }
    }
}
