use std::collections::HashMap;
use std::iter::{once, repeat_n};
use std::mem::replace;
use std::ops::Deref;
use wgpu::util::*;
use wgpu::*;

use super::*;
use crate::label;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InternalTextureId(u32);

impl InternalTextureId {
    const WHITE: Self = Self(0);
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureId {
    Internal(InternalTextureId),
}

impl From<InternalTextureId> for TextureId {
    fn from(value: InternalTextureId) -> Self {
        Self::Internal(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureRef(u32);

#[derive(Debug)]
pub enum InsertionError {
    Occupied,
    InsufficientMemory,
}

#[derive(Debug)]
pub struct TextureMap {
    version: u32,
    vec: Vec<TextureView>,
    bitmap: Vec<u128>,
    route: HashMap<TextureId, usize>,
}

impl TextureMap {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        let version = 0;

        let white = device
            .create_texture_with_data(
                queue,
                &TextureDescriptor {
                    label: label!("tex_white"),
                    size: Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    usage: TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                TextureDataOrder::LayerMajor,
                &[0xFF, 0xFF, 0xFF, 0xFF],
            )
            .create_view(&TextureViewDescriptor {
                label: label!("tex_white_view"),
                ..Default::default()
            });

        Self {
            version,
            vec: vec![white],
            bitmap: Vec::new(),
            route: [(InternalTextureId::WHITE.into(), 0)].into_iter().collect(),
        }
    }

    fn find_empty(&self) -> Option<usize> {
        for (i, &chunk) in self.bitmap.iter().enumerate() {
            if let Some(j) = (!chunk).lowest_one() {
                return Some(i * 128 + j as usize);
            }
        }

        None
    }

    pub fn insert(
        &mut self,
        key: TextureId,
        value: TextureView,
    ) -> Result<TextureRef, InsertionError> {
        if self.route.contains_key(&key) {
            return Err(InsertionError::Occupied);
        }

        let index = if let Some(index) = self.find_empty() {
            self.route.insert(key, index);
            self.bitmap[index / 128] |= 1 << (index % 128);
            self.vec[index] = value;
            index
        } else {
            let index = self.vec.len() * 128;

            let white = self.vec[0].clone();
            self.vec.extend(once(value).chain(repeat_n(white, 127)));
            self.bitmap.push(1);
            self.route.insert(key, index);
            self.version += 1;

            index
        };

        let index32 = index
            .try_into()
            .map_err(|_| InsertionError::InsufficientMemory)?;

        Ok(TextureRef(index32))
    }

    pub fn get(&self, key: &TextureId) -> Option<TextureRef> {
        let &index = self.route.get(&key)?;
        let index32 = index.try_into().expect("is checked at insert()");
        Some(TextureRef(index32))
    }

    pub fn update(&mut self, tex: TextureRef, value: TextureView) -> Option<TextureView> {
        if self.bitmap[(tex.0 as usize) / 128] & (1 << (tex.0 % 128)) == 0 {
            return None;
        }

        let old = replace(&mut self.vec[tex.0 as usize], value);
        Some(old)
    }

    pub fn remove(&mut self, key: &TextureId) -> Option<TextureView> {
        let index = self.route.remove(key)?;
        self.bitmap[index / 128] &= !(1 << (index % 128));

        let white = self.vec[0].clone();
        let old = replace(&mut self.vec[index], white);
        Some(old)
    }
}

#[derive(Debug)]
pub struct TextureMapBind {
    version: u32,
    sampler: Sampler,
    bind_group: WgpuBindGroup1,
}

impl TextureMapBind {
    pub fn new(device: &Device) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: label!("sampler"),
            ..Default::default()
        });

        let bind_group = WgpuBindGroup1::from_bindings(
            device,
            WgpuBindGroup1Entries::new(WgpuBindGroup1EntriesParams {
                texs: &[],
                samp: &sampler,
            }),
        );

        Self {
            version: u32::MAX,
            sampler,
            bind_group,
        }
    }

    pub fn update(&mut self, device: &Device, map: &TextureMap) -> &WgpuBindGroup1 {
        if self.version != map.version {
            self.version = map.version;

            let refs: Vec<_> = map.vec.iter().collect();
            self.bind_group = WgpuBindGroup1::from_bindings(
                device,
                WgpuBindGroup1Entries::new(WgpuBindGroup1EntriesParams {
                    texs: &refs,
                    samp: &self.sampler,
                }),
            );
        }

        &self.bind_group
    }
}

impl Deref for TextureMapBind {
    type Target = WgpuBindGroup1;

    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}
