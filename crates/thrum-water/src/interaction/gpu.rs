use super::{
    BodyPressure, CELL_METRES, CELLS, DENSITY, FAR, FINE, FINE_TICKS, FineEndpoint, FineReport,
    GRAVITY, GRID_CELLS, GRID_WIDTH, MAX_TICKS, Mailbox, NEAR, Report, SIDE, Witness,
    regions::{Cell, DEPTHS, DRY, REST, RegionActivity, WetHollows, Window},
    static_dent,
};
use bytemuck::{Pod, Zeroable};
use thrum_render::binding;
use thrum_render::{
    contribution::{
        ContributionFrame, EffectAccess, EffectCadence, EffectGraph, EffectPass, EffectTime,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};
use thrum_water_model::{CAPILLARY_COEFFICIENT, Map};
use web_time::Instant;

const PLANE_BYTES: u64 = (CELLS * size_of::<[f32; 2]>()) as u64;
const GROUPS: u32 = GRID_CELLS / 64;
const PASS: &str = "interaction waves";
const BASE_LEVELS: u32 = 2;
const BASE_PLANES: u32 = BASE_LEVELS * 2;
const FINE_SCRATCH: u32 = BASE_PLANES + 2;
const FINE_SPLIT: u32 = 1 << 2;

fn fine_base(bank: u32) -> u32 {
    (BASE_PLANES + bank) * GRID_CELLS
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Step {
    shift: [i32; 2],
    current: [f32; 2],
    old_base: u32,
    new_base: u32,
    cell_base: u32,
    region: u32,
    source_first: u32,
    source_count: u32,
    depth_index: u32,
    level: u32,
    metres: f32,
    depth: f32,
    seconds: f32,
    ticks: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Patch {
    place: [f32; 2],
    axis: [f32; 2],
    footprint: [f32; 2],
    impulse: f32,
    padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SampleSettings {
    origins: [[f32; 4]; 4],
    clocks: [f32; 4],
    bases: [u32; 4],
    fine_origins: [[f32; 4]; 2],
    fine_clock: [f32; 4],
    fine_bases: [u32; 4],
}

#[derive(Clone, Copy)]
struct Endpoint {
    origins: [[i32; 2]; 3],
    tick: u64,
    bank: u32,
    live: [bool; 3],
}

impl Endpoint {
    #[expect(
        clippy::cast_precision_loss,
        reason = "enclose the wake shader's rounded f32 origin and extent"
    )]
    fn overlaps_live_window(self, [low, high]: [[f64; 2]; 2]) -> bool {
        self.live
            .into_iter()
            .zip(self.origins)
            .zip(CELL_METRES)
            .any(|((live, origin), metres)| {
                if !live {
                    return false;
                }
                // Match the shader's f32 world origin before enclosing the
                // region in which it can interpolate the endpoint.
                (0..2).all(|axis| {
                    let start = f64::from(origin[axis] as f32 * metres);
                    let end = start + f64::from((GRID_WIDTH - 1) as f32 * metres);
                    let rounding =
                        f64::from(f32::EPSILON) * 4.0 * start.abs().max(end.abs()).max(1.0);
                    high[axis] >= start - rounding && low[axis] <= end + rounding
                })
            })
    }
}

#[derive(Clone, Copy)]
struct Frame {
    endpoint: [Endpoint; 2],
    fine_previous: FineEndpoint,
    time: EffectTime,
    generation: u64,
    follow: [f64; 2],
    direction: [f64; 2],
    report: Report,
}

struct Dispatch {
    offset: u32,
    load: bool,
}

struct Source {
    patch: Patch,
    region: u32,
    tick: u64,
    body: Option<usize>,
}

struct Pressure {
    tick: u64,
    body: BodyPressure,
    levels: u32,
}

pub(crate) struct Interaction {
    pub(super) mailbox: Mailbox,
    pub(crate) endpoints: wgpu::Buffer,
    pub(crate) sample: wgpu::Buffer,
    cells: wgpu::Buffer,
    patches: wgpu::Buffer,
    steps: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipelines: [wgpu::ComputePipeline; 6],
    stride: usize,
    initial_report: Report,
    committed: Option<Frame>,
    recorded: Option<Frame>,
    windows: [Window; 3],
    wet_hollows: WetHollows,
    committed_activity: [RegionActivity; 3],
    recorded_activity: [RegionActivity; 3],
    dispatches: Vec<Dispatch>,
    step_bytes: Vec<u8>,
    patch_rows: Vec<Patch>,
    candidates: Vec<Source>,
    regions: Vec<(u32, u64, usize)>,
    committed_pressures: [Vec<Pressure>; 2],
    recorded_pressures: Vec<Pressure>,
}

fn buffer(device: &DeviceSite, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

pub(crate) fn shader_constants() -> String {
    format!(
        "const WAKE_SIDE: u32 = {SIDE}u;\nconst REST_DEPTH: u32 = {REST}u;\nconst WATER_GRAVITY: f32 = {GRAVITY:?};\nconst WATER_DENSITY: f32 = {DENSITY:?};\nconst WAKE_DIVIDER_METRES: f32 = {:?};\nconst FINE_DIVIDER_METRES: f32 = {:?};\nconst CAPILLARY_COEFFICIENT: f32 = {CAPILLARY_COEFFICIENT:?};\nconst FINE_SPLIT: u32 = {FINE_SPLIT}u;\n",
        static_dent::DIVIDER_METRES,
        static_dent::FINE_DIVIDER_METRES
    )
}

impl Interaction {
    pub(crate) fn sample_source(group: u32, storage: u32, uniform: u32, enabled: bool) -> String {
        if !enabled {
            return "fn wake_at(_point: vec2<f32>, _dt: f32) -> vec4<f32> { return vec4(0.0); }"
                .into();
        }
        shader_constants()
            + &format!(
                "@group({group}) @binding({storage}) var<storage, read> wake_endpoints: array<vec2<f32>>;\n@group({group}) @binding({uniform}) var<uniform> wake_settings: WakeSettings;\n"
            )
            + include_str!("sample.wgsl")
    }

    pub(crate) fn sample_layout(
        first: u32,
        visibility: wgpu::ShaderStages,
    ) -> [wgpu::BindGroupLayoutEntry; 2] {
        let layout = [
            wgpu::BufferBindingType::Storage { read_only: true },
            wgpu::BufferBindingType::Uniform,
        ];
        [0_u32, 1]
            .map(|index| binding::buffer(first + index, visibility, layout[index as usize], 0))
    }

    pub(crate) fn sample_entries(&self, first: u32) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: first,
                resource: self.endpoints.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: first + 1,
                resource: self.sample.as_entire_binding(),
            },
        ]
    }

    pub(crate) fn query_entries<'a>(
        &'a self,
        settings: &'a wgpu::Buffer,
    ) -> [wgpu::BindGroupEntry<'a>; 2] {
        let mut entries = self.sample_entries(5);
        entries[1].resource = settings.as_entire_binding();
        entries
    }

    pub(crate) fn query_settings(&self) -> Option<(u64, SampleSettings)> {
        let frame = self.committed.as_ref()?;
        let tick = frame.endpoint[1].tick;
        let settings = self.sample_settings(frame, frame.time, true);
        Some((tick, settings))
    }

    pub(crate) fn declare(&self, graph: &mut EffectGraph) -> EffectAccess {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: self.endpoints.size(),
                usage: self.endpoints.usage(),
            },
            ResourceScope::Persistent {
                owner: 0x7761_6b65,
                epoch: 0,
                initialized: true,
            },
        ));
        let read = EffectAccess {
            resource,
            range: 0..self.endpoints.size(),
            kind: AccessKind::StorageRead,
            view: None,
            history: None,
        };
        graph.passes.push(EffectPass {
            name: PASS.into(),
            cadence: EffectCadence::Skippable,
            async_eligible: false,
            accesses: vec![EffectAccess {
                kind: AccessKind::StorageReadWrite,
                ..read.clone()
            }],
        });
        read
    }

    pub(crate) fn resource_bytes(&self) -> [(String, u64); 6] {
        [
            ("interaction wave endpoints".into(), self.endpoints.size()),
            ("interaction transform workspace".into(), 6 * PLANE_BYTES),
            ("interaction window cells".into(), self.cells.size()),
            ("interaction pressure patches".into(), self.patches.size()),
            ("interaction step settings".into(), self.steps.size()),
            ("interaction sample settings".into(), self.sample.size()),
        ]
    }

    pub(crate) fn new(device: &DeviceSite, mailbox: Mailbox, initial_report: Report) -> Self {
        use wgpu::BufferUsages as Usage;
        let stride =
            (device.limits().min_uniform_buffer_offset_alignment as usize).max(size_of::<Step>());
        let plane_count = if mailbox.fine {
            FINE_SCRATCH + 1
        } else {
            BASE_PLANES
        };
        let levels = BASE_LEVELS + u32::from(mailbox.fine);
        let level_steps = u64::from(BASE_LEVELS) + if mailbox.fine { FINE_TICKS } else { 0 };
        let endpoints = buffer(
            device,
            "interaction wave endpoints",
            u64::from(plane_count) * PLANE_BYTES,
            Usage::STORAGE | Usage::COPY_DST | Usage::COPY_SRC,
        );
        let work = buffer(
            device,
            "interaction transform workspace",
            6 * PLANE_BYTES,
            Usage::STORAGE,
        );
        let sample = buffer(
            device,
            "interaction sample settings",
            size_of::<SampleSettings>() as u64,
            Usage::UNIFORM | Usage::COPY_DST,
        );
        let cells = buffer(
            device,
            "interaction window cells",
            u64::from(levels) * (CELLS * size_of::<Cell>()) as u64,
            Usage::STORAGE | Usage::COPY_DST,
        );
        let patches = buffer(
            device,
            "interaction pressure patches",
            (u64::from(mailbox.limits.bodies + mailbox.limits.impulses)
                * (MAX_TICKS * u64::from(BASE_LEVELS) + if mailbox.fine { FINE_TICKS } else { 0 }))
            .max(1)
                * size_of::<Patch>() as u64,
            Usage::STORAGE | Usage::COPY_DST,
        );
        let steps = buffer(
            device,
            "interaction step settings",
            stride as u64 * level_steps * u64::from(mailbox.limits.solves),
            Usage::UNIFORM | Usage::COPY_DST,
        );
        let layout_entries = [
            wgpu::BufferBindingType::Uniform,
            wgpu::BufferBindingType::Storage { read_only: false },
            wgpu::BufferBindingType::Storage { read_only: false },
            wgpu::BufferBindingType::Storage { read_only: true },
            wgpu::BufferBindingType::Storage { read_only: true },
        ];
        let step_size = std::num::NonZeroU64::new(size_of::<Step>() as u64);
        let entries: Vec<_> = layout_entries
            .into_iter()
            .zip(0_u32..)
            .map(|(ty, binding)| {
                if binding == 0 {
                    // Binding 0 takes a dynamic offset, so it keeps a literal.
                    wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty,
                            has_dynamic_offset: true,
                            min_binding_size: step_size,
                        },
                        count: None,
                    }
                } else {
                    binding::buffer(binding, wgpu::ShaderStages::COMPUTE, ty, 0)
                }
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &entries,
        });
        let resources = [
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &steps,
                offset: 0,
                size: step_size,
            }),
            endpoints.as_entire_binding(),
            work.as_entire_binding(),
            cells.as_entire_binding(),
            patches.as_entire_binding(),
        ];
        let entries: Vec<_> = resources
            .into_iter()
            .zip(0_u32..)
            .map(|(resource, binding)| wgpu::BindGroupEntry { binding, resource })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let source = shader_constants() + include_str!("step.wgsl");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipelines = [
            "forward_row",
            "forward_column",
            "advance",
            "inverse_row",
            "inverse_column",
            "blend_depth",
        ]
        .map(|entry| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(PASS),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        });
        Self {
            mailbox,
            endpoints,
            sample,
            cells,
            patches,
            steps,
            group,
            pipelines,
            stride,
            initial_report,
            committed: None,
            recorded: None,
            windows: Default::default(),
            wet_hollows: WetHollows::default(),
            committed_activity: Default::default(),
            recorded_activity: Default::default(),
            dispatches: Vec::new(),
            step_bytes: Vec::new(),
            patch_rows: Vec::new(),
            candidates: Vec::new(),
            regions: Vec::new(),
            committed_pressures: Default::default(),
            recorded_pressures: Vec::new(),
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "window coordinates are clamped and packed in bounded GPU records"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "the tentative endpoint transition and its per-level dispatch record are one transaction"
    )]
    pub(crate) fn record(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &ContributionFrame,
        map: &Map,
    ) {
        self.recorded = None;
        self.dispatches.clear();
        self.recorded_pressures.clear();
        let time = frame.time;
        if time.capture {
            if let Some(committed) = self.committed {
                self.write_sample(queue, &committed, time);
            }
            return;
        }
        let reset = self.committed.is_none_or(|old| {
            time.epoch != old.time.epoch
                || time.timeline != old.time.timeline
                || time.tick < old.time.tick
                || time.tick_duration != old.time.tick_duration
                || frame.generation != old.generation
        });
        if !reset && let Some(old) = self.committed.filter(|old| old.time.tick == time.tick) {
            self.write_sample(queue, &old, time);
            return;
        }
        let mailbox = self.mailbox.clone();
        let pages = mailbox.pages.lock().expect("water interaction mailbox");
        let page = pages.bodies.get(&time.tick);
        let previous = self.committed.map(|old| old.endpoint[1]);
        let first = if reset {
            time.tick
        } else {
            previous
                .expect("previous wake")
                .tick
                .saturating_add(1)
                .max(time.tick.saturating_sub(MAX_TICKS - 1))
        };
        let impact = pages
            .impulses
            .range(first..=time.tick)
            .rev()
            .find_map(|(_, page)| page.impulses.first());
        let named =
            page.and_then(|page| page.bodies.iter().find(|body| Some(body.id) == page.follow));
        let mut origin = self.committed.map_or([0.0; 2], |old| old.follow);
        let mut direction = self.committed.map_or([0.0; 2], |old| old.direction);
        if let Some(body) = named {
            if !reset {
                let delta = [body.place[0] - origin[0], body.place[1] - origin[1]];
                let distance = delta[0].hypot(delta[1]);
                if distance > 0.001 {
                    // The window stays ahead of centre without flipping as the
                    // hull pitches, pauses or reverses heading within a tick.
                    for (axis, movement) in direction.iter_mut().zip(delta) {
                        *axis = 0.96 * *axis + 0.04 * movement / distance;
                    }
                }
            }
            origin = body.place;
        } else if let Some(impulse) = impact {
            origin = impulse.place;
            direction = [0.0; 2];
        }
        let origins = CELL_METRES.map(|metres| {
            std::array::from_fn(|axis| {
                let anchor = (origin[axis] / f64::from(metres)).clamp(-1.0e8, 1.0e8);
                (anchor - direction[axis] * (SIDE as f64 / 8.0)).floor() as i32
                    - i32::try_from(SIDE / 2).expect("window half width")
            })
        });
        let mut report = self.committed.map_or(self.initial_report, |old| old.report);
        let mut next = Endpoint {
            origins,
            tick: time.tick,
            bank: previous.map_or(0, |old| 1 - old.bank),
            live: [false; 3],
        };
        let mut fine_report = self.mailbox.fine.then(|| FineReport {
            dropped_ticks: report.fine.unwrap_or_default().dropped_ticks,
            refused_regions: report.fine.unwrap_or_default().refused_regions,
            ..FineReport::default()
        });
        let mut fine_previous = Self::fine_endpoint(next);
        let endpoint = if reset {
            for activity in &mut self.recorded_activity {
                activity.reset();
            }
            encoder.clear_buffer(&self.endpoints, 0, None);
            report.resets += 1;
            report.regions = [0; 2];
            report.solves = [0; 2];
            report.labelling_ms = 0.0;
            [
                Endpoint {
                    bank: 1 - next.bank,
                    ..next
                },
                next,
            ]
        } else {
            let previous = previous.expect("previous wake");
            next.live = previous.live;
            let elapsed = time.tick - previous.tick;
            let ticks = elapsed.min(MAX_TICKS);
            report.dropped_ticks += elapsed - ticks;
            report.refused_bodies += pages
                .bodies
                .range(first..=time.tick)
                .map(|(_, p)| p.refused_bodies)
                .sum::<u64>();
            report.refused_impulses += pages
                .impulses
                .range(first..=time.tick)
                .map(|(_, p)| p.refused_impulses)
                .sum::<u64>();
            report.labelling_ms = 0.0;
            encoder.clear_buffer(
                &self.endpoints,
                u64::from(next.bank * BASE_LEVELS) * PLANE_BYTES,
                Some(PLANE_BYTES * u64::from(BASE_LEVELS)),
            );
            if self.mailbox.fine {
                encoder.clear_buffer(
                    &self.endpoints,
                    u64::from(BASE_PLANES + next.bank) * PLANE_BYTES,
                    Some(PLANE_BYTES),
                );
                if elapsed >= FINE_TICKS {
                    encoder.clear_buffer(
                        &self.endpoints,
                        u64::from(FINE_SCRATCH) * PLANE_BYTES,
                        Some(PLANE_BYTES),
                    );
                }
                fine_previous = Self::fine_endpoint(previous);
            }
            self.step_bytes.clear();
            self.patch_rows.clear();
            for (&tick, page) in pages.bodies.range(first..=time.tick) {
                self.recorded_pressures
                    .extend(page.bodies.iter().map(|&body| Pressure {
                        tick,
                        body,
                        levels: 0,
                    }));
            }
            let fine_ticks = if let Some(fine) = &mut fine_report {
                let steps = elapsed.min(FINE_TICKS);
                fine.steps = steps as u32;
                fine.dropped_ticks += elapsed - steps;
                steps
            } else {
                0
            };
            // Both fine substeps use this frame's published wet support and window.
            // At a hitch, the most recent two source ticks survive, as on the other levels.
            let start = Instant::now();
            self.wet_hollows.update(map);
            // On the fine profile, a yard window overlaps the ambient bodies
            // as well as several small puddles. Never transform an untouched
            // region's zero state merely because it is inside that window.
            let track_hollows = mailbox.fine && self.wet_hollows.is_present();
            report.labelling_ms += start.elapsed().as_secs_f64() * 1000.0;
            for (level, first, last) in [(NEAR, first, time.tick), (FAR, first, time.tick)]
                .into_iter()
                .chain((0..fine_ticks).map(|step| {
                    let tick = time.tick - (fine_ticks - 1 - step);
                    (FINE, tick, tick)
                }))
            {
                let fine = level == FINE;
                let track_activity = fine || track_hollows;
                let activity = &mut self.recorded_activity[level];
                let second = fine && fine_ticks == 2 && last == time.tick;
                let to_scratch = fine && fine_ticks == 2 && !second;
                let start = Instant::now();
                let window = &mut self.windows[level];
                let relabelled = window.label(
                    map,
                    &self.wet_hollows,
                    origins[level],
                    CELL_METRES[level],
                    level,
                );
                if track_activity && !second {
                    activity.inherit(&self.committed_activity[level], window);
                }
                if relabelled {
                    report.labelling_ms += start.elapsed().as_secs_f64() * 1000.0;
                    queue.write_buffer(
                        &self.cells,
                        (level * CELLS * size_of::<Cell>()) as u64,
                        bytemuck::cast_slice(&window.cells),
                    );
                }
                let metres = CELL_METRES[level];
                let local = |world: [f64; 2]| {
                    std::array::from_fn(|axis| {
                        (world[axis] / f64::from(metres) - f64::from(origins[level][axis])) as f32
                    })
                };
                let dt = time.tick_duration.as_secs_f32();
                self.candidates.clear();
                let pressure_start = self.recorded_pressures.partition_point(|p| p.tick < first);
                let mut pressures = self
                    .recorded_pressures
                    .iter()
                    .enumerate()
                    .skip(pressure_start)
                    .peekable();
                for tick in first..=last {
                    while let Some((index, pressure)) =
                        pressures.next_if(|(_, pressure)| pressure.tick == tick)
                    {
                        let body = pressure.body;
                        if track_activity && body.force == 0.0 {
                            continue;
                        }
                        let place = local(body.place);
                        let (sin, cos) = body.heading.sin_cos();
                        self.candidates.push(Source {
                            patch: Patch {
                                place,
                                axis: [cos as f32, sin as f32],
                                footprint: body.footprint.map(|v| v as f32),
                                impulse: body.force as f32 * dt,
                                padding: 0.0,
                            },
                            region: window.source_region(place, body.place, &self.wet_hollows),
                            tick,
                            body: Some(index),
                        });
                    }
                    for impulse in pages
                        .impulses
                        .get(&tick)
                        .into_iter()
                        .flat_map(|page| &page.impulses)
                    {
                        let place = local(impulse.place);
                        self.candidates.push(Source {
                            patch: Patch {
                                place,
                                axis: [1.0, 0.0],
                                footprint: [impulse.radius as f32; 2],
                                impulse: impulse.vertical as f32,
                                padding: 0.0,
                            },
                            region: window.source_region(place, impulse.place, &self.wet_hollows),
                            tick,
                            body: None,
                        });
                    }
                }
                let priority = window.region(local(origin));
                self.regions.clear();
                self.regions
                    .extend(window.depths.iter().enumerate().map(|(region, _)| {
                        let latest = if track_activity {
                            activity.latest[region].unwrap_or(0)
                        } else {
                            0
                        };
                        (region as u32, latest, 0)
                    }));
                for source in &self.candidates {
                    if track_activity && source.patch.impulse == 0.0 {
                        continue;
                    }
                    if let Some((_, newest, count)) = self.regions.get_mut(source.region as usize) {
                        *newest = (*newest).max(source.tick);
                        *count += 1;
                    }
                }
                self.regions.sort_unstable_by_key(|&(region, tick, _)| {
                    (region != priority, std::cmp::Reverse(tick), region)
                });
                let mut remaining = self.mailbox.limits.solves;
                for &(region, latest, count) in &self.regions {
                    if track_activity && latest == 0 {
                        continue;
                    }
                    let depths = window.depths[region as usize];
                    if depths.count_ones() > remaining {
                        if track_activity {
                            activity.latest[region as usize] = None;
                        }
                        if let Some(fine) = fine_report.as_mut().filter(|_| fine) {
                            fine.refused_regions += count as u64;
                        } else {
                            report.refused_regions[level] += count as u64;
                        }
                        continue;
                    }
                    remaining -= depths.count_ones();
                    if track_activity {
                        activity.latest[region as usize] = Some(latest);
                    }
                    let source_first = self.patch_rows.len() as u32;
                    for source in self
                        .candidates
                        .iter()
                        .filter(|source| source.region == region)
                    {
                        self.patch_rows.push(source.patch);
                        next.live[level] |= depths != 0 && source.patch.impulse != 0.0;
                        if let Some(body) = source.body {
                            self.recorded_pressures[body].levels |= 1 << level;
                        }
                    }
                    let mut load = true;
                    for (index, depth) in DEPTHS[level].iter().copied().enumerate() {
                        if depths & (1 << index) == 0 {
                            continue;
                        }
                        let offset = self.step_bytes.len();
                        self.step_bytes.resize(offset + self.stride, 0);
                        let step = Step {
                            shift: if second {
                                [0; 2]
                            } else {
                                std::array::from_fn(|axis| {
                                    origins[level][axis] - previous.origins[level][axis]
                                })
                            },
                            current: window.current,
                            old_base: if second {
                                FINE_SCRATCH * GRID_CELLS
                            } else {
                                Self::level_base(previous.bank, level)
                            },
                            new_base: if to_scratch {
                                FINE_SCRATCH * GRID_CELLS
                            } else {
                                Self::level_base(next.bank, level)
                            },
                            cell_base: (level * CELLS) as u32,
                            region,
                            source_first,
                            source_count: self.patch_rows.len() as u32 - source_first,
                            depth_index: index as u32,
                            level: level as u32 | if mailbox.fine { FINE_SPLIT } else { 0 },
                            metres,
                            depth: if index + 1 == DEPTHS[level].len() {
                                0.0
                            } else {
                                depth
                            },
                            seconds: if fine { dt } else { dt * ticks as f32 },
                            ticks: if fine { 1.0 } else { ticks as f32 },
                        };
                        self.step_bytes[offset..offset + size_of::<Step>()]
                            .copy_from_slice(bytemuck::bytes_of(&step));
                        self.dispatches.push(Dispatch {
                            offset: offset as u32,
                            load,
                        });
                        load = false;
                    }
                }
                let regions = window.depths.len() as u32;
                let solves = self.mailbox.limits.solves - remaining;
                if let Some(fine) = fine_report.as_mut().filter(|_| fine) {
                    fine.regions = regions;
                    fine.solves += solves;
                    next.live[FINE] = solves != 0;
                } else {
                    report.regions[level] = regions;
                    report.solves[level] = solves;
                }
                if to_scratch {
                    fine_previous = FineEndpoint {
                        tick: last,
                        offset: (FINE_SCRATCH * GRID_CELLS) as usize,
                        ..Self::fine_endpoint(next)
                    };
                }
                debug_assert!(!self.regions.iter().any(|&(region, _, _)| region == DRY));
            }
            if !self.step_bytes.is_empty() {
                queue.write_buffer(&self.steps, 0, &self.step_bytes);
            }
            if !self.patch_rows.is_empty() {
                queue.write_buffer(&self.patches, 0, bytemuck::cast_slice(&self.patch_rows));
            }
            [previous, next]
        };
        report.endpoints = Some(endpoint.map(|bank| bank.tick));
        report.fine = fine_report;
        let recorded = Frame {
            endpoint,
            fine_previous,
            time,
            generation: frame.generation,
            follow: origin,
            direction,
            report,
        };
        self.recorded = Some(recorded);
        self.write_sample(queue, &recorded, time);
    }

    pub(crate) fn pending(&self) -> bool {
        !self.dispatches.is_empty()
    }

    pub(crate) fn live_wake_intersects(&self, rest: [[f64; 2]; 2]) -> bool {
        self.recorded.or(self.committed).is_some_and(|frame| {
            frame
                .endpoint
                .into_iter()
                .any(|end| end.overlaps_live_window(rest))
        })
    }

    pub(crate) fn dent(&self, tick: u64, body: u32, point: [f32; 2]) -> [f32; 4] {
        let Some(frame) = self.committed else {
            return [0.0; 4];
        };
        let [previous, current] = frame.endpoint;
        if previous.tick == current.tick {
            return [0.0; 4];
        }
        let tick = tick.clamp(previous.tick, current.tick);
        let pressures = if tick == previous.tick {
            &self.committed_pressures[previous.bank as usize]
        } else {
            &self.committed_pressures[current.bank as usize]
        };
        let Ok(index) = pressures.binary_search_by_key(&(tick, body), |p| (p.tick, p.body.id))
        else {
            return [0.0; 4];
        };
        let pressure = &pressures[index];
        let endpoint = if tick == previous.tick {
            previous
        } else {
            current
        };
        static_dent::at(
            pressure.body,
            pressure.levels,
            point.map(f64::from),
            endpoint.origins,
            self.mailbox.fine,
        )
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        for dispatch in &self.dispatches {
            pass.set_bind_group(0, &self.group, &[dispatch.offset]);
            let first = if dispatch.load { 0 } else { 2 };
            for (pipeline, groups) in self.pipelines[first..].iter().zip(
                [
                    GRID_WIDTH, GRID_WIDTH, GROUPS, GRID_WIDTH, GRID_WIDTH, GROUPS,
                ][first..]
                    .iter(),
            ) {
                pass.set_pipeline(pipeline);
                pass.dispatch_workgroups(*groups, 1, 1);
            }
        }
    }

    pub(crate) fn submit(&mut self) -> Option<Report> {
        if let Some(new) = self.recorded.take() {
            let reseek = self.committed.is_some_and(|old| {
                old.time.epoch != new.time.epoch
                    || old.time.timeline != new.time.timeline
                    || old.time.tick > new.time.tick
            });
            let mut pages = self
                .mailbox
                .pages
                .lock()
                .expect("water interaction mailbox");
            if reseek {
                pages.bodies.retain(|tick, _| *tick == new.time.tick);
            }
            pages.retire_impulses(new.time.tick, reseek);
            if new.endpoint[0].tick == new.endpoint[1].tick {
                for pressures in &mut self.committed_pressures {
                    pressures.clear();
                }
            }
            std::mem::swap(
                &mut self.recorded_pressures,
                &mut self.committed_pressures[new.endpoint[1].bank as usize],
            );
            std::mem::swap(&mut self.committed_activity, &mut self.recorded_activity);
            self.committed = Some(new);
        }
        self.committed.map(|frame| frame.report)
    }

    fn level_base(bank: u32, level: usize) -> u32 {
        if level == FINE {
            fine_base(bank)
        } else {
            (bank * BASE_LEVELS + u32::try_from(level).expect("base level")) * GRID_CELLS
        }
    }

    fn fine_endpoint(endpoint: Endpoint) -> FineEndpoint {
        FineEndpoint {
            tick: endpoint.tick,
            origin: endpoint.origins[FINE].map(|cell| cell as f32 * CELL_METRES[FINE]),
            offset: fine_base(endpoint.bank) as usize,
        }
    }

    pub(crate) fn witness(&self) -> Option<Witness> {
        self.committed.map(|frame| Witness {
            buffer: self.endpoints.clone(),
            endpoints: std::array::from_fn(|i| {
                let endpoint = frame.endpoint[i];
                super::Endpoint {
                    tick: endpoint.tick,
                    origins: std::array::from_fn(|level| {
                        endpoint.origins[level].map(|cell| cell as f32 * CELL_METRES[level])
                    }),
                    offsets: std::array::from_fn(|level| {
                        Self::level_base(endpoint.bank, level) as usize
                    }),
                    fine: self.mailbox.fine.then(|| {
                        if i == 0 {
                            frame.fine_previous
                        } else {
                            Self::fine_endpoint(endpoint)
                        }
                    }),
                }
            }),
        })
    }

    fn write_sample(&self, queue: &wgpu::Queue, frame: &Frame, time: EffectTime) {
        let settings = self.sample_settings(frame, time, false);
        queue.write_buffer(&self.sample, 0, bytemuck::bytes_of(&settings));
    }

    fn sample_settings(&self, frame: &Frame, time: EffectTime, query: bool) -> SampleSettings {
        let endpoint = frame.endpoint;
        let clock = |previous: u64, current: u64| {
            let interval = (current - previous) as f32;
            let from_previous = if time.tick >= previous {
                (time.tick - previous) as f32
            } else {
                -((previous - time.tick) as f32)
            };
            let blend = if query || interval == 0.0 {
                1.0
            } else {
                ((from_previous - 1.0 + time.alpha) / interval).clamp(0.0, 1.0)
            };
            [blend, interval * time.tick_duration.as_secs_f32(), 1.0, 0.0]
        };
        let fine = [frame.fine_previous, Self::fine_endpoint(endpoint[1])];
        SampleSettings {
            origins: std::array::from_fn(|i| {
                let metres = CELL_METRES[i % 2];
                let [x, z] = endpoint[i / 2].origins[i % 2];
                [x as f32 * metres, z as f32 * metres, metres, 0.0]
            }),
            clocks: clock(endpoint[0].tick, endpoint[1].tick),
            bases: std::array::from_fn(|i| Self::level_base(endpoint[i / 2].bank, i % 2)),
            fine_origins: fine.map(|end| [end.origin[0], end.origin[1], CELL_METRES[FINE], 0.0]),
            fine_clock: if self.mailbox.fine {
                clock(fine[0].tick, fine[1].tick)
            } else {
                [0.0; 4]
            },
            fine_bases: [
                u32::try_from(fine[0].offset).expect("fine plane offset"),
                u32::try_from(fine[1].offset).expect("fine plane offset"),
                0,
                0,
            ],
        }
    }
}

#[cfg(test)]
mod tests;
