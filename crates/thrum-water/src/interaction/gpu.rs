use super::{
    BodyPressure, CELL_METRES, CELLS, DENSITY, GRAVITY, GRID_CELLS, GRID_WIDTH, MAX_TICKS, Mailbox,
    Report, SIDE, Witness,
    regions::{Cell, DEPTHS, DRY, REST, Window},
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
use thrum_water_model::Map;
use web_time::Instant;

const PLANE_BYTES: u64 = (CELLS * size_of::<[f32; 2]>()) as u64;
const GROUPS: u32 = GRID_CELLS / 64;
const PASS: &str = "interaction waves";
const LEVELS: u32 = 2;

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
struct SampleSettings {
    origins: [[f32; 4]; 4],
    clocks: [f32; 4],
    bases: [u32; 4],
}

#[derive(Clone, Copy)]
struct Endpoint {
    origins: [[i32; 2]; 2],
    tick: u64,
    bank: u32,
}

#[derive(Clone, Copy)]
struct Frame {
    endpoint: [Endpoint; 2],
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
    windows: [Window; 2],
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
        "const WAKE_SIDE: u32 = {SIDE}u;\nconst REST_DEPTH: u32 = {REST}u;\nconst WATER_GRAVITY: f32 = {GRAVITY:?};\nconst WATER_DENSITY: f32 = {DENSITY:?};\nconst WAKE_DIVIDER_METRES: f32 = {:?};\n",
        static_dent::DIVIDER_METRES
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
        let endpoints = buffer(
            device,
            "interaction wave endpoints",
            4 * PLANE_BYTES,
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
            (2 * CELLS * size_of::<Cell>()) as u64,
            Usage::STORAGE | Usage::COPY_DST,
        );
        let patches = buffer(
            device,
            "interaction pressure patches",
            (u64::from(mailbox.limits.bodies + mailbox.limits.impulses) * MAX_TICKS * 2).max(1)
                * size_of::<Patch>() as u64,
            Usage::STORAGE | Usage::COPY_DST,
        );
        let steps = buffer(
            device,
            "interaction step settings",
            stride as u64 * u64::from(LEVELS * mailbox.limits.solves.max(1)),
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
            .map(|(ty, binding)| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty,
                    has_dynamic_offset: binding == 0,
                    min_binding_size: if binding == 0 { step_size } else { None },
                },
                count: None,
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
                self.write_sample(queue, committed.endpoint, time);
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
            self.write_sample(queue, old.endpoint, time);
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
        let next = Endpoint {
            origins,
            tick: time.tick,
            bank: previous.map_or(0, |old| 1 - old.bank),
        };
        let endpoint = if reset {
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
                u64::from(next.bank) * PLANE_BYTES * 2,
                Some(PLANE_BYTES * 2),
            );
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
            for level in 0..2 {
                let start = Instant::now();
                let window = &mut self.windows[level];
                if window.label(map, origins[level], CELL_METRES[level], level) {
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
                let mut pressures = self.recorded_pressures.iter().enumerate().peekable();
                for tick in first..=time.tick {
                    while let Some((index, pressure)) =
                        pressures.next_if(|(_, pressure)| pressure.tick == tick)
                    {
                        let body = pressure.body;
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
                            region: window.region(place),
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
                            region: window.region(place),
                            tick,
                            body: None,
                        });
                    }
                }
                let priority = window.region(local(origin));
                self.regions.clear();
                self.regions.extend(
                    window
                        .depths
                        .iter()
                        .enumerate()
                        .map(|(region, _)| (region as u32, 0, 0)),
                );
                for source in &self.candidates {
                    if let Some((_, newest, count)) = self.regions.get_mut(source.region as usize) {
                        *newest = (*newest).max(source.tick);
                        *count += 1;
                    }
                }
                self.regions.sort_unstable_by_key(|&(region, tick, _)| {
                    (region != priority, std::cmp::Reverse(tick), region)
                });
                let mut remaining = self.mailbox.limits.solves;
                for &(region, _, count) in &self.regions {
                    let depths = window.depths[region as usize];
                    if depths.count_ones() > remaining {
                        report.refused_regions[level] += count as u64;
                        continue;
                    }
                    remaining -= depths.count_ones();
                    let source_first = self.patch_rows.len() as u32;
                    for source in self
                        .candidates
                        .iter()
                        .filter(|source| source.region == region)
                    {
                        self.patch_rows.push(source.patch);
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
                            shift: std::array::from_fn(|axis| {
                                origins[level][axis] - previous.origins[level][axis]
                            }),
                            current: window.current,
                            old_base: (previous.bank * LEVELS + level as u32) * GRID_CELLS,
                            new_base: (next.bank * LEVELS + level as u32) * GRID_CELLS,
                            cell_base: (level * CELLS) as u32,
                            region,
                            source_first,
                            source_count: self.patch_rows.len() as u32 - source_first,
                            depth_index: index as u32,
                            level: level as u32,
                            metres,
                            depth: if index + 1 == DEPTHS[level].len() {
                                0.0
                            } else {
                                depth
                            },
                            seconds: dt * ticks as f32,
                            ticks: ticks as f32,
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
                report.regions[level] = window.depths.len() as u32;
                report.solves[level] = self.mailbox.limits.solves - remaining;
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
        self.recorded = Some(Frame {
            endpoint,
            time,
            generation: frame.generation,
            follow: origin,
            direction,
            report,
        });
        self.write_sample(queue, endpoint, time);
    }

    pub(crate) fn pending(&self) -> bool {
        !self.dispatches.is_empty()
    }

    pub(crate) fn dent(&self, tick: u64, body: u32, point: [f32; 2]) -> [f32; 4] {
        let Some(frame) = self.recorded.or(self.committed) else {
            return [0.0; 4];
        };
        let [previous, current] = frame.endpoint;
        if previous.tick == current.tick {
            return [0.0; 4];
        }
        let tick = tick.clamp(previous.tick, current.tick);
        let pressures = if tick == previous.tick {
            &self.committed_pressures[previous.bank as usize]
        } else if self.recorded.is_some() {
            &self.recorded_pressures
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
            self.committed = Some(new);
        }
        self.committed.map(|frame| frame.report)
    }

    pub(crate) fn witness(&self) -> Option<Witness> {
        self.committed.map(|frame| Witness {
            buffer: self.endpoints.clone(),
            endpoints: frame.endpoint.map(|endpoint| super::Endpoint {
                tick: endpoint.tick,
                origins: std::array::from_fn(|level| {
                    endpoint.origins[level].map(|cell| cell as f32 * CELL_METRES[level])
                }),
                offsets: std::array::from_fn(|level| (endpoint.bank as usize * 2 + level) * CELLS),
            }),
        })
    }

    fn write_sample(&self, queue: &wgpu::Queue, endpoint: [Endpoint; 2], time: EffectTime) {
        let [previous, current] = endpoint;
        let interval = (current.tick - previous.tick) as f32;
        let from_previous = if time.tick >= previous.tick {
            (time.tick - previous.tick) as f32
        } else {
            -((previous.tick - time.tick) as f32)
        };
        let blend = if interval == 0.0 {
            1.0
        } else {
            ((from_previous - 1.0 + time.alpha) / interval).clamp(0.0, 1.0)
        };
        let settings = SampleSettings {
            origins: std::array::from_fn(|i| {
                let metres = CELL_METRES[i % 2];
                let [x, z] = endpoint[i / 2].origins[i % 2];
                [x as f32 * metres, z as f32 * metres, metres, 0.0]
            }),
            clocks: [blend, interval * time.tick_duration.as_secs_f32(), 1.0, 0.0],
            bases: [0_u32, 1, 2, 3]
                .map(|i| (endpoint[(i / 2) as usize].bank * LEVELS + i % LEVELS) * GRID_CELLS),
        };
        queue.write_buffer(&self.sample, 0, bytemuck::bytes_of(&settings));
    }
}

#[cfg(test)]
mod tests;
