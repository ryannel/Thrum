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
    let source = Source {
        view: scene.view,
        lighting,
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
            render_extent: [SIDE, SIDE],
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
