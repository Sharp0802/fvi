use wgpu::*;

use crate::{Id, ShapeData, label};

/// The command recorder for one frame.
#[derive(Debug)]
pub struct Canvas {
    encoder: CommandEncoder,
}

impl Canvas {
    /// Starts recording a frame on the given device.
    #[must_use]
    pub fn new(device: &Device) -> Self {
        let encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: label!("encoder"),
        });

        Self { encoder }
    }

    /// Clears a render attachment to the given color.
    pub fn clear(&mut self, view: &TextureView, color: Color) {
        self.encoder.begin_render_pass(&RenderPassDescriptor {
            label: label!("clear"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(color),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    /// Finishes recording and returns the commands to submit to the device's queue.
    #[must_use]
    pub fn finish(self) -> CommandBuffer {
        self.encoder.finish()
    }
}

/// Recording shapes of a given type into a frame.
pub trait Draw<T: ShapeData> {
    /// Draws the shape identified by the given id.
    fn draw(&mut self, id: Id, shape: T);
}
