//! Submitted soft Sun visibility for the opaque forward pass.
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

use super::super::{
    PreparedSynthetic, RenderError, SceneContext, SyntheticTargetInputs, SyntheticView,
};
use crate::binding;
use crate::contribution::{QualityInvalidation, QualityKnob, QualitySettings};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphTextures, PassBody, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope, TextureKind,
};
use crate::pass_key::{PassKey, PipelineRole};
use crate::records::SoftSunFrame;
use crate::runtime::DeviceSite;
use crate::shaders::wgsl;

const TILE: u32 = 8;
const RAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Uint;
const SAMPLE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Uint;
const VALUE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const USES: wgpu::TextureUsages =
    wgpu::TextureUsages::TEXTURE_BINDING.union(wgpu::TextureUsages::STORAGE_BINDING);
const PROBE_COPY: wgpu::TextureUsages = if cfg!(feature = "creation-counters") {
    wgpu::TextureUsages::COPY_SRC.union(wgpu::TextureUsages::COPY_DST)
} else {
    wgpu::TextureUsages::empty()
};
const FRAME_BYTES: u64 = size_of::<SoftSunFrame>() as u64;
const FLOAT: wgpu::TextureSampleType = wgpu::TextureSampleType::Float { filterable: false };
const UINT: wgpu::TextureSampleType = wgpu::TextureSampleType::Uint;
const COMPUTE: wgpu::ShaderStages = wgpu::ShaderStages::COMPUTE;
const FRAGMENT: wgpu::ShaderStages = wgpu::ShaderStages::FRAGMENT;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum Stage {
    Trace,
    Reduce,
    Accumulate,
    Filter,
}

impl Stage {
    pub(crate) const ALL: [Self; 4] = [Self::Trace, Self::Reduce, Self::Accumulate, Self::Filter];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Trace => "traced Sun visibility trace",
            Self::Reduce => "traced Sun visibility neighbourhood",
            Self::Accumulate => "traced Sun visibility accumulation",
            Self::Filter => "traced Sun visibility filter",
        }
    }

    const fn entry(self) -> &'static str {
        match self {
            Self::Trace => "trace_sun",
            Self::Reduce => "reduce_sun",
            Self::Accumulate => "accumulate_sun",
            Self::Filter => "filter_sun",
        }
    }
}

#[derive(Clone)]
pub(in crate::synthetic) struct SoftPipelines {
    pipelines: [wgpu::ComputePipeline; 4],
    inputs: [wgpu::BindGroupLayout; 4],
    structure: wgpu::BindGroupLayout,
}

pub(in crate::synthetic) struct SoftSun {
    pub(in crate::synthetic) pipelines: SoftPipelines,
    pub(in crate::synthetic) settings: QualitySettings,
    submitted: Option<SoftSample>,
    pending: Option<SoftSample>,
    phase: u32,
    directions: VecDeque<glam::Vec3>,
}

struct SoftSample {
    target: Arc<()>,
    drawn: [u32; 2],
    direction: glam::Vec3,
    valid: bool,
}

pub(in crate::synthetic) struct SoftResources {
    rays: ResourceId,
    tiles: ResourceId,
}

pub(in crate::synthetic) struct SoftTarget {
    history: SoftHistory,
    uniform: wgpu::Buffer,
    rays: wgpu::TextureView,
    tiles: wgpu::TextureView,
    groups: [[wgpu::BindGroup; 2]; 4],
    lights: Mutex<Option<(wgpu::Tlas, [[wgpu::BindGroup; 2]; 2])>>,
}

#[derive(Clone)]
struct SoftHistory {
    identity: Arc<()>,
    values: [wgpu::TextureView; 2],
    samples: [wgpu::TextureView; 2],
}

pub(super) fn sampled(
    slot: u32,
    stage: wgpu::ShaderStages,
    ty: wgpu::TextureSampleType,
) -> wgpu::BindGroupLayoutEntry {
    binding::texture(slot, stage, ty, wgpu::TextureViewDimension::D2)
}

pub(super) fn write(slot: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    binding::storage_texture(
        slot,
        COMPUTE,
        wgpu::StorageTextureAccess::WriteOnly,
        format,
        wgpu::TextureViewDimension::D2,
    )
}

