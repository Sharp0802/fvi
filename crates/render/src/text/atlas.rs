use etagere::euclid::Size2D;
use etagere::{AllocId, AtlasAllocator};
use std::fmt::Debug;
use tracing::warn;
use wgpu::*;

use crate::label;
use crate::text::Image;

const PAGE_SIZE: u32 = 2048;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct AtlasView {
    page: usize,
    id: AllocId,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct AtlasPart {
    pub tex: TextureView,
    pub pos: [u32; 2],
    pub size: [u32; 2],
}

#[derive(Debug)]
pub struct Atlas {
    size: u32,
    colored: bool,
    pages: Vec<Page>,
}

impl Atlas {
    pub fn new(colored: bool) -> Self {
        Self::with_size(PAGE_SIZE, colored)
    }

    fn with_size(size: u32, colored: bool) -> Self {
        Self {
            size,
            colored,
            pages: Vec::new(),
        }
    }

    pub fn read(&self, view: &AtlasView) -> AtlasPart {
        let rect = self.pages[view.page].alloc.get(view.id);

        AtlasPart {
            tex: self.pages[view.page]
                .tex
                .create_view(&TextureViewDescriptor {
                    usage: Some(TextureUsages::TEXTURE_BINDING),
                    ..Default::default()
                }),
            pos: rect.min.to_u32().to_array(),
            size: rect.size().to_u32().to_array(),
        }
    }

    pub fn store(&mut self, device: &Device, queue: &Queue, image: &Image) -> AtlasView {
        for (i, page) in self.pages.iter_mut().enumerate() {
            if let Some(alloc) = page.store(queue, image) {
                return AtlasView {
                    page: i,
                    id: alloc.id,
                };
            }
        }

        let size = self.size.max(image.width).max(image.height);
        if size != self.size {
            warn!(
                "too large image requested ({}x{}; {}x{} preferred)",
                image.width, image.height, self.size, self.size,
            );
        }

        let page_id = self.pages.len();
        let page = self.pages.push_mut(Page::new(device, size, self.colored));
        let alloc = page.store(queue, image).unwrap();

        AtlasView {
            page: page_id,
            id: alloc.id,
        }
    }

    pub fn remove(&mut self, view: AtlasView) {
        self.pages[view.page].remove(view.id);
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct PageAlloc {
    id: AllocId,
}

struct Page {
    tex: Texture,
    view: TextureView,
    alloc: AtlasAllocator,
}

impl Page {
    pub fn new(device: &Device, size: u32, colored: bool) -> Self {
        let tex = device.create_texture(&TextureDescriptor {
            label: label!("tex"),
            size: Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: if colored {
                TextureFormat::Rgba8Unorm
            } else {
                TextureFormat::R8Unorm
            },
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = tex.create_view(&TextureViewDescriptor {
            label: label!("tex"),
            ..Default::default()
        });

        let alloc = AtlasAllocator::new(Size2D::new(size, size).to_i32());

        Self { tex, view, alloc }
    }

    pub fn store(&mut self, queue: &Queue, image: &Image) -> Option<PageAlloc> {
        let alloc = self
            .alloc
            .allocate(Size2D::new(image.width, image.height).to_i32())?;

        let pos = alloc.rectangle.min.to_u32();

        image.write_to(
            queue,
            &self.tex,
            Origin3d {
                x: pos.x,
                y: pos.y,
                z: 0,
            },
        );

        Some(PageAlloc { id: alloc.id })
    }

    pub fn remove(&mut self, alloc: AllocId) {
        self.alloc.deallocate(alloc);
    }
}

impl Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Atlas")
            .field("tex", &self.tex)
            .field("view", &self.view)
            .field("used", &self.alloc.allocated_space())
            .field("free", &self.alloc.free_space())
            .finish_non_exhaustive()
    }
}
