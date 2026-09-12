use wgpu::*;

use super::*;
use crate::context::TextureMap;
use crate::label;

#[derive(Debug)]
pub struct TextureMapBind {
    sampler: Sampler,
    bind_group: Option<(u32, WgpuBindGroup1)>,
}

impl TextureMapBind {
    pub fn new(device: &Device) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: label!("sampler"),
            ..Default::default()
        });

        Self {
            sampler,
            bind_group: None,
        }
    }

    pub fn update(&mut self, device: &Device, map: &TextureMap) -> &WgpuBindGroup1 {
        let snapshot = map.snapshot();

        // fixme: refactor after polonious
        if self
            .bind_group
            .as_ref()
            .is_none_or(|(version, _)| *version != snapshot.version)
        {
            let refs: Vec<_> = snapshot.views.iter().collect();
            let bind_group = WgpuBindGroup1::from_bindings(
                device,
                WgpuBindGroup1Entries::new(WgpuBindGroup1EntriesParams {
                    texs: &refs,
                    samp: &self.sampler,
                }),
            );
            self.bind_group = Some((snapshot.version, bind_group));
        }

        &self.bind_group.as_ref().unwrap().1
    }
}
