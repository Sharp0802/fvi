use std::marker::PhantomData;
use tracing::instrument;
use wgpu::*;

use super::*;
use crate::context::Shader;
use crate::label;

#[derive(Debug)]
pub struct Pipeline<T> {
    #[cfg(debug_assertions)]
    msaa: u32,
    cull: ComputePipeline,
    render: RenderPipeline,
    _marker: PhantomData<fn() -> T>,
}

pub struct PipelineDescriptor<'a> {
    shader: &'a Shader,
    format: TextureFormat,
    msaa: u32,
    args: &'a ArgsBuffer,
}

impl<T: Shape> Pipeline<T> {
    #[must_use]
    pub fn new(device: &Device, desc: &PipelineDescriptor) -> Self {
        let buffer: ShapeBuffer<T> = ShapeBuffer::new(device);
        let indirect_args = IndirectArgsBuffer::new(device);

        let cull_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: label!("cull_pipeline_layout"),
            bind_group_layouts: &[
                Some(indirect_args.as_layout()),
                Some(&buffer.as_layout().writable),
            ],
            immediate_size: 0,
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: label!("render_pipeline_layout"),
            bind_group_layouts: &[
                Some(desc.args.as_layout()),
                Some(&buffer.as_layout().readonly),
            ],
            immediate_size: 0,
        });

        let cull = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: label!("cull"),
            layout: Some(&cull_pipeline_layout),
            module: &desc.shader[T::CULL],
            entry_point: None,
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        let render = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: label!("render"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &desc.shader[T::VERTEX],
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: desc.msaa,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &desc.shader[T::FRAGMENT],
                entry_point: None,
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: desc.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::all(),
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            #[cfg(debug_assertions)]
            msaa: desc.msaa,
            cull,
            render,
            _marker: PhantomData,
        }
    }

    #[instrument]
    pub fn dispatch(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        state: &RenderState<T>,
        args: &ArgsBuffer,
    ) {
        self.dispatch_cull(encoder, state);
        self.dispatch_render(encoder, view, state, args);
    }

    #[instrument]
    fn dispatch_cull(&self, encoder: &mut CommandEncoder, state: &RenderState<T>) {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: label!("cull_pass"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.cull);
        state.bind_compute(&mut pass);
        pass.dispatch_workgroups(64, 1, 1);
    }

    #[instrument]
    fn dispatch_render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        state: &RenderState<T>,
        args: &ArgsBuffer,
    ) {
        debug_assert_eq!(
            self.msaa,
            view.texture().sample_count(),
            "target texture has invalid MSAA sample count",
        );

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: label!("pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.render);
        pass.set_bind_group(0, args.as_binding(), &[]);
        state.bind_render(&mut pass);
        state.draw(&mut pass);
    }
}