pub(super) fn view_entry(slot: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding: slot,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

impl SoftPipelines {
    fn new(device: &DeviceSite) -> Self {
        let uniform = binding::uniform(0, COMPUTE, FRAME_BYTES);
        let depth = sampled(1, COMPUTE, wgpu::TextureSampleType::Depth);
        let rays = sampled(4, COMPUTE, UINT);
        let entries = [
            vec![
                uniform,
                depth,
                binding::uniform(2, COMPUTE, crate::light::DIRECTIONAL_BYTES),
                write(3, RAY_FORMAT),
            ],
            vec![uniform, rays, write(5, RAY_FORMAT)],
            vec![
                uniform,
                depth,
                rays,
                sampled(6, COMPUTE, UINT),
                sampled(7, COMPUTE, FLOAT),
                sampled(8, COMPUTE, UINT),
                write(9, SAMPLE_FORMAT),
            ],
            vec![
                uniform,
                depth,
                sampled(10, COMPUTE, UINT),
                write(11, VALUE_FORMAT),
            ],
        ];
        let inputs = Stage::ALL.map(|stage| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(stage.name()),
                entries: &entries[stage as usize],
            })
        });
        let structure = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("traced Sun visibility structure"),
            entries: &[binding::acceleration_structure(15, COMPUTE)],
        });
        let shader = crate::shaders::Shader::SoftSun.compile(device, "traced Sun visibility");
        let pipelines = Stage::ALL.map(|stage| {
            let groups = [Some(&inputs[stage as usize]), Some(&structure)];
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(stage.name()),
                bind_group_layouts: &groups[..if matches!(stage, Stage::Trace) { 2 } else { 1 }],
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
            pipelines,
            inputs,
            structure,
        }
    }

    fn bind_inputs(
        &self,
        device: &wgpu::Device,
        (uniform, rays, tiles, history): (
            &wgpu::Buffer,
            &wgpu::TextureView,
            &wgpu::TextureView,
            &SoftHistory,
        ),
        depth: &wgpu::TextureView,
        motion: &wgpu::TextureView,
        directional: &wgpu::Buffer,
    ) -> [[wgpu::BindGroup; 2]; 4] {
        Stage::ALL.map(|stage| {
            std::array::from_fn(|read| {
                let write = 1 - read;
                let mut entries = vec![wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }];
                entries.extend(match stage {
                    Stage::Trace => vec![
                        view_entry(1, depth),
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: directional.as_entire_binding(),
                        },
                        view_entry(3, rays),
                    ],
                    Stage::Reduce => vec![view_entry(4, rays), view_entry(5, tiles)],
                    Stage::Accumulate => vec![
                        view_entry(1, depth),
                        view_entry(4, rays),
                        view_entry(6, tiles),
                        view_entry(7, motion),
                        view_entry(8, &history.samples[read]),
                        view_entry(9, &history.samples[write]),
                    ],
                    Stage::Filter => vec![
                        view_entry(1, depth),
                        view_entry(10, &history.samples[write]),
                        view_entry(11, &history.values[write]),
                    ],
                });
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(stage.name()),
                    layout: &self.inputs[stage as usize],
                    entries: &entries,
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
            label: Some("traced Sun visibility structure"),
            layout: &self.structure,
            entries: &[wgpu::BindGroupEntry {
                binding: 15,
                resource: tlas.as_binding(),
            }],
        })
    }

    pub(in crate::synthetic) fn light_entries() -> [wgpu::BindGroupLayoutEntry; 3] {
        [
            binding::uniform(19, FRAGMENT, FRAME_BYTES),
            sampled(20, FRAGMENT, FLOAT),
            sampled(21, FRAGMENT, UINT),
        ]
    }
}

pub(crate) fn shader_source() -> String {
    format!(
        "enable wgpu_ray_query;\nconst SOFT_SUN_TILE: u32 = {TILE}u;\n{}\n{}\n{}\n{}",
        wgsl!("directional_light.wgsl"),
        wgsl!("synthetic/raytrace/shadows.wgsl"),
        wgsl!("synthetic/raytrace/soft_common.wgsl"),
        wgsl!("synthetic/raytrace/soft_visibility.wgsl"),
    )
}

impl SoftSun {
    const MAX_SETTLE: u32 = 31;
    pub(in crate::synthetic) const PENUMBRA: &str = "Sun penumbra";
    const SETTLE: &str = "Sun penumbra settle length";
    const DIVE: &str = "Sun penumbra dive limit";
    const RIM: &str = "Sun penumbra rim rays";

