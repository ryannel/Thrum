mod coarse;
mod correction;

use super::{BodyAnswers, BodyQuery, QueryAnswer, QueryLimits, QueryMailbox, seconds_between};
use crate::{
    column::{COMPONENT_LIMIT, Component},
    interaction::{Interaction, SampleSettings},
    map_gpu::{self, MapGpu},
    surf::{QUERY_ONE_SHEET, SurfGpu},
};
use bytemuck::{Pod, Zeroable};
use coarse::CoarsePlan;
use correction::{CascadeSample, MapPrediction};
pub(super) use correction::{Correction, ReadyBody};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use thrum_render::binding;
use thrum_render::contribution::{EffectAccess, EffectGraph, EffectState, EffectTime, TickPass};
use thrum_render::{extract::SourceTick, graph::ResourceScope};
use thrum_sea_model::{Sample, Spectrum};
use thrum_water_model::{Map, Surface};

#[cfg(test)]
mod tests;

const QUERY_PASS: &str = "water body query";
const CARRY_TICKS: f64 = 5.0;
const SLOT_COUNT: usize = 4;
const GROUP_WIDTH: u32 = 64;
const NODES_PER_GROUP: u32 = 8;
const SURF_GROUP: u32 = 1;
const SURF_BINDING: u32 = 7;
const COARSE_GROUP: u32 = 2;
const COARSE_RECORD_BYTES: u64 = 16;
const PLAIN_QUERY: &str = "
@compute @workgroup_size(QUERY_WIDTH)
fn sample_query(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    if index >= settings.count { return; }
    let point = points[index];
    let gains = query_gains(point);
    var result: QueryWave;
    if point.body != 0xffffffffu { result = plain_query(point, gains.x); }
    write_answer(index, point, gains, result);
}";

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Point {
    position: [f32; 2],
    radius: f32,
    dt: f32,
    level_current_bed: [f32; 4],
    slope: [f32; 2],
    body: u32,
    flags: u32,
    gains: [f32; 4],
    dent: [f32; 4],
    surf_stage: [f32; 4],
    fine_bounds: [i32; 4],
    coarse_grid: [i32; 4],
    carry: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Settings {
    origin: [f32; 2],
    count: u32,
    components: u32,
    lengths_chop: [f32; 4],
    heights: [f32; 4],
    swash_margin: [f32; 4],
    coarse_jobs: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Answer {
    height_slope_rate: [f32; 4],
    velocity_depth: [f32; 4],
    layers: [f32; 4],
    current_bed: [f32; 4],
    body_flags: [u32; 4],
    cascades: [CascadeSample; 3],
    cut_fine_nodes: u32,
    cut_coarse_nodes: u32,
    padding: [u32; 3],
    wake: [f32; 4],
}

struct Slot {
    group: wgpu::BindGroup,
    settings: wgpu::Buffer,
    components: wgpu::Buffer,
    points: wgpu::Buffer,
    answers: wgpu::Buffer,
    buffer: wgpu::Buffer,
    busy: Arc<AtomicBool>,
    coarse: Option<CoarseScratch>,
    wake: Option<wgpu::Buffer>,
}

impl Slot {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        cascades: &wgpu::TextureView,
        count: u64,
        wake: Option<&Interaction>,
        coarse: Option<&CoarseGpu>,
    ) -> Self {
        use wgpu::BufferUsages as Usage;
        let settings = buffer(
            device,
            "query settings",
            size_of::<Settings>() as u64,
            Usage::UNIFORM | Usage::COPY_DST,
        );
        let components = buffer(
            device,
            "query components",
            (COMPONENT_LIMIT * size_of::<Component>()) as u64,
            Usage::STORAGE | Usage::COPY_DST,
        );
        let points = buffer(
            device,
            "query points",
            count * size_of::<Point>() as u64,
            Usage::STORAGE | Usage::COPY_DST,
        );
        let answers = buffer(
            device,
            "query answers",
            count * size_of::<Answer>() as u64,
            Usage::STORAGE | Usage::COPY_SRC,
        );
        let staging = buffer(
            device,
            "query readback",
            answers.size(),
            Usage::MAP_READ | Usage::COPY_DST,
        );
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: settings.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: points.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: components.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: answers.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(cascades),
            },
        ];
        let wake_settings = wake.map(|_| {
            buffer(
                device,
                "query wake settings",
                size_of::<SampleSettings>() as u64,
                Usage::UNIFORM | Usage::COPY_DST,
            )
        });
        if let Some((wake, settings)) = wake.zip(wake_settings.as_ref()) {
            entries.extend(wake.query_entries(settings));
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water query bindings"),
            layout,
            entries: &entries,
        });
        Self {
            group,
            settings,
            components,
            points,
            answers,
            buffer: staging,
            busy: Arc::default(),
            wake: wake_settings,
            coarse: coarse.map(|gpu| CoarseScratch::new(gpu, COARSE_RECORD_BYTES)),
        }
    }
}

