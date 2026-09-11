use wgpu::*;
use winit::dpi::PhysicalSize;

use crate::context::RenderContext;
use crate::draw::{Draw, RectBuffer};
use crate::gfx::blit::*;
use crate::label;
use crate::raw::View;

#[derive(Debug)]
pub struct Frame {
    version: u32,
    scale: f32,
    size: PhysicalSize<u32>,
    target: TextureView,
    rect_buffer: RectBuffer,
    pipeline: Draw,
    dirty: bool,
    bind_group: WgpuBindGroup0,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameDescriptor {
    pub scale: f32,
    pub size: PhysicalSize<u32>,
    pub format: TextureFormat,
    pub sample_count: u32,
}

impl Frame {
    pub(crate) fn new(device: &Device, desc: &FrameDescriptor) -> Self {
        let target = Self::create_texture(
            0,
            device,
            desc.size.width,
            desc.size.height,
            desc.format,
            desc.sample_count,
        );

        let bind_group = Self::create_bind_group(device, &target);

        Self {
            version: 0,
            scale: desc.scale,
            size: desc.size,
            target,
            bind_group,
            rect_buffer: RectBuffer::new(device),
            pipeline: Draw::new(device, desc.format, desc.sample_count),
            dirty: false,
        }
    }

    pub(crate) fn desc(&self) -> FrameDescriptor {
        FrameDescriptor {
            scale: self.scale,
            size: self.size,
            format: self.target.texture().format(),
            sample_count: self.target.texture().sample_count(),
        }
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        if self.size == size {
            return;
        }

        self.size = size;
        self.dirty = true;
    }

    pub(crate) fn update(&mut self, cx: &RenderContext, pass: &mut RenderPass) -> &WgpuBindGroup0 {
        if !self.dirty {
            return &self.bind_group;
        }

        self.dirty = false;

        let device = &cx.device;
        let queue = cx.device.as_ref();
        let tex_map = &cx.texture_map;

        let old_tex = self.target.texture();
        if self.size.width != self.target.texture().size().width
            || self.size.height != self.target.texture().size().height
        {
            self.version += 1;
            self.target = Self::create_texture(
                self.version,
                device,
                self.size.width,
                self.size.height,
                old_tex.format(),
                old_tex.sample_count(),
            );
            self.bind_group = Self::create_bind_group(device, &self.target);
        }

        self.rect_buffer.apply(device, queue);
        self.pipeline.run(
            device,
            queue,
            pass,
            &self.rect_buffer,
            tex_map,
            View {
                size: [
                    self.target.texture().size().width,
                    self.target.texture().size().height,
                ],
                scale: self.scale,
                _pad: 0,
            },
        );

        &self.bind_group
    }

    fn create_bind_group(device: &Device, view: &TextureView) -> WgpuBindGroup0 {
        WgpuBindGroup0::from_bindings(
            device,
            WgpuBindGroup0Entries::new(WgpuBindGroup0EntriesParams { tex: view }),
        )
    }

    fn create_texture(
        version: u32,
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        sample_count: u32,
    ) -> TextureView {
        device
            .create_texture(&TextureDescriptor {
                label: label!("target/{}", version),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&TextureViewDescriptor {
                label: label!("target_view/{}", version),
                ..Default::default()
            })
    }
}
