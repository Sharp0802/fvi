use tracing::instrument;
use wgpu::*;

use super::*;
use crate::Id;
use crate::types::*;

macro_rules! decl_bundle {
    ($($name:ident),+ $(,)?) => {
        #[derive(Debug)]
        #[allow(non_snake_case, reason = "for simplicity of macro")]
        pub struct RenderStateBundle {
            $($name: RenderState<$name>,)+
        }

        impl RenderStateBundle {
            #[must_use]
            pub fn new(device: &Device) -> Self {
                Self {
                    $($name: RenderState::new(device),)+
                }
            }

            #[must_use]
            pub const fn open(&mut self) -> RenderStateBundleScope {
                RenderStateBundleScope {
                    $($name: self.$name.open(),)+
                }
            }
        }

        #[derive(Debug)]
        pub struct RenderStateBundleScope<'a> {
            $($name: RenderStateScope<'a, $name>,)+
        }

        impl<'a> RenderStateBundleScope<'a> {
            pub fn write<T>(&mut self, id: Id, shape: T)
            where
                T: Shape,
                Self: AsMut<RenderStateScope<'a, T>>,
            {
                self.as_mut().write(id, shape);
            }

            pub fn close_unchecked(&mut self, device: &Device, encoder: &mut CommandEncoder) {
                $(self.$name.close_unchecked(device, encoder);)+
            }
        }

        $(impl<'a> AsRef<RenderStateScope<'a, $name>> for RenderStateBundleScope<'a> {
            fn as_ref(&self) -> &RenderStateScope<'a, $name> {
                &self.$name
            }
        }

        impl<'a> AsMut<RenderStateScope<'a, $name>> for RenderStateBundleScope<'a> {
            fn as_mut(&mut self) -> &mut RenderStateScope<'a, $name> {
                &mut self.$name
            }
        })+

        #[derive(Debug)]
        #[allow(non_snake_case, reason = "for simplicity of macro")]
        pub struct PipelineBundle {
            $($name: Pipeline<$name>,)+
        }

        impl PipelineBundle {
            pub fn new(device: &Device, desc: &PipelineDescriptor) -> Self {
                Self {
                    $($name: Pipeline::new(device, desc),)+
                }
            }

            #[instrument]
            pub fn dispatch(
                &self,
                encoder: &mut CommandEncoder,
                view: &TextureView,
                state: &RenderStateBundle,
                args: &ArgsBuffer,
            ) {
                $(self.$name.dispatch(encoder, view, &state.$name, args);)+
            }
        }
    };
}

#[rustfmt::skip]
decl_bundle![
    Rect,
];
