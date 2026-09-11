use std::iter::repeat_n;
use std::ops::Range;

use bytemuck::checked::cast_slice;
use bytemuck::{Zeroable, bytes_of};
use wgpu::util::*;
use wgpu::*;

use super::*;
use crate::label;

const CHUNK_SIZE: BufferAddress = 4096;
const MAX_HOLE: usize = 3;

const USAGE: BufferUsages = BufferUsages::COPY_DST
    .union(BufferUsages::COPY_SRC)
    .union(BufferUsages::STORAGE);

#[derive(Debug)]
pub struct RectBuffer {
    version: u32,
    guest: Buffer,
    host: Vec<Rect>,
    diffmap: Vec<u128>,
    belt: StagingBelt,
}

impl RectBuffer {
    pub fn new(device: &Device) -> Self {
        let version = 0;
        let host = Vec::with_capacity(8192);
        let guest = device.create_buffer(&BufferDescriptor {
            label: label!("guest/{}", version),
            size: (host.capacity() * size_of::<Rect>()) as BufferAddress,
            usage: USAGE,
            mapped_at_creation: false,
        });
        let diffmap = Vec::with_capacity(host.capacity() / 128);
        let belt = StagingBelt::new(device.clone(), CHUNK_SIZE);

        Self {
            version,
            guest,
            host,
            diffmap,
            belt,
        }
    }

    pub const fn len(&self) -> usize {
        self.host.len()
    }

    pub fn write(&mut self, index: usize, value: Rect) {
        if self.host.len() <= index {
            let size = self.host.len() - index + 1;
            self.host.extend(repeat_n(Rect::zeroed(), size));
        }

        if bytes_of(&self.host[index]) == bytes_of(&value) {
            return;
        }

        self.host[index] = value;
        set(&mut self.diffmap, index);
    }

    pub fn apply(&mut self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: label!("apply"),
        });

        if self.guest.size() < (self.host.len() * size_of::<Rect>()) as BufferAddress {
            self.version += 1;

            let new = device.create_buffer(&BufferDescriptor {
                label: label!("guest/{}", self.version),
                size: (self.host.capacity() * size_of::<Rect>()) as BufferAddress,
                usage: USAGE,
                mapped_at_creation: false,
            });

            encoder.copy_buffer_to_buffer(&self.guest, 0, &new, 0, self.guest.size());
        }

        for diff in DiffIter::new(&self.diffmap) {
            let start = (diff.start * size_of::<Rect>()) as BufferAddress;
            let size = (diff.len() * size_of::<Rect>()) as BufferAddress;

            let mut view = self.belt.write_buffer(
                &mut encoder,
                &self.guest,
                start,
                BufferSize::new(size).unwrap(),
            );

            view.copy_from_slice(cast_slice(&self.host[diff]));
        }

        let command = encoder.finish();
        queue.submit([command]);
    }
}

#[derive(Debug)]
pub struct RectBufferBind(Option<(u32, WgpuBindGroup0)>);

impl RectBufferBind {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn update(&mut self, device: &Device, buffer: &RectBuffer) -> &WgpuBindGroup0 {
        if self
            .0
            .as_ref()
            .is_none_or(|(ver, _)| *ver == buffer.version)
        {
            let bind_group = WgpuBindGroup0::from_bindings(
                device,
                WgpuBindGroup0Entries::new(WgpuBindGroup0EntriesParams {
                    rects: buffer.guest.as_entire_buffer_binding(),
                }),
            );

            *self = Self(Some((buffer.version, bind_group)));
        }

        &self.0.as_ref().unwrap().1
    }
}

#[derive(Debug)]
struct DiffIter<'a> {
    diffmap: &'a [u128],
    index: usize,
}

impl<'a> DiffIter<'a> {
    pub fn new(diffmap: &'a [u128]) -> Self {
        let index = diffmap
            .iter()
            .enumerate()
            .filter_map(|(i, &chk)| chk.lowest_one().map(|off| i * 128 + off as usize))
            .next()
            .unwrap_or(diffmap.len() * 128);

        Self { diffmap, index }
    }
}

impl Iterator for DiffIter<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index * 128 >= self.diffmap.len() || !is_set(self.diffmap, self.index) {
            return None;
        }

        let start = self.index;

        let mut hole = 0;
        while self.index * 128 < self.diffmap.len()
            && (self.index - start) < (CHUNK_SIZE as usize / size_of::<Rect>())
        {
            if is_set(self.diffmap, self.index) {
                hole = 0;
                self.index += 1;
            } else if hole >= MAX_HOLE {
                break;
            } else {
                hole += 1;
            }
        }

        let end = self.index;

        // TODO: use more efficient algorithm to skip holes
        while self.index * 128 < self.diffmap.len() && !is_set(self.diffmap, self.index) {
            self.index += 1;
        }

        Some(start..end)
    }
}

const fn is_set(bitmap: &[u128], index: usize) -> bool {
    bitmap[index / 128] & (1 << (index % 128)) != 0
}

const fn set(bitmap: &mut [u128], index: usize) {
    bitmap[index / 128] |= 1 << (index % 128);
}
