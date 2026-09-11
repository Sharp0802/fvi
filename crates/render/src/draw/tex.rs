use std::ops::Deref;
use wgpu::*;

use super::*;
use crate::context::TextureMap;
use crate::label;

#[derive(Debug)]
pub struct TextureMapBind {
    version: u32,
    sampler: Sampler,
    bind_group: WgpuBindGroup1,
}

impl TextureMapBind {
    pub fn new(device: &Device) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: label!("sampler"),
            ..Default::default()
        });

        let bind_group = WgpuBindGroup1::from_bindings(
            device,
            WgpuBindGroup1Entries::new(WgpuBindGroup1EntriesParams {
                texs: &[],
                samp: &sampler,
            }),
        );

        Self {
            version: u32::MAX,
            sampler,
            bind_group,
        }
    }

    pub fn update(&mut self, device: &Device, map: &TextureMap) -> &WgpuBindGroup1 {
        if self.version != map.version {
            self.version = map.version;

            let refs: Vec<_> = map.vec.iter().collect();
            self.bind_group = WgpuBindGroup1::from_bindings(
                device,
                WgpuBindGroup1Entries::new(WgpuBindGroup1EntriesParams {
                    texs: &refs,
                    samp: &self.sampler,
                }),
            );
        }

        &self.bind_group
    }
}

impl Deref for TextureMapBind {
    type Target = WgpuBindGroup1;

    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}
