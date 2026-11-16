//! Public-seam fixture: the contribution asks during upload and samples its
//! own colour, depth and record in a declared late compute pass.
#![allow(
    dead_code,
    reason = "shared fixture targets exercise different subsets of these helpers"
)]
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
        EffectCadence, EffectGraph, SceneContribution, SceneSurface, SecondaryView,
        SecondaryViewRequest, SecondaryViewSlot,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView},
};

pub(crate) const SIDE: u32 = 64;
pub(crate) const RECORD_BYTES: usize = 320;
pub(crate) const OUTPUT_BYTES: u64 = RECORD_BYTES as u64 + SIDE as u64 * SIDE as u64 * 32;

pub(crate) struct World;
fn tick() -> SourceTick {
    SourceTick {
        epoch: 1,
        tick: 0,
        timeline: 1,
    }
}
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        tick()
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, String> {
        Ok(WorldAdvance {
            tick: 0,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, String> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: 0,
            },
            timeline: 1,
            page: tick(),
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
}

pub(crate) struct Source {
    pub(crate) view: SyntheticView,
    pub(crate) lighting: FrameLighting,
    pub(crate) instances: Vec<([f64; 3], u32)>,
    pub(crate) lights: Vec<thrum_render::synthetic::SyntheticPointLight>,
}
impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            for (slot, (position, material)) in (0_u32..).zip(&self.instances) {
                sink.push(
                    slot,
                    u64::from(slot) + 1,
                    u128::from(slot) + 1,
                    *position,
                    u64::from(*material),
                );
            }
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        override_light: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: self.view,
            lighting: override_light.unwrap_or(self.lighting),
        })
    }
    fn sample_local_lights(
        &self,
        _: SourceTick,
        _: f32,
        lights: &mut [thrum_render::synthetic::SyntheticPointLight],
    ) {
        lights.copy_from_slice(&self.lights);
    }
}
pub(crate) type Host = FrameHost<World, Source>;
pub(crate) fn camera(x: f32, z: f32) -> SyntheticView {
    SyntheticView::perspective(
        [x, 0.0, z],
        [x, 0.0, z + 1.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        5000.0,
    )
    .expect("camera")
}
pub(crate) async fn host(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    mut scene: SyntheticScene,
) -> Host {
    scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
    scene.materials[0].emissive_cutoff = [1024.0, 512.0, 256.0, 0.0];
    scene.instances[0].current[3] = [0.0, 0.0, 0.5, 1.0];
    scene.instances[0].previous = scene.instances[0].current;
    host_with_lighting(
        capabilities,
        queue,
        scene,
        FrameLighting::Diagnostic(SyntheticLight::default()),
    )
    .await
}

pub(crate) async fn host_with_lighting(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: SyntheticScene,
    lighting: FrameLighting,
) -> Host {
    host_with_extent(capabilities, queue, scene, lighting, [SIDE, SIDE]).await
}
pub(crate) async fn host_with_extent(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: SyntheticScene,
    lighting: FrameLighting,
    extent: [u32; 2],
) -> Host {
    let source = Source {
        view: scene.view,
        lighting,
        lights: scene.local_lights.clone(),
        instances: scene
            .instances
            .iter()
            .map(|instance| {
                (
                    std::array::from_fn(|axis| f64::from(instance.current[3][axis])),
                    instance.ids[1],
                )
            })
            .collect(),
    };
    FrameHost::new(
        capabilities,
        queue,
        FrameConfig {
            render_extent: extent,
            surface: None,
        },
        World,
        source,
        scene,
        1.0,
    )
    .await
    .expect("public host")
}

#[derive(Default)]
pub(crate) struct Control {
    pub(crate) requests: Vec<Option<SecondaryViewRequest>>,
    pub(crate) order: Vec<usize>,
    pub(crate) outputs: Vec<wgpu::Buffer>,
    pub(crate) handles: Vec<SecondaryView>,
}
pub(crate) struct Observer {
    pub(crate) control: Arc<Mutex<Control>>,
    pub(crate) slots: usize,
    prepared: Vec<(wgpu::ComputePipeline, wgpu::BindGroup)>,
}
impl Observer {
    pub(crate) fn new(slots: usize) -> (Self, Arc<Mutex<Control>>) {
        let control = Arc::new(Mutex::new(Control {
            requests: vec![None; slots],
            order: (0..slots).collect(),
            ..Control::default()
        }));
        (
            Self {
                control: Arc::clone(&control),
                slots,
                prepared: Vec::new(),
            },
            control,
        )
    }
}
fn late(index: usize) -> AfterOpaquePass {
    AfterOpaquePass {
        name: format!("view observer {index}"),
        kind: AfterOpaqueKind::Compute,
        cadence: EffectCadence::Skippable,
        accesses: vec![EffectAccess {
            resource: index,
            range: 0..OUTPUT_BYTES,
            kind: AccessKind::StorageWrite,
            view: None,
            history: None,
        }],
        depth: false,
        refraction: false,
    }
}
impl SceneContribution for Observer {
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        vec![SecondaryViewSlot::new(1.0).expect("declaration"); self.slots]
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            resources: (0..self.slots)
                .map(|index| {
                    (
                        ResourceDescriptor::Buffer {
                            bytes: OUTPUT_BYTES,
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        },
                        ResourceScope::Persistent {
                            owner: 500 + index as u64,
                            epoch: 1,
                            initialized: false,
                        },
                    )
                })
                .collect(),
            after_opaque: (0..self.slots).map(late).collect(),
            ..EffectGraph::default()
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        assert_eq!(
            surface.secondary_views.len(),
            self.slots,
            "only owned slots are lent"
        );
        let device = surface.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view observer"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(RECORD_BYTES as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(OUTPUT_BYTES),
                    },
                    count: None,
                },
            ],
        });
        let input = late(0).input_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("view observer"),
            bind_group_layouts: &[Some(surface.view_layout), Some(&input), Some(&layout)],
            immediate_size: 0,
        });
        let source = format!(
            "{}\n{}",
            SecondaryView::record_wgsl(),
            include_str!("support/scene_view/observer.wgsl")
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("view observer"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("view observer"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("observe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let mut control = self.control.lock().expect("control");
        control.handles = surface.secondary_views.to_vec();
        control.outputs.clear();
        self.prepared.clear();
        for slot in surface.secondary_views {
            let output = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("view observer output"),
                size: OUTPUT_BYTES,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("view observer"),
                layout: &layout,
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
                        resource: slot.record().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output.as_entire_binding(),
                    },
                ],
            });
            self.prepared.push((pipeline.clone(), group));
            control.outputs.push(output);
        }
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        let control = self.control.lock().expect("control");
        for &index in &control.order {
            if let Some(request) = control.requests[index] {
                control.handles[index]
                    .request(&request)
                    .expect("public ask");
            }
        }
    }
    fn after_opaque_runs(&self, index: usize, _: &ContributionFrame) -> bool {
        self.control.lock().expect("control").requests[index].is_some()
    }
    fn encode_after_opaque(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (pipeline, group) = &self.prepared[index];
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, group, &[]);
        pass.dispatch_workgroups(SIDE / 8, SIDE / 8, 1);
    }
    fn computes(&self, _: &ContributionFrame) -> bool {
        false
    }
    fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {}
    fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
    fn draw(&self, _: &mut wgpu::RenderPass<'_>) {}
    fn submitted(&mut self, _: bool) {}
}

