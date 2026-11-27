//! Draw each refracted sheet into its own caustic depth rung.

mod filter;
mod mean;
pub(crate) mod quality;
pub(crate) mod shafts;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
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
const RUNGS: u32 = 13;
const BORDER_CELLS: u32 = 16;
const RUNGS_PER_GRID_SCALE: u32 = 4;
// At sqrt(2) per rung from 0.25 m, rung six is the first at 2 m.
const GRID_RUNG_OFFSET: u32 = 2;
const COLUMNS_PASS: &str = "water caustic columns";
const RECEIVER: &str = concat!(
    include_str!("caustics/refraction.wgsl"),
    include_str!("caustics/sampling.wgsl"),
    include_str!("caustics/receiver.wgsl"),
);

pub(super) const fn provider() -> thrum_render::sun_path::SunPathProvider {
    thrum_render::sun_path::SunPathProvider::new(RECEIVER, SIDE, RUNGS)
}

struct Columns {
    pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    target: EffectDrawTarget,
}

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
    quality: [u32; 4],
}

pub(super) struct Caustics {
    first: usize,
    texture: wgpu::Texture,
    filter: filter::Filter,
    columns: Option<Columns>,
    targets: Vec<EffectDrawTarget>,
    names: Vec<String>,
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    frame: wgpu::BindGroup,
    view: wgpu::BindGroup,
    grids: [wgpu::Buffer; 3],
    current: Option<quality::Values>,
    submitted: u64,
    pending: bool,
}

