use std::error::Error;
use std::fmt::Display;

use crate::context::InsertionError;

#[derive(Debug)]
pub struct RasterizationError;

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