pub(crate) fn output(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer) -> Vec<u8> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("view observer readback"),
        size: OUTPUT_BYTES,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, OUTPUT_BYTES);
    encoder.map_buffer_on_submit(&staging, wgpu::MapMode::Read, .., |result| {
        result.expect("observer mapped");
    });
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("observer completion");
    staging
        .slice(..)
        .get_mapped_range()
        .expect("observer bytes")
        .to_vec()
}
pub(crate) fn pixel(bytes: &[u8], index: usize) -> ([f32; 4], f32) {
    let start = RECORD_BYTES + index * 32;
    let channel = |offset| {
        f32::from_le_bytes(
            bytes[start + offset..start + offset + 4]
                .try_into()
                .expect("channel"),
        )
    };
    (std::array::from_fn(|i| channel(i * 4)), channel(16))
}

/// Store raw aerial texels in the scene/view colour attachments, then use
/// declared late compute reads to copy them. The ordinary main volume is
/// observed through refraction; a slot is observed through its owner's handle.
pub(crate) const AIR_TEXELS: u64 = 32 * 32 * 33 * 2;
pub(crate) const AIR_BYTES: u64 = AIR_TEXELS * 16;
pub(crate) const AIR_EXTENT: [u32; 2] = [256, 272];
pub(crate) struct AirCapture {
    pub(crate) outputs: Vec<wgpu::Buffer>,
    pub(crate) request: Option<SecondaryViewRequest>,
}
pub(crate) struct AirProbe {
    control: Arc<Mutex<AirCapture>>,
    declared: bool,
    render: Vec<wgpu::RenderPipeline>,
    copy: Vec<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    slot: Option<SecondaryView>,
}
fn air_late(index: usize) -> AfterOpaquePass {
    AfterOpaquePass {
        name: format!("aerial stored texels {index}"),
        kind: AfterOpaqueKind::Compute,
        cadence: EffectCadence::Skippable,
        depth: false,
        refraction: true,
        accesses: vec![EffectAccess {
            resource: index,
            range: 0..AIR_BYTES,
            kind: AccessKind::StorageWrite,
            view: None,
            history: None,
        }],
    }
}
impl AirProbe {
    pub(crate) fn new(request: Option<&SecondaryViewRequest>) -> (Self, Arc<Mutex<AirCapture>>) {
        let declared = request.is_some();
        let control = Arc::new(Mutex::new(AirCapture {
            outputs: Vec::new(),
            request: request.copied(),
        }));
        (
            Self {
                control: Arc::clone(&control),
                declared,
                render: Vec::new(),
                copy: Vec::new(),
                slot: None,
            },
            control,
        )
    }
}
impl SceneContribution for AirProbe {
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        if self.declared {
            vec![SecondaryViewSlot::new(1.0).expect("air slot")]
        } else {
            Vec::new()
        }
    }
    fn secondary_views_enabled(&self) -> bool {
        true
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: self.declared,
            compute: false,
        }
    }
    fn effect_graph(&self) -> EffectGraph {
        let count = 1 + usize::from(self.declared);
        EffectGraph {
            resources: (0..count)
                .map(|index| {
                    (
                        ResourceDescriptor::Buffer {
                            bytes: AIR_BYTES,
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        },
                        ResourceScope::Persistent {
                            owner: 800 + index as u64,
                            epoch: 1,
                            initialized: false,
                        },
                    )
                })
                .collect(),
            after_opaque: (0..count).map(air_late).collect(),
            ..EffectGraph::default()
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let device = surface.device;
        self.slot = surface.secondary_views.first().cloned();
        self.render.clear();
        self.copy.clear();
        let mut control = self.control.lock().expect("air control");
        control.outputs.clear();
        let descriptions = std::iter::once((
            surface.shader_source,
            surface.view_layout,
            surface.light_layout,
            surface.color_formats.to_vec(),
            surface.depth_format,
            "probe_main",
        ))
        .chain(surface.secondary_surfaces.iter().map(|view| {
            (
                view.shader_source,
                view.view_layout,
                view.light_layout,
                vec![view.colour_format],
                view.depth_format,
                "probe_view",
            )
        }));
        for (index, (shader, view, light, formats, depth, entry)) in descriptions.enumerate() {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("aerial texel cards"),
                bind_group_layouts: &[Some(view), Some(light)],
                immediate_size: 0,
            });
            let source = format!(
                "{shader}\n{}",
                include_str!("support/scene_view/air_probe.wgsl")
            );
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("aerial texel cards"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let targets = formats
                .into_iter()
                .map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                })
                .collect::<Vec<_>>();
            self.render
                .push(device.render(&wgpu::RenderPipelineDescriptor {
                    label: Some("aerial texel cards"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("probe_vertex"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: depth,
                        depth_write_enabled: Some(true),
                        depth_compare: Some(wgpu::CompareFunction::Always),
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some(entry),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &targets,
                    }),
                    multiview_mask: None,
                    cache: None,
                }));
            let mut entries = vec![wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(AIR_BYTES),
                },
                count: None,
            }];
            if index > 0 {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                });
            }
            let output_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("aerial texel readback"),
                entries: &entries,
            });
            let input = air_late(index).input_layout(device);
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("aerial texel readback"),
                bind_group_layouts: &[
                    Some(surface.view_layout),
                    Some(&input),
                    Some(&output_layout),
                ],
                immediate_size: 0,
            });
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("aerial texel readback"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("support/scene_view/air_copy.wgsl").into(),
                ),
            });
            let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("aerial texel readback"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(if index == 0 { "copy_main" } else { "copy_view" }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
            let output = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("aerial stored texels"),
                size: AIR_BYTES,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let mut entries = vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }];
            if index > 0 {
                entries.push(wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        self.slot.as_ref().expect("owned air slot").colour(),
                    ),
                });
            }
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("aerial texel readback"),
                layout: &output_layout,
                entries: &entries,
            });
            self.copy.push((pipeline, group));
            control.outputs.push(output);
        }
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        let control = self.control.lock().expect("air control");
        if let Some(request) = control.request {
            self.slot
                .as_ref()
                .expect("air slot")
                .request(&request)
                .expect("air ask");
        }
    }
    fn computes(&self, _: &ContributionFrame) -> bool {
        false
    }
    fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {}
    fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.render[0]);
        pass.draw(0..3, 0..1);
    }
    fn draw_secondary(&self, index: usize, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.render[index + 1]);
        pass.draw(0..3, 0..1);
    }
    fn after_opaque_runs(&self, index: usize, _: &ContributionFrame) -> bool {
        index == 0 || self.control.lock().expect("air control").request.is_some()
    }
    fn encode_after_opaque(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (pipeline, group) = &self.copy[index];
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, group, &[]);
        pass.dispatch_workgroups(AIR_EXTENT[0] / 8, AIR_EXTENT[1] / 8, 1);
    }
    fn submitted(&mut self, _: bool) {}
}

pub(crate) fn air_output(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Vec<f32> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("aerial observation readback"),
        size: AIR_BYTES,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, AIR_BYTES);
    encoder.map_buffer_on_submit(&staging, wgpu::MapMode::Read, .., |result| {
        result.expect("air observation mapped");
    });
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("air observation completion");
    staging
        .slice(..)
        .get_mapped_range()
        .expect("air observation bytes")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect()
}
