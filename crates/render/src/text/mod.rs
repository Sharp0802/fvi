//! A module for text system.

use etagere::{AllocId, AtlasAllocator};
use std::fmt::Debug;
use tracing::warn;
use wgpu::*;

use crate::context::{TextureMap, TextureRef};
use crate::label;
use crate::{Cache, Dp};

mod error;
mod font;
mod style;

pub use error::*;
pub use font::*;
pub use style::*;

/// An identifier of a styled glyph.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    id: u16,
    style: TextStyle,
}

/// A glyph.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    /// Atlas texture of glyph.
    pub tex: TextureRef,
    /// Origin coord on atlas texture, in px.
    pub origin: [u32; 2],
    /// Size of glyph, in dp.
    pub size: [Dp; 2],
    /// Offset of glyph from baseline, in dp.
    pub off: [Dp; 2],
    atlas: AtlasId,
    alloc: AllocId,
}

#[derive(Clone, Debug, PartialEq)]
struct GlyphData {
    size: [u32; 2],
    off: [f32; 2],
    data: Vec<u8>,
}

/// An identifier of an atlas.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtlasId(u32);

struct Atlas {
    id: AtlasId,
    alloc: AtlasAllocator,
    tex_ref: TextureRef,
    tex: TextureView,
}

impl Atlas {
    pub fn new(
        device: &Device,
        tex_map: &mut TextureMap,
        id: AtlasId,
        size: [u32; 2],
    ) -> Result<Self, AtlasError> {
        let alloc = AtlasAllocator::new(
            [
                size[0].try_into().expect("too large width"),
                size[1].try_into().expect("too large height"),
            ]
            .into(),
        );

        let tex = device
            .create_texture(&TextureDescriptor {
                label: label!("tex"),
                size: Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&TextureViewDescriptor::default());

        let tex_ref = tex_map.insert(id.into(), tex.clone())?;

        Ok(Self {
            id,
            alloc,
            tex_ref,
            tex,
        })
    }

    fn dealloc(&mut self, alloc: AllocId) {
        self.alloc.deallocate(alloc);
    }

    fn try_alloc(&mut self, scale: f32, data: &GlyphData) -> Option<Glyph> {
        let size_signed = [
            data.size[0].try_into().expect("too large width"),
            data.size[1].try_into().expect("too large height"),
        ];

        let alloc = self.alloc.allocate(size_signed.into())?;
        let rect = alloc.rectangle.to_u32();
        Some(Glyph {
            tex: self.tex_ref,
            origin: rect.min.to_array(),
            #[expect(clippy::cast_precision_loss, reason = "size may not exceed 2^23")]
            size: *data
                .size
                .map(|px| Dp::from_px(px as f32, scale))
                .as_array::<2>()
                .unwrap(),
            off: *data
                .off
                .map(|px| Dp::from_px(px, scale))
                .as_array::<2>()
                .unwrap(),
            atlas: self.id,
            alloc: alloc.id,
        })
    }

    fn clear(&mut self) {
        self.alloc.clear();
    }
}

impl Debug for Atlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Atlas")
            .field("id", &self.id)
            .field("tex_ref", &self.tex_ref)
            .field("tex", &self.tex)
            .finish_non_exhaustive()
    }
}

/// A set of glyph atlases.
#[derive(Debug)]
pub struct AtlasSet {
    rasterizer: Rasterizer,
    cache: Cache<GlyphKey, Glyph>,
    atlases: Vec<Atlas>,
    atlas_size: [u32; 2],
    scale: f32,
}

impl AtlasSet {
    pub(crate) fn new(scale: f32) -> Self {
        Self {
            rasterizer: Rasterizer::new(),
            cache: Cache::new(),
            atlases: Vec::new(),
            atlas_size: [2048; 2],
            scale,
        }
    }

    pub(crate) fn rescale(&mut self, scale: f32) {
        if (scale - self.scale).abs() <= 0.01 {
            warn!("atlas rescaling ignored");
            return;
        }

        self.scale = scale;
        self.cache.clear();

        for atlas in &mut self.atlases {
            atlas.clear();
        }
    }

    /// Fetches a glyph from given key.
    ///
    /// # Panics
    ///
    /// This function may panic if any of conditions below met.
    ///
    /// - New texture failed to be allocated on device.
    /// - Glyph is too big to be drawn on atlas.
    pub fn fetch(
        &mut self,
        device: &Device,
        queue: &Queue,
        tex_map: &mut TextureMap,
        key: GlyphKey,
    ) -> &Glyph {
        self.cache.fetch(key, |key| {
            let data = self.rasterizer.rasterize(key);

            let mut opt = None;
            for atlas in &mut self.atlases {
                opt = atlas
                    .try_alloc(self.scale, &data)
                    .map(|glyph| (atlas.tex_ref, glyph));
                if opt.is_some() {
                    break;
                }
            }
            let (tex_ref, glyph) = if let Some(pair) = opt {
                pair
            } else {
                let id = AtlasId(self.atlases.len().try_into().expect("too many atlas"));

                let mut atlas = Atlas::new(device, tex_map, id, self.atlas_size)
                    .expect("couldn't allocate atlas");
                let tex_ref = atlas.tex_ref;

                let glyph = atlas
                    .try_alloc(self.scale, &data)
                    .expect("couldn't allocate glyph");
                self.atlases.push(atlas);
                (tex_ref, glyph)
            };

            let tex = tex_map.get(tex_ref);

            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: tex.texture(),
                    mip_level: 0,
                    origin: Origin3d {
                        x: glyph.origin[0],
                        y: glyph.origin[1],
                        z: 0,
                    },
                    aspect: TextureAspect::All,
                },
                &data.data,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(data.size[0]),
                    rows_per_image: Some(data.size[1]),
                },
                Extent3d {
                    width: data.size[0],
                    height: data.size[1],
                    depth_or_array_layers: 1,
                },
            );

            glyph
        })
    }

    pub(crate) fn update(&mut self) {
        for (_, glyph) in self.cache.sweep() {
            self.atlases[glyph.atlas.0 as usize].dealloc(glyph.alloc);
        }
        self.cache.tick();
    }
}

#[derive(Debug)]
struct Rasterizer {}

impl Rasterizer {
    pub fn new() -> Self {
        todo!()
    }

    fn rasterize(&mut self, key: &GlyphKey) -> GlyphData {
        todo!()
    }
}
