use thrum_render::{
    atmosphere::{EnvironmentEpoch, EnvironmentFrame, EnvironmentProducts, TrueSun},
    binding,
    contribution::{
        ContributionFrame, EffectAccess, EffectCadence, EffectGraph, EffectPass, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};

pub(crate) const PASS: &str = "cloud sky overlay";
pub(crate) const DEFAULT_OVERLAY_SLICES: u32 = 8;

#[derive(Clone, Copy, PartialEq)]
struct OverlayInput {
    tick: u64,
    camera: [f32; 3],
    epoch: EnvironmentEpoch,
    sun: TrueSun,
}

impl OverlayInput {
    fn hemisphere_matches(self, other: Self) -> bool {
        self.epoch == other.epoch
            && self.sun == other.sun
            && self.camera[1].to_bits() == other.camera[1].to_bits()
    }
}

pub(super) struct Overlay {
    pub(super) texture: wgpu::Texture,
    rows: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    output: wgpu::BindGroup,
    row_count: u32,
    committed: Option<OverlayInput>,
    pending: Option<OverlayInput>,
    phase: u32,
    slices: u32,
    remaining: u32,
    pending_remaining: u32,
    ambient_due: bool,
}

impl Overlay {
    pub(super) fn new(
        surface: &SceneSurface<'_>,
        texture: wgpu::Texture,
        environment: &EnvironmentProducts,
        trace: &wgpu::BindGroupLayout,
        shapes: Option<&crate::shapes::Shapes>,
    ) -> Self {
        let device = surface.device;
        let visibility = wgpu::ShaderStages::COMPUTE;
        let rows = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud overlay rows"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[
                binding::storage_texture(
                    0,
                    visibility,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba16Float,
                    wgpu::TextureViewDimension::D2Array,
                ),
                binding::uniform(1, visibility, 16),
            ],
        });
        let output = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rows.as_entire_binding(),
                },
            ],
        });
        let mut layouts = vec![Some(&environment.layout), Some(&layout), Some(trace)];
        if let Some(shapes) = shapes {
            layouts.push(Some(&shapes.layout));
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(
                super::prepare::overlay_source(environment, shapes.is_some()).into(),
            ),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("trace_overlay"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            texture,
            rows,
            pipeline,
            output,
            row_count: 0,
            committed: None,
            pending: None,
            phase: 0,
            slices: DEFAULT_OVERLAY_SLICES,
            remaining: 0,
            pending_remaining: 0,
            ambient_due: false,
        }
    }

    pub(super) fn append_graph(
        &self,
        graph: &mut EffectGraph,
        epoch: u64,
        mut accesses: Vec<EffectAccess>,
    ) {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::of_texture(&self.texture),
            ResourceScope::Persistent {
                owner: 106,
                epoch,
                initialized: true,
            },
        ));
        accesses.push(EffectAccess {
            resource,
            range: 0..6,
            kind: AccessKind::StorageWrite,
            history: None,
            view: None,
        });
        let rows = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: self.rows.size(),
                usage: self.rows.usage(),
            },
            ResourceScope::Imported { initialized: true },
        ));
        accesses.push(EffectAccess {
            resource: rows,
            range: 0..16,
            kind: AccessKind::UniformRead,
            history: None,
            view: None,
        });
        graph.sky_overlays.push(resource);
        graph.passes.push(EffectPass {
            name: PASS.into(),
            cadence: EffectCadence::TimeSliced,
            async_eligible: true,
            accesses,
        });
    }

    pub(super) fn set_cadence(&mut self, slices: u32) {
        if self.slices != slices {
            self.slices = slices;
            self.phase = 0;
            self.remaining = slices;
        }
    }

    pub(super) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        frame: &ContributionFrame,
        environment: EnvironmentFrame,
        reset: bool,
    ) {
        let input = OverlayInput {
            tick: frame.time.tick,
            camera: frame.camera,
            epoch: environment.epoch,
            sun: environment.true_sun,
        };
        let changed = self.committed != Some(input);
        let remaining = if changed || reset {
            self.slices
        } else {
            self.remaining
        };
        self.pending = None;
        self.row_count = 0;
        if remaining == 0 {
            return;
        }
        self.ambient_due = self
            .committed
            .is_none_or(|old| !old.hemisphere_matches(input));
        let height = self.texture.height();
        let full = reset || (changed && frame.time.capture);
        let row = if full {
            0
        } else {
            self.phase * height / self.slices
        };
        let end = if full {
            height
        } else {
            (self.phase + 1) * height / self.slices
        };
        self.row_count = end - row;
        self.pending = Some(input);
        self.pending_remaining = if full { 0 } else { remaining - 1 };
        queue.write_buffer(
            &self.rows,
            0,
            bytemuck::cast_slice(&[row, self.row_count, 0, 0]),
        );
    }

    pub(super) fn runs(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn ambient_due(&self) -> bool {
        self.ambient_due
    }

    pub(super) fn encode(
        &self,
        pass: &mut wgpu::ComputePass<'_>,
        environment: &EnvironmentProducts,
        trace: &wgpu::BindGroup,
        shapes: Option<&crate::shapes::Shapes>,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, environment.group(0), &[]);
        pass.set_bind_group(1, &self.output, &[]);
        pass.set_bind_group(2, trace, &[]);
        if let Some(shapes) = shapes {
            pass.set_bind_group(3, &shapes.group, &[]);
        }
        pass.dispatch_workgroups(
            self.texture.width().div_ceil(8),
            self.row_count.div_ceil(4),
            6,
        );
    }

    pub(super) fn submitted(&mut self) {
        if let Some(input) = self.pending.take() {
            self.committed = Some(input);
            self.remaining = self.pending_remaining;
            self.phase = (self.phase + 1) % self.slices;
        }
    }
}
