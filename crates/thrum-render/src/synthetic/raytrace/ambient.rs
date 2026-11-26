//! Radius-bounded ambient visibility published after submission.
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use super::{
    super::{
        OffscreenTarget, PreparedSynthetic, SceneContext, SyntheticTargetInputs, SyntheticView,
    },
    soft::{SoftTarget, descriptor, history_views, sampled, view_entry, write},
};
use crate::{
    binding,
    contribution::{QualityError, QualityInvalidation, QualityKnob, QualitySettings},
    error::RenderError,
    graph::{
        Access, AccessKind, CompiledPass, FrameGraph, PassBody, PassScope, PipelineKey,
        ResourceDescriptor, ResourceId, ResourceScope,
    },
    pass_key::{PassKey, PipelineRole},
    records::AmbientFrame,
    runtime::DeviceSite,
    shaders::wgsl,
};

const TILE: u32 = 8;
const SAMPLES: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Uint;
pub(in crate::synthetic) const RECEIVER_FORMAT: wgpu::TextureFormat =
    wgpu::TextureFormat::Rgba32Float;
pub(in crate::synthetic) const RECEIVER_ENTRY: &str = "ambient_receiver";
const VALUES: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Float;
const COMPUTE: wgpu::ShaderStages = wgpu::ShaderStages::COMPUTE;
const FRAGMENT: wgpu::ShaderStages = wgpu::ShaderStages::FRAGMENT;
const FLOAT: wgpu::TextureSampleType = wgpu::TextureSampleType::Float { filterable: false };
const UINT: wgpu::TextureSampleType = wgpu::TextureSampleType::Uint;

const FRAME_BYTES: u64 = size_of::<AmbientFrame>() as u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum Stage {
    Trace,
    Filter,
}

impl Stage {
    const ALL: [Self; 2] = [Self::Trace, Self::Filter];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Trace => "traced ambient visibility",
            Self::Filter => "traced ambient visibility filter",
        }
    }

    const fn entry(self) -> &'static str {
        match self {
            Self::Trace => "trace_ambient",
            Self::Filter => "filter_ambient",
        }
    }
}

#[derive(Clone)]
pub(in crate::synthetic) struct Pipelines {
    stages: [wgpu::ComputePipeline; 2],
    inputs: [crate::runtime::PipelineInput<wgpu::BindGroupLayout>; 2],
    structure: wgpu::BindGroupLayout,
    depth: wgpu::BindGroupLayout,
}

struct Sample {
    target: Arc<()>,
    drawn: [u32; 2],
}

pub(in crate::synthetic) struct Ambient {
    pub(in crate::synthetic) pipelines: Pipelines,
    settings: QualitySettings,
    radius: f32,
    submitted: Option<Sample>,
    pending: Option<Sample>,
    phase: u32,
}

#[derive(Clone)]
struct History {
    identity: Arc<()>,
    samples: [wgpu::TextureView; 2],
    filtered: [wgpu::TextureView; 2],
}

type LightGroups = [[wgpu::BindGroup; 2]; 2];

pub(in crate::synthetic) struct AmbientPasses {
    pub trace: Option<usize>,
    pub filter: usize,
    pub combined_accesses: Vec<Access>,
}

pub(in crate::synthetic) struct Target {
    history: History,
    uniform: wgpu::Buffer,
    depth: wgpu::BindGroup,
    groups: [[wgpu::BindGroup; 2]; 2],
    lights: Mutex<Option<(wgpu::Tlas, LightGroups)>>,
}

pub(super) fn trace_source() -> &'static str {
    concat!(
        wgsl!("synthetic/raytrace/ambient_common.wgsl"),
        wgsl!("synthetic/raytrace/ambient_trace.wgsl"),
    )
}

pub(crate) fn shader_source() -> String {
    format!(
        "enable wgpu_ray_query;\nconst AMBIENT_TILE: u32 = {TILE}u;\n{}\n{}\n{}\n{}",
        wgsl!("synthetic/raytrace/shadows.wgsl"),
        wgsl!("synthetic/raytrace/visibility_receiver.wgsl"),
        trace_source(),
        wgsl!("synthetic/raytrace/ambient_visibility.wgsl"),
    )
}