    pub(in crate::synthetic) fn new(device: &DeviceSite) -> Self {
        let knob = |name: &str, effect, value, range| QualityKnob {
            name: name.into(),
            effect,
            value,
            range,
            invalidation: QualityInvalidation::EffectHistory,
            governor_steps: None,
        };
        let settings = QualitySettings::new(vec![
            knob(Self::PENUMBRA, "Trace and filter the Sun disc", 1, 0..=1),
            knob(
                Self::SETTLE,
                "Lifetime of a Sun disc sample in submitted frames",
                16,
                1..=Self::MAX_SETTLE,
            ),
            knob(
                Self::DIVE,
                "Maximum angle below the receiver plane in degrees",
                30,
                1..=89,
            ),
            knob(
                Self::RIM,
                "Current rim rays used to verify a soft edge",
                4,
                0..=4,
            ),
        ])
        .expect("valid Sun penumbra controls");
        Self {
            pipelines: SoftPipelines::new(device),
            settings,
            submitted: None,
            pending: None,
            phase: 0,
            directions: VecDeque::with_capacity(Self::MAX_SETTLE as usize),
        }
    }

    pub(in crate::synthetic) fn enabled(&self) -> bool {
        self.settings.knobs()[0].value != 0
    }

    fn frame(
        &self,
        view: &SyntheticView,
        previous: &SyntheticView,
        drawn: [u32; 2],
        valid: bool,
        direction: glam::Vec3,
    ) -> SoftSunFrame {
        let now_jitter = if view.raster_jitter_enabled() {
            view.raster_jitter_pixels()
        } else {
            [0.0; 2]
        };
        let old_jitter = if previous.raster_jitter_enabled() {
            previous.raster_jitter_pixels()
        } else {
            [0.0; 2]
        };
        let camera = glam::Vec3::from_array(view.camera_position());
        let relative = (view.raster_projection() * glam::Mat4::from_translation(camera)).inverse();
        let settle = self.settings.knobs()[1].value;
        let radius = crate::shader_constants::SUN_ANGULAR_RADIUS;
        let oldest = self
            .directions
            .get(self.directions.len().min(settle as usize).saturating_sub(1))
            .copied()
            .unwrap_or(direction);
        let sweep = if valid {
            (oldest.distance(direction) / radius).min(1.0)
        } else {
            0.0
        };
        SoftSunFrame {
            inverse: relative.to_cols_array_2d(),
            previous: previous.view_proj_columns(),
            camera: camera.extend(0.0).to_array(),
            extent: drawn,
            jitter_delta: [old_jitter[0] - now_jitter[0], old_jitter[1] - now_jitter[1]],
            enabled: u32::from(self.enabled()),
            valid: u32::from(valid),
            phase: self.phase,
            settle,
            dive_cosine: (self.settings.knobs()[2].value as f32).to_radians().sin(),
            radius,
            rim_rays: self.settings.knobs()[3].value,
            padding: 0,
            oldest_sun: oldest.extend(sweep).to_array(),
        }
    }

    pub(in crate::synthetic) fn update(
        &mut self,
        queue: &wgpu::Queue,
        target: &SoftTarget,
        (view, previous): (&SyntheticView, &SyntheticView),
        drawn: [u32; 2],
        transform_valid: bool,
        direction: glam::Vec3,
    ) {
        let valid = transform_valid
            && self.submitted.as_ref().is_some_and(|old| {
                old.drawn == drawn && Arc::ptr_eq(&old.target, &target.history.identity)
            });
        queue.write_buffer(
            &target.uniform,
            0,
            bytemuck::bytes_of(&self.frame(view, previous, drawn, valid, direction)),
        );
        self.pending = self.enabled().then(|| SoftSample {
            target: Arc::clone(&target.history.identity),
            drawn,
            direction,
            valid,
        });
    }

    pub(in crate::synthetic) fn commit_settings(&mut self) -> QualityInvalidation {
        let invalidation = self.settings.commit();
        if invalidation != QualityInvalidation::None {
            self.submitted = None;
        }
        invalidation
    }

    pub(in crate::synthetic) fn submitted(&mut self) {
        self.submitted = self.pending.take();
        if self.submitted.as_ref().is_none_or(|sample| !sample.valid) {
            self.directions.clear();
        }
        if let Some(sample) = &self.submitted {
            if self.directions.len() == Self::MAX_SETTLE as usize {
                self.directions.pop_back();
            }
            self.directions.push_front(sample.direction);
        }
        self.phase = self.phase.wrapping_add(1);
    }
}

