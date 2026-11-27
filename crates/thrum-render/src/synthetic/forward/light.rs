//! The shared light-group schema, with camera-dependent products supplied by
//! each view and the world's light/environment/exposure resources borrowed.

pub(in crate::synthetic) type AerialBindings<'a> = (
    [&'a wgpu::TextureView; 2],
    &'a wgpu::Sampler,
    &'a wgpu::Buffer,
);

pub(in crate::synthetic) struct LightBindings<'a> {
    pub(in crate::synthetic) directional: &'a wgpu::Buffer,
    pub(in crate::synthetic) environment_light: &'a wgpu::Buffer,
    pub(in crate::synthetic) local: &'a wgpu::Buffer,
    pub(in crate::synthetic) heads: &'a wgpu::Buffer,
    pub(in crate::synthetic) indices: &'a wgpu::Buffer,
    pub(in crate::synthetic) atmosphere: Option<&'a crate::atmosphere::AtmospherePrepared>,
    pub(in crate::synthetic) cloud: Option<&'a crate::atmosphere::CloudPrepared>,
    pub(in crate::synthetic) wetness: Option<&'a crate::wetness::PreparedWetness>,
    pub(in crate::synthetic) aerial: Option<AerialBindings<'a>>,
}

impl LightBindings<'_> {
    pub(in crate::synthetic) fn bind(
        self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        exposure: &[wgpu::Buffer; 2],
        structure: Option<&wgpu::Tlas>,
        lamps: Option<&super::super::raytrace::LampShadows>,
        soft: Option<(&super::super::raytrace::soft::SoftTarget, usize)>,
        ambient: Option<(&super::super::raytrace::ambient::Target, usize)>,
    ) -> [wgpu::BindGroup; 2] {
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.directional.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.local.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: self.heads.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: self.indices.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: crate::light::ENVIRONMENT_BINDING,
                resource: self.environment_light.as_entire_binding(),
            },
        ];
        if let Some(atmosphere) = self.atmosphere {
            if let Some(lighting) = atmosphere.chart_lighting() {
                entries.push(wgpu::BindGroupEntry {
                    binding: crate::atmosphere::overlay::CHART_LIGHTING_BINDING,
                    resource: lighting.as_entire_binding(),
                });
            }
            let (views, sampler, uniform) =
                self.aerial.unwrap_or_else(|| atmosphere.aerial_sampled());
            let (reflection, dfg, reflection_sampler) = atmosphere.reflection_sampled();
            entries.extend([
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(reflection),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(dfg),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(reflection_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: wgpu::BindingResource::TextureView(atmosphere.transmittance_view()),
                },
            ]);
        }
        if let Some(cloud) = self.cloud {
            entries.extend([
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: wgpu::BindingResource::TextureView(&cloud.view),
                },
                wgpu::BindGroupEntry {
                    binding: 14,
                    resource: cloud.uniform.as_entire_binding(),
                },
            ]);
        }
        if let Some(band) = self.cloud.and_then(|cloud| cloud.band.as_ref()) {
            entries.extend(band.entries());
        }
        if let Some(wetness) = self.wetness {
            entries.push(wetness.forward_entry());
        }
        if let Some(structure) = structure {
            entries.push(wgpu::BindGroupEntry {
                binding: 15,
                resource: structure.as_binding(),
            });
        }
        if let Some((target, side)) = soft {
            entries.extend(target.light_bindings(side));
        }
        if let Some((target, side)) = ambient {
            entries.extend(target.light_bindings(side));
        }
        if let Some(lamps) = lamps {
            entries.extend(lamps.bindings());
        }
        std::array::from_fn(|side| {
            entries.push(wgpu::BindGroupEntry {
                binding: 12,
                resource: exposure[side].as_entire_binding(),
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("synthetic clustered light lists"),
                layout,
                entries: &entries,
            });
            entries.pop();
            group
        })
    }
}