impl Pipelines {
    fn new(device: &DeviceSite) -> Self {
        let uniform = binding::uniform(0, COMPUTE, FRAME_BYTES);
        let entries = [
            vec![
                uniform,
                sampled(2, COMPUTE, FLOAT),
                sampled(3, COMPUTE, UINT),
                write(4, SAMPLES),
                sampled(5, COMPUTE, FLOAT),
            ],
            vec![uniform, sampled(5, COMPUTE, UINT), write(6, VALUES)],
        ];
        let inputs = Stage::ALL.map(|stage| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(stage.name()),
                entries: &entries[stage as usize],
            })
        });
        let depth = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ambient receiver depth"),
            entries: &[sampled(1, COMPUTE, wgpu::TextureSampleType::Depth)],
        });
        let structure = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("traced ambient visibility structure"),
            entries: &[binding::acceleration_structure(15, COMPUTE)],
        });
        let shader = crate::shaders::Shader::AmbientVisibility.compile(device, Stage::Trace.name());
        let stages = Stage::ALL.map(|stage| {
            let trace: [Option<&wgpu::BindGroupLayout>; 3] = [
                Some(&depth),
                Some(&structure),
                Some(&inputs[Stage::Trace as usize]),
            ];
            let filter: [Option<&wgpu::BindGroupLayout>; 1] =
                [Some(&inputs[Stage::Filter as usize])];
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(stage.name()),
                bind_group_layouts: match stage {
                    Stage::Trace => &trace,
                    Stage::Filter => &filter,
                },
                immediate_size: 0,
            });
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(stage.name()),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(stage.entry()),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        });
        Self {
            stages,
            inputs,
            structure: structure.into_raw(),
            depth: depth.into_raw(),
        }
    }

    pub(super) fn trace_layout(&self) -> &wgpu::BindGroupLayout {
        &self.inputs[Stage::Trace as usize]
    }

    fn bind_inputs(
        &self,
        device: &wgpu::Device,
        uniform: &wgpu::Buffer,
        history: &History,
        motion: &wgpu::TextureView,
        receiver: &wgpu::TextureView,
    ) -> [[wgpu::BindGroup; 2]; 2] {
        Stage::ALL.map(|stage| {
            std::array::from_fn(|read| {
                let uniform = wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                };
                let write = 1 - read;
                let trace = [
                    uniform.clone(),
                    view_entry(2, motion),
                    view_entry(3, &history.samples[read]),
                    view_entry(4, &history.samples[write]),
                    view_entry(5, receiver),
                ];
                let filter = [
                    uniform,
                    view_entry(5, &history.samples[write]),
                    view_entry(6, &history.filtered[write]),
                ];
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(stage.name()),
                    layout: &self.inputs[stage as usize],
                    entries: match stage {
                        Stage::Trace => &trace,
                        Stage::Filter => &filter,
                    },
                })
            })
        })
    }

    pub(in crate::synthetic) fn bind_structure(
        &self,
        device: &wgpu::Device,
        tlas: &wgpu::Tlas,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("traced ambient visibility structure"),
            layout: &self.structure,
            entries: &[wgpu::BindGroupEntry {
                binding: 15,
                resource: tlas.as_binding(),
            }],
        })
    }
}

impl Ambient {
    const EXTENT: &str = "Ambient occlusion extent";
    const SETTLE: &str = "Ambient occlusion settle length";

    pub(in crate::synthetic) fn new(device: &DeviceSite, radius: f32) -> Self {
        let knob = |name: &str, effect, value, range| QualityKnob {
            name: name.into(),
            effect,
            value,
            range,
            invalidation: QualityInvalidation::EffectHistory,
            governor_steps: None,
        };
        let settings = QualitySettings::new(vec![
            knob(
                Self::EXTENT,
                "Trace no ambient rays, at half extent, or at full extent",
                1,
                0..=2,
            ),
            knob(
                Self::SETTLE,
                "Lifetime of an ambient ray sample in submitted frames",
                16,
                1..=31,
            ),
        ])
        .expect("valid ambient occlusion controls");
        Self {
            pipelines: Pipelines::new(device),
            settings,
            radius,
            submitted: None,
            pending: None,
            phase: 0,
        }
    }

