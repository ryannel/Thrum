mod coarse;
mod correction;
mod shoaling;

use super::{BodyAnswers, BodyQuery, QueryAnswer, QueryLimits, QueryMailbox, seconds_between};
use crate::{
    column::{COMPONENT_LIMIT, Component},
    interaction::Interaction,
    map_gpu::{self, MapGpu},
    surf::SurfGpu,
};
use bytemuck::{Pod, Zeroable};
use coarse::CoarsePlan;
use correction::{CascadeSample, MapPrediction};
pub(super) use correction::{Correction, ReadyBody};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use thrum_render::binding;
use thrum_render::contribution::{
    EffectAccess, EffectCadence, EffectGraph, EffectPass, EffectTime,
};
use thrum_sea_model::{Sample, Spectrum};
use thrum_water_model::{Map, Surface};

#[cfg(test)]
mod tests;

const SLOT_COUNT: usize = 4;
const GROUP_WIDTH: u32 = 64;
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
    has_bed: u32,
    gains: [f32; 4],
    dent: [f32; 4],
    surf_stage: [f32; 4],
    fine_bounds: [i32; 4],
    coarse_grid: [i32; 4],
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
}

struct CoarseScratch {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
}

struct CoarseGpu {
    device: wgpu::Device,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    shoaling: wgpu::Buffer,
}

impl CoarseScratch {
    fn new(gpu: &CoarseGpu, size: u64) -> Self {
        let buffer = buffer(
            &gpu.device,
            "water query coarse lattice",
            size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water query coarse lattice"),
            layout: &gpu.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gpu.shoaling.as_entire_binding(),
                },
            ],
        });
        Self { buffer, group }
    }
}

struct Batch {
    slot: usize,
    tick: u64,
    epoch: u64,
    frame: EffectTime,
    seconds: f64,
    count: u32,
    bodies: Vec<BodyQuery>,
    map_revisions: Vec<u64>,
    coarse_groups: u32,
}

