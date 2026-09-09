use wgpu::*;

use crate::Id;
use crate::backend::*;

/// A canvas.
#[derive(Debug)]
pub struct Canvas<'a> {
    device: &'a Device,
    encoder: &'a mut CommandEncoder,
    scope: RenderStateBundleScope<'a>,
}

impl<'a> Canvas<'a> {
    #[must_use]
    pub(crate) const fn new(
        device: &'a Device,
        encoder: &'a mut CommandEncoder,
        state: &'a mut RenderStateBundle,
    ) -> Self {
        Self {
            device,
            encoder,
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
}

impl Drop for Canvas<'_> {
    fn drop(&mut self) {
        self.scope.close_unchecked(self.device, self.encoder);
    }
}
