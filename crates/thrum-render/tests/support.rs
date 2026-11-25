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
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
        EffectCadence, EffectGraph, SceneContribution, SceneSurface, SecondaryView,
        SecondaryViewRequest, SecondaryViewSlot,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::{FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{
        FrameLighting, SyntheticLight, SyntheticPointLight, SyntheticScene, SyntheticView,
    },
};
// Ordinary main-view colour and depth through the public late-effect inputs.

pub(crate) struct MainObserver {
    extent: [u32; 2],
    control: Arc<Mutex<Control>>,
    prepared: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
}
impl MainObserver {
    pub(crate) fn new(extent: [u32; 2]) -> (Self, Arc<Mutex<Control>>) {
        let control = Arc::new(Mutex::new(Control::default()));
        (
            Self {
                extent,
                control: Arc::clone(&control),
                prepared: None,
            },
            control,
        )
    }
    fn pass(&self) -> AfterOpaquePass {
        let mut pass = late(
            0,
            RECORD_BYTES as u64 + u64::from(self.extent[0]) * u64::from(self.extent[1]) * 32,
        );
        pass.name = "ordinary main observer".into();
        pass.depth = true;
        pass.refraction = true;
        pass
    }
}
impl SceneContribution for MainObserver {
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
                    bytes: RECORD_BYTES as u64
                        + u64::from(self.extent[0]) * u64::from(self.extent[1]) * 32,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                },
                ResourceScope::Persistent {
                    owner: 900,
                    epoch: 1,
                    initialized: false,
                },
            )],
            after_opaque: vec![self.pass()],
            ..EffectGraph::default()
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let device = surface.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ordinary main observer"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(RECORD_BYTES as u64 + 32),
                },
                count: None,
            }],
        });
        let input = self.pass().input_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ordinary main observer"),
            bind_group_layouts: &[Some(surface.view_layout), Some(&input), Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ordinary main observer"),
            source: wgpu::ShaderSource::Wgsl(include_str!("support/scene_view/main.wgsl").into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("ordinary main observer"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("observe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ordinary main observation"),
            size: RECORD_BYTES as u64 + u64::from(self.extent[0]) * u64::from(self.extent[1]) * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ordinary main observer"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        self.control.lock().expect("main control").outputs = vec![buffer];
        self.prepared = Some((pipeline, group));
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
    fn encode_after_opaque(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (pipeline, group) = self.prepared.as_ref().expect("main observer prepared");
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, group, &[]);
        pass.dispatch_workgroups(self.extent[0].div_ceil(8), self.extent[1].div_ceil(8), 1);
    }
}

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
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
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
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
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
    fn display_phase_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

pub(crate) struct Source {
    pub(crate) view: SyntheticView,
    pub(crate) lighting: FrameLighting,
    pub(crate) instances: Vec<([f64; 3], u32)>,
    pub(crate) lights: Vec<SyntheticPointLight>,
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
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.view,
            lighting: override_light.unwrap_or(self.lighting),
        })
    }
    fn sample_local_lights(&self, _: SourceTick, _: f32, lights: &mut [SyntheticPointLight]) {
        lights.copy_from_slice(&self.lights);
    }
}
pub(crate) type Host = FrameHost<World, Source>;

pub(crate) fn selected_capabilities(capabilities: &DeviceCapabilities) -> DeviceCapabilities {
    let recipe = std::env::var("THRUM_VIEW_RECIPE").unwrap_or_else(|_| "portable".into());
    capabilities
        .clone()
        .with_recipe_override(&recipe)
        .expect("requested view test recipe must be admitted")
}

