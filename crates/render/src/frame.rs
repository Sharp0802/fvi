use wgpu::*;
use winit::dpi::PhysicalSize;

use crate::context::TextureMap;
use crate::draw::{Draw, RectBuffer};
use crate::gfx::blit::*;
use crate::raw::{Rect, View};
use crate::{Canvas, Dp, label};

/// A render frame.
#[derive(Debug)]
pub struct Frame {
    version: u32,
    scale: f32,
    size: PhysicalSize<u32>,
    format: TextureFormat,
    sample_count: u32,
    target: Option<FrameTarget>,
    rect_buffer: RectBuffer,
    pipeline: Draw,
    dirty: bool,
    texture_version: Option<u32>,
}

#[derive(Debug)]
struct FrameTarget {
    view: TextureView,
    msaa: Option<TextureView>,
    bind_group: WgpuBindGroup0,
}

/// A descriptor of [`Frame`].
#[derive(Clone, Debug, PartialEq)]
pub struct FrameDescriptor {
    /// The window scale factor.
    pub scale: f32,
    /// A size of frame, in px.
    pub size: PhysicalSize<u32>,
    /// A format of frame.
    pub format: TextureFormat,
    /// A sample count of frame.
    pub sample_count: u32,
}

impl Frame {
    pub(crate) fn new(device: &Device, desc: &FrameDescriptor) -> Self {
        Self {
            version: 0,
            scale: desc.scale,
            size: desc.size,
            format: desc.format,
            sample_count: desc.sample_count,
            target: None,
            rect_buffer: RectBuffer::new(device),
            pipeline: Draw::new(device, desc.format, desc.sample_count),
            dirty: true,
            texture_version: None,
        }
    }

    /// Begins a session.
    pub const fn canvas(&mut self) -> Canvas<'_> {
        Canvas::new(self)
    }

    /// Resizes `self` as given size, invalidating its cached target.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if self.size != size {
            self.size = size;
            self.dirty = true;
        }
    }

    pub(crate) fn write_rect(&mut self, index: usize, rect: Rect) {
        self.dirty |= self.rect_buffer.write(index, rect);
    }

    pub(crate) fn truncate_rects(&mut self, len: usize) {
        self.dirty |= self.rect_buffer.truncate(len);
    }

    pub(crate) const fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub(crate) fn update(
        &mut self,
        device: &Device,
        queue: &Queue,
        tex_map: &TextureMap,
        encoder: &mut CommandEncoder,
    ) -> Option<&WgpuBindGroup0> {
        if self.size.width == 0 || self.size.height == 0 {
            return None;
        }

        let texture_version = tex_map.snapshot().version;
        if self.texture_version != Some(texture_version) {
            self.dirty = true;
        }

        if self.dirty {
            if self.target.as_ref().is_none_or(|target| {
                let size = target.view.texture().size();
                size.width != self.size.width || size.height != self.size.height
            }) {
                self.target = Some(self.create_target(device));
                self.version = self.version.wrapping_add(1);
            }

            self.rect_buffer.apply(device, encoder);
            let target = self.target.as_ref().unwrap();
            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: label!("draw"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: target.msaa.as_ref().unwrap_or(&target.view),
                        depth_slice: None,
                        resolve_target: target.msaa.as_ref().map(|_| &target.view),
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: if target.msaa.is_some() {
                                StoreOp::Discard
                            } else {
                                StoreOp::Store
                            },
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                if self.rect_buffer.len() != 0 {
                    self.pipeline.run(
                        device,
                        queue,
                        &mut pass,
                        &self.rect_buffer,
                        tex_map,
                        View {
                            size: [self.size.width, self.size.height],
                            scale: Dp(1.0).to_px(self.scale),
                            _pad: 0,
                        },
                    );
                }
            }
            self.dirty = false;
            self.texture_version = Some(texture_version);
        }

        self.target.as_ref().map(|target| &target.bind_group)
    }

    fn create_target(&self, device: &Device) -> FrameTarget {
        let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        let view = self.create_texture(device, 1, usage);
        let msaa = (self.sample_count > 1).then(|| {
            self.create_texture(device, self.sample_count, TextureUsages::RENDER_ATTACHMENT)
        });
        let bind_group = WgpuBindGroup0::from_bindings(
            device,
            WgpuBindGroup0Entries::new(WgpuBindGroup0EntriesParams { tex: &view }),
        );
        FrameTarget {
            view,
            msaa,
            bind_group,
        }
    }

    fn create_texture(
        &self,
        device: &Device,
        sample_count: u32,
        usage: TextureUsages,
    ) -> TextureView {
        device
            .create_texture(&TextureDescriptor {
                label: label!("target/{}/{}", self.version, sample_count),
                size: Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: TextureDimension::D2,
                format: self.format,
                usage,
                view_formats: &[],
            })
            .create_view(&TextureViewDescriptor {
                label: label!("target_view/{}/{}", self.version, sample_count),
                ..Default::default()
            })
    }
}