    pub(super) fn stride(&self) -> Option<u32> {
        match self.settings.knobs()[0].value {
            0 => None,
            1 => Some(2),
            _ => Some(1),
        }
    }

    pub(in crate::synthetic) fn light_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
        [
            binding::uniform(24, FRAGMENT, FRAME_BYTES),
            sampled(25, FRAGMENT, FLOAT),
        ]
    }

    pub(in crate::synthetic) fn update(
        &mut self,
        queue: &wgpu::Queue,
        target: &Target,
        (view, previous): (&SyntheticView, &SyntheticView),
        drawn: [u32; 2],
        transform_valid: bool,
    ) {
        let stride = self.stride();
        let valid = stride.is_some()
            && transform_valid
            && self.submitted.as_ref().is_some_and(|old| {
                old.drawn == drawn && Arc::ptr_eq(&old.target, &target.history.identity)
            });
        let jitter = |view: &SyntheticView| {
            if view.raster_jitter_enabled() {
                view.raster_jitter_pixels()
            } else {
                [0.0; 2]
            }
        };
        let [jx, jy] = jitter(view);
        let [px, py] = jitter(previous);
        let camera = glam::Vec3::from_array(view.camera_position());
        let relative = (view.raster_projection() * glam::Mat4::from_translation(camera)).inverse();
        let frame = AmbientFrame {
            inverse: relative.to_cols_array_2d(),
            previous: previous.view_proj_columns(),
            camera: camera.extend(0.0).to_array(),
            extent: drawn,
            jitter_delta: [px - jx, py - jy],
            stride: stride.unwrap_or(0),
            valid: u32::from(valid),
            phase: self.phase,
            settle: self.settings.knobs()[1].value,
            radius: self.radius,
            padding0: 0,
            padding1: 0,
            padding2: 0,
        };
        queue.write_buffer(&target.uniform, 0, bytemuck::bytes_of(&frame));
        self.pending = stride.map(|_| Sample {
            target: Arc::clone(&target.history.identity),
            drawn,
        });
    }

    pub(in crate::synthetic) fn submitted(&mut self) {
        self.submitted = self.pending.take();
        self.phase = self.phase.wrapping_add(1);
    }
}

impl Target {
    pub(super) fn trace_group(&self, read: usize) -> &wgpu::BindGroup {
        &self.groups[Stage::Trace as usize][read]
    }

    pub(in crate::synthetic) fn light_bindings(
        &self,
        side: usize,
    ) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: 24,
                resource: self.uniform.as_entire_binding(),
            },
            view_entry(25, &self.history.filtered[side]),
        ]
    }

    pub(in crate::synthetic) fn light_group(
        &self,
        tlas: &wgpu::Tlas,
        scene: &PreparedSynthetic,
        soft: Option<&SoftTarget>,
        history: usize,
        exposure: usize,
    ) -> Result<wgpu::BindGroup, RenderError> {
        let mut groups = self
            .lights
            .lock()
            .map_err(|_| RenderError::GraphPassMismatch)?;
        if groups.as_ref().is_none_or(|(old, _)| old != tlas) {
            *groups = Some((
                tlas.clone(),
                std::array::from_fn(|side| {
                    scene
                        .create_ray_light_groups(
                            tlas,
                            soft.map(|soft| (soft, side)),
                            Some((self, side)),
                        )
                        .expect("prepared ambient lights")
                }),
            ));
        }
        Ok(groups.as_ref().expect("ambient light cache").1[history][exposure].clone())
    }
}