pub(crate) fn lit_view_scene() -> (SyntheticScene, FrameLighting, SyntheticView) {
    let mut scene = SyntheticScene::grid(5).expect("scene");
    scene.view = camera(100.0, 997.5);
    scene.sun_shadows = true;
    scene.camera_culling = thrum_render::synthetic::CullMode::Frustum;
    let toward = glam::Vec3::new(0.0, 1.0, 1.0).normalize();
    let eye = glam::Vec3::new(100.0, 48.0, 997.5);
    let far = eye + toward * 3000.0;
    for (instance, center) in scene.instances.iter_mut().zip([
        [0.0, 0.0, 0.5],
        [100.0, 50.5, 1000.0],
        [0.35, 0.35, 0.45],
        [100.35, 50.85, 999.95],
        far.to_array(),
    ]) {
        instance.current[3] = [center[0], center[1], center[2], 1.0];
        instance.previous = instance.current;
    }
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        let half = match index {
            0 | 1 => [0.9, 0.9, 0.02],
            2 | 3 => [0.2; 3],
            _ => [500.0; 3],
        };
        for (axis, extent) in half.into_iter().enumerate() {
            for row in 0..3 {
                instance.current[axis][row] = if axis == row { extent } else { 0.0 };
                instance.normal[axis][row] = if axis == row { extent.recip() } else { 0.0 };
            }
        }
        instance.previous = instance.current;
    }
    scene.local_lights = vec![SyntheticPointLight {
        position: [100.0, 50.5, 999.5],
        radius: 1.0,
        candela: [10_000.0, 1000.0, 100.0],
    }];
    scene.materials.push(thrum_render::abi::MaterialRecord {
        base_rgba: [0.0, 0.0, 0.0, 1.0],
        emissive_cutoff: [1024.0, 256.0, 64.0, 0.0],
        ..thrum_render::abi::MaterialRecord::default()
    });
    let mut lamp = thrum_render::abi::InstanceRecord::identity(5, 2);
    for axis in 0..3 {
        lamp.current[axis][axis] = 0.03;
        lamp.normal[axis][axis] = 0.03_f32.recip();
        lamp.current[3][axis] = scene.local_lights[0].position[axis];
    }
    lamp.previous = lamp.current;
    scene.instances.push(lamp);
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.7, 0.7, -0.1], 100_000.0, [1.0; 3], 0.0, 1).expect("air"),
    );
    let camera = SyntheticView::perspective(
        eye.to_array(),
        (eye + toward).to_array(),
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        5000.0,
    )
    .expect("second camera");
    (scene, lighting, camera)
}
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
pub(crate) fn aimed(eye: [f32; 3], direction: [f32; 3]) -> SyntheticView {
    SyntheticView::perspective(
        eye,
        (glam::Vec3::from_array(eye) + glam::Vec3::from_array(direction)).to_array(),
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        10000.0,
    )
    .expect("aimed camera")
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
    host_with_config(
        capabilities,
        queue,
        scene,
        lighting,
        FrameConfig {
            render_extent: extent,
            surface: None,
        },
    )
    .await
}
pub(crate) fn drawable(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    [width, height]: [u32; 2],
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("test drawable"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(crate) async fn host_with_config(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: SyntheticScene,
    lighting: FrameLighting,
    config: FrameConfig,
) -> Host {
    host_with_config_exact(
        &selected_capabilities(capabilities),
        queue,
        scene,
        lighting,
        config,
    )
    .await
}

pub(crate) async fn host_with_config_exact(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: SyntheticScene,
    lighting: FrameLighting,
    config: FrameConfig,
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
                    instance.material(),
                )
            })
            .collect(),
    };
    Box::pin(FrameHost::new(
        capabilities,
        queue,
        config,
        World,
        source,
        scene,
        1.0,
    ))
    .await
    .expect("public host")
}

#[derive(Default)]
pub(crate) struct Control {
    pub(crate) requests: Vec<Option<SecondaryViewRequest>>,
    pub(crate) order: Vec<usize>,
    pub(crate) outputs: Vec<wgpu::Buffer>,
    pub(crate) handles: Vec<SecondaryView>,
    pub(crate) main_shader: String,
    pub(crate) main_frame: Option<ContributionFrame>,
    pub(crate) allow_refusal: bool,
    pub(crate) results: Vec<Result<(), thrum_render::contribution::SecondaryViewError>>,
    admitted: Vec<bool>,
}
pub(crate) struct Observer {
    pub(crate) control: Arc<Mutex<Control>>,
    pub(crate) slots: usize,
    pipeline: Option<(wgpu::Device, wgpu::BindGroupLayout, wgpu::ComputePipeline)>,
    prepared: Vec<wgpu::BindGroup>,
}
impl Observer {
    pub(crate) fn new(slots: usize) -> (Self, Arc<Mutex<Control>>) {
        let control = Arc::new(Mutex::new(Control {
            requests: vec![None; slots],
            order: (0..slots).collect(),
            admitted: vec![false; slots],
            ..Control::default()
        }));
        (
            Self {
                control: Arc::clone(&control),
                slots,
                pipeline: None,
                prepared: Vec::new(),
            },
            control,
        )
    }

    fn bindings(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        slots: &[SecondaryView],
    ) -> (Vec<wgpu::Buffer>, Vec<wgpu::BindGroup>) {
        slots
            .iter()
            .map(|slot| {
                let extent = slot.maximum_extent();
                let bytes = RECORD_BYTES as u64 + u64::from(extent[0]) * u64::from(extent[1]) * 32;
                let output = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("view observer output"),
                    size: bytes,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("view observer"),
                    layout,
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
                (output, group)
            })
            .unzip()
    }

