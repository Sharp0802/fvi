//! A module for configurations and preferences.

pub use crate::render::config::*;

/// An user preference for features of this crate.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Preference {
    /// A preference for rendering itself.
    pub render: RenderPref,
}
