use std::ops::Deref;

use bytemuck::{Pod, Zeroable, bytes_of};
use wgpu::util::*;
use wgpu::*;

use crate::label;

macro_rules! decl_args {
    ($(#[$meta:meta])* $vis:vis struct $name:ident {
        $($(#[$field_meta:meta])* $field_vis:vis $field:ident: $ty:ty),* $(,)?
    }) => {
        $(#[$meta])*
        $vis struct $name {
            $($(#[$field_meta])* $field_vis $field: $ty ,)*
        }

        impl $name {
            const fn layout_entries() -> [wgpu::BindGroupLayoutEntry; [$(stringify!($field),)*].len()] {
                {
                    let mut i = 0;

                    [$(wgpu::BindGroupLayoutEntry {
                        binding: {
                            let _ = stringify!($field);
                            let j = i;
                            #[allow(unused_assignments, reason = "last assignment is always unused")]
                            { i += 1; }
                            j
                        },
                        visibility: wgpu::ShaderStages::all(),
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }),*]
                }
            }

            const fn entries(
                buffer: &wgpu::Buffer,
            ) -> [wgpu::BindGroupEntry<'_>; [$(stringify!($field),)*].len()] {
                {
                    let mut i = 0;

                    [$(wgpu::BindGroupEntry {
                        binding: {
                            let j = i;
                            #[allow(unused_assignments, reason = "last assignment is always unused")]
                            { i += 1; }
                            j
                        },
                        resource: wgpu::BindingResource::Buffer(BufferBinding {
                            buffer,
                            offset: std::mem::offset_of!(Self, $field) as wgpu::BufferAddress,
                            size: Some(
                                const {
                                    wgpu::BufferSize::new(
                                        size_of_val(&Self::new().$field) as wgpu::BufferAddress
                                    )
                                    .unwrap()
                                },
                            ),
                        }),
                    }),*]
                }
            }
        }

    };
}

decl_args! {
    /// A common arguments passed to shaders.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    pub struct Args {
        /// A render scale factor for `dp x scale = px`.
        pub scale: f32,
    }
}

impl Default for Args {
    fn default() -> Self {
        Self::new()
    }
}

impl Args {
    const fn new() -> Self {
        Self { scale: 1.0 }
    }
}

/// A bindable buffer for [`Args`].
#[derive(Debug)]
pub struct ArgsBuffer {
    cache: Args,
    buffer: Buffer,
    layout: BindGroupLayout,
    bind: BindGroup,
}

impl ArgsBuffer {
    /// Creates a new [`ArgsBuffer`].
    #[must_use]
    pub fn new(device: &Device, value: Args) -> Self {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: label!("buffer"),
            contents: bytes_of(&value),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: label!("layout"),
            entries: &Args::layout_entries(),
        });

        let bind = device.create_bind_group(&BindGroupDescriptor {
            label: label!("bind"),
            layout: &layout,
            entries: &Args::entries(&buffer),
        });

        Self {
            cache: value,
            buffer,
            layout,
            bind,
        }
    }

    /// Dispatches an operation that writes
    /// new value to the inherent buffer.
    ///
    /// Note that it implies same semantics with [`Queue::write_buffer`],
    /// excepting that it ignores given value
    /// if it has byte-to-byte equality with cached value.
    pub fn write(&mut self, queue: &Queue, value: Args) {
        if bytes_of(&self.cache) == bytes_of(&value) {
            return;
        }

        queue.write_buffer(&self.buffer, 0, bytes_of(&value));
    }

    /// Returns a readonly layout of `self`.
    #[must_use]
    pub const fn as_layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    /// Returns a readonly bind group of `self`.
    #[must_use]
    pub const fn as_binding(&self) -> &BindGroup {
        &self.bind
    }
}

/// A bindable buffer for [`DrawIndexedIndirectArgs`].
#[derive(Debug)]
pub struct IndirectArgsBuffer {
    buffer: Buffer,
    layout: BindGroupLayout,
    bind: BindGroup,
}

impl IndirectArgsBuffer {
    /// Creates a new [`IndirectArgsBuffer`].
    #[must_use]
    pub fn new(device: &Device) -> Self {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: label!("buffer"),
            contents: DrawIndexedIndirectArgs {
                index_count: 0,
                instance_count: 0,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            }
            .as_bytes(),
            usage: BufferUsages::STORAGE,
        });

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: label!("layout_writable"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let entries = &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            }),
        }];

        let bind = device.create_bind_group(&BindGroupDescriptor {
            label: label!("bind_writable"),
            layout: &layout,
            entries,
        });

        Self {
            buffer,
            layout,
            bind,
        }
    }

    /// Returns a layout of `self`.
    #[must_use]
    pub const fn as_layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    /// Returns a bind group of `self`.
    #[must_use]
    pub const fn as_binding(&self) -> &BindGroup {
        &self.bind
    }
}

impl Deref for IndirectArgsBuffer {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
