use std::marker::PhantomData;
use wgpu::*;

use super::raw::ShapeBufferRaw;
use crate::label;

#[derive(Debug)]
pub struct ShapeBufferBindGroupLayout<T> {
    readonly: BindGroupLayout,
    writable: BindGroupLayout,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ShapeBufferBindGroupLayout<T> {
    pub fn new(device: &Device) -> Self {
        let readonly = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: label!("readonly"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let writable = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: label!("writable"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        Self {
            readonly,
            writable,
            _marker: PhantomData,
        }
    }
}

/// A set of [`BindGroup`] for [`ShapeBuffer`](super::ShapeBuffer).
#[derive(Debug)]
pub struct ShapeBufferBindGroup<T> {
    /// A readonly bind group.
    ///
    /// ```wgsl
    /// @binding(0) var<storage, read> x: array<T>;
    /// @binding(1) var<storage, read> visibles: array<u32>;
    /// ```
    pub readonly: BindGroup,

    /// A writable bind group.
    ///
    /// ```wgsl
    /// @binding(0) var<storage, read_write> x: array<T>;
    /// @binding(1) var<storage, read_write> visibles: array<u32>;
    /// ```
    pub writable: BindGroup,

    _marker: PhantomData<fn() -> T>,
}

impl<T> ShapeBufferBindGroup<T> {
    pub(crate) fn new(
        device: &Device,
        layout: &ShapeBufferBindGroupLayout<T>,
        buffer: &ShapeBufferRaw<T>,
        version: u32,
    ) -> Self {
        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(as_binding(buffer.contents())),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Buffer(as_binding(buffer.visibles())),
            },
        ];

        let readonly = device.create_bind_group(&BindGroupDescriptor {
            label: label!("readonly/{}", version),
            layout: &layout.readonly,
            entries,
        });

        let writable = device.create_bind_group(&BindGroupDescriptor {
            label: label!("writable/{}", version),
            layout: &layout.writable,
            entries,
        });

        Self {
            readonly,
            writable,
            _marker: PhantomData,
        }
    }
}

fn as_binding(slice: BufferSlice<'_>) -> BufferBinding<'_> {
    BufferBinding {
        buffer: slice.buffer(),
        offset: slice.offset(),
        size: Some(BufferSize::new(slice.size()).unwrap()),
    }
}
