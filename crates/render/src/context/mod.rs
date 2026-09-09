//! A renewable rendering context.

use std::fmt::Debug;
use std::sync::Arc;
use tracing::{info, instrument, warn};
use wgpu::*;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::config::*;
use crate::{InitError, RenderError};

mod adapter;
mod device;
mod shader;

use adapter::list_adapters;
use device::*;
pub use shader::*;

const BACKENDS: Backends = Backends::PRIMARY;

/// A renewable rendering context.
#[derive(Debug)]
pub struct RenderContext {
    config: RenderConfig,
    pref: RenderPref,
    window: Arc<Window>,
    size: PhysicalSize<u32>,
    instance: Instance,
    surface: Surface<'static>,
    device: RenderDevice,
    shader: Shader,
    format: TextureFormat,
}

impl RenderContext {
    /// Creates a new [`RenderContext`].
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot be created.
    /// See [`InitError`] for details.
    #[instrument]
    pub async fn new(
        window: Arc<Window>,
        pref: &RenderPref,
        config: RenderConfig,
    ) -> Result<Self, InitError> {
        let instance = Instance::new(InstanceDescriptor {
            backends: BACKENDS,
            flags: if config.debug {
                InstanceFlags::DEBUG
                    | InstanceFlags::GPU_BASED_VALIDATION
                    | InstanceFlags::VALIDATION_INDIRECT_CALL
            } else {
                InstanceFlags::DISCARD_HAL_LABELS
            },
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
            backend_options: BackendOptions::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone())?;

        let device = RenderDevice::new(&instance, &surface, &pref.device, &config).await?;
        let shader = Shader::new(&device);

        let cap = surface.get_capabilities(device.as_ref());
        let format = cap.formats[0];

        let this = Self {
            config,
            pref: pref.clone(),
            size: window.inner_size(),
            window,
            instance,
            surface,
            device,
            shader,
            format,
        };

        this.configure_surface();

        Ok(this)
    }

    #[instrument]
    fn configure_surface(&self) {
        if self.size.height == 0 || self.size.width == 0 {
            warn!("attempt to do zeroing surface size ignored");
            return;
        }

        let conf = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            color_space: SurfaceColorSpace::Auto,
            width: self.size.width,
            height: self.size.height,
            present_mode: if self.pref.surface.vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![self.format.add_srgb_suffix()],
        };

        self.surface.configure(&self.device, &conf);
    }

    /// The name of currently selected adapter.
    ///
    /// It can differ to preferred name set using [`RenderPref`],
    /// If preferred adapter doesn't satisfies requirements.
    #[must_use]
    pub const fn name(&self) -> &str {
        self.device.name()
    }

    /// Lists adapters compatible with current state.
    pub async fn list_adapters(&self) -> impl Iterator<Item = String> {
        list_adapters(&self.instance, &self.surface, &self.config).await
    }

    #[instrument]
    async fn renew_device(&mut self) -> Result<(), InitError> {
        self.device = RenderDevice::new(
            &self.instance,
            &self.surface,
            &self.pref.device,
            &self.config,
        )
        .await?;

        Ok(())
    }

    /// Renews this [`RenderContext`].
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot be renewed.
    /// See [`InitError`] for details.
    #[instrument]
    pub async fn renew(&mut self) -> Result<(), InitError> {
        self.surface = self.instance.create_surface(self.window.clone())?;

        let renew_device = self.device.is_lost() || {
            let adapter: &Adapter = self.device.as_ref();
            let supported = adapter.is_surface_supported(&self.surface);
            if !supported {
                warn!("renewed surface is incompatibe with old adapter");
            }

            supported
        };

        if renew_device {
            self.renew_device().await?;
        }

        self.configure_surface();

        Ok(())
    }

