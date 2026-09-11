use std::collections::HashMap;
use std::iter::{once, repeat_n};
use std::mem::replace;
use wgpu::util::*;
use wgpu::*;

use crate::label;

/// A key for internal textures.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InternalTextureId(u32);

impl InternalTextureId {
    const WHITE: Self = Self(0);
}

/// A key of texture map.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureId {
    /// An internal texture.
    Internal(InternalTextureId),
}

impl From<InternalTextureId> for TextureId {
    fn from(value: InternalTextureId) -> Self {
        Self::Internal(value)
    }
}

/// An opaque reference to texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureRef(u32);

/// An error during texture insertion.
#[derive(Debug)]
pub enum InsertionError {
    /// The specified slot is already occupied.
    Occupied,
    /// System has insufficient memory for requested operation.
    InsufficientMemory,
}

/// A texture map.
#[derive(Debug)]
pub struct TextureMap {
    pub(crate) version: u32,
    pub(crate) vec: Vec<TextureView>,
    bitmap: Vec<u128>,
    route: HashMap<TextureId, usize>,
}

impl TextureMap {
    pub(crate) fn new(device: &Device, queue: &Queue) -> Self {
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
            route: once((InternalTextureId::WHITE.into(), 0)).collect(),
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

    /// Inserts a texture into `self`.
    ///
    /// # Errors
    ///
    /// This function may return `Err`.
    /// See [`InsertionError`] for more details.
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

    /// Returns a texture corresponding to given key,
    /// returning `None` if there is no such texture.
    #[must_use]
    pub fn get(&self, key: &TextureId) -> Option<TextureRef> {
        let &index = self.route.get(key)?;
        #[expect(clippy::missing_panics_doc, reason = "is checked at insert()")]
        let index32 = index.try_into().unwrap();
        Some(TextureRef(index32))
    }

    /// Updates existing texture at reference as given,
    /// returning `Some` for existing old value;
    /// otherwise `None` without any mutation of `self`.
    pub fn update(&mut self, tex: TextureRef, value: TextureView) -> Option<TextureView> {
        if self.bitmap[(tex.0 as usize) / 128] & (1 << (tex.0 % 128)) == 0 {
            return None;
        }

        let old = replace(&mut self.vec[tex.0 as usize], value);
        Some(old)
    }

    /// Removes a texture corresponding to given key,
    /// returning `Some` for removed value; otherwise `None`.
    pub fn remove(&mut self, key: &TextureId) -> Option<TextureView> {
        let index = self.route.remove(key)?;
        self.bitmap[index / 128] &= !(1 << (index % 128));

        let white = self.vec[0].clone();
        let old = replace(&mut self.vec[index], white);
        Some(old)
    }
}
