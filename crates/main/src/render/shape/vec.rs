use bytemuck::cast_slice;
use std::any::type_name;
use std::collections::BTreeSet;
use std::iter::{FusedIterator, Peekable};
use std::marker::PhantomData;
use std::num::NonZero;
use std::ops::Range;
use wgpu::util::*;
use wgpu::*;

use crate::label;
use crate::render::RenderContext;
use crate::render::shape::ShapeData;

const MAX_BATCH_BYTES: usize = 2 * 1024;
const MAX_HOLE_BYTES: usize = 256;
const CHUNK_BYTES: BufferAddress = 64 * 1024;

#[derive(Debug)]
pub struct ShapeVec<T> {
    delta: BTreeSet<usize>,
    inner: Vec<T>,
    buffer: Buffer,
    belt: StagingBelt,
}

#[derive(Clone, Copy, Debug)]
pub struct Finish<'a> {
    pub buffer: &'a Buffer,
    pub realloc: bool,
}

impl<T: ShapeData> ShapeVec<T> {
    pub fn new(cx: &RenderContext, capacity: NonZero<usize>) -> Self {
        const {
            assert!(size_of::<T>() != 0, "size of type must not be zero");
            assert!(
                (size_of::<T>() as BufferAddress).is_multiple_of(COPY_BUFFER_ALIGNMENT),
                "size of type must be multiple of wgpu::COPY_BUFFER_ALIGNMENT",
            );
        }

        let inner = Vec::with_capacity(capacity.get());

        let buffer = cx.device.create_buffer(&BufferDescriptor {
            label: label!("<{}>.buffer", type_name::<T>()),
            size: (capacity.get() * size_of::<T>()) as BufferAddress,
            usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let belt = StagingBelt::new(cx.device.clone(), CHUNK_BYTES);

        Self {
            delta: BTreeSet::new(),
            inner,
            buffer,
            belt,
        }
    }

    pub const fn capacity(&self) -> NonZero<usize> {
        NonZero::new(self.inner.capacity()).unwrap()
    }

    pub fn finish<'a>(
        &'a mut self,
        cx: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Finish<'a> {
        let Some(&max) = self.delta.last() else {
            return Finish {
                buffer: &self.buffer,
                realloc: false,
            };
        };

        let bmax = (max * size_of::<T>()) as BufferAddress;
        let realloc = bmax >= self.buffer.size();

        if realloc {
            let init_size = self.inner.capacity() * size_of::<T>();
            let new = cx.device.create_buffer(&BufferDescriptor {
                label: label!("<{}>.buffer", type_name::<T>()),
                size: init_size as BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
                mapped_at_creation: false,
            });

            let old_size = self.buffer.size();
            encoder.copy_buffer_to_buffer(&self.buffer, 0, &new, 0, old_size);

            self.buffer = new;
        }

        let coarsed: CoarseIter<_, T> = self.delta.iter().copied().into();
        for range in coarsed {
            let off = (range.start * size_of::<T>()) as BufferAddress;
            let src: &[u8] = cast_slice(&self.inner[range]);
            let size = BufferSize::new(src.len() as BufferAddress).unwrap();

            let mut dst = self.belt.write_buffer(encoder, &self.buffer, off, size);
            dst.copy_from_slice(src);
        }

        self.belt.finish_and_recall_on_submit(encoder);

        self.delta.clear();
        Finish {
            buffer: &self.buffer,
            realloc,
        }
    }

    pub fn get(&self, id: usize) -> T {
        self.inner[id]
    }

    pub fn set(&mut self, id: usize, shape: T) {
        debug_assert!(shape.is_visible(), "setting dead rect at {id}");

        if let Some(old) = self.inner.get_mut(id) {
            if *old != shape {
                *old = shape;
                self.delta.insert(id);
            }
        } else {
            self.delta.insert(id);

            let diff = id - self.inner.len() + 1;
            self.inner.reserve(diff);
            for _ in 1..diff {
                self.inner.push(T::zeroed());
            }
            self.inner.push(shape);
        }
    }
}

struct CoarseIter<I: Iterator, T> {
    iter: Peekable<I>,
    _marker: PhantomData<fn() -> T>,
}

impl<I, T> From<I> for CoarseIter<I, T>
where
    I: Iterator,
{
    fn from(value: I) -> Self {
        Self {
            iter: value.peekable(),
            _marker: PhantomData,
        }
    }
}

impl<I, T> Iterator for CoarseIter<I, T>
where
    I: Iterator<Item = usize>,
{
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let max_hole_size: usize = MAX_HOLE_BYTES / size_of::<T>();
        let max_batch_size: usize = (MAX_BATCH_BYTES / size_of::<T>()).max(1);

        let start = self.iter.next()?;
        let mut end = start + 1;

        while let Some(&next) = self.iter.peek() {
            if next - end > max_hole_size || next - start + 1 > max_batch_size {
                break;
            }

            end = next + 1;
            self.iter.next();
        }

        Some(start..end)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iter.size_hint();
        (min.min(1), max)
    }
}

impl<I, T> FusedIterator for CoarseIter<I, T>
where
    Self: Iterator,
    I: FusedIterator,
{
}