impl SoftTarget {
    pub(in crate::synthetic) fn light_bindings(
        &self,
        side: usize,
    ) -> [wgpu::BindGroupEntry<'_>; 3] {
        [
            wgpu::BindGroupEntry {
                binding: 19,
                resource: self.uniform.as_entire_binding(),
            },
            view_entry(20, &self.history.values[side]),
            view_entry(21, &self.history.samples[side]),
        ]
    }

    pub(in crate::synthetic) fn light_group(
        &self,
        tlas: &wgpu::Tlas,
        scene: &PreparedSynthetic,
        history: usize,
        exposure: usize,
    ) -> Result<wgpu::BindGroup, RenderError> {
        let mut cache = self
            .lights
            .lock()
            .map_err(|_| RenderError::GraphPassMismatch)?;
        if cache.as_ref().is_none_or(|(old, _)| *old != *tlas) {
            *cache = Some((
                tlas.clone(),
                std::array::from_fn(|side| {
                    scene
                        .create_ray_light_groups(tlas, Some((self, side)), None)
                        .expect("traced Sun lights")
                }),
            ));
        }
        Ok(cache.as_ref().expect("prepared light cache").1[history][exposure].clone())
    }
}

pub(super) fn descriptor(
    [width, height]: [u32; 2],
    format: wgpu::TextureFormat,
) -> ResourceDescriptor {
    ResourceDescriptor::Texture {
        dimension: TextureKind::D2,
        width,
        height,
        layers: 1,
        mips: 1,
        samples: 1,
        format,
        usage: USES | PROBE_COPY,
    }
}

pub(super) fn history_views(
    device: &wgpu::Device,
    label: &str,
    [width, height]: [u32; 2],
    format: wgpu::TextureFormat,
) -> [wgpu::TextureView; 2] {
    std::array::from_fn(|_| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: USES | PROBE_COPY,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    })
}

impl SyntheticTargetInputs {
    pub(in crate::synthetic) fn prepare_soft_target(
        &self,
        device: &wgpu::Device,
        textures: &GraphTextures,
        ids: Option<&SoftResources>,
        depth: &wgpu::TextureView,
        motion: &wgpu::TextureView,
        extent: [u32; 2],
    ) -> Result<Option<SoftTarget>, RenderError> {
        let Some((pipelines, ids)) = self.soft_sun.as_ref().zip(ids) else {
            return Ok(None);
        };
        let view = |id| {
            textures
                .texture(id)
                .ok_or(RenderError::GraphPassMismatch)
                .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
        };
        let rays = view(&ids.rays)?;
        let tiles = view(&ids.tiles)?;
        let history = SoftHistory {
            identity: Arc::new(()),
            values: history_views(device, "submitted Sun visibility", extent, VALUE_FORMAT),
            samples: history_views(device, "submitted Sun samples", extent, SAMPLE_FORMAT),
        };
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("traced Sun visibility frame"),
            size: FRAME_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let groups = pipelines.bind_inputs(
            device,
            (&uniform, &rays, &tiles, &history),
            depth,
            motion,
            &self.directional_buffer,
        );
        Ok(Some(SoftTarget {
            history,
            uniform,
            rays,
            tiles,
            groups,
            lights: Mutex::new(None),
        }))
    }