struct CoarseScratch {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    count: wgpu::Buffer,
}

struct CoarseGpu {
    device: wgpu::Device,
    layout: wgpu::BindGroupLayout,
    select: wgpu::ComputePipeline,
    general: SurfPipelines,
    single_sheet: SurfPipelines,
}

struct SurfPipelines {
    coarse: wgpu::ComputePipeline,
    fine: wgpu::ComputePipeline,
    answer: wgpu::ComputePipeline,
}

impl CoarseScratch {
    fn new(gpu: &CoarseGpu, size: u64) -> Self {
        let lattice = buffer(
            &gpu.device,
            "water query coarse lattice",
            size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let count = buffer(
            &gpu.device,
            "water query selected cells",
            size_of::<u32>() as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water query coarse lattice"),
            layout: &gpu.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lattice.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: count.as_entire_binding(),
                },
            ],
        });
        Self {
            buffer: lattice,
            group,
            count,
        }
    }
}

struct Batch {
    slot: usize,
    endpoint: SourceTick,
    epoch: u64,
    frame: EffectTime,
    seconds: f64,
    wake_tick: Option<u64>,
    count: u32,
    bodies: Vec<BodyQuery>,
    map_revisions: Vec<u64>,
    coarse_groups: u32,
    fine_groups: u32,
    single_sheet: bool,
    fine_width: u32,
}

