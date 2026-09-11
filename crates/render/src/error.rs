use std::error::Error;
use std::fmt::{Display, Formatter};

/// An error during initialization.
#[derive(Debug, Clone)]
pub enum InitError {
    /// An error during creating surface.
    CreateSurface(wgpu::CreateSurfaceError),
    /// An error during requesting adapter.
    RequestAdapter(wgpu::RequestAdapterError),
    /// An error during requesting device.
    RequestDevice(wgpu::RequestDeviceError),
}

impl Display for InitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateSurface(e) => e.fmt(f),
            Self::RequestAdapter(e) => e.fmt(f),
            Self::RequestDevice(e) => e.fmt(f),
        }
    }
}

impl Error for InitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateSurface(e) => Some(e),
            Self::RequestAdapter(e) => Some(e),
            Self::RequestDevice(e) => Some(e),
        }
    }
}

impl From<wgpu::CreateSurfaceError> for InitError {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Self::CreateSurface(value)
    }
}

impl From<wgpu::RequestAdapterError> for InitError {
    fn from(value: wgpu::RequestAdapterError) -> Self {
        Self::RequestAdapter(value)
    }
}

impl From<wgpu::RequestDeviceError> for InitError {
    fn from(value: wgpu::RequestDeviceError) -> Self {
        Self::RequestDevice(value)
    }
}

/// An error during rendering.
#[derive(Debug, Clone)]
pub enum RenderError {
    /// The validation for current texture of the surface was failed.
    Invalid,
    /// Device was lost.
    DeviceLost,
    /// The surface was lost and failed to be recovered.
    ///
    /// This is an unrecoverable error,
    /// So recreating [`RenderContext`](crate::RenderContext)
    /// may not solve the failure, Also this failure cannot be notified
    /// to the user using [`RenderContext`](crate::RenderContext).
    Lost(InitError),
    /// Maximum attempt exceeded for acquisition of next frame.
    Fault,
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => write!(f, "frame validation failed"),
            Self::DeviceLost => write!(f, "device lost"),
            Self::Lost(e) => write!(f, "couldn't recover surface lost: {e}"),
            Self::Fault => write!(f, "max attempt exceeded for frame acquisition"),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Invalid | Self::DeviceLost | Self::Fault => None,
            Self::Lost(init) => Some(init),
        }
    }
}

impl From<InitError> for RenderError {
    fn from(value: InitError) -> Self {
        Self::Lost(value)
    }
}
