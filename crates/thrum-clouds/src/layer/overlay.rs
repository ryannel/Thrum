use thrum_render::{
    atmosphere::EnvironmentProducts,
    binding,
    contribution::{
        ContributionFrame, EffectAccess, EffectCadence, EffectGraph, EffectPass, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};

pub(crate) const PASS: &str = "cloud sky overlay";
const SLICES: u32 = 8;

pub(super) struct Overlay {
    pub(super) texture: wgpu::Texture,
    rows: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    output: wgpu::BindGroup,
    row_count: u32,
    initialized: bool,
    phase: u32,
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
            initialized: false,
            phase: 0,
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

    pub(super) fn upload(&mut self, queue: &wgpu::Queue, frame: &ContributionFrame) {
        let height = self.texture.height();
        let full = !self.initialized || frame.history_reset || frame.time.capture;
        let step = height.div_ceil(SLICES);
        let row = if full { 0 } else { self.phase * step };
        self.row_count = if full { height } else { step.min(height - row) };
        queue.write_buffer(
            &self.rows,
            0,
            bytemuck::cast_slice(&[row, self.row_count, 0, 0]),
        );
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
        self.initialized = true;
        self.phase = (self.phase + 1) % SLICES;
    }
}