impl Caustics {
    pub(super) fn new(
        surface: &SceneSurface<'_>,
        source: &str,
        field: &wgpu::BindGroupLayout,
        first: usize,
        mapped: bool,
        optics: Option<super::Optics>,
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
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
        let uniform = surface.sun_path.map_or_else(
            || {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("water caustic window"),
                    size: size_of::<DrawFrame>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            },
            |target| target.mapping.clone(),
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water caustic window"),
            entries: &[binding::uniform(
                0,
                wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE,
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
            "{source}\nconst CAUSTIC_RUNGS_PER_GRID_SCALE: u32 = {RUNGS_PER_GRID_SCALE}u;\n\
             const CAUSTIC_GRID_RUNG_OFFSET: u32 = {GRID_RUNG_OFFSET}u;\n{}\n{}",
            if mapped {
                "fn caustic_slot(point: vec2<f32>) -> u32 {
                    let answer = map_sample(point);
                    return select(answer.body + 1u, 1u, answer.body >= map.size_count_sea.z);
                }"
            } else {
                "fn caustic_slot(_point: vec2<f32>) -> u32 { return 0u; }"
            },
            concat!(
                include_str!("caustics/frame.wgsl"),
                include_str!("caustics/rungs.wgsl"),
                include_str!("caustics/refraction.wgsl"),
                include_str!("caustics/draw.wgsl"),
            ),
        );
        let columns = surface
            .sun_path
            .map(|target| {
                Columns::new(
                    surface,
                    target.columns.clone(),
                    &source,
                    &layout,
                    field,
                    optics,
                )
            })
            .transpose()?;
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
        let grids = [512, 256, 128].map(|side| grid(device, side + 2 * BORDER_CELLS));
        let filter = filter::Filter::new(
            device,
            &texture,
            &layout,
            surface.sun_path.map(|target| &target.field),
        );
        Ok(Self {
            first,
            texture,
            filter,
            columns,
            targets,
            names,
            pipeline,
            uniform,
            frame,
            view,
            grids,
            current: None,
            submitted: 0,
            pending: false,
        })
    }

    pub(super) fn declare(
        &self,
        graph: &mut EffectGraph,
        wave_reads: &[EffectAccess],
    ) -> Option<[EffectAccess; 2]> {
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
        let field = self.filter.declare(graph, resource);
        self.columns.as_ref().map(|columns| {
            let columns = columns.declare(graph);
            graph.opaque_reads.extend([field.clone(), columns.clone()]);
            [field, columns]
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the GPU reads f32 world levels and wave slopes"
    )]
    pub(super) fn record(
        &mut self,
        queue: &wgpu::Queue,
        frame: &ContributionFrame,
        level: f64,
        tail_variance: f64,
        quality: quality::Values,
    ) {
        self.pending = !frame.time.capture
            && (self.current != Some(quality)
                || frame.history_reset
                || self.submitted.is_multiple_of(u64::from(quality.cadence)));
        if !self.pending {
            return;
        }
        self.current = Some(quality);
        let sun = frame
            .environment
            .map_or([0.0, 1.0, 0.0], |light| light.true_sun.toward_sun);
        let span = f64::from(quality.window) as f32;
        let origin = window_origin(frame, level as f32, span);
        let mapping = DrawFrame {
            window: [
                origin[0],
                origin[1],
                span,
                f32::from(u8::from(sun[1] > 0.0)),
            ],
            sun: [sun[0], sun[1], sun[2], tail_variance.sqrt() as f32],
            quality: [
                quality.cell_scale,
                quality.rungs,
                quality.shaft_steps,
                quality.shaft_divisor,
            ],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&mapping));
    }

    pub(super) fn runs(&self, index: usize) -> bool {
        self.pending
            && (index - self.first >= RUNGS as usize
                || self
                    .current
                    .is_some_and(|values| index - self.first < values.rungs as usize))
    }

    pub(super) fn submitted(&mut self) {
        self.submitted = self.submitted.wrapping_add(1);
    }

    pub(super) fn target(&self, index: usize) -> Option<&EffectDrawTarget> {
        let pass = index.checked_sub(self.first)?;
        if pass == (RUNGS + filter::PASSES) as usize {
            return self.columns.as_ref().map(|columns| &columns.target);
        }
        self.targets.get(pass)
    }

    pub(super) fn contains(&self, index: usize) -> bool {
        let count = (RUNGS + filter::PASSES) as usize + usize::from(self.columns.is_some());
        (self.first..self.first + count).contains(&index)
    }

    pub(super) fn encode(&self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        let mip = u32::try_from(index - self.first - RUNGS as usize).expect("caustic filter index");
        self.filter.encode(
            mip,
            pass,
            &self.frame,
            self.current.expect("caustic values").rungs,
        );
    }

    pub(super) fn draw(
        &self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
        field: &wgpu::BindGroup,
    ) {
        let rung = u32::try_from(index - self.first).expect("caustic draw index");
        if rung == RUNGS + filter::PASSES {
            if let Some(columns) = &self.columns {
                pass.set_pipeline(&columns.pipeline);
                pass.set_bind_group(1, &self.frame, &[]);
                pass.set_bind_group(3, field, &[]);
                pass.draw(0..3, 0..1);
            }
            return;
        }
        let cell_scale = self.current.expect("caustic values").cell_scale;
        let depth_scale = rung.saturating_sub(GRID_RUNG_OFFSET) / RUNGS_PER_GRID_SCALE;
        let grid = &self.grids[depth_scale.max(cell_scale).min(2) as usize];
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
            + self.filter.bytes()
            + self
                .columns
                .as_ref()
                .map_or(0, |_| u64::from(SIDE).pow(2) * 16)
            + self.grids.iter().map(wgpu::Buffer::size).sum::<u64>()
    }

    pub(super) fn texture(&self) -> &wgpu::Texture {
        &self.filter.texture
    }
}

fn window_origin(frame: &ContributionFrame, level: f32, span: f32) -> [f32; 2] {
    let eye = Vec3::from_array(frame.camera);
    let inverse = Mat4::from_cols_array_2d(&frame.view_proj).inverse();
    let clip = inverse * Vec4::new(0.0, 0.0, 1.0, 1.0);
    let ray = (clip.truncate() / clip.w - eye).normalize_or_zero();
    let rise = level - eye.y;
    let ahead = if ray.y.abs() > 0.05 && rise * ray.y > 0.0 {
        (rise / ray.y).min(span * 0.5)
    } else {
        0.0
    };
    let focus = eye + ray * ahead;
    // An integral step of the coarsest grid preserves the nodes of all rungs.
    let step = 4.0 * span / 512.0;
    [focus.x, focus.z].map(|axis| ((axis - span * 0.5) / step).floor() * step)
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

impl Columns {
    fn new(
        surface: &SceneSurface<'_>,
        texture: wgpu::Texture,
        field_source: &str,
        mapping: &wgpu::BindGroupLayout,
        field: &wgpu::BindGroupLayout,
        optics: Option<super::Optics>,
    ) -> Result<Self, String> {
        let target = EffectDrawTarget::new(
            COLUMNS_PASS,
            &texture,
            0,
            0,
            wgpu::Color::TRANSPARENT,
            false,
            surface,
        )
        .map_err(|error| error.to_string())?;
        let device = surface.device;
        let optics = optics.unwrap_or_default();
        let difference = (Vec3::from_array(optics.extinction)
            - Vec3::from_array(optics.downwelling))
        .dot(Vec3::new(0.2126, 0.7152, 0.0722))
        .max(0.0);
        let source = format!(
            "{field_source}\nconst CAUSTIC_BEAM_DIFFERENCE: f32 = {difference:?};\n{}",
            include_str!("caustics/columns.wgsl")
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(COLUMNS_PASS),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(COLUMNS_PASS),
            bind_group_layouts: &[None, Some(mapping), None, Some(field)],
            immediate_size: 0,
        });
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some(COLUMNS_PASS),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("caustic_columns_vertex"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("caustic_columns_fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            pipeline,
            texture,
            target,
        })
    }

    fn declare(&self, graph: &mut EffectGraph) -> EffectAccess {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::of_texture(&self.texture),
            ResourceScope::Persistent {
                owner: 0x6361_7563,
                epoch: 0,
                initialized: false,
            },
        ));
        let access = EffectAccess {
            resource,
            range: 0..1,
            kind: AccessKind::AttachmentClear,
            view: None,
            history: None,
        };
        graph.passes.push(EffectPass {
            name: COLUMNS_PASS.into(),
            cadence: EffectCadence::Skippable,
            async_eligible: false,
            accesses: vec![access.clone()],
        });
        EffectAccess {
            kind: AccessKind::SampledRead,
            view: Some(wgpu::TextureViewDimension::D2),
            ..access
        }
    }
}
