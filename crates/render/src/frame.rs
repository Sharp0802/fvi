use wgpu::*;
use winit::dpi::PhysicalSize;

use crate::context::TextureMap;
use crate::draw::{Draw, RectBuffer};
use crate::label;
use crate::raw::View;

#[derive(Debug)]
pub struct Frame {
    version: u32,
    scale: f32,
    target: Texture,
    rect_buffer: RectBuffer,
    pipeline: Draw,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameDescriptor {
    pub scale: f32,
    pub size: PhysicalSize<u32>,
    pub format: TextureFormat,
    pub sample_count: u32,
}

impl Frame {
    pub fn new(device: &Device, desc: &FrameDescriptor) -> Self {
        Self {
            version: 0,
            scale: desc.scale,
            target: Self::create_texture(
                0,
                device,
                desc.size.width,
                desc.size.height,
                desc.format,
                desc.sample_count,
            ),
            rect_buffer: RectBuffer::new(device),
            pipeline: Draw::new(device, desc.format, desc.sample_count),
        }
    }

    pub fn resize(&mut self, device: &Device) {
        self.version += 1;
        self.target = Self::create_texture(
            self.version,
            device,
            self.target.size().width,
            self.target.size().height,
            self.target.format(),
            self.target.sample_count(),
        );
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        pass: &mut RenderPass,
        tex_map: &TextureMap,
    ) {
        self.rect_buffer.apply(device, queue);
        self.pipeline.run(
            device,
            queue,
            pass,
            &self.rect_buffer,
            tex_map,
            View {
                size: [self.target.size().width, self.target.size().height],
                scale: self.scale,
                _pad: 0,
            },
        );
    }

    fn create_texture(
        version: u32,
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        sample_count: u32,
    ) -> Texture {
        device.create_texture(&TextureDescriptor {
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
    }
}
