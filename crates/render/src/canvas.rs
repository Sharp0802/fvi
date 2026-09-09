use wgpu::*;

use crate::backend::*;
use crate::{Id, label};

/// A canvas.
#[derive(Debug)]
pub struct Canvas<'a> {
    device: &'a Device,
    encoder: CommandEncoder,
    scope: RenderStateBundleScope<'a>,
}

impl<'a> Canvas<'a> {
    #[must_use]
    pub(crate) fn new(device: &'a Device, state: &'a mut RenderStateBundle) -> Self {
        Self {
            device,
            encoder: device.create_command_encoder(&CommandEncoderDescriptor {
                label: label!("encoder"),
            }),
            scope: state.open(),
        }
    }

    /// Draws a shape indentified by given id.
    pub fn draw<T>(&mut self, id: Id, shape: T)
    where
        T: Shape,
        RenderStateBundleScope<'a>: AsMut<RenderStateScope<'a, T>>,
    {
        self.scope.write(id, shape);
    }

    pub(crate) fn close(mut self) -> CommandBuffer {
        self.scope.close(self.device, &mut self.encoder);
        self.encoder.finish()
    }
}
