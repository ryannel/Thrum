use std::sync::{Arc, Mutex};

use mirror_scene::Pool;
use thrum_render::{
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
        EffectCadence, EffectGraph, QualityError, QualityInvalidation, QualityKnob,
        SceneContribution, SceneSurface, SecondaryView, SecondaryViewFrame, SecondaryViewRequest,
        SecondaryViewSlot,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};

pub(super) const VIEWPORT_REPORT_BYTES: u64 = 16;

fn viewport_pass() -> AfterOpaquePass {
    AfterOpaquePass {
        name: "reflection viewport witness".into(),
        kind: AfterOpaqueKind::Compute,
        cadence: EffectCadence::EveryFrame,
        accesses: vec![EffectAccess {
            resource: 0,
            range: 0..VIEWPORT_REPORT_BYTES,
            kind: AccessKind::StorageWrite,
            history: None,
            view: None,
        }],
        depth: false,
        refraction: false,
        images: thrum_render::contribution::AfterOpaqueImages::None,
        write_depth: false,
    }
}

#[derive(Default)]
pub(super) struct Observation {
    pub(super) reference: Option<SecondaryViewRequest>,
    pub(super) view: Option<SecondaryViewFrame>,
    pub(super) measure_viewport: bool,
}

// Render the real pool in both runs. In the reference run supersede its
// request with one whose values are supplied directly, without the knob
// implementation. This gives an expected image for each endpoint on the
// same host and device, with exactly the same main frame and pool shader.
pub(super) struct PoolWitness {
    extent: [u32; 2],
    pool: Pool,
    slot: Option<SecondaryView>,
    observation: Arc<Mutex<Observation>>,
    viewport_report: wgpu::Buffer,
    viewport_probe: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
}

impl PoolWitness {
    pub(super) fn new(
        extent: [u32; 2],
        pool: Pool,
        observation: Arc<Mutex<Observation>>,
        viewport_report: wgpu::Buffer,
    ) -> Self {
        Self {
            extent,
            pool,
            observation,
            viewport_report,
            slot: None,
            viewport_probe: None,
        }
    }
}

impl SceneContribution for PoolWitness {
    fn quality_knobs(&self) -> Vec<QualityKnob> {
        self.pool.quality_knobs()
    }
    fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
        self.pool.set_quality_knob(name, value)
    }
    fn commit_quality_knobs(&mut self) -> QualityInvalidation {
        self.pool.commit_quality_knobs()
    }
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        self.pool.secondary_view_slots()
    }
    fn secondary_views_enabled(&self) -> bool {
        true
    }
    fn effect_graph(&self) -> EffectGraph {
        let mut graph = self.pool.effect_graph();
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: VIEWPORT_REPORT_BYTES,
                usage: self.viewport_report.usage(),
            },
            ResourceScope::Persistent {
                owner: 1,
                epoch: 1,
                initialized: false,
            },
        ));
        graph.after_opaque.push(viewport_pass());
        graph
    }
    fn passes(&self) -> ContributionPasses {
        self.pool.passes()
    }
    fn rebuild(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        let slot = surface.secondary_views.first().expect("pool slot");
        assert_eq!(slot.maximum_extent(), self.extent);
        self.slot = Some(slot.clone());
        let device = surface.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reflection viewport witness"),
            source: wgpu::ShaderSource::Wgsl(include_str!("viewport.wgsl").into()),
        });
        let entries = [
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        ];
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("reflection viewport witness"),
            entries: &[0_u32, 1, 2].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: entries[binding as usize],
                count: None,
            }),
        });
        let inputs = viewport_pass().input_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("reflection viewport witness"),
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&inputs),
                Some(&group_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("reflection viewport witness"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("inspect_viewport"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reflection viewport witness"),
            layout: &group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(slot.colour()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(slot.depth()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.viewport_report.as_entire_binding(),
                },
            ],
        });
        self.viewport_probe = Some((pipeline, group));
        self.pool.rebuild(surface)
    }
    fn after_opaque_runs(&self, index: usize, _: &ContributionFrame) -> bool {
        index == 0
            || self
                .observation
                .lock()
                .expect("observation")
                .measure_viewport
    }
    fn encode_after_opaque(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (pipeline, group) = self
            .viewport_probe
            .as_ref()
            .expect("prepared viewport probe");
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.pool.encode_upload(encoder, frame);
        if let Some(reference) = self.observation.lock().expect("observation").reference {
            self.slot
                .as_ref()
                .expect("prepared slot")
                .request(&reference)
                .expect("reference request");
        }
    }
    fn encode_secondary_upload(
        &mut self,
        _: &mut wgpu::CommandEncoder,
        _: &ContributionFrame,
        views: &[SecondaryViewFrame],
    ) {
        assert_eq!(views.len(), 1);
        self.observation.lock().expect("observation").view = Some(views[0]);
    }
    fn draw_after_opaque(
        &self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        self.pool.draw_after_opaque(index, pass, frame);
    }
}

pub(super) fn read_viewport(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    report: &wgpu::Buffer,
) -> [u32; 4] {
    let bytes = thrum_test_support::readback::buffer(device, queue, report, ..);
    std::array::from_fn(|i| u32::from_le_bytes(bytes[i * 4..][..4].try_into().expect("word")))
}
