//! An opt-in, render-only GPU load. Dispatch area follows the drawn subrectangle.

use std::ops::Range;
use thrum_render::{
    contribution::{
        ContributionFrame, ContributionPasses, EffectAccess, EffectCadence, EffectGraph,
        EffectPass, SceneContribution, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::FrameError,
};

const PASS: &str = "debug load";
const MAX_AXIS: u32 = 256;
const BYTES: u64 = MAX_AXIS as u64 * MAX_AXIS as u64 * 4;

pub(crate) struct DebugLoad {
    ticks: Range<u64>,
    rounds: u32,
    prepared: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
}

impl DebugLoad {
    pub(crate) const MAX_ROUNDS: u32 = 8192;
    pub(crate) fn new(ticks: Range<u64>, rounds: u32) -> Self {
        Self {
            ticks,
            rounds,
            prepared: None,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn alongside(self, scene: Box<dyn SceneContribution>) -> Box<dyn SceneContribution> {
        Box::new(thrum_render::contribution::SceneContributions::new(vec![
            scene,
            Box::new(self),
        ]))
    }
}

impl SceneContribution for DebugLoad {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let device = surface.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(include_str!("governor_load.wgsl").into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: None,
            module: &shader,
            entry_point: Some("load"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[("rounds", f64::from(self.rounds))],
                ..Default::default()
            },
            cache: None,
        });
        let scratch = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(PASS),
            size: BYTES,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scratch.as_entire_binding(),
            }],
        });
        self.prepared = Some((pipeline, group));
        Ok(())
    }

    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            resources: vec![(
                ResourceDescriptor::Buffer {
                    bytes: BYTES,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                ResourceScope::Persistent {
                    owner: 0,
                    epoch: 0,
                    initialized: true,
                },
            )],
            passes: vec![EffectPass {
                name: PASS.into(),
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses: vec![EffectAccess {
                    resource: 0,
                    range: 0..BYTES,
                    kind: AccessKind::StorageReadWrite,
                    view: None,
                    history: None,
                }],
            }],
            ..Default::default()
        }
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn effect_runs(&self, _: usize, frame: &ContributionFrame) -> bool {
        self.rounds != 0 && self.ticks.contains(&frame.time.tick)
    }

    fn encode_effect(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        let (pipeline, group) = self
            .prepared
            .as_ref()
            .expect("debug load was prepared at attach");
        let [x, y] = frame
            .render_extent
            .map(|size| size.div_ceil(8).min(MAX_AXIS));
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.dispatch_workgroups(x, y, 1);
    }
}