    /// Sets rendering preference as given.
    ///
    /// It invalidates the current adapter and device
    /// if device preference of given one differs to configured one.
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot be renewed.
    /// See [`InitError`] for details.
    #[instrument]
    pub async fn set_preference(&mut self, pref: RenderPref) -> Result<(), InitError> {
        let old_pref = std::mem::replace(&mut self.pref, pref);

        if self.pref.device != old_pref.device {
            info!("device preference changed");
            self.renew_device().await?;
            self.configure_surface();
        } else if self.pref.surface != old_pref.surface {
            info!("surface preference changed");
            self.configure_surface();
        }

        Ok(())
    }

    /// Resizes the surface as given.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if self.size != size {
            self.size = size;
            self.configure_surface();
        }
    }

    /// Returns preferred texture format.
    #[must_use]
    pub fn texture_format(&self) -> TextureFormat {
        self.format.add_srgb_suffix()
    }

    /// Returns the size of surface.
    #[must_use]
    pub const fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    #[instrument]
    async fn get_frame(&mut self) -> Result<Option<(SurfaceTexture, bool)>, RenderError> {
        let mut configure = false;
        let mut retry = 0;

        let frame = loop {
            retry += 1;
            if retry > 3 {
                return Err(RenderError::Fault);
            }

            match self.surface.get_current_texture() {
                CurrentSurfaceTexture::Success(texture) => break texture,
                CurrentSurfaceTexture::Suboptimal(texture) => {
                    warn!("got suboptimal frame");
                    configure = true;
                    break texture;
                }
                CurrentSurfaceTexture::Timeout => {
                    warn!("frame acquisition timed out");
                    return Ok(None);
                }
                CurrentSurfaceTexture::Occluded => {
                    warn!("rendering requested for occluded surface");
                    return Ok(None);
                }
                CurrentSurfaceTexture::Outdated => {
                    warn!("surface outdated");
                    self.configure_surface();
                }
                CurrentSurfaceTexture::Lost => {
                    warn!("surface lost");
                    self.renew().await.map_err(RenderError::Lost)?;
                }
                CurrentSurfaceTexture::Validation => return Err(RenderError::Invalid),
            }
        };

        Ok(Some((frame, configure)))
    }

    /// Starts rendering phase, returning the contextual scope.
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot be rendered.
    /// See [`RenderError`] for details.
    pub async fn start(&mut self) -> Result<Option<RenderScope<'_>>, RenderError> {
        let Some((frame, configure)) = self.get_frame().await? else {
            return Ok(None);
        };

        let view = frame.texture.create_view(&TextureViewDescriptor {
            label: Some("Surface.view"),
            format: Some(self.texture_format()),
            dimension: Some(TextureViewDimension::D2),
            usage: Some(TextureUsages::RENDER_ATTACHMENT),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: None,
        });

        Ok(Some(RenderScope {
            device: &self.device,
            queue: self.device.as_ref(),
            shader: &self.shader,
            view,
            frame,
            configure,
        }))
    }

    /// Submits given state and renders onto surface.
    pub fn done<I>(&self, state: Done<I>)
    where
        I: Iterator<Item = CommandBuffer>,
    {
        let queue: &Queue = self.device.as_ref();
        queue.submit(state.ops);
        queue.present(state.frame);

        if state.configure {
            self.configure_surface();
        }
    }
}

/// A contextual render scope.
#[derive(Debug)]
pub struct RenderScope<'a> {
    /// The current device.
    pub device: &'a Device,
    /// A queue corresponding to current device.
    pub queue: &'a Queue,
    /// The shader store.
    pub shader: &'a Shader,
    /// The current surface texture.
    pub view: TextureView,
    frame: SurfaceTexture,
    configure: bool,
}

impl RenderScope<'_> {
    /// Closes `self`, returning result state.
    pub fn done<I>(self, ops: I) -> Done<I>
    where
        I: Iterator<Item = CommandBuffer>,
    {
        Done {
            ops,
            frame: self.frame,
            configure: self.configure,
        }
    }
}

/// A result state from [`RenderScope`].
#[derive(Debug)]
pub struct Done<I> {
    ops: I,
    frame: SurfaceTexture,
    configure: bool,
}
