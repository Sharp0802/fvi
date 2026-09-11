use crate::Frame;
use crate::context::RenderContext;

#[derive(Debug)]
pub struct Canvas<'a> {
    frame: &'a Frame,
    cx: &'a RenderContext,
}

impl<'a> Canvas<'a> {
    pub(crate) fn new(frame: &'a Frame, cx: &'a RenderContext) -> Self {
        Self { frame, cx }
    }

    // TODO: WIP
}
