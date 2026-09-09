use std::num::NonZero;
use wgpu::*;

use super::*;
use crate::Id;
use crate::id::IdMap;

#[derive(Debug)]
pub struct RenderState<T> {
    map: IdMap,
    buffer: ShapeBuffer<T>,
    indirect_args: IndirectArgsBuffer,
}

impl<T: Shape> RenderState<T> {
    #[must_use]
    pub fn new(device: &Device) -> Self {
        Self {
            map: IdMap::new(const { NonZero::new(T::MAX_AGE).unwrap() }),
            buffer: ShapeBuffer::new(device),
            indirect_args: IndirectArgsBuffer::new(device),
        }
    }

    #[must_use]
    pub const fn open(&mut self) -> RenderStateScope<'_, T> {
        RenderStateScope {
            map: &mut self.map,
            inner: self.buffer.open(),
        }
    }

    pub fn bind_compute(&self, pass: &mut ComputePass) {
        pass.set_bind_group(0, self.indirect_args.as_binding(), &[]);
        pass.set_bind_group(1, &self.buffer.as_binding().writable, &[]);
    }

    pub fn bind_render(&self, pass: &mut RenderPass) {
        pass.set_bind_group(1, &self.buffer.as_binding().readonly, &[]);
    }

    pub fn draw(&self, pass: &mut RenderPass) {
        pass.draw_indexed_indirect(&self.indirect_args, 0);
    }
}

#[derive(Debug)]
pub struct RenderStateScope<'a, T: Shape> {
    map: &'a mut IdMap,
    inner: ShapeBufferScope<'a, T>,
}

impl<T: Shape> RenderStateScope<'_, T> {
    pub fn write(&mut self, id: Id, shape: T) {
        let index = self.map.map(id);
        self.inner.write(index, shape);
    }

    pub fn close_unchecked(&mut self, device: &Device, encoder: &mut CommandEncoder) {
        self.inner.close_unchecked(device, encoder);
    }
}
