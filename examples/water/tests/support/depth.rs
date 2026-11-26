//! Full-precision reverse-Z from the scene depth after water has written it.
use std::sync::{Arc, Mutex};

use thrum_render::{
    contribution::{
        AfterOpaqueImages, AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses,
        EffectAccess, EffectCadence, EffectGraph, SceneContribution, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::FrameError,
};

#[derive(Clone, Default)]
pub(crate) struct DepthTap(Arc<Mutex<Option<wgpu::Buffer>>>);

impl DepthTap {
    pub(crate) fn contribution(&self, extent: [u32; 2]) -> impl SceneContribution + use<> {
        DepthProbe {
            extent,
            output: self.clone(),
            prepared: None,
        }
    }

    pub(crate) fn read(&self, scene: &super::Fixture) -> Vec<f32> {
        let buffer = self.0.lock().expect("depth tap");
        super::foam_buffer(scene, buffer.as_ref().expect("submitted depth observer"))
    }
}

struct DepthProbe {
    extent: [u32; 2],
    output: DepthTap,
    prepared: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
}

impl DepthProbe {
    fn size(&self) -> u64 {
        self.extent.map(u64::from).into_iter().product::<u64>() * 4
    }

    fn declaration(&self) -> AfterOpaquePass {
        AfterOpaquePass {
            name: "submitted depth observer".into(),
            kind: AfterOpaqueKind::Compute,
            cadence: EffectCadence::EveryFrame,
            accesses: vec![EffectAccess {
                resource: 0,
                range: 0..self.size(),
                kind: AccessKind::StorageWrite,
                history: None,
                view: None,
            }],
            depth: true,
            write_depth: false,
            refraction: false,
            images: AfterOpaqueImages::None,
        }
    }
}

impl SceneContribution for DepthProbe {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: false,
            compute: false,
        }
    }

    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            resources: vec![(
                ResourceDescriptor::Buffer {
                    bytes: self.size(),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                },
                ResourceScope::Persistent {
                    owner: 1,
                    epoch: 1,
                    initialized: false,
                },
            )],
            after_opaque: vec![self.declaration()],
            ..EffectGraph::default()
        }
    }

    fn rebuild(&mut self, site: &SceneSurface<'_>) -> Result<(), FrameError> {
        let device = site.device;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("submitted depth readout"),
            size: self.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let output = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &output,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let inputs = self.declaration().input_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(site.view_layout), Some(&inputs), Some(&output)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r"
                @group(1) @binding(0) var scene: texture_depth_2d;
                @group(2) @binding(0) var<storage, read_write> depths: array<f32>;

                @compute @workgroup_size(8, 8)
                fn read_depth(@builtin(global_invocation_id) id: vec3<u32>) {
                    let extent = textureDimensions(scene);
                    if all(id.xy < extent) {
                        depths[id.y * extent.x + id.x] = textureLoad(scene, vec2<i32>(id.xy), 0);
                    }
                }
            "
                .into(),
            ),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("submitted depth observer"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("read_depth"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        self.prepared = Some((pipeline, group));
        *self.output.0.lock().expect("depth tap") = Some(buffer);
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn encode_after_opaque(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (pipeline, group) = self.prepared.as_ref().expect("depth observer");
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, group, &[]);
        pass.dispatch_workgroups(self.extent[0].div_ceil(8), self.extent[1].div_ceil(8), 1);
    }
}

pub(crate) fn matrix(camera: thrum_app::sky::Camera, extent: [u32; 2]) -> glam::Mat4 {
    let view = thrum_render::synthetic::SyntheticView::perspective(
        camera.position,
        camera.target,
        [0.0, 1.0, 0.0],
        camera.vertical_fov_degrees.to_radians(),
        extent[0] as f32 / extent[1] as f32,
        0.1,
        30_000.0,
    )
    .expect("depth camera");
    glam::Mat4::from_cols_array_2d(&view.view_proj_columns())
}
