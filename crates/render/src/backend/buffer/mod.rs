use bytemuck::{bytes_of, cast_slice};
use std::collections::BTreeSet;
use std::num::NonZero;
use tracing::{error, instrument};
use wgpu::util::*;
use wgpu::*;

use crate::shape::Shape;

mod bind;
mod iter;
mod raw;

pub use bind::*;
use iter::*;
use raw::*;

const MAX_BATCH_BYTES: usize = 2 * 1024;
const MAX_HOLE_BYTES: usize = 256;
const CHUNK_BYTES: BufferAddress = 64 * 1024;

const DEFAULT_CAPACITY: usize = 8192;

/// A bindable buffer for the shape types.
#[derive(Debug)]
pub struct ShapeBuffer<T> {
    version: u32,
    host: Vec<T>,
    guest: ShapeBufferRaw<T>,
    delta: BTreeSet<usize>,
    belt: StagingBelt,
    layout: ShapeBufferBindGroupLayout<T>,
    bind: ShapeBufferBindGroup<T>,
}

impl<T: Shape> ShapeBuffer<T> {
    /// Create a new [`ShapeBuffer`].
    #[must_use]
    pub fn new(device: &Device) -> Self {
        Self::with_capacity(device, const { NonZero::new(DEFAULT_CAPACITY).unwrap() })
    }

    /// Create a new [`ShapeBuffer`] with capacity.
    #[must_use]
    pub fn with_capacity(device: &Device, capacity: NonZero<usize>) -> Self {
        const {
            assert!(size_of::<T>() != 0, "size of type must not be zero");
            assert!(
                (size_of::<T>() as BufferAddress).is_multiple_of(COPY_BUFFER_ALIGNMENT),
                "size of type must be multiple of wgpu::COPY_BUFFER_ALIGNMENT",
            );
        }

        let len = capacity.get();
        let version = 0;

        let guest = ShapeBufferRaw::new(device, len, version);
        let layout = ShapeBufferBindGroupLayout::new(device);
        let bind = ShapeBufferBindGroup::new(device, &layout, &guest, version);

        Self {
            version,
            host: Vec::with_capacity(len),
            guest,
            delta: BTreeSet::new(),
            belt: StagingBelt::new(device.clone(), CHUNK_BYTES),
            layout,
            bind,
        }
    }

    /// Opens a mutable scope for `self`.
    ///
    /// See [`ShapeBufferScope::close`] for more details.
    #[must_use]
    pub const fn open(&mut self) -> ShapeBufferScope<'_, T> {
        ShapeBufferScope::new(self)
    }

    /// Returns a set of [`BindGroup`] corresponding to `self`.
    #[must_use]
    pub const fn as_binding(&self) -> &ShapeBufferBindGroup<T> {
        &self.bind
    }

    /// Returns a set of [`BindGroupLayout`] corresponding to `self`.
    #[must_use]
    pub const fn as_layout(&self) -> &ShapeBufferBindGroupLayout<T> {
        &self.layout
    }

    fn write(&mut self, id: usize, shape: T) -> bool {
        debug_assert!(shape.is_visible(), "setting dead rect at {id}");

        if let Some(old) = self.host.get_mut(id) {
            if bytes_of(old) != bytes_of(&shape) {
                *old = shape;
                self.delta.insert(id);
            }

            false
        } else {
            self.delta.insert(id);

            let diff = id - self.host.len() + 1;
            self.host.reserve(diff);
            for _ in 1..diff {
                self.host.push(T::zeroed());
            }
            self.host.push(shape);

            true
        }
    }

    fn realloc_unchecked(&mut self, device: &Device, encoder: &mut CommandEncoder) {
        self.version += 1;

        let new = ShapeBufferRaw::new(device, self.host.capacity(), self.version);
        self.guest.copy_to(&new, encoder);
        self.guest = new;

        self.bind = ShapeBufferBindGroup::new(device, &self.layout, &self.guest, self.version);
    }

    fn done_unchecked(&mut self, encoder: &mut CommandEncoder) {
        let coarsed: CoarseIter<_, T> = self.delta.iter().copied().into();
        for range in coarsed {
            let src: &[u8] = cast_slice(&self.host[range.clone()]);

            let slice = self.guest.contents();
            let offset = slice.offset() + (range.start * size_of::<T>()) as BufferAddress;
            let size = slice.size() + src.len() as BufferAddress;

            let mut dst = self.belt.write_buffer(
                encoder,
                slice.buffer(),
                offset,
                BufferSize::new(size).unwrap(),
            );
            dst.copy_from_slice(src);
        }

        self.belt.finish_and_recall_on_submit(encoder);
        self.delta.clear();
    }
}

/// A mutable scope for [`ShapeBufferScope`].
#[derive(Debug)]
pub struct ShapeBufferScope<'a, T: Shape> {
    done: bool,
    realloc: bool,
    inner: &'a mut ShapeBuffer<T>,
}

impl<'a, T: Shape> ShapeBufferScope<'a, T> {
    #[must_use]
    const fn new(inner: &'a mut ShapeBuffer<T>) -> Self {
        Self {
            done: false,
            realloc: false,
            inner,
        }
    }

    /// Writes given shape at specified index,
    /// enlarging internal buffer with zero when index out of range.
    pub fn write(&mut self, id: usize, shape: T) {
        self.realloc |= self.inner.write(id, shape);
    }

    /// Closes `self`.
    ///
    /// The specified encoder must be submitted before creating
    /// new `ShapeBufferScope` to prevent memory leak.
    #[instrument]
    pub fn close(mut self, device: &Device, encoder: &mut CommandEncoder) {
        if self.realloc {
            self.inner.realloc_unchecked(device, encoder);
        }

        self.inner.done_unchecked(encoder);
        self.done = true;
    }
}

impl<T: Shape> Drop for ShapeBufferScope<'_, T> {
    fn drop(&mut self) {
        // there is no way to do assertion before drop;
        // this is best-effort to do assertion.
        debug_assert!(self.inner.delta.is_empty(), "delta corrupted");

        if self.done {
            return;
        }

        error!("scope dropped without finalization");
    }
}
