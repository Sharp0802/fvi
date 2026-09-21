use swash::scale::image::Content;
use wgpu::*;

use super::SwashImage;

#[derive(Clone, Debug)]
pub struct Image<'a> {
    format: TextureFormat,
    width: u32,
    height: u32,
    data: &'a [u8],
}

impl<'a> Image<'a> {
    pub fn from(value: &'a SwashImage) -> Self {
        Self {
            format: match value.content {
                Content::Mask => TextureFormat::R8Unorm,
                Content::SubpixelMask => TextureFormat::Rgba8Unorm,
                Content::Color => TextureFormat::Rgba8Unorm,
            },
            width: value.placement.width,
            height: value.placement.height,
            data: &value.data,
        }
    }

    pub fn write_to(&self, queue: &Queue, dst: &Texture, origin: Origin3d) {
        debug_assert_eq!((self.width * self.height) as usize, self.data.len());
        debug_assert_eq!(self.format, dst.format());

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: dst,
                mip_level: 0,
                origin,
                aspect: TextureAspect::All,
            },
            &self.data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width),
                rows_per_image: Some(self.height),
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }
}
