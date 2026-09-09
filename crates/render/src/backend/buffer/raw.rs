use std::marker::PhantomData;
use wgpu::*;

use crate::label;

const USAGE: BufferUsages = BufferUsages::COPY_DST
    .union(BufferUsages::COPY_SRC)
    .union(BufferUsages::STORAGE);

#[derive(Debug)]
pub struct ShapeBufferRaw<T> {
    len: usize,
    inner: Buffer,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ShapeBufferRaw<T> {
    pub fn new(device: &Device, len: usize, version: u32) -> Self {
        let buffer_size = len * const { size_of::<T>() + size_of::<usize>() };

        let inner = device.create_buffer(&BufferDescriptor {
            label: label!("buffer/{}", version),
            size: buffer_size as BufferAddress,
            usage: USAGE,
            mapped_at_creation: false,
        });

        Self {
            len,
            inner,
            _marker: PhantomData,
        }
    }

    pub fn copy_to(&self, dst: &Self, encoder: &mut CommandEncoder) {
        encoder.copy_buffer_to_buffer(&self.inner, 0, &dst.inner, 0, self.contents().size());
    }

    pub fn contents(&self) -> BufferSlice<'_> {
        let contents_len = self.len * const { size_of::<T>() };
        self.inner.slice(..(contents_len as BufferAddress))
    }

    pub fn visibles(&self) -> BufferSlice<'_> {
        let contents_len = self.len * const { size_of::<T>() };
        self.inner.slice((contents_len as BufferAddress)..)
    }
}
