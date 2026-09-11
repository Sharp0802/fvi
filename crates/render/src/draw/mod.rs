mod rect;
mod tex;
mod view;

use crate::gfx::draw::*;
use crate::label;

pub use rect::*;
pub use tex::*;
pub use view::*;

use wgpu::*;

#[derive(Debug)]
pub struct Draw {
    rect_bind: RectBufferBind,
    tex_bind: TextureMapBind,
    view_bind: ViewBufferBind,
    pipeline: RenderPipeline,
}

impl Draw {
    pub fn new(device: &Device, format: TextureFormat, sample_count: u32) -> Self {
        let shader = create_shader_module_embed_source(device);

        let pipeline_layout = create_pipeline_layout(device);
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: label!("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: vertex_state(&shader, &vs_main_entry()),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(IndexFormat::Uint32),
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(fragment_state(
                &shader,
                &fs_main_entry([Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })]),
            )),
            multiview_mask: None,
            cache: None,
        });

        Self {
            rect_bind: RectBufferBind::new(),
            tex_bind: TextureMapBind::new(device),
            view_bind: ViewBufferBind::new(device),
            pipeline,
        }
    }

    pub fn run(
        &mut self,
        device: &Device,
        queue: &Queue,
        pass: &mut RenderPass,
        rect_buf: &RectBuffer,
        tex_map: &TextureMap,
        view: View,
    ) {
        let groups = WgpuBindGroups {
            bind_group0: self.rect_bind.update(device, rect_buf),
            bind_group1: self.tex_bind.update(device, tex_map),
            bind_group2: self.view_bind.update(queue, view),
        };

        pass.set_pipeline(&self.pipeline);
        groups.set(pass);
        pass.draw(
            0..4,
            0..(rect_buf.len().try_into().expect("too many rects")),
        );
    }
}
