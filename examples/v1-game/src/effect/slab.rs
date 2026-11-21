//! Late slab using the public renderer seam, with submission-owned correspondence.
use super::EffectControls;
use thrum_render::binding;
use thrum_render::contribution::{AfterOpaquePass, ContributionFrame, SceneSurface};

pub(super) struct Slab {
    refraction: bool,
    gpu: Option<Gpu>,
    previous: Option<f32>,
    pending: f32,
}
struct Gpu {
    queue: wgpu::Queue,
    params: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    group: wgpu::BindGroup,
}
impl Slab {
    pub(super) fn new(refraction: bool) -> Self {
        Self {
            refraction,
            gpu: None,
            previous: None,
            pending: 0.0,
        }
    }
    pub(super) fn declaration(&self) -> AfterOpaquePass {
        AfterOpaquePass {
            name: "diagnostic slab".into(),
            kind: thrum_render::contribution::AfterOpaqueKind::Render,
            cadence: thrum_render::contribution::EffectCadence::EveryFrame,
            accesses: vec![
                (2, 0..1, wgpu::TextureViewDimension::D2),
                (3, 0..1, wgpu::TextureViewDimension::D3),
                (4, 0..6, wgpu::TextureViewDimension::D2Array),
                (5, 0..6, wgpu::TextureViewDimension::D2Array),
            ]
            .into_iter()
            .map(
                |(resource, range, view)| thrum_render::contribution::EffectAccess {
                    resource,
                    range,
                    view: Some(view),
                    kind: thrum_render::graph::AccessKind::SampledRead,
                    history: None,
                },
            )
            .chain(std::iter::once(thrum_render::contribution::EffectAccess {
                resource: 7,
                range: 0..32,
                view: None,
                kind: thrum_render::graph::AccessKind::UniformRead,
                history: None,
            }))
            .collect(),
            depth: true,
            write_depth: false,
            refraction: self.refraction,
        }
    }
    pub(super) fn rebuild(
        &mut self,
        surface: &SceneSurface<'_>,
        textures: &[Vec<wgpu::Texture>],
    ) -> Result<(), String> {
        let source = shader(self.refraction);
        let module = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("diagnostic slab"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let params = surface.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("slab displacement"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut entries = vec![binding::uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT, 32)];
        for (i, dimension) in [
            wgpu::TextureViewDimension::D2,
            wgpu::TextureViewDimension::D3,
            wgpu::TextureViewDimension::D2Array,
            wgpu::TextureViewDimension::D2Array,
        ]
        .into_iter()
        .enumerate()
        {
            entries.push(binding::texture(
                u32::try_from(i + 1).map_err(|e| e.to_string())?,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Uint,
                dimension,
            ));
        }
        let own = surface
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("slab own inputs"),
                entries: &entries,
            });
        let late = self.declaration().input_layout(surface.device);
        let layout = surface
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("slab"),
                bind_group_layouts: &[Some(surface.view_layout), Some(&late), Some(&own)],
                immediate_size: 0,
            });
        let pipeline = surface.device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("diagnostic slab"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &surface.color_formats.map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: if format == surface.color_formats[0] && !self.refraction {
                            Some(wgpu::BlendState::ALPHA_BLENDING)
                        } else {
                            None
                        },
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                }),
            }),
            multiview_mask: None,
            cache: None,
        });
        let views = [2, 3, 4, 5].map(|i| {
            textures[i][0].create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(if i == 3 {
                    wgpu::TextureViewDimension::D3
                } else if i >= 4 {
                    wgpu::TextureViewDimension::D2Array
                } else {
                    wgpu::TextureViewDimension::D2
                }),
                ..Default::default()
            })
        });
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: params.as_entire_binding(),
        }];
        for (i, view) in views.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: u32::try_from(i + 1).map_err(|e| e.to_string())?,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        let group = surface
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("slab fields"),
                layout: &own,
                entries: &entries,
            });
        self.gpu = Some(Gpu {
            queue: surface.queue.clone(),
            params,
            pipeline,
            group,
        });
        self.previous = None;
        Ok(())
    }
    pub(super) fn update(&mut self, frame: &ContributionFrame, controls: EffectControls) {
        self.pending = controls.slab_offset;
        let valid = self.previous.is_some() && !frame.history_reset && !controls.slab_reset;
        let words = [
            controls.slab_offset,
            self.previous.unwrap_or(controls.slab_offset),
            if valid { 1.0 } else { 0.0 },
            0.0,
        ];
        let mut bytes = [0; 32];
        for (i, v) in words.into_iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        for (i, v) in controls.slab_origin_scale.into_iter().enumerate() {
            bytes[16 + i * 4..20 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        if let Some(gpu) = &self.gpu {
            gpu.queue.write_buffer(&gpu.params, 0, &bytes);
        }
    }
    pub(super) fn submitted(&mut self) {
        self.previous = Some(self.pending);
    }
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(gpu) = &self.gpu {
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(2, &gpu.group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}

fn shader(refraction: bool) -> String {
    format!(
        "{}\n{}",
        include_str!("slab.wgsl"),
        if refraction {
            "@group(1) @binding(1) var scene_copy:texture_2d<f32>; fn colour(p:vec2<i32>, tint:vec3<f32>, z:f32)->vec4<f32>{ let size=vec2<i32>(textureDimensions(scene_copy)); var q=clamp(p+vec2(6,0),vec2(0),size-vec2(1)); if(textureLoad(depth,q,0)>z){q=p;} return vec4(textureLoad(scene_copy,q,0).rgb*(vec3(0.65)+tint),1.0); }"
        } else {
            "fn colour(p:vec2<i32>, tint:vec3<f32>, z:f32)->vec4<f32>{return vec4(0.0,0.0,0.0,0.35);}"
        }
    )
}

#[cfg(test)]
mod tests;