pub(crate) struct Readback {
    mailbox: QueryMailbox,
    pipeline: wgpu::ComputePipeline,
    surf: Option<wgpu::BindGroup>,
    coarse: Option<CoarseGpu>,
    slots: [Slot; SLOT_COUNT],
    pending: Vec<Batch>,
    submitted: Option<(u128, u64, u64)>,
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
        surf: Option<(&MapGpu, &SurfGpu)>,
    ) -> Self {
        use wgpu::BufferUsages as Usage;
        let count = u64::from(mailbox.limits.points);
        let surf_source = if surf.is_some() {
            map_gpu::declarations_at(SURF_GROUP)
                + &SurfGpu::sheet_source(SURF_GROUP, SURF_BINDING)
                + &shoaling::source()
                + include_str!("../surf/cut.wgsl")
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
        let surf_layout = surf.map(|_| {
            let mut entries = map_gpu::bindings(visibility).to_vec();
            entries.extend(SurfGpu::layout(SURF_BINDING, visibility));
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("water query surf"),
                entries: &entries,
            })
        });
        let coarse_layout = surf.map(|_| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("water query coarse lattice"),
                entries: &[
                    binding::storage(0, visibility, false, COARSE_RECORD_BYTES),
                    binding::uniform(1, visibility, shoaling::BYTES),
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
        let surf = surf.zip(surf_layout.as_ref()).map(|((map, surf), layout)| {
            let view = map.view();
            let mut entries = map.entries(&view).to_vec();
            entries.extend(surf.entries(SURF_BINDING));
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("water query surf"),
                layout,
                entries: &entries,
            })
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("water body query"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("sample_query"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let coarse = coarse_layout.map(|layout| CoarseGpu {
            device: (**device).clone(),
            layout,
            shoaling: shoaling::buffer(device),
            pipeline: device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("water query coarse lattice"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("sample_coarse"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            }),
        });
        let slots = std::array::from_fn(|_| {
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
                    resource: wgpu::BindingResource::TextureView(&ambient.filtered_views()[0]),
                },
            ];
            if let Some(wake) = wake {
                entries.extend(wake.sample_entries(5));
            }
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("water query bindings"),
                layout: &layout,
                entries: &entries,
            });
            Slot {
                group,
                settings,
                components,
                points,
                answers,
                buffer: staging,
                busy: Arc::default(),
                coarse: coarse
                    .as_ref()
                    .map(|gpu| CoarseScratch::new(gpu, COARSE_RECORD_BYTES)),
            }
        });
        mailbox.reset_readback();
        Self {
            mailbox,
            pipeline,
            surf,
            coarse,
            slots,
            pending: Vec::with_capacity(SLOT_COUNT),
            submitted: None,
        }
    }

    pub(crate) fn declare(
        ambient: &thrum_sea::Ambient,
        graph: &mut EffectGraph,
        wake: Option<EffectAccess>,
    ) {
        graph.passes.push(EffectPass {
            name: "water body query".into(),
            // The graph has no retained read of this output. The contribution
            // itself controls execution and the staging copy as one operation.
            cadence: EffectCadence::Skippable,
            async_eligible: false,
            accesses: ambient
                .filtered_accesses()
                .into_iter()
                .take(1)
                .chain(wake)
                .collect(),
        });
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPU parameters use local f32 metres and bounded tick offsets"
    )]
    pub(crate) fn record(
        &mut self,
        queue: &wgpu::Queue,
        time: EffectTime,
        spectrum: &Spectrum,
        chop: f64,
        heights: [f32; 4],
        map: &Map,
        wake: Option<&Interaction>,
    ) {
        // A recording that never reached submit did not start a map.
        self.pending.clear();
        if time.capture {
            return;
        }
        if self.submitted.is_some_and(|(epoch, timeline, tick)| {
            epoch != time.epoch || timeline != time.timeline || tick > time.tick
        }) {
            self.mailbox.reset_readback();
            self.submitted = None;
        }
        let mut state = self.mailbox.state.lock().expect("water query mailbox");
        let slots = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| !slot.busy.load(Ordering::Acquire))
            .map(|(index, _)| index);
        while state.pages.front().is_some_and(|page| {
            page.tick < time.tick && time.tick - page.tick >= u64::from(self.mailbox.limits.max_age)
        }) {
            let page = state.pages.pop_front().expect("expired query page");
            state.stats.unavailable += page.bodies.len() as u64;
        }
        let mut pages = state.pages.iter().take_while(|page| page.tick <= time.tick);
        let work: Vec<_> = slots
            .zip(&mut pages)
            .map(|(slot, page)| (slot, page.tick, page.bodies.clone()))
            .collect();
        if pages.next().is_some() {
            state.stats.busy += 1;
        }
        if work.is_empty() {
            return;
        }
        let epoch = state.epoch;
        let maps = state.maps.clone();
        drop(state);
        let seconds = thrum_sea::display_seconds(time);
        let frame_offset = (1.0 - f64::from(time.alpha)) * time.tick_duration.as_secs_f64();
        let lengths_chop = std::array::from_fn(|i| {
            spectrum
                .cascades
                .get(i)
                .map_or(chop, thrum_sea_model::Cascade::length) as f32
        });
        let profile = map.profile();
        let mut reach = map.surf_throw_reach(&spectrum.long, chop);
        for (slot, tick, bodies) in work {
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
                    // A future use tick holds the latest authored stage. A
                    // known past tick uses its saved stage and connectivity.
                    let index = maps.partition_point(|(tick, _)| *tick <= point.use_tick);
                    let domain = if point.use_tick < time.tick && index > 0 {
                        &maps[index - 1].1
                    } else {
                        map
                    };
                    let mut input = Point::new(point, dt, domain);
                    if let Some(wake) = wake {
                        input.dent = wake.dent(point.use_tick.min(time.tick), id, point.position);
                    }
                    (input, domain.spatial_revision())
                })
                .unzip();
            let count = u32::try_from(points.len()).expect("points admitted by query capacity");
            let buffers = &mut self.slots[slot];
            let coarse_groups = if let Some((gpu, (reach, profile))) = self
                .coarse
                .as_ref()
                .zip(reach.as_mut().zip(map.surf_profile()))
            {
                let limits = gpu.device.limits();
                let max_bytes = limits
                    .max_storage_buffer_binding_size
                    .min(limits.max_buffer_size);
                let plan = CoarsePlan::new(
                    &mut points,
                    reach,
                    profile.cut,
                    max_bytes / COARSE_RECORD_BYTES,
                    limits.max_compute_workgroups_per_dimension,
                );
                let scratch = buffers.coarse.as_mut().expect("surf scratch");
                let bytes = plan.records * COARSE_RECORD_BYTES;
                if scratch.buffer.size() < bytes {
                    *scratch = CoarseScratch::new(gpu, bytes.next_power_of_two().min(max_bytes));
                }
                if !plan.jobs.is_empty() {
                    queue.write_buffer(&scratch.buffer, 0, bytemuck::cast_slice(&plan.jobs));
                }
                u32::try_from(plan.jobs.len()).expect("bounded coarse job count")
            } else {
                0
            };
            let settings = Settings {
                origin,
                count,
                components: u32::try_from(components.len()).expect("bounded component count"),
                lengths_chop,
                heights,
                coarse_jobs: [coarse_groups, 0, 0, 0],
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
                tick,
                epoch,
                frame: time,
                seconds,
                count,
                bodies,
                map_revisions,
                coarse_groups,
            });
        }
    }

    pub(crate) fn pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(crate) fn resource_bytes(&self) -> Vec<(String, u64)> {
        let mut bytes = vec![
            (
                "water query inputs".into(),
                self.slots
                    .iter()
                    .map(|slot| slot.settings.size() + slot.components.size() + slot.points.size())
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
        if let Some(gpu) = &self.coarse {
            bytes.push((
                "water query coarse lattice".into(),
                self.slots
                    .iter()
                    .filter_map(|slot| slot.coarse.as_ref())
                    .map(|scratch| scratch.buffer.size())
                    .sum::<u64>()
                    + gpu.shoaling.size(),
            ));
        }
        bytes
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        for batch in &self.pending {
            let slot = &self.slots[batch.slot];
            pass.set_bind_group(0, &slot.group, &[]);
            if let Some(surf) = &self.surf {
                pass.set_bind_group(SURF_GROUP, surf, &[]);
                pass.set_bind_group(
                    COARSE_GROUP,
                    &slot.coarse.as_ref().expect("surf scratch").group,
                    &[],
                );
                if batch.coarse_groups != 0 {
                    pass.set_pipeline(
                        &self.coarse.as_ref().expect("surf coarse pipeline").pipeline,
                    );
                    pass.dispatch_workgroups(batch.coarse_groups, 1, 1);
                }
            }
            pass.set_pipeline(&self.pipeline);
            let groups = if self.surf.is_some() {
                batch.count
            } else {
                batch.count.div_ceil(GROUP_WIDTH)
            };
            pass.dispatch_workgroups(groups, 1, 1);
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
        self.submitted = Some((last.frame.epoch, last.frame.timeline, last.frame.tick));
        {
            let mut state = self.mailbox.state.lock().expect("water query mailbox");
            if state.epoch == last.epoch {
                while state
                    .pages
                    .front()
                    .is_some_and(|page| page.tick <= last.tick)
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
            input.has_bed = 1;
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
                        sources[4] = Some(
                            point.use_tick.min(self.frame.tick) as f64
                                * self.frame.tick_duration.as_secs_f64(),
                        );
                    }
                    (
                        raw.decode(index, point, sources),
                        MapPrediction::new(revision, raw.cascades, raw.wake),
                    )
                })
                .unzip();
            state.ready.insert(
                (body.id, self.tick),
                ReadyBody {
                    answers: BodyAnswers {
                        id: body.id,
                        generation: body.generation,
                        posted: self.tick,
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
    ) -> QueryAnswer {
        let [body, wet, has_bed, _] = self.body_flags;
        let [height, sx, sz, height_dt] = self.height_slope_rate.map(f64::from);
        let [vx, vy, vz, depth] = self.velocity_depth.map(f64::from);
        let [cx, cz, bed, _] = self.current_bed;
        QueryAnswer {
            index,
            point,
            sources,
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
) -> Result<(), String> {
    let groups = if surf {
        limits.points
    } else {
        limits.points.div_ceil(GROUP_WIDTH)
    };
    if u64::from(limits.points) * size_of::<Answer>() as u64
        > device.max_storage_buffer_binding_size
        || groups > device.max_compute_workgroups_per_dimension
    {
        return Err("water query exceeds the device buffer or dispatch limit".into());
    }
    Ok(())
}
