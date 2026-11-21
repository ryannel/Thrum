//! The same stable scan/scatter as camera visibility, with view-local tests
//! and outputs. It never executes either occlusion phase or writes history.
use super::{
    Access, AccessKind, CompiledPass, FrameGraph, GpuTimings, GraphError, PassScope, PipelineKey,
    TimingError, VisibilityPrepared, VisibilityTarget,
};
use crate::graph::PassBody;
use crate::pass_key::{PassKey, SecondaryStage};

pub(crate) const PASS: &str = "secondary view visibility";
#[derive(Clone)]
pub(crate) struct SecondaryVisibility {
    pipelines: [wgpu::ComputePipeline; 3],
    record: wgpu::BindGroup,
}

impl SecondaryVisibility {
    pub(crate) fn new(
        world: &VisibilityPrepared,
        device: &crate::runtime::DeviceSite,
        record: &wgpu::Buffer,
    ) -> Self {
        let record_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size_of::<
                        crate::contribution::SecondaryViewRecord,
                    >() as u64),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&world.layout), Some(&record_layout)],
            immediate_size: 0,
        });
        let module = crate::shaders::Shader::SecondaryVisibility.compile(device, PASS);
        let suffix = if world.members.is_some() {
            "_members"
        } else {
            ""
        };
        let pipelines = ["count", "offsets", "scatter"].map(|name| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(PASS),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(&format!("{name}{suffix}")),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        });
        Self {
            pipelines,
            record: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(PASS),
                layout: &record_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: record.as_entire_binding(),
                }],
            }),
        }
    }

    pub(crate) fn append(
        world: &VisibilityPrepared,
        graph: &mut FrameGraph,
        key: PipelineKey,
        index: usize,
        mut reads: Vec<Access>,
        opaque: &mut Vec<Access>,
    ) -> Result<(), GraphError> {
        let (visible, args) = world.outputs(graph, &mut reads)?;
        opaque.extend([
            Access::new(visible, 0..world.visible_bytes(), AccessKind::StorageRead),
            Access::new(args, 0..world.args_bytes(), AccessKind::IndirectRead),
        ]);
        graph.declare(
            PassKey::Secondary {
                view: crate::pass_key::pass_index(index),
                stage: SecondaryStage::Visibility,
            },
            PassBody {
                scope: PassScope::View(index as u64 + 2),
                pipeline: Some(key),
                accesses: reads,
                external_side_effect: false,
            },
        );
        Ok(())
    }

    pub(crate) fn encode(
        &self,
        world: &VisibilityPrepared,
        target: &VisibilityTarget,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<bool, TimingError> {
        let (groups, records) = world
            .members
            .as_ref()
            .map_or((world.groups(), 1), |members| {
                (members.groups.max(1), members.shape.buckets)
            });
        let Some(mut pass) = compiled.begin_compute(encoder, generation, timings)? else {
            return Ok(false);
        };
        pass.set_bind_group(0, &target.groups[0], &[0, 0]);
        pass.set_bind_group(1, &self.record, &[]);
        for (pipeline, groups) in self.pipelines.iter().zip([groups, records, groups]) {
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(groups, 1, 1);
        }
        Ok(true)
    }
}