impl SyntheticTargetInputs {
    pub(in crate::synthetic) fn prepare_ambient_target(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        motion: &wgpu::TextureView,
        receiver: Option<&wgpu::TextureView>,
        extent: [u32; 2],
    ) -> Option<Target> {
        let pipelines = self.ambient.as_ref()?;
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("traced ambient visibility frame"),
            size: FRAME_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let history = History {
            identity: Arc::new(()),
            samples: history_views(device, "submitted ambient ray samples", extent, SAMPLES),
            filtered: history_views(device, "submitted ambient visibility", extent, VALUES),
        };
        let groups = pipelines.bind_inputs(device, &uniform, &history, motion, receiver?);
        let depth = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ambient receiver depth"),
            layout: &pipelines.depth,
            entries: &[view_entry(1, depth)],
        });
        Some(Target {
            history,
            uniform,
            depth,
            groups,
            lights: Mutex::new(None),
        })
    }

    pub(in crate::synthetic) fn append_ambient_visibility(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward: &mut Vec<Access>,
        extent: [u32; 2],
        [depth, motion, receiver]: [&ResourceId; 3],
        structure: Option<&ResourceId>,
    ) -> Result<AmbientPasses, RenderError> {
        let structure = structure.ok_or(RenderError::GraphPassMismatch)?;
        let history = |graph: &mut FrameGraph, owner, format| {
            graph.resource(
                descriptor(extent, format),
                ResourceScope::History {
                    owner,
                    epoch: self.key.device_generation,
                    initialized: true,
                },
            )
        };
        let previous_samples = history(graph, 0x414f, SAMPLES)?;
        let samples = history(graph, 0x414f, SAMPLES)?;
        let previous_values = history(graph, 0x4150, VALUES)?;
        let values = history(graph, 0x4150, VALUES)?;
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: FRAME_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let params = Access::new(uniform, 0..FRAME_BYTES, AccessKind::UniformRead);
        let read =
            |resource: &ResourceId| Access::new(resource.clone(), 0..1, AccessKind::SampledRead);
        let write =
            |resource: &ResourceId| Access::new(resource.clone(), 0..1, AccessKind::StorageWrite);
        forward.extend([params.clone(), read(&previous_values)]);
        let trace_accesses = vec![
            params.clone(),
            read(motion),
            read(receiver),
            read(&previous_samples),
            write(&samples),
        ];
        let mut declare = |stage, accesses| {
            let key = PipelineRole::Ambient(stage as u32).key(self.key);
            keys.insert(key);
            graph.declare(
                PassKey::Ambient(stage),
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: Some(key),
                    accesses,
                    external_side_effect: true,
                },
            )
        };
        let (trace, combined_accesses) = if self.soft_sun.is_some() {
            (None, trace_accesses)
        } else {
            let mut accesses = trace_accesses;
            accesses.extend([
                read(depth),
                Access::new(structure.clone(), 0..1, AccessKind::RayQuery),
            ]);
            (Some(declare(Stage::Trace, accesses)), Vec::new())
        };
        let filter = declare(Stage::Filter, vec![params, read(&samples), write(&values)]);
        Ok(AmbientPasses {
            trace,
            filter,
            combined_accesses,
        })
    }
}

impl OffscreenTarget {
    pub(in crate::synthetic) fn share_ambient_history(
        &mut self,
        prepared: &SyntheticTargetInputs,
        first: &Self,
    ) {
        if let Some((((target, first), pipelines), receiver)) = self
            .ambient
            .as_mut()
            .zip(first.ambient.as_ref())
            .zip(prepared.ambient.as_ref())
            .zip(self.ambient_receiver_view.as_ref())
        {
            target.history.clone_from(&first.history);
            target.groups = pipelines.bind_inputs(
                &self.device,
                &target.uniform,
                &target.history,
                &self.motion_view,
                receiver,
            );
        }
    }
}

