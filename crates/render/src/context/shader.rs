use std::ops::Index;
use wgpu::{Device, ShaderModule};

use crate::gfx::ShaderEntry;

macro_rules! decl_shader {
    ($($name:ident),+ $(,)?) => {
        #[derive(Debug)]
        #[allow(non_snake_case, reason = "for simplicity of macro")]
        pub struct Shader {
            $($name: ShaderModule,)+
        }

        impl Shader {
            pub fn new(device: &Device) -> Self {
                Self {
                    $($name: ShaderEntry::$name.create_shader_module_embed_source(device),)+
                }
            }
        }

        impl Index<ShaderEntry> for Shader {
            type Output = ShaderModule;

            fn index(&self, index: ShaderEntry) -> &Self::Output {
                match index {
                    $(ShaderEntry::$name => &self.$name,)+
                }
            }
        }
    };
}

#[rustfmt::skip]
decl_shader![
    RectCull,
    RectDraw,
];
