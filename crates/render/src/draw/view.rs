use bytemuck::bytes_of;
use wgpu::util::*;
use wgpu::*;

use super::*;
use crate::label;

#[derive(Debug)]
pub struct ViewBufferBind {
    value: View,
    buffer: Buffer,
    bind_group: WgpuBindGroup2,
}

impl ViewBufferBind {
    pub fn new(device: &Device) -> Self {
        let value = View {
            size: [0; 2],
            scale: 1.0,
            _pad: 0,
        };

        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: label!("buffer"),
            contents: bytes_of(&value),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let bind_group = WgpuBindGroup2::from_bindings(
            device,
            WgpuBindGroup2Entries::new(WgpuBindGroup2EntriesParams {
                view: buffer.as_entire_buffer_binding(),
            }),
        );

        Self {
            value,
            buffer,
            bind_group,
        }
    }

    pub fn update(&mut self, queue: &Queue, value: View) -> &WgpuBindGroup2 {
        if self.value != value {
            self.value = value;
            queue.write_buffer(&self.buffer, 0, bytes_of(&value));
        }

        &self.bind_group
    }
}