impl PreparedSynthetic {
    /// Raw per-receiver ambient ray history from the last submitted frame.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn ambient_ray_samples_for_probe<'a>(
        &self,
        target: &'a OffscreenTarget,
    ) -> Option<&'a wgpu::Texture> {
        if self.check_owner(target).is_err() {
            return None;
        }
        Some(target.ambient.as_ref()?.history.samples[1 - self.history.write_side()].texture())
    }

    /// Requested radius, admitted radius and any fallback reason.
    pub fn ambient_occlusion_selection(&self) -> crate::capabilities::AmbientOcclusionSelection {
        self.ambient_selection
    }

    /// Runtime controls for the prepared ambient visibility effect.
    pub fn ambient_occlusion_quality(&self) -> &[QualityKnob] {
        self.ambient
            .as_ref()
            .map_or(&[], |ambient| ambient.settings.knobs())
    }

    pub(crate) fn set_ambient_quality(
        &mut self,
        name: &str,
        value: u32,
    ) -> Option<Result<(), QualityError>> {
        let ambient = self.ambient.as_mut()?;
        ambient
            .settings
            .knobs()
            .iter()
            .any(|knob| knob.name == name)
            .then(|| ambient.settings.set(name, value))
    }

    pub(crate) fn commit_ambient_quality(&mut self) {
        if let Some(ambient) = &mut self.ambient
            && ambient.settings.commit() != QualityInvalidation::None
        {
            ambient.submitted = None;
        }
    }

    /// Apply the runtime ambient extent control to a standalone renderer.
    /// # Errors
    /// Refuses an unprepared effect or extent outside 0 (off), 1 (half), 2 (full).
    #[cfg(feature = "creation-counters")]
    pub fn set_ambient_extent_for_probe(&mut self, extent: u32) -> Result<(), QualityError> {
        self.set_ambient_quality(Ambient::EXTENT, extent)
            .ok_or(QualityError::NoContribution)??;
        self.commit_ambient_quality();
        Ok(())
    }
}

impl SceneContext<'_> {
    pub(in crate::synthetic) fn ambient_receiver(
        &mut self,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        use super::super::forward::CameraDraw;
        use crate::graph::{GraphColorAttachment, GraphDepthAttachment};

        let ambient = self
            .prepared
            .ambient
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        if ambient.stride().is_none() {
            return Ok(());
        }
        let target = self.target;
        let (resource, view) = target
            .ids
            .ambient_receiver
            .as_ref()
            .zip(target.ambient_receiver_view.as_ref())
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = compiled
            .begin_render(
                self.encoder,
                [Some(GraphColorAttachment {
                    resource,
                    range: 0..1,
                    view,
                    clear: wgpu::Color::TRANSPARENT,
                })],
                Some(GraphDepthAttachment {
                    resource: &target.ids.depth,
                    range: 0..1,
                    view: target.forward_depth(),
                    clear: 0.0,
                }),
                self.generation,
                self.timings.as_deref_mut(),
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        target.set_drawn_viewport(&mut pass);
        self.prepared.bind_forward(
            &mut pass,
            target,
            &target.view_bind_group,
            self.exposure_side,
            CameraDraw::AmbientReceiver,
        )?;
        self.prepared
            .draw_camera(&mut pass, target, CameraDraw::AmbientReceiver)?;
        if let Some(contribution) = self.contribution.as_deref() {
            contribution.draw_ambient_receivers(&mut pass);
        }
        Ok(())
    }

    pub(in crate::synthetic) fn ambient_visibility(
        &mut self,
        stage: Stage,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        let (ambient, target) = self
            .prepared
            .ambient
            .as_ref()
            .zip(self.target.ambient.as_ref())
            .ok_or(RenderError::GraphPassMismatch)?;
        let Some(stride) = ambient.stride() else {
            return Ok(());
        };
        let structure = (stage == Stage::Trace)
            .then(|| self.prepared.ambient_structure_group())
            .transpose()?;
        let mut pass = compiled
            .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&ambient.pipelines.stages[stage as usize]);
        let read = 1 - self.prepared.history.write_side();
        if let Some(structure) = &structure {
            pass.set_bind_group(0, &target.depth, &[]);
            pass.set_bind_group(1, structure, &[]);
            pass.set_bind_group(2, target.trace_group(read), &[]);
        } else {
            pass.set_bind_group(0, &target.groups[stage as usize][read], &[]);
        }
        let [x, y] = self.target.drawn.map(|axis| axis.div_ceil(stride * TILE));
        pass.dispatch_workgroups(x, y, 1);
        Ok(())
    }
}
