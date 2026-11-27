use super::{Hull, Report, Whitewater, Witness};
use crate::{
    column::{COMPONENT_LIMIT, Component},
    surf::SurfGpu,
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

#[cfg(test)]
mod tests;

const SIDE: u32 = 256;
const CELLS: u64 = (SIDE * SIDE) as u64;
const SPACING: [f32; 2] = [0.25, 2.0];
const MAX_TICKS: u64 = 8;
const STOCK_RECORDS: u64 = 4 * CELLS + 2;
const PASS: &str = "whitewater";
const SURF_FIRST: u32 = 12;
const SHORE_BIRTHS: u32 = 17;
pub(super) const HULL_LIMIT: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HullRecord {
    location_axis: [f32; 4],
    extent_rate: [f32; 4],
    velocity: [f32; 4],
}

impl From<&Hull> for HullRecord {
    fn from(hull: &Hull) -> Self {
        let [x, z] = hull.place;
        let [dx, dz] = hull.forward;
        let [length, width] = hull.extent;
        Self {
            location_axis: [x, z, dx, dz],
            extent_rate: [length, width, hull.drag_power, 0.0],
            velocity: [hull.velocity[0], hull.velocity[1], 0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Settings {
    origins: [[i32; 4]; 4],
    banks: [u32; 4],
    time_wind: [f32; 4],
    camera: [f32; 4],
    jacobian_variance: [f32; 4],
    long_bounds: [f32; 4],
    hulls: [HullRecord; HULL_LIMIT],
    components: [Component; COMPONENT_LIMIT],
}

#[derive(Clone, Copy)]
struct Endpoint {
    tick: u64,
    origins: [[i32; 2]; 2],
    bank: u32,
}

#[derive(Clone, Copy)]
struct Frame {
    time: EffectTime,
    generation: u64,
    endpoints: [Endpoint; 2],
    report: Report,
}

pub(crate) struct Foam {
    whitewater: Whitewater,
    stocks: wgpu::Buffer,
    settings: wgpu::Buffer,
    scratch: wgpu::Buffer,
    shore_births: Option<wgpu::Buffer>,
    group: wgpu::BindGroup,
    pipelines: [wgpu::ComputePipeline; 3],
    prepare_shore_births: Option<wgpu::ComputePipeline>,
    committed: Option<Frame>,
    recorded: Option<Frame>,
    step: bool,
    initial_report: Report,
    flow: bool,
}

fn buffer(device: &DeviceSite, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

impl Foam {
    fn source(surf: bool) -> String {
        let [fine, coarse] = SPACING;
        let handoff_cells = if surf { 2 * CELLS } else { 1 };
        format!(
            "const FOAM_SIDE: i32 = {SIDE};\nconst FOAM_SPACING = array({fine:?}, {coarse:?});\n\
             const FOAM_STOCK_RECORDS: u32 = {STOCK_RECORDS}u;\n\
             const FOAM_HANDOFF_CELLS: u32 = {handoff_cells}u;\nconst FOAM_HANDOFF: bool = {surf};\n\
             const HULL_LIMIT: u32 = {HULL_LIMIT}u;\nconst COMPONENT_LIMIT: u32 = {COMPONENT_LIMIT}u;\n"
        ) + include_str!("../column/records.wgsl")
            + include_str!("../column.wgsl")
            + include_str!("sample.wgsl")
    }

    pub(crate) fn sample_source(&self) -> String {
        Self::source(self.prepare_shore_births.is_some())
    }

    pub(crate) fn sample_layout() -> [wgpu::BindGroupLayoutEntry; 2] {
        [
            binding::storage(9, wgpu::ShaderStages::FRAGMENT, true, 0),
            binding::uniform(10, wgpu::ShaderStages::FRAGMENT, 0),
        ]
    }

    pub(crate) fn sample_entries(&self) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: 9,
                resource: self.stocks.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: self.settings.as_entire_binding(),
            },
        ]
    }

    pub(crate) fn new(
        device: &DeviceSite,
        whitewater: Whitewater,
        initial_report: Report,
        field_layout: &[wgpu::BindGroupLayoutEntry],
        field: &[wgpu::BindGroupEntry<'_>],
        wake: bool,
        surf: Option<&SurfGpu>,
        map: Option<&crate::map_gpu::MapGpu>,
        flow: Option<(thrum_water_model::FlowBandsProfile, u32)>,
    ) -> Self {
        use wgpu::BufferUsages as Usage;
        use wgpu::ShaderStages as Stages;
        let stocks = buffer(
            device,
            "foam endpoints",
            STOCK_RECORDS * 16 + if surf.is_some() { 2 * CELLS * 4 } else { 16 },
            Usage::STORAGE | Usage::COPY_SRC | Usage::COPY_DST,
        );
        let settings = buffer(
            device,
            "foam settings",
            size_of::<Settings>() as u64,
            Usage::UNIFORM | Usage::COPY_DST,
        );
        let scratch = buffer(
            device,
            "foam transport",
            2 * CELLS * 32,
            Usage::STORAGE | Usage::COPY_SRC,
        );
        let shore_births = surf.map(|_| {
            buffer(
                device,
                "shore birth sources",
                2 * CELLS * 48,
                Usage::STORAGE,
            )
        });
        let mut entries: Vec<_> = field_layout
            .iter()
            .map(|&entry| {
                let mut entry = entry;
                entry.visibility = Stages::COMPUTE;
                entry
            })
            .collect();
        entries.extend([
            binding::storage(9, Stages::COMPUTE, false, 0),
            binding::uniform(10, Stages::COMPUTE, 0),
            binding::storage(11, Stages::COMPUTE, false, 0),
        ]);
        if surf.is_some() {
            entries.extend(SurfGpu::layout(SURF_FIRST, Stages::COMPUTE));
            entries.push(binding::storage(SHORE_BIRTHS, Stages::COMPUTE, false, 0));
        }
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &entries,
        });
        let mut bindings: Vec<_> = field
            .iter()
            .cloned()
            .chain([
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: stocks.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: settings.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: scratch.as_entire_binding(),
                },
            ])
            .collect();
        if let Some((surf, births)) = surf.zip(shore_births.as_ref()) {
            bindings.extend(surf.entries(SURF_FIRST));
            bindings.push(wgpu::BindGroupEntry {
                binding: SHORE_BIRTHS,
                resource: births.as_entire_binding(),
            });
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &group_layout,
            entries: &bindings,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&group_layout)],
            immediate_size: 0,
        });
        let shore = if surf.is_some() {
            SurfGpu::source(0, SURF_FIRST) + include_str!("shore.wgsl")
        } else {
            "fn foam_wet(_level: u32, _local: vec2<i32>, plain: bool) -> bool { return plain; }
             fn stranded_shore(_local: vec2<i32>) -> bool { return false; }
             fn shore_uprush(_level: u32, _local: vec2<i32>, plain: vec2<f32>) -> vec2<f32> { return plain; }
             fn shore_rates(_level: u32, _local: vec2<i32>, loss: f32) -> vec2<f32> {
                 return vec2(loss, 0.22 * loss);
             }"
            .to_owned()
        };
        let mut source = (include_str!("../field.wgsl").to_owned() + &Self::source(surf.is_some()))
            .replace("@group(3)", "@group(0)")
            .replace(
                "var<storage, read> foam_stocks",
                "var<storage, read_write> foam_stocks",
            )
            + &map.map_or_else(|| crate::map_gpu::declarations_at(0), |map| map.source(0))
            + &crate::interaction::Interaction::sample_source(0, 7, 8, wake)
            + include_str!("../carriage.wgsl")
            + &shore
            + include_str!("births.wgsl")
            + if flow.is_some() {
                include_str!("river_transport.wgsl")
            } else {
                include_str!("transport.wgsl")
            }
            + include_str!("step.wgsl");
        if let Some((profile, first)) = flow {
            use std::fmt::Write;
            source.push_str(&crate::flow::constants(profile, first));
            source.push_str(include_str!("flow.wgsl"));
            let [onset, full] = super::CHART_LOSS_BREAKING;
            writeln!(
                &mut source,
                "const CHART_LOSS_BIRTH_RATE: f32 = {:?};
const CHART_LOSS_BREAKING = vec2<f32>({onset:?}, {full:?});",
                super::CHART_LOSS_BIRTH_RATE
            )
            .expect("foam loss constant");
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = |entry_point| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let pipelines = ["prepare_transport", "step_foam", "measure_held"].map(pipeline);
        let prepare_shore_births = surf.map(|_| pipeline("prepare_shore_births"));
        Self {
            whitewater,
            stocks,
            settings,
            scratch,
            shore_births,
            group,
            pipelines,
            prepare_shore_births,
            committed: None,
            recorded: None,
            step: false,
            initial_report,
            flow: flow.is_some(),
        }
    }

    pub(crate) fn declare(
        &self,
        graph: &mut EffectGraph,
        mut inputs: Vec<EffectAccess>,
    ) -> EffectAccess {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: self.stocks.size(),
                usage: self.stocks.usage(),
            },
            ResourceScope::Persistent {
                owner: 0x666f_616d,
                epoch: 0,
                initialized: true,
            },
        ));
        let read = EffectAccess {
            resource,
            range: 0..self.stocks.size(),
            kind: AccessKind::StorageRead,
            view: None,
            history: None,
        };
        inputs.push(EffectAccess {
            kind: AccessKind::StorageReadWrite,
            ..read.clone()
        });
        graph.passes.push(EffectPass {
            name: PASS.into(),
            cadence: EffectCadence::Skippable,
            async_eligible: false,
            accesses: inputs,
        });
        read
    }

    pub(crate) fn resource_bytes(&self) -> impl Iterator<Item = (String, u64)> + '_ {
        [
            ("foam endpoints".into(), self.stocks.size()),
            ("foam transport".into(), self.scratch.size()),
            ("foam settings".into(), self.settings.size()),
        ]
        .into_iter()
        .chain(
            self.shore_births
                .iter()
                .map(|buffer| ("shore birth sources".into(), buffer.size())),
        )
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "window and step values have bounded GPU representations"
    )]
    pub(crate) fn record(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &ContributionFrame,
        spectrum: &thrum_sea_model::Spectrum,
        chop: f64,
        seconds: f64,
    ) -> bool {
        self.recorded = None;
        self.step = false;
        let time = frame.time;
        let mut settings = Settings::zeroed();
        let previous = self.committed;
        let reset = previous.is_none_or(|old| {
            old.time.epoch != time.epoch
                || old.time.timeline != time.timeline
                || old.time.tick_duration != time.tick_duration
                || old.time.tick > time.tick
                || old.generation != frame.generation
        });
        let endpoints = if time.capture
            || (!reset && previous.is_some_and(|old| old.time.tick == time.tick))
        {
            previous.map(|old| old.endpoints)
        } else {
            let old = previous.map(|old| old.endpoints[1]);
            let origins = SPACING.map(|spacing| {
                [frame.camera[0], frame.camera[2]]
                    .map(|v| (v / spacing).floor().clamp(-1e8, 1e8) as i32 - SIDE.cast_signed() / 2)
            });
            let next = Endpoint {
                tick: time.tick,
                origins,
                bank: old.map_or(0, |old| 1 - old.bank),
            };
            let mut report = previous.map_or(self.initial_report, |old| old.report);
            let endpoints = if reset {
                encoder.clear_buffer(&self.stocks, 0, None);
                report.resets += 1;
                [
                    Endpoint {
                        bank: 1 - next.bank,
                        ..next
                    },
                    next,
                ]
            } else {
                let old = old.expect("foam previous endpoint");
                let elapsed = time.tick - old.tick;
                let ticks = elapsed.min(MAX_TICKS);
                report.dropped_ticks += elapsed - ticks;
                settings.time_wind[0] = time.tick_duration.as_secs_f32() * ticks as f32;
                self.step = true;
                [old, next]
            };
            report.ticks = endpoints.map(|end| end.tick);
            self.recorded = Some(Frame {
                time,
                generation: frame.generation,
                endpoints,
                report,
            });
            Some(endpoints)
        };
        if let Some([old, next]) = endpoints {
            settings.banks = [old.bank, next.bank, 0, 0];
            for (row, coordinates) in settings
                .origins
                .iter_mut()
                .zip(old.origins.into_iter().chain(next.origins))
            {
                row[..2].copy_from_slice(&coordinates);
            }
            let interval = (next.tick - old.tick) as f32;
            settings.time_wind[1] = if interval == 0.0 {
                1.0
            } else {
                let offset = if time.tick >= old.tick {
                    (time.tick - old.tick) as f32
                } else {
                    -((old.tick - time.tick) as f32)
                };
                ((offset - 1.0 + time.alpha) / interval).clamp(0.0, 1.0)
            };
        }
        settings.time_wind[2..].copy_from_slice(&self.whitewater.wind);
        settings.jacobian_variance = self.whitewater.jacobian_variance;
        settings.long_bounds = self.whitewater.long_bounds;
        if self.flow {
            settings.long_bounds[2] = f32::from(self.whitewater.input.chart_loss_births());
        }
        let origin = [frame.camera[0], frame.camera[2]];
        let tick_seconds = time.tick_duration.as_secs_f32();
        // A normal step samples its committed tick. Catch-up steps sample
        // the middle of the elapsed interval they deposit and transport over.
        let midpoint = if settings.time_wind[0] > tick_seconds {
            0.5 * settings.time_wind[0]
        } else {
            0.0
        };
        settings.camera = [
            origin[0],
            origin[1],
            (1.0 - time.alpha) * tick_seconds - midpoint,
            chop as f32,
        ];
        settings.banks[3] = u32::try_from(spectrum.long.len()).expect("bounded wave components");
        for (out, component) in settings.components.iter_mut().zip(&spectrum.long) {
            *out = Component::at(component, origin, seconds);
        }
        let page = self.whitewater.input.0.lock().expect("foam hull input");
        if self.step && page.0 == time.tick {
            settings.banks[2] = u32::try_from(page.1.len()).expect("bounded foam sources");
            for (out, body) in settings.hulls.iter_mut().zip(&page.1) {
                *out = body.into();
            }
        }
        queue.write_buffer(&self.settings, 0, bytemuck::bytes_of(&settings));
        reset && !time.capture
    }

    pub(crate) fn pending(&self) -> bool {
        self.step
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_bind_group(0, &self.group, &[]);
        if let Some(pipeline) = &self.prepare_shore_births {
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(SIDE / 8, SIDE / 8, 2);
        }
        let [prepare, step, held] = &self.pipelines;
        for pipeline in [prepare, step] {
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(SIDE / 8, SIDE / 8, 2);
        }
        pass.set_pipeline(held);
        pass.dispatch_workgroups(1, 1, 1);
    }

    pub(crate) fn submit(&mut self) -> Option<Report> {
        if let Some(frame) = self.recorded.take() {
            self.committed = Some(frame);
        }
        self.committed.map(|frame| frame.report)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "diagnostics use the same f32 plane placement as the shader"
    )]
    pub(crate) fn witness(&self) -> Option<Witness> {
        self.committed.map(|frame| Witness {
            buffer: self.stocks.clone(),
            parcels: self.scratch.clone(),
            side: SIDE,
            spacing: SPACING,
            origins: frame.endpoints.map(|end| {
                std::array::from_fn(|level| {
                    end.origins[level].map(|cell| cell as f32 * SPACING[level])
                })
            }),
            banks: frame.endpoints.map(|end| end.bank as usize),
            report: frame.report,
        })
    }
}
