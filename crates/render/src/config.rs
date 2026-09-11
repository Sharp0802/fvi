//! A module for configurations and preferences.

use wgpu::{Features, Limits, MemoryHints, TextureUsages};

/// A configuration for the application.
///
/// To configure rendering behaviour
/// specified to the user, See [`RenderPref`].
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Whether the debug features are enabled.
    pub debug: bool,
    /// The required limits.
    pub limits: Limits,
    /// The required features.
    pub features: Features,
    /// The required texture usages.
    pub texture_usages: TextureUsages,
    /// The memory hints.
    pub memory_hints: MemoryHints,
}

impl RenderConfig {
    /// Normalizes `self` to compatible with the `fvi`'s requirements.
    #[must_use]
    pub fn normalize(&self) -> Self {
        let min = Self::default();

        Self {
            debug: self.debug,
            limits: self.limits.clone().or_better_values_from(&min.limits),
            features: self.features | min.features,
            texture_usages: self.texture_usages | min.texture_usages,
            memory_hints: self.memory_hints.clone(),
        }
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            debug: cfg!(debug_assertions),
            limits: Limits {
                max_texture_array_layers: 1,
                ..Default::default()
            },
            features: Features::TEXTURE_BINDING_ARRAY
                | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            texture_usages: TextureUsages::RENDER_ATTACHMENT,
            memory_hints: MemoryHints::default(),
        }
    }
}

/// An user preference for adapter and device.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DevicePref {
    /// A name string of the adapter.
    ///
    /// Fallback will be used if no such adapter found,
    /// or specified adapter is not suitable.
    pub name: Option<String>,
    /// Whether the low-power hint should be passed to the backend.
    pub low_power: bool,
}

/// An user preference for the surface.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SurfacePref {
    /// Whether the vsync should be enabled.
    ///
    /// It can be ignored if not supported.
    pub vsync: bool,
}

/// An user preference for rendering.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RenderPref {
    /// A preference for adapter and device.
    pub device: DevicePref,
    /// A preference for the surface.
    pub surface: SurfacePref,
}
