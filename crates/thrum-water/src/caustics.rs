//! Draw each refracted sheet into its own caustic depth rung.

use bytemuck::{Pod, Zeroable};
use std::sync::{Arc, Mutex};
use thrum_render::{
    binding,
    contribution::{
        ContributionFrame, EffectAccess, EffectCadence, EffectDrawTarget, EffectGraph, EffectPass,
        SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};
use wgpu::util::DeviceExt;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
const SIDE: u32 = 512;
const WINDOW: f32 = 18.0;
const RUNGS: u32 = 13;

/// Diagnostic access to the caustic field after an ordinary submitted frame.
#[derive(Clone, Default)]
pub struct CausticTap(pub(super) Arc<Mutex<Option<wgpu::Texture>>>);

impl CausticTap {
    /// The current generation's caustic depth ladder, when prepared.
    /// # Panics
    /// Panics if the diagnostic tap's lock was poisoned.
    #[must_use]
    pub fn texture(&self) -> Option<wgpu::Texture> {
        self.0.lock().expect("caustic tap").clone()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DrawFrame {
    // Window origin in world x/z, span, and admission.
    window: [f32; 4],
    // World direction toward the Sun.
    sun: [f32; 4],
}

pub(super) struct Caustics {
    first: usize,
    texture: wgpu::Texture,
    targets: Vec<EffectDrawTarget>,
    names: Vec<String>,
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    frame: wgpu::BindGroup,
    view: wgpu::BindGroup,
    grids: [wgpu::Buffer; 3],
}

impl Caustics {
    pub(super) fn new(
        surface: &SceneSurface<'_>,
        source: &str,
        field: &wgpu::BindGroupLayout,
        first: usize,
        mapped: bool,
    ) -> Result<Self, String> {
        let device = surface.device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water caustic field"),
            size: wgpu::Extent3d {
                width: SIDE,
                height: SIDE,
                depth_or_array_layers: RUNGS,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let names: Vec<_> = (0..RUNGS)
            .map(|index| format!("water caustic rung {}", index + 1))
            .collect();
        let targets = (0..RUNGS)
            .zip(&names)
            .map(|(layer, name)| {
                EffectDrawTarget::new(
                    name,
                    &texture,
                    0,
                    layer,
                    wgpu::Color::TRANSPARENT,
                    true,
                    surface,
                )
                .map_err(|error| error.to_string())
            })
            .collect::<Result<_, _>>()?;
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water caustic window"),
            size: size_of::<DrawFrame>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water caustic window"),
            entries: &[binding::uniform(
                0,
                wgpu::ShaderStages::VERTEX,
                uniform.size(),
            )],
        });
        let frame = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water caustic window"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let mut view_entry = binding::uniform(0, wgpu::ShaderStages::VERTEX, 0);
        if let wgpu::BindingType::Buffer {
            has_dynamic_offset, ..
        } = &mut view_entry.ty
        {
            *has_dynamic_offset = true;
        }
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water caustic view"),
            entries: &[view_entry],
        });
        let view = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water caustic view"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(surface.mesh_view_uniform.clone()),
            }],
        });
        let source = format!(
            "{source}\n{}\n{}",
            if mapped {
                "fn caustic_slot(point: vec2<f32>) -> u32 {
                    let answer = map_sample(point);
                    return select(answer.body + 1u, 1u, answer.body >= map.size_count_sea.z);
                }"
            } else {
                "fn caustic_slot(_point: vec2<f32>) -> u32 { return 0u; }"
            },
            include_str!("caustics/draw.wgsl"),
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water caustic refraction"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water caustic refraction"),
            bind_group_layouts: &[Some(&view_layout), Some(&layout), None, Some(field)],
            immediate_size: 0,
        });
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("water caustic refraction"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("caustic_vertex"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("caustic_fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: additive(),
                        alpha: additive(),
                    }),
                    write_mask: wgpu::ColorWrites::RED,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let grids = [512, 256, 128].map(|side| grid(device, side));
        Ok(Self {
            first,
            texture,
            targets,
            names,
            pipeline,
            uniform,
            frame,
            view,
            grids,
        })
    }

    pub(super) fn declare(&self, graph: &mut EffectGraph, wave_reads: &[EffectAccess]) {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::of_texture(&self.texture),
            ResourceScope::Persistent {
                owner: 0x6361_7573,
                epoch: 0,
                initialized: false,
            },
        ));
        for (rung, name) in (0..RUNGS).zip(&self.names) {
            let mut accesses = wave_reads.to_vec();
            accesses.push(EffectAccess {
                resource,
                range: u64::from(rung)..u64::from(rung + 1),
                kind: AccessKind::AttachmentClear,
                view: None,
                history: None,
            });
            graph.passes.push(EffectPass {
                name: name.clone(),
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses,
            });
        }
    }

    pub(super) fn record(&self, queue: &wgpu::Queue, frame: &ContributionFrame) {
        if frame.time.capture {
            return;
        }
        let sun = frame
            .environment
            .map_or([0.0, 1.0, 0.0], |light| light.true_sun.toward_sun);
        let cell = WINDOW / 512.0;
        // The sample grid lives in world space. A sub-cell eye move leaves it fixed.
        let origin = [frame.camera[0], frame.camera[2]]
            .map(|axis| ((axis - WINDOW * 0.5) / cell).floor() * cell);
        let mapping = DrawFrame {
            window: [
                origin[0],
                origin[1],
                WINDOW,
                f32::from(u8::from(sun[1] > 0.0)),
            ],
            sun: [sun[0], sun[1], sun[2], 0.0],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&mapping));
    }

    pub(super) fn target(&self, index: usize) -> Option<&EffectDrawTarget> {
        self.targets.get(index.checked_sub(self.first)?)
    }

    pub(super) fn draw(
        &self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
        field: &wgpu::BindGroup,
    ) {
        let rung = u32::try_from(index - self.first).expect("caustic draw index");
        let grid = &self.grids[(rung as usize / 4).min(2)];
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view, &[frame.view_uniform_offset]);
        pass.set_bind_group(1, &self.frame, &[]);
        pass.set_bind_group(3, field, &[]);
        pass.set_index_buffer(grid.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(
            0..u32::try_from(grid.size() / 4).expect("caustic grid size"),
            0,
            rung..rung + 1,
        );
    }

    pub(super) fn bytes(&self) -> u64 {
        2 * u64::from(SIDE) * u64::from(SIDE) * u64::from(RUNGS)
            + self.uniform.size()
            + self.grids.iter().map(wgpu::Buffer::size).sum::<u64>()
    }

    pub(super) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

fn grid(device: &thrum_render::runtime::DeviceSite, side: u32) -> wgpu::Buffer {
    let indices: Vec<_> = (0..side)
        .flat_map(|z| {
            (0..side).flat_map(move |x| {
                let origin = z * (side + 1) + x;
                let next = origin + side + 1;
                [origin, origin + 1, next, next, origin + 1, next + 1]
            })
        })
        .collect();
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("water caustic grid"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    })
}

fn additive() -> wgpu::BlendComponent {
    wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    }
}
