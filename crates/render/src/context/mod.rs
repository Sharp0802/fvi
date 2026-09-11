//! A renewable rendering context.

use std::fmt::Debug;
use std::sync::Arc;
use tracing::{info, instrument, warn};
use wgpu::*;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::config::*;
use crate::gfx::blit;
use crate::{Frame, InitError, RenderError, label};

mod adapter;
mod device;
mod texture;

use adapter::*;
use device::*;
pub use texture::*;

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
    pub(crate) device: RenderDevice,
    pub(crate) texture_map: TextureMap,
    format: TextureFormat,
    pipeline: RenderPipeline,
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
        let texture_map = TextureMap::new(&device, device.as_ref());
        let format = surface.get_capabilities(device.as_ref()).formats[0];

        let shader = blit::create_shader_module_embed_source(&device);
        let pipeline_layout = blit::create_pipeline_layout(&device);
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: label!("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: blit::vertex_state(&shader, &blit::vs_main_entry()),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(IndexFormat::Uint32),
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(blit::fragment_state(
                &shader,
                &blit::fs_main_entry([Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })]),
            )),
            multiview_mask: None,
            cache: None,
        });

        let this = Self {
            config,
            pref: pref.clone(),
            size: window.inner_size(),
            window,
            instance,
            surface,
            device,
            texture_map,
            format,
            pipeline,
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

    fn renew(&mut self) -> Result<(), RenderError> {
        self.surface = self
            .instance
            .create_surface(self.window.clone())
            .map_err(InitError::from)?;

        if self.device.is_lost() || {
            let adapter: &Adapter = self.device.as_ref();
            let supported = adapter.is_surface_supported(&self.surface);
            !supported
        } {
            return Err(RenderError::DeviceLost);
        }

        self.configure_surface();

        Ok(())
    }

    /// Sets surface preference as given.
    pub fn set_preference(&mut self, pref: SurfacePref) {
        if self.pref.surface != pref {
            self.pref.surface = pref;
            info!("surface preference changed");
            self.configure_surface();
        }
    }

    /// Resizes the surface as given.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if self.size == size {
            return;
        }

        self.size = size;
        self.configure_surface();
    }

    #[instrument]
    fn get_frame(&mut self) -> Result<Option<(SurfaceTexture, bool)>, RenderError> {
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
                    self.renew()?;
                }
                CurrentSurfaceTexture::Validation => return Err(RenderError::Invalid),
            }
        };

        Ok(Some((frame, configure)))
    }

    /// Blits given frames onto the current surface.
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot be rendered.
    /// See [`RenderError`] for details.
    #[instrument(skip_all)]
    pub fn render<'a>(
        &mut self,
        frames: impl Iterator<Item = &'a mut Frame>,
        clear: Color,
    ) -> Result<(), RenderError> {
        if self.size.width == 0 || self.size.height == 0 {
            return Ok(());
        }

        let Some((frame, configure)) = self.get_frame()? else {
            return Ok(());
        };

        let view = frame.texture.create_view(&TextureViewDescriptor {
            label: label!("surface_view"),
            format: Some(self.format),
            dimension: Some(TextureViewDimension::D2),
            usage: Some(TextureUsages::RENDER_ATTACHMENT),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: None,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: label!("encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: label!("render"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            for frame in frames {
                let bind_group = frame.update(self, &mut pass);
                pass.set_pipeline(&self.pipeline);
                bind_group.set(&mut pass);
                pass.draw(0..3, 0..1);
            }
        }

        let command = encoder.finish();
        let queue: &Queue = self.device.as_ref();
        queue.submit([command]);
        queue.present(frame);

        if configure {
            self.configure_surface();
        }

        Ok(())
    }
}