pub(crate) struct Readback {
    mailbox: QueryMailbox,
    plain: Option<wgpu::ComputePipeline>,
    surf: Option<wgpu::BindGroup>,
    coarse: Option<CoarseGpu>,
    slots: [Slot; SLOT_COUNT],
    pending: Vec<Batch>,
    frame: EffectState,
    recorded_map: Option<Arc<Map>>,
    committed_map: Option<Arc<Map>>,
    surf_map: Option<MapGpu>,
    bed_ranges: HashMap<[i32; 2], [f64; 2]>,
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

impl Readback {
    pub(crate) fn new(
        device: &thrum_render::runtime::DeviceSite,
        ambient: &thrum_sea::Ambient,
        mailbox: QueryMailbox,
        wake: Option<&Interaction>,
        surf: Option<(MapGpu, &SurfGpu)>,
    ) -> Self {
        let count = u64::from(mailbox.limits.points);
        let surf_source = if surf.is_some() {
            map_gpu::declarations_at(SURF_GROUP) + &SurfGpu::query_source(SURF_GROUP, SURF_BINDING)
        } else {
            PLAIN_QUERY.to_owned()
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water body query"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}\n{}\n{}\n{}",
                    if surf.is_some() {
                        ""
                    } else {
                        include_str!("../gains.wgsl")
                    },
                    Interaction::sample_source(0, 5, 6, wake.is_some()),
                    include_str!("../column/records.wgsl"),
                    include_str!("../column.wgsl"),
                    include_str!("sample.wgsl"),
                    surf_source,
                )
                .into(),
            ),
        });
        let visibility = wgpu::ShaderStages::COMPUTE;
        let mut layout = vec![
            binding::uniform(0, visibility, 0),
            binding::storage(1, visibility, true, 0),
            binding::storage(2, visibility, true, 0),
            binding::storage(3, visibility, false, 0),
            binding::texture(
                4,
                visibility,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2Array,
            ),
        ];
        if wake.is_some() {
            layout.extend(Interaction::sample_layout(5, visibility));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water query bindings"),
            entries: &layout,
        });
        let surf_layout = surf.as_ref().map(|_| {
            let mut entries = map_gpu::bindings(visibility).to_vec();
            entries.extend(SurfGpu::layout(SURF_BINDING, visibility));
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("water query surf"),
                entries: &entries,
            })
        });
        let coarse_layout = surf.as_ref().map(|_| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("water query coarse lattice"),
                entries: &[
                    binding::storage(0, visibility, false, COARSE_RECORD_BYTES),
                    binding::storage(1, visibility, false, size_of::<u32>() as u64),
                ],
            })
        });
        let mut groups = vec![Some(&layout)];
        groups.extend(surf_layout.as_ref().map(Some));
        groups.extend(coarse_layout.as_ref().map(Some));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water query pipeline layout"),
            bind_group_layouts: &groups,
            immediate_size: 0,
        });
        let surf_group = surf
            .as_ref()
            .zip(surf_layout.as_ref())
            .map(|((map, surf), layout)| {
                let view = map.view();
                let mut entries = map.entries(&view).to_vec();
                entries.extend(surf.entries(SURF_BINDING));
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("water query surf"),
                    layout,
                    entries: &entries,
                })
            });
        let make_pipeline = |label, entry, constants| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                cache: None,
            })
        };
        let plain = surf
            .is_none()
            .then(|| make_pipeline(QUERY_PASS, "sample_query", &[]));
        let make_surf = |coarse, fine, constants| SurfPipelines {
            coarse: make_pipeline("water query coarse lattice", coarse, &[]),
            fine: make_pipeline("water query fine lattice", fine, &[]),
            answer: make_pipeline(QUERY_PASS, "sample_query", constants),
        };
        let coarse = coarse_layout.map(|layout| CoarseGpu {
            device: (**device).clone(),
            layout,
            select: make_pipeline("water query select fine cells", "select_fine", &[]),
            general: make_surf("sample_coarse", "sample_fine", &[]),
            single_sheet: make_surf(
                "sample_coarse_wide",
                "sample_fine_wide",
                &[("single_sheet", 1.0)],
            ),
        });
        let slots = std::array::from_fn(|_| {
            Slot::new(
                device,
                &layout,
                &ambient.filtered_views()[0],
                count,
                wake,
                coarse.as_ref(),
            )
        });
        mailbox.reset_readback();
        Self {
            mailbox,
            plain,
            surf: surf_group,
            coarse,
            slots,
            pending: Vec::with_capacity(SLOT_COUNT),
            frame: EffectState::new(0),
            recorded_map: None,
            committed_map: None,
            surf_map: surf.map(|(map, _)| map),
            bed_ranges: HashMap::new(),
        }
    }

    pub(crate) fn declare(
        ambient: &thrum_sea::Ambient,
        graph: &mut EffectGraph,
        wake: Option<EffectAccess>,
    ) {
        let cascade = ambient.filtered_accesses().remove(0);
        // The cascade texture is allocated by Ambient and retained across views.
        graph.resources[cascade.resource].1 = ResourceScope::Persistent {
            owner: 0x7761_7465,
            epoch: 0,
            initialized: false,
        };
        graph.tick_passes.push(TickPass {
            name: QUERY_PASS.into(),
            reads: std::iter::once(cascade).chain(wake).collect(),
        });
    }

    pub(crate) fn record_frame(&mut self, time: EffectTime, map: &Map) {
        self.frame.begin(time, u64::MAX, [0; 2]);
        if !time.capture && self.surf_map.is_some() {
            let snapshot = self
                .committed_map
                .as_ref()
                .filter(|old| old.spatial_revision() == map.spatial_revision());
            self.recorded_map = Some(snapshot.map_or_else(|| Arc::new(map.clone()), Arc::clone));
        }
    }

    pub(crate) fn submit_frame(&mut self) {
        self.frame.submitted();
        self.committed_map = self.recorded_map.take();
    }

    pub(crate) fn has_work(&self) -> bool {
        let state = self.mailbox.state.lock().expect("water query mailbox");
        state
            .pages
            .front()
            .is_some_and(|page| self.frame.committed_for(page.endpoint).is_some())
            && self
                .slots
                .iter()
                .any(|slot| !slot.busy.load(Ordering::Acquire))
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPU parameters use local f32 metres and bounded tick offsets"
    )]
    pub(crate) fn record(
        &mut self,
        queue: &wgpu::Queue,
        spectrum: &Spectrum,
        chop: f64,
        heights: [f32; 4],
        map: &Map,
        frame_map: Option<&MapGpu>,
        wake: Option<&Interaction>,
    ) {
        self.pending.clear();
        let mut state = self.mailbox.state.lock().expect("water query mailbox");
        let slots = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| !slot.busy.load(Ordering::Acquire))
            .map(|(index, _)| index);
        let Some(time) = state
            .pages
            .front()
            .and_then(|page| self.frame.committed_for(page.endpoint))
        else {
            return;
        };
        let work: Vec<_> = slots
            .zip(&state.pages)
            .map(|(slot, page)| (slot, page.endpoint, page.bodies.clone()))
            .collect();
        if work.len() < state.pages.len() {
            state.stats.busy += 1;
        }
        let epoch = state.epoch;
        let maps = state.maps.clone();
        drop(state);
        if let Some((gpu, committed)) = self.surf_map.as_mut().zip(self.committed_map.as_ref()) {
            gpu.record_query(queue, committed, frame_map.expect("surf frame map"));
        }
        let seconds = thrum_sea::display_seconds(time);
        let step = time.tick_duration.as_secs_f64();
        let frame_offset = (1.0 - f64::from(time.alpha)) * step;
        let wake_settings = wake.and_then(Interaction::query_settings);
        let lengths_chop = std::array::from_fn(|i| {
            spectrum
                .cascades
                .get(i)
                .map_or(chop, thrum_sea_model::Cascade::length) as f32
        });
        let profile = map.profile();
        let mut reach = map.surf_throw_reach(&spectrum.long, chop);
        for (slot, endpoint, bodies) in work {
            let origin = bodies[0].points[0].position;
            let components: Vec<_> = spectrum
                .long
                .iter()
                .filter(|component| component.amplitude > 0.0)
                .map(|component| Component::at(component, origin, seconds))
                .collect();
            let (mut points, map_revisions): (Vec<_>, Vec<_>) = bodies
                .iter()
                .flat_map(|body| body.points.iter().map(move |point| (body.id, point)))
                .map(|(id, point)| {
                    let dt = frame_offset
                        + seconds_between(point.use_tick, time.tick, time.tick_duration);
                    let bound = CARRY_TICKS * step;
                    // A future use tick holds the latest authored stage. A
                    // known past tick uses its saved stage and connectivity.
                    let index = maps.partition_point(|(tick, _)| *tick <= point.use_tick);
                    let domain = if index > 0 { &maps[index - 1].1 } else { map };
                    let mut input = Point::new(point, dt, domain);
                    input.carry[0] = dt.clamp(-bound, bound) as f32;
                    if let Some((wake_tick, _)) = wake_settings {
                        let wake_dt =
                            seconds_between(point.use_tick, wake_tick, time.tick_duration);
                        input.carry[1] = wake_dt.clamp(-bound, bound) as f32;
                        input.dent =
                            wake.expect("wake settings")
                                .dent(wake_tick, id, point.position);
                    }
                    (input, domain.spatial_revision())
                })
                .unzip();
            let count = u32::try_from(points.len()).expect("points admitted by query capacity");
            let buffers = &mut self.slots[slot];
            if let Some((settings, (_, sample))) = buffers.wake.as_ref().zip(wake_settings) {
                queue.write_buffer(settings, 0, bytemuck::bytes_of(&sample));
            }
            let (coarse_groups, coarse_nodes, fine_groups) = if let Some((gpu, (reach, profile))) =
                self.coarse
                    .as_ref()
                    .zip(reach.as_mut().zip(map.surf_profile()))
            {
                let limits = gpu.device.limits();
                let max_bytes = limits
                    .max_storage_buffer_binding_size
                    .min(limits.max_buffer_size);
                let plan = CoarsePlan::new(
                    &mut points,
                    map,
                    reach,
                    profile.cut,
                    max_bytes / COARSE_RECORD_BYTES,
                    limits.max_compute_workgroups_per_dimension,
                    &mut self.bed_ranges,
                );
                let scratch = buffers.coarse.as_mut().expect("surf scratch");
                let bytes = plan.records * COARSE_RECORD_BYTES;
                if scratch.buffer.size() < bytes {
                    *scratch = CoarseScratch::new(gpu, bytes.next_power_of_two().min(max_bytes));
                }
                if !plan.jobs.is_empty() {
                    queue.write_buffer(&scratch.buffer, 0, bytemuck::cast_slice(&plan.jobs));
                    queue.write_buffer(
                        &scratch.buffer,
                        plan.jobs.len() as u64 * COARSE_RECORD_BYTES,
                        bytemuck::cast_slice(&plan.masks),
                    );
                }
                if plan.fine_groups != 0 {
                    queue.write_buffer(&scratch.count, 0, bytemuck::bytes_of(&0_u32));
                }
                (
                    u32::try_from(plan.jobs.len()).expect("bounded coarse job count"),
                    plan.coarse_nodes,
                    plan.fine_groups,
                )
            } else {
                (0, 0, 0)
            };
            let settings = Settings {
                origin,
                count,
                components: u32::try_from(components.len()).expect("bounded component count"),
                lengths_chop,
                heights,
                coarse_jobs: [coarse_groups, coarse_nodes, 0, 0],
                swash_margin: [
                    profile.swash_ratio as f32,
                    profile.margin_height as f32,
                    0.0,
                    0.0,
                ],
            };
            queue.write_buffer(&buffers.settings, 0, bytemuck::bytes_of(&settings));
            queue.write_buffer(&buffers.points, 0, bytemuck::cast_slice(&points));
            if !components.is_empty() {
                queue.write_buffer(&buffers.components, 0, bytemuck::cast_slice(&components));
            }
            self.pending.push(Batch {
                slot,
                endpoint,
                epoch,
                frame: time,
                seconds,
                wake_tick: wake_settings.map(|(tick, _)| tick),
                count,
                bodies,
                map_revisions,
                coarse_groups,
                fine_groups,
                fine_width: map
                    .surf_profile()
                    .map_or(0, |profile| profile.cut.coarse_cells + 1),
                single_sheet: points.iter().all(|point| {
                    point.body == u32::MAX
                        || point.flags & 1 == 0
                        || point.flags & QUERY_ONE_SHEET != 0
                }),
            });
        }
    }

    pub(crate) fn resource_bytes(&self) -> Vec<(String, u64)> {
        let mut bytes = vec![
            (
                "water query inputs".into(),
                self.slots
                    .iter()
                    .map(|slot| {
                        slot.settings.size()
                            + slot.components.size()
                            + slot.points.size()
                            + slot.wake.as_ref().map_or(0, wgpu::Buffer::size)
                    })
                    .sum(),
            ),
            (
                "water query answers".into(),
                self.slots.iter().map(|slot| slot.answers.size()).sum(),
            ),
            (
                "water query readback".into(),
                self.slots.iter().map(|slot| slot.buffer.size()).sum(),
            ),
        ];
        if self.coarse.is_some() {
            bytes.push((
                "water query coarse lattice".into(),
                self.slots
                    .iter()
                    .filter_map(|slot| slot.coarse.as_ref())
                    .map(|scratch| scratch.buffer.size() + scratch.count.size())
                    .sum(),
            ));
        }
        if let Some(map) = &self.surf_map {
            let size = map.texture.size();
            bytes.push((
                "water query map lattice".into(),
                16 * u64::from(size.width)
                    * u64::from(size.height)
                    * u64::from(size.depth_or_array_layers),
            ));
            bytes.push(("water query map records".into(), map.info.size()));
        }
        bytes
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        for batch in &self.pending {
            let slot = &self.slots[batch.slot];
            pass.set_bind_group(0, &slot.group, &[]);
            if let Some((surf, cut)) = self.surf.as_ref().zip(self.coarse.as_ref()) {
                pass.set_bind_group(SURF_GROUP, surf, &[]);
                pass.set_bind_group(
                    COARSE_GROUP,
                    &slot.coarse.as_ref().expect("surf scratch").group,
                    &[],
                );
                let pipelines = if batch.single_sheet {
                    &cut.single_sheet
                } else {
                    &cut.general
                };
                if batch.coarse_groups != 0 {
                    pass.set_pipeline(&pipelines.coarse);
                    let wide = if batch.single_sheet {
                        GROUP_WIDTH / NODES_PER_GROUP
                    } else {
                        1
                    };
                    pass.dispatch_workgroups(batch.coarse_groups, wide, 1);
                }
                if batch.fine_groups != 0 {
                    pass.set_pipeline(&cut.select);
                    pass.dispatch_workgroups(batch.fine_groups, 1, 1);
                    pass.set_pipeline(&pipelines.fine);
                    let groups = cut
                        .device
                        .limits()
                        .max_compute_workgroups_per_dimension
                        .min(256);
                    let wide = if batch.single_sheet {
                        batch.fine_width.pow(2).div_ceil(NODES_PER_GROUP)
                    } else {
                        1
                    };
                    pass.dispatch_workgroups(groups, wide, 1);
                }
                pass.set_pipeline(&pipelines.answer);
                pass.dispatch_workgroups(batch.count, 1, 1);
            } else {
                pass.set_pipeline(self.plain.as_ref().expect("plain query pipeline"));
                pass.dispatch_workgroups(batch.count.div_ceil(GROUP_WIDTH), 1, 1);
            }
        }
    }

    pub(crate) fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        for batch in &self.pending {
            let slot = &self.slots[batch.slot];
            let size = u64::from(batch.count) * size_of::<Answer>() as u64;
            encoder.copy_buffer_to_buffer(&slot.answers, 0, &slot.buffer, 0, size);
        }
    }

    pub(crate) fn submit(&mut self) {
        let Some(last) = self.pending.last() else {
            return;
        };
        {
            let mut state = self.mailbox.state.lock().expect("water query mailbox");
            if state.epoch == last.epoch {
                while state
                    .pages
                    .front()
                    .is_some_and(|page| page.endpoint.tick <= last.endpoint.tick)
                {
                    state.pages.pop_front();
                }
            }
            state.stats.dispatches += self.pending.len() as u64;
        }
        for batch in self.pending.drain(..) {
            Self::map_batch(batch, &self.slots, &self.mailbox);
        }
    }

    fn map_batch(batch: Batch, slots: &[Slot; SLOT_COUNT], mailbox: &QueryMailbox) {
        let slot = &slots[batch.slot];
        slot.busy.store(true, Ordering::Release);
        let mailbox = mailbox.clone();
        let busy = Arc::clone(&slot.busy);
        let buffer = slot.buffer.clone();
        let size = u64::from(batch.count) * size_of::<Answer>() as u64;
        slot.buffer
            .slice(..size)
            .map_async(wgpu::MapMode::Read, move |result| {
                if result.is_ok() {
                    if let Ok(mapped) = buffer.slice(..size).get_mapped_range() {
                        batch.publish(bytemuck::cast_slice(&mapped), &mailbox);
                    }
                    buffer.unmap();
                }
                busy.store(false, Ordering::Release);
            });
    }
}

