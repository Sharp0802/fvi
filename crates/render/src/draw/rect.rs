use std::ops::Range;

use bytemuck::{Zeroable, bytes_of, cast_slice};
use wgpu::util::*;
use wgpu::*;

use super::*;
use crate::label;

const CHUNK_SIZE: usize = 4096;
const MAX_HOLE: usize = 3;

const USAGE: BufferUsages = BufferUsages::COPY_DST.union(BufferUsages::STORAGE);

#[derive(Debug)]
pub struct RectBuffer {
    version: u32,
    guest: Buffer,
    state: RectState,
    belt: StagingBelt,
}

impl RectBuffer {
    pub fn new(device: &Device) -> Self {
        let version = 0;
        let state = RectState::new();
        let guest = device.create_buffer(&BufferDescriptor {
            label: label!("guest/{}", version),
            size: (state.host.capacity() * size_of::<Rect>()) as BufferAddress,
            usage: USAGE,
            mapped_at_creation: false,
        });
        let belt = StagingBelt::new(device.clone(), CHUNK_SIZE as BufferAddress);

        Self {
            version,
            guest,
            state,
            belt,
        }
    }

    pub const fn len(&self) -> usize {
        self.state.host.len()
    }

    pub fn write(&mut self, index: usize, value: Rect) -> bool {
        self.state.write(index, value)
    }

    pub fn truncate(&mut self, len: usize) -> bool {
        self.state.truncate(len)
    }

    pub fn apply(&mut self, device: &Device, encoder: &mut CommandEncoder) {
        if self.guest.size() < (self.state.host.len() * size_of::<Rect>()) as BufferAddress {
            self.version += 1;

            self.guest = device.create_buffer(&BufferDescriptor {
                label: label!("guest/{}", self.version),
                size: (self.state.host.capacity() * size_of::<Rect>()) as BufferAddress,
                usage: USAGE,
                mapped_at_creation: false,
            });
            self.state.mark_all();
        }

        for diff in DiffIter::new(&self.state.diffmap, self.state.host.len()) {
            let start = (diff.start * size_of::<Rect>()) as BufferAddress;
            let size = (diff.len() * size_of::<Rect>()) as BufferAddress;

            let mut view =
                self.belt
                    .write_buffer(encoder, &self.guest, start, BufferSize::new(size).unwrap());

            view.copy_from_slice(cast_slice(&self.state.host[diff]));
        }

        self.belt.finish_and_recall_on_submit(encoder);
        self.state.diffmap.fill(0);
    }
}

#[derive(Debug)]
struct RectState {
    host: Vec<Rect>,
    diffmap: Vec<u128>,
}

impl RectState {
    fn new() -> Self {
        let host = Vec::with_capacity(8192);
        let diffmap = Vec::with_capacity(host.capacity().div_ceil(128));
        Self { host, diffmap }
    }

    fn write(&mut self, index: usize, value: Rect) -> bool {
        let old_len = self.host.len();
        if old_len <= index {
            let len = index.checked_add(1).expect("too many rects");
            self.host.resize(len, Rect::zeroed());
            self.diffmap.resize(len.div_ceil(128), 0);
            for added in old_len..len {
                set(&mut self.diffmap, added);
            }
        } else if bytes_of(&self.host[index]) == bytes_of(&value) {
            return false;
        }

        self.host[index] = value;
        set(&mut self.diffmap, index);
        true
    }

    fn truncate(&mut self, len: usize) -> bool {
        if self.host.len() <= len {
            return false;
        }

        self.host.truncate(len);
        self.diffmap.truncate(len.div_ceil(128));
        self.mask_tail();
        true
    }

    fn mark_all(&mut self) {
        self.diffmap.fill(u128::MAX);
        self.mask_tail();
    }

    fn mask_tail(&mut self) {
        let remainder = self.host.len() % 128;
        if remainder != 0 {
            *self.diffmap.last_mut().unwrap() &= (1 << remainder) - 1;
        }
    }
}

#[derive(Debug)]
pub struct RectBufferBind(Option<(u32, WgpuBindGroup0)>);

impl RectBufferBind {
    pub const fn new() -> Self {
        Self(None)
    }

    pub fn update(&mut self, device: &Device, buffer: &RectBuffer) -> &WgpuBindGroup0 {
        if self
            .0
            .as_ref()
            .is_none_or(|(ver, _)| *ver != buffer.version)
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
    len: usize,
    index: usize,
}

impl<'a> DiffIter<'a> {
    pub const fn new(diffmap: &'a [u128], len: usize) -> Self {
        Self {
            diffmap,
            len,
            index: 0,
        }
    }
}

impl Iterator for DiffIter<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.len && !is_set(self.diffmap, self.index) {
            self.index += 1;
        }
        if self.index == self.len {
            return None;
        }

        let start = self.index;
        let limit = self
            .len
            .min(start.saturating_add(CHUNK_SIZE / size_of::<Rect>()));
        let mut end = start + 1;
        self.index += 1;
        while self.index < limit {
            if is_set(self.diffmap, self.index) {
                end = self.index + 1;
            } else if self.index - end >= MAX_HOLE {
                break;
            }
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