    fn install_bindings(
        &mut self,
        handles: Vec<SecondaryView>,
        outputs: Vec<wgpu::Buffer>,
        groups: Vec<wgpu::BindGroup>,
    ) {
        let mut control = self.control.lock().expect("control");
        control.handles = handles;
        control.outputs = outputs;
        self.prepared = groups;
    }
}
fn late(index: usize, bytes: u64) -> AfterOpaquePass {
    AfterOpaquePass {
        name: format!("view observer {index}"),
        kind: AfterOpaqueKind::Compute,
        cadence: EffectCadence::Skippable,
        accesses: vec![EffectAccess {
            resource: index,
            range: 0..bytes,
            kind: AccessKind::StorageWrite,
            view: None,
            history: None,
        }],
        depth: false,
        write_depth: false,
        refraction: false,
    }
}
impl SceneContribution for Observer {
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        vec![SecondaryViewSlot::new(1.0).expect("declaration"); self.slots]
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: self.slots != 0,
            compute: false,
        }
    }
    fn effect_graph(&self) -> EffectGraph {
        observer_graph(&self.control.lock().expect("control").outputs)
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn secondary_resize_job(
        &self,
    ) -> Result<Option<thrum_render::contribution::SecondaryResizeJob>, FrameError> {
        let Some((device, layout, _)) = self.pipeline.clone() else {
            return Ok(None);
        };
        Ok(Some(Box::new(move |slots| {
            let (outputs, groups) = Self::bindings(&device, &layout, slots);
            let slots = slots.to_vec();
            Ok(thrum_render::contribution::SecondaryResize::new(
                observer_graph(&outputs),
                move |owner: &mut Self| owner.install_bindings(slots, outputs, groups),
            ))
        })))
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.prepare_observer(surface);
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        let mut control = self.control.lock().expect("control");
        control.main_frame = Some(*frame);
        control.results.clear();
        control.admitted.fill(false);
        for index in control.order.clone() {
            if let Some(request) = control.requests[index] {
                let result = control.handles[index].request(&request);
                if !control.allow_refusal {
                    result.expect("public ask");
                }
                control.admitted[index] = result.is_ok();
                control.results.push(result);
            }
        }
    }
    fn after_opaque_runs(&self, index: usize, _: &ContributionFrame) -> bool {
        self.control.lock().expect("control").admitted[index]
    }
    fn encode_after_opaque(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (_, _, pipeline) = self.pipeline.as_ref().expect("observer pipeline");
        let extent = self.control.lock().expect("control").handles[index].maximum_extent();
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, &self.prepared[index], &[]);
        pass.dispatch_workgroups(extent[0].div_ceil(8), extent[1].div_ceil(8), 1);
    }
}

fn observer_graph(outputs: &[wgpu::Buffer]) -> EffectGraph {
    EffectGraph {
        resources: outputs
            .iter()
            .enumerate()
            .map(|(index, output)| {
                (
                    ResourceDescriptor::Buffer {
                        bytes: output.size(),
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
        after_opaque: outputs
            .iter()
            .enumerate()
            .map(|(index, output)| late(index, output.size()))
            .collect(),
        ..EffectGraph::default()
    }
}

impl Observer {
    fn prepare_observer(&mut self, surface: &SceneSurface<'_>) {
        assert!(
            surface.secondary_surfaces.is_empty(),
            "an observer has not opted its geometry into views"
        );
        surface
            .shader_source
            .clone_into(&mut self.control.lock().expect("control").main_shader);
        assert_eq!(
            surface.secondary_views.len(),
            self.slots,
            "only owned slots are lent"
        );
        if self.slots == 0 {
            return;
        }
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
                        min_binding_size: wgpu::BufferSize::new(RECORD_BYTES as u64 + 32),
                    },
                    count: None,
                },
            ],
        });
        let input = late(0, 1).input_layout(device);
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
        let (outputs, groups) = Self::bindings(device, &layout, surface.secondary_views);
        self.pipeline = Some(((**device).clone(), layout, pipeline));
        self.install_bindings(surface.secondary_views.to_vec(), outputs, groups);
    }
}

pub(crate) fn output(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer) -> Vec<u8> {
    thrum_test_support::readback::buffer(device, queue, buffer, ..)
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
        write_depth: false,
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
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
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
}

pub(crate) fn air_output(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Vec<f32> {
    thrum_test_support::readback::buffer(device, queue, buffer, 0..AIR_BYTES)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect()
}