impl Point {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "query inputs use GPU metres and bounded body indices"
    )]
    fn new(point: &super::QueryPoint, dt: f64, map: &Map) -> Self {
        let world = point.position.map(f64::from);
        let mut input = Self {
            position: point.position,
            radius: point.radius,
            dt: dt as f32,
            body: u32::MAX,
            ..Self::zeroed()
        };
        let sample = map.sample(world);
        if let Some(bed) = sample.map_or_else(|| map.bed().height(world), |row| row.bed) {
            input.flags = 1;
            input.level_current_bed[3] = bed as f32;
        }
        if let Some(sample) = sample {
            let row = sample.row;
            input.body = sample.body as u32;
            input.level_current_bed[..3].copy_from_slice(&[
                row.level as f32,
                row.current[0] as f32,
                row.current[1] as f32,
            ]);
            input.slope = map
                .level_slope(sample.body, world)
                .map(|slope| slope as f32);
            input.gains = row.gains.map(|gain| gain as f32);
            input.surf_stage[0] = map.body(sample.body).expect("sampled body").stage.value as f32;
        }
        input
    }
}

impl Batch {
    fn publish(self, values: &[Answer], mailbox: &QueryMailbox) {
        let mut state = mailbox.state.lock().expect("water query mailbox");
        if state.epoch != self.epoch {
            return;
        }
        for answer in values {
            let coarse = u64::from(answer.cut_coarse_nodes);
            let fine = u64::from(answer.cut_fine_nodes);
            state.stats.cut_coarse_nodes += coarse;
            state.stats.cut_fine_nodes += fine;
            state.stats.cut_nodes += coarse + fine;
        }
        let mut values = values.iter().zip(self.map_revisions);
        for body in self.bodies {
            let mut fields = (&mut values).take(body.points.len());
            if state.generations.get(&body.id) != Some(&body.generation) {
                fields.nth(body.points.len() - 1);
                continue;
            }
            let mut cut_nodes = [0; 2];
            let step = self.frame.tick_duration;
            let (points, predictions) = body
                .points
                .into_iter()
                .zip(fields)
                .enumerate()
                .map(|(index, (point, (raw, revision)))| {
                    cut_nodes[0] += u64::from(raw.cut_coarse_nodes);
                    cut_nodes[1] += u64::from(raw.cut_fine_nodes);
                    let mut sources = [None; 5];
                    for (layer, source) in sources[..4].iter_mut().enumerate() {
                        if raw.body_flags[3] & (1 << layer) != 0 {
                            *source = Some(if layer == 0 {
                                point.use_tick as f64 * self.frame.tick_duration.as_secs_f64()
                            } else {
                                self.seconds
                            });
                        }
                    }
                    if raw.body_flags[3] & 16 != 0 {
                        sources[4] = self.wake_tick.map(|tick| tick as f64 * step.as_secs_f64());
                    }
                    let cascade_ticks = 1.0 - f64::from(self.frame.alpha)
                        + seconds_between(point.use_tick, self.frame.tick, Duration::from_secs(1));
                    let mut supported = sources.map(|source| source.is_some());
                    for flag in &mut supported[1..4] {
                        *flag &= cascade_ticks.abs() <= CARRY_TICKS;
                    }
                    supported[4] &= self
                        .wake_tick
                        .is_some_and(|tick| point.use_tick.abs_diff(tick) as f64 <= CARRY_TICKS);
                    (
                        raw.decode(index, point, sources, supported),
                        MapPrediction::new(revision, raw.cascades, raw.wake),
                    )
                })
                .unzip();
            state.ready.insert(
                (body.id, self.endpoint.tick),
                ReadyBody {
                    answers: BodyAnswers {
                        id: body.id,
                        generation: body.generation,
                        endpoint: self.endpoint,
                        posted: self.endpoint.tick,
                        cut_nodes,
                        points,
                    },
                    predictions,
                    frame_offset: (1.0 - f64::from(self.frame.alpha))
                        * self.frame.tick_duration.as_secs_f64(),
                    frame_tick: self.frame.tick,
                    frame_seconds: self.seconds,
                    step: self.frame.tick_duration,
                },
            );
        }
    }
}

