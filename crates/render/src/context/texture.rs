use std::collections::HashMap;
use std::iter::once;
use std::mem::replace;
use wgpu::util::*;
use wgpu::*;

use crate::label;

/// A key for internal textures.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InternalTextureId(u32);

impl InternalTextureId {
    const WHITE: Self = Self(0);
    const COUNT: u32 = 1;
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

impl TextureRef {
    /// A white texture.
    pub const WHITE: Self = Self(InternalTextureId::WHITE.0);

    pub(crate) const fn index(self) -> u32 {
        self.0
    }

    pub(crate) const fn is_internal(self) -> bool {
        self.index() < InternalTextureId::COUNT
    }
}

impl From<InternalTextureId> for TextureRef {
    fn from(value: InternalTextureId) -> Self {
        Self(value.0)
    }
}

/// An error during texture insertion.
#[derive(Debug)]
pub enum InsertionError {
    /// The specified slot is already occupied.
    Occupied,
    /// System has insufficient memory for requested operation.
    InsufficientMemory,
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "it's crate-scoped although parent is pub"
)]
#[derive(Debug)]
pub(crate) struct Snapshot<'a> {
    pub version: u32,
    pub views: &'a [TextureView],
}

/// A texture map.
#[derive(Debug)]
pub struct TextureMap {
    version: u32,
    vec: Vec<TextureView>,
    bitmap: Vec<u128>,
    route: HashMap<TextureId, u32>,
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
            bitmap: vec![1],
            route: once((InternalTextureId::WHITE.into(), 0)).collect(),
        }
    }

    pub(crate) const fn snapshot(&self) -> Snapshot<'_> {
        Snapshot {
            version: self.version,
            views: self.vec.as_slice(),
        }
    }

    fn find_empty(&self) -> Option<usize> {
        for (i, &chunk) in self.bitmap.iter().enumerate() {
            if let Some(j) = (!chunk).lowest_one() {
                let index = i * 128 + j as usize;
                if index < self.vec.len() {
                    return Some(index);
                }
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

        let index = self.find_empty().unwrap_or(self.vec.len());
        let index32 = index
            .try_into()
            .map_err(|_| InsertionError::InsufficientMemory)?;

        if index == self.vec.len() {
            if index / 128 == self.bitmap.len() {
                self.bitmap.push(0);
            }
            self.vec.push(value);
        } else {
            self.vec[index] = value;
        }

        self.bitmap[index / 128] |= 1 << (index % 128);
        self.route.insert(key, index32);
        self.version = self.version.wrapping_add(1);

        Ok(TextureRef(index32))
    }

    /// Returns a texture corresponding to given key,
    /// returning `None` if there is no such texture.
    #[must_use]
    pub fn get(&self, key: &TextureId) -> Option<TextureRef> {
        let &index = self.route.get(key)?;
        Some(TextureRef(index))
    }

    /// Updates existing texture at reference as given,
    /// returning `Some` for existing old value;
    /// otherwise `None` without any mutation of `self`.
    ///
    /// The internal textures cannot be updated.
    pub fn update(&mut self, tex: TextureRef, value: TextureView) -> Option<TextureView> {
        let index = tex.0 as usize;
        if tex.is_internal() || self.bitmap.get(index / 128)? & (1 << (index % 128)) == 0 {
            return None;
        }

        let old = replace(self.vec.get_mut(index)?, value);
        self.version = self.version.wrapping_add(1);
        Some(old)
    }

    /// Removes a texture corresponding to given key,
    /// returning `Some` for removed value; otherwise `None`.
    ///
    /// The internal textures cannot be removed.
    pub fn remove(&mut self, key: &TextureId) -> Option<TextureView> {
        let &index = self.route.get(key)?;
        if TextureRef(index).is_internal() || (index as usize) >= self.vec.len() {
            return None;
        }
        let occupied = self.bitmap.get_mut((index as usize) / 128)?;
        let mask = 1 << (index % 128);
        if *occupied & mask == 0 {
            return None;
        }
        *occupied &= !mask;
        self.route.remove(key);

        let white = self.vec[0].clone();
        let old = replace(&mut self.vec[index as usize], white);
        self.version = self.version.wrapping_add(1);
        Some(old)
    }
}
