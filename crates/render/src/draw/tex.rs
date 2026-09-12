use std::num::NonZero;

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
            let len: u32 = refs.len().try_into().expect("texture map too large");
            let len_nz = NonZero::new(len).expect("texture map size cannot be zero");

            let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: label!("layout/{}", snapshot.version),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        // wgpu-bindgen generates None for binding_array<T>
                        count: Some(len_nz),
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: label!("bind_group/{}", snapshot.version),
                layout: &layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureViewArray(&refs),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            #[expect(unsafe_code, reason = "wgpu-bindgen generates invalid bind-group")]
            let bind_group_typed = unsafe { WgpuBindGroup1::from_raw(bind_group) };

            self.bind_group = Some((snapshot.version, bind_group_typed));
        }

        &self.bind_group.as_ref().unwrap().1
    }
}