impl Answer {
    fn decode(
        self,
        index: usize,
        point: super::QueryPoint,
        sources: [Option<f64>; 5],
        supported: [bool; 5],
    ) -> QueryAnswer {
        let [body, wet, has_bed, _] = self.body_flags;
        let [height, sx, sz, height_dt] = self.height_slope_rate.map(f64::from);
        let [vx, vy, vz, depth] = self.velocity_depth.map(f64::from);
        let [cx, cz, bed, _] = self.current_bed;
        QueryAnswer {
            index,
            point,
            sources,
            supported,
            wake_height: self.wake[0],
            wave_heights: self.layers,
            bed: (has_bed != 0).then_some(bed),
            surface: Surface {
                body: (body != u32::MAX).then_some(body as usize),
                sample: Sample {
                    height,
                    slope: [sx, sz],
                    velocity: [vx, vy, vz],
                    height_dt,
                },
                depth: if body != u32::MAX && has_bed == 0 {
                    f64::INFINITY
                } else {
                    depth
                },
                current: [f64::from(cx), f64::from(cz)],
                wet: wet != 0,
            },
        }
    }
}

pub(crate) fn validate(
    limits: QueryLimits,
    device: &wgpu::Limits,
    surf: bool,
    wake: bool,
) -> Result<(), String> {
    let groups = if surf {
        limits.points
    } else {
        limits.points.div_ceil(GROUP_WIDTH)
    };
    if u64::from(limits.points) * size_of::<Answer>() as u64
        > device.max_storage_buffer_binding_size
        || groups > device.max_compute_workgroups_per_dimension
        || 3 + 2 * u32::from(surf) + u32::from(wake) > device.max_storage_buffers_per_shader_stage
    {
        return Err("water query exceeds the device buffer or dispatch limit".into());
    }
    Ok(())
}
