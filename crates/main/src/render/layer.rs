use std::num::NonZero;
use std::ops::Index;
use wgpu::*;
use winit::dpi::PhysicalSize;

use crate::gfx::BLEND_LAYERS;

#[derive(Debug)]
pub struct LayerVec {
    views: Vec<TextureView>,
    bundle: RenderBundle,
}

impl LayerVec {
    #[expect(clippy::too_many_lines, reason = "decoupling seems harm readability")]
    pub fn new(
        device: &Device,
        format: TextureFormat,
        size: PhysicalSize<u32>,
        len: NonZero<u32>,
    ) -> Self {
        debug_assert!(
            len.get() <= device.limits().max_texture_array_layers,
            "should be checked at RenderDevice::new",
        );

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Layer"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: len.get(),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: Some("Layer.view"),
            usage: Some(TextureUsages::TEXTURE_BINDING),
            format: Some(format),
            dimension: Some(TextureViewDimension::D2Array),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: None,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Layer.blend.bind_group.layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Layer.blend.bind_group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            }],
        });

        let module = device.create_shader_module(BLEND_LAYERS);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Layer.blend.pipeline.layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Layer.blend.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &module,
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let views: Vec<_> = (0..len.get())
            .map(|index| {
                texture.create_view(&TextureViewDescriptor {
                    label: Some(&format!("Layer.view.{index}")),
                    usage: Some(TextureUsages::RENDER_ATTACHMENT),
                    format: Some(format),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    base_array_layer: index,
                    array_layer_count: Some(1),
                })
            })
            .collect();

        let bundle = {
            let mut encoder = device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
                label: Some("Layer.blend.encoder"),
                color_formats: &[Some(format)],
                depth_stencil: None,
                sample_count: 1,
                multiview: None,
            });

            encoder.set_pipeline(&pipeline);
            encoder.set_bind_group(0, &bind_group, &[]);
            encoder.draw(0..3, 0..1);

            encoder.finish(&RenderBundleDescriptor {
                label: Some("Layer.blend"),
            })
        };

        Self { views, bundle }
    }

    #[must_use]
    pub const fn bundle(&self) -> &RenderBundle {
        &self.bundle
    }
}

impl Index<u32> for LayerVec {
    type Output = TextureView;

    fn index(&self, index: u32) -> &Self::Output {
        &self.views[index as usize]
    }
}
