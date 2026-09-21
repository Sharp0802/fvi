use swash::scale::image::Content;
use wgpu::*;

use super::SwashImage;

#[derive(Clone, Debug)]
pub struct Image<'a> {
    pub colored: bool,
    pub width: u32,
    pub height: u32,
    pub data: &'a [u8],
}

impl<'a> Image<'a> {
    pub fn from(value: &'a SwashImage) -> Self {
        Self {
            colored: match value.content {
                Content::Mask => false,
                Content::SubpixelMask | Content::Color => true,
            },
            width: value.placement.width,
            height: value.placement.height,
            data: &value.data,
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0 || self.data.is_empty()
    }

    pub fn write_to(&self, queue: &Queue, dst: &Texture, origin: Origin3d) {
        debug_assert_eq!((self.width * self.height) as usize, self.data.len());
        debug_assert_eq!(
            if self.colored {
                TextureFormat::Rgba8Unorm
            } else {
                TextureFormat::R8Unorm
            },
            dst.format()
        );

        if self.is_empty() {
            return;
        }

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: dst,
                mip_level: 0,
                origin,
                aspect: TextureAspect::All,
            },
            self.data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width * if self.colored { 4 } else { 1 }),
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