    pub(in crate::synthetic) fn append_soft_visibility(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward_reads: &mut Vec<Access>,
        extent: [u32; 2],
        depth: &ResourceId,
        motion: &ResourceId,
        light: &ResourceId,
        structure: Option<&ResourceId>,
    ) -> Result<Option<(SoftResources, [usize; 4])>, RenderError> {
        if self.soft_sun.is_none() {
            return Ok(None);
        }
        let structure = structure.ok_or(RenderError::GraphPassMismatch)?;
        let tiles = graph.resource(descriptor(extent, RAY_FORMAT), ResourceScope::Transient)?;
        let rays = graph.resource(descriptor(extent, RAY_FORMAT), ResourceScope::Transient)?;
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
        let history_samples = history(graph, 0x5355, SAMPLE_FORMAT)?;
        let write_samples = history(graph, 0x5355, SAMPLE_FORMAT)?;
        let history_values = history(graph, 0x5356, VALUE_FORMAT)?;
        let write_values = history(graph, 0x5356, VALUE_FORMAT)?;
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: FRAME_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let read = |id: &ResourceId| Access::new(id.clone(), 0..1, AccessKind::SampledRead);
        let write = |id: &ResourceId| Access::new(id.clone(), 0..1, AccessKind::StorageWrite);
        let params = Access::new(uniform, 0..FRAME_BYTES, AccessKind::UniformRead);
        forward_reads.extend([
            params.clone(),
            read(&history_samples),
            read(&history_values),
        ]);
        let passes = Stage::ALL.map(|stage| {
            let role = PipelineRole::SoftSun(stage as u32);
            let key = role.key(self.key);
            keys.insert(key);
            let mut accesses = vec![params.clone()];
            accesses.extend(match stage {
                Stage::Trace => vec![
                    read(depth),
                    write(&rays),
                    Access::new(structure.clone(), 0..1, AccessKind::RayQuery),
                    Access::new(
                        light.clone(),
                        0..crate::light::DIRECTIONAL_BYTES,
                        AccessKind::UniformRead,
                    ),
                ],
                Stage::Reduce => vec![read(&rays), write(&tiles)],
                Stage::Accumulate => vec![
                    read(depth),
                    read(motion),
                    read(&rays),
                    read(&tiles),
                    read(&history_samples),
                    write(&write_samples),
                ],
                Stage::Filter => vec![read(depth), read(&write_samples), write(&write_values)],
            });
            graph.declare(
                PassKey::SoftSun(stage),
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: Some(key),
                    accesses,
                    external_side_effect: true,
                },
            )
        });
        Ok(Some((SoftResources { rays, tiles }, passes)))
    }
}

impl super::super::OffscreenTarget {
    pub(in crate::synthetic) fn share_soft_history(
        &mut self,
        prepared: &SyntheticTargetInputs,
        first: &Self,
    ) {
        if let Some(((target, first), pipelines)) = self
            .soft_sun
            .as_mut()
            .zip(first.soft_sun.as_ref())
            .zip(prepared.soft_sun.as_ref())
        {
            target.history.clone_from(&first.history);
            target.groups = pipelines.bind_inputs(
                &self.device,
                (
                    &target.uniform,
                    &target.rays,
                    &target.tiles,
                    &target.history,
                ),
                &self.depth,
                &self.motion_view,
                &prepared.directional_buffer,
            );
            target.lights = Mutex::new(None);
        }
    }

    /// The raw disc ray results of this target's last submission.
    #[must_use]
    #[cfg(feature = "creation-counters")]
    pub fn sun_disc_witness(&self) -> Option<&wgpu::Texture> {
        Some(self.soft_sun.as_ref()?.rays.texture())
    }

    /// Soft visibility and its samples for the named retained side.
    #[must_use]
    #[cfg(feature = "creation-counters")]
    pub fn sun_penumbra_textures_for_probe(&self, side: usize) -> Option<[&wgpu::Texture; 2]> {
        let history = &self.soft_sun.as_ref()?.history;
        Some([
            history.values.get(side)?.texture(),
            history.samples.get(side)?.texture(),
        ])
    }

    /// Nominal bytes of the soft Sun history shared by a prepared ring.
    #[must_use]
    pub fn sun_penumbra_history_bytes(&self) -> u64 {
        u64::from(self.soft_sun.is_some()) * u64::from(self.width) * u64::from(self.height) * 48
    }
}

impl PreparedSynthetic {
    /// Index of the last submitted side in the retained Sun history.
    #[must_use]
    #[cfg(feature = "creation-counters")]
    pub fn sun_penumbra_side_for_probe(&self) -> usize {
        1 - self.history.write_side()
    }
}

impl SceneContext<'_> {
    pub(in crate::synthetic) fn soft_sun(
        &mut self,
        stage: Stage,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        let (soft, target) = self
            .prepared
            .soft_sun
            .as_ref()
            .zip(self.target.soft_sun.as_ref())
            .ok_or(RenderError::GraphPassMismatch)?;
        if !soft.enabled() {
            return Ok(());
        }
        let structure = matches!(stage, Stage::Trace)
            .then(|| self.prepared.soft_structure_group())
            .transpose()?;
        let mut pass = compiled
            .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&soft.pipelines.pipelines[stage as usize]);
        let read = 1 - self.prepared.history.write_side();
        pass.set_bind_group(0, &target.groups[stage as usize][read], &[]);
        if let Some(structure) = &structure {
            pass.set_bind_group(1, structure, &[]);
        }
        let [x, y] = self.target.drawn.map(|axis| axis.div_ceil(TILE));
        pass.dispatch_workgroups(x, y, 1);
        Ok(())
    }
}
