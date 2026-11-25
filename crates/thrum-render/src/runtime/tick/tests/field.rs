use super::{Answer, Event, Mailbox, Post};
use crate::{
    contribution::{
        ContributionFrame, ContributionPasses, EffectAccess, EffectCadence, EffectGraph,
        EffectPass, EffectState, EffectStep, HistoryBinding, SceneContribution, SceneSurface,
        TickPass,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::FrameError,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub(super) const PASS: &str = "query committed test field";
const BYTES: u64 = 16;
const READBACKS: usize = super::super::IN_FLIGHT + 1;

pub(super) struct Field {
    pub(super) declare_tick: bool,
    pub(super) mail: Arc<Mutex<Mailbox>>,
    state: EffectState,
    gpu: Option<Gpu>,
    batch: Vec<(Post, bool)>,
    readback: usize,
}

struct Gpu {
    generation: u64,
    queue: wgpu::Queue,
    frame_params: wgpu::Buffer,
    query_params: wgpu::Buffer,
    output: wgpu::Buffer,
    history: [wgpu::BindGroup; 2],
    queries: [wgpu::BindGroup; 2],
    update: wgpu::ComputePipeline,
    query: wgpu::ComputePipeline,
    readbacks: [wgpu::Buffer; READBACKS],
}

impl Field {
    pub(super) fn new(mail: Arc<Mutex<Mailbox>>, declare_tick: bool) -> Self {
        Self {
            declare_tick,
            mail,
            state: EffectState::new(1),
            gpu: None,
            batch: Vec::new(),
            readback: 0,
        }
    }
}

impl SceneContribution for Field {
    fn effect_graph(&self) -> EffectGraph {
        let access = |kind, history| EffectAccess {
            resource: 0,
            range: 0..BYTES,
            kind,
            history: Some(history),
            view: None,
        };
        let previous = access(AccessKind::StorageRead, HistoryBinding::Previous);
        EffectGraph {
            resources: vec![(
                ResourceDescriptor::Buffer {
                    bytes: BYTES,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                ResourceScope::History {
                    owner: 0,
                    epoch: 0,
                    initialized: true,
                },
            )],
            passes: vec![EffectPass {
                name: "step test field".into(),
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses: vec![
                    previous.clone(),
                    access(AccessKind::StorageWrite, HistoryBinding::Next),
                ],
            }],
            tick_passes: self
                .declare_tick
                .then(|| TickPass {
                    name: PASS.into(),
                    reads: vec![previous],
                })
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        if self
            .gpu
            .as_ref()
            .is_none_or(|gpu| gpu.generation != surface.generation)
        {
            self.state.rebuild(surface.generation, 1);
            {
                let mut mail = self.mail.lock().expect("test field mailbox");
                mail.generation = surface.generation;
                mail.answers.clear();
                mail.busy.fill(false);
            }
            self.gpu = Some(Gpu::new(surface));
        }
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        let step = self.state.begin(frame.time, 64, [0; 2]);
        let (steps, reset) = match step {
            EffectStep::Keep => (0, 0),
            EffectStep::Reset => (0, 1),
            EffectStep::Advance { ticks, .. } => {
                (u32::try_from(ticks).expect("bounded field steps"), 0)
            }
        };
        if let Some(gpu) = &self.gpu {
            gpu.queue.write_buffer(
                &gpu.frame_params,
                0,
                bytemuck::cast_slice(&[steps, reset, 0, 0]),
            );
        }
    }

    fn effect_runs(&self, _: usize, frame: &ContributionFrame) -> bool {
        !frame.time.capture
            && self.state.committed().is_none_or(|time| {
                time.epoch != frame.time.epoch
                    || time.timeline != frame.time.timeline
                    || time.tick != frame.time.tick
            })
    }

    fn encode_effect(&mut self, _: usize, pass: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {
        let gpu = self.gpu.as_ref().expect("prepared test field");
        pass.set_pipeline(&gpu.update);
        pass.set_bind_group(
            0,
            &gpu.history[self.state.side(0, HistoryBinding::Previous)],
            &[],
        );
        pass.dispatch_workgroups(1, 1, 1);
        self.state.produced(0);
    }

    fn submitted(&mut self, history_advanced: bool) {
        if history_advanced {
            self.state.submitted();
        }
    }

    fn tick_has_work(&self, _: usize) -> bool {
        let mail = self.mail.lock().expect("test field mailbox");
        !mail.posts.is_empty() && mail.busy.iter().any(|busy| !busy)
    }

    fn encode_tick_upload(&mut self, _: usize, _: &mut wgpu::CommandEncoder, duration: Duration) {
        assert_eq!(duration, super::STEP);
        let gpu = self.gpu.as_ref().expect("prepared test field");
        let mut mail = self.mail.lock().expect("test field mailbox");
        mail.events.push(Event::Record);
        self.readback = mail
            .busy
            .iter()
            .position(|busy| !busy)
            .expect("admitted test readback");
        self.batch.clear();
        let mut points = [4_u32; 4];
        for (post, point) in mail.posts.iter().zip(&mut points) {
            let seeded = self.state.committed_for(post.endpoint).is_some();
            self.batch.push((*post, seeded));
            if seeded {
                *point = post.point;
            }
        }
        gpu.queue
            .write_buffer(&gpu.query_params, 0, bytemuck::cast_slice(&points));
    }

    fn encode_tick(&mut self, _: usize, pass: &mut wgpu::ComputePass<'_>) {
        let gpu = self.gpu.as_ref().expect("prepared test field");
        pass.set_pipeline(&gpu.query);
        pass.set_bind_group(
            0,
            &gpu.queries[self.state.side(0, HistoryBinding::Previous)],
            &[],
        );
        pass.dispatch_workgroups(1, 1, 1);
    }

    fn encode_tick_readback(&mut self, _: usize, encoder: &mut wgpu::CommandEncoder) {
        let gpu = self.gpu.as_ref().expect("prepared test field");
        encoder.copy_buffer_to_buffer(&gpu.output, 0, &gpu.readbacks[self.readback], 0, BYTES);
    }

    fn tick_submitted(&mut self, passes: &[usize]) {
        assert_eq!(passes, [0]);
        let gpu = self.gpu.as_ref().expect("prepared test field");
        let generation = gpu.generation;
        let index = self.readback;
        let batch = std::mem::take(&mut self.batch);
        let mail = Arc::clone(&self.mail);
        {
            let mut mail = mail.lock().expect("test field mailbox");
            mail.events.push(Event::Callback);
            mail.busy[index] = true;
            mail.posts.drain(..batch.len());
        }
        thrum_test_support::readback::map_submitted_buffer(&gpu.readbacks[index], move |bytes| {
            let answers = bytes.map(|bytes| {
                batch
                    .into_iter()
                    .zip(bytes.as_chunks::<4>().0)
                    .map(|((post, seeded), value)| Answer {
                        post,
                        value: seeded.then(|| u32::from_le_bytes(*value)),
                    })
                    .collect::<Vec<_>>()
            });
            let mut mail = mail.lock().expect("test field mailbox");
            if mail.generation == generation {
                mail.busy[index] = false;
                mail.answers.extend(answers.into_iter().flatten());
            } else {
                mail.retired_callbacks += 1;
            }
        });
    }
}

impl Gpu {
    fn new(surface: &SceneSurface<'_>) -> Self {
        use wgpu::BufferUsages as Usage;

        let device = surface.device;
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: BYTES,
                usage,
                mapped_at_creation: false,
            })
        };
        let frame_params = buffer("test field steps", Usage::UNIFORM | Usage::COPY_DST);
        let query_params = buffer("test field points", Usage::UNIFORM | Usage::COPY_DST);
        let output = buffer("test field answers", Usage::STORAGE | Usage::COPY_SRC);
        let fields = [
            buffer("test field first", Usage::STORAGE),
            buffer("test field second", Usage::STORAGE),
        ];
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("test tick field"),
            source: wgpu::ShaderSource::Wgsl(include_str!("field.wgsl").into()),
        });
        let pipeline = |entry| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let update = pipeline("step");
        let query = pipeline("query");
        let group = |pipeline: &wgpu::ComputePipeline, entries: &[(u32, &wgpu::Buffer)]| {
            let entries: Vec<_> = entries
                .iter()
                .map(|&(binding, buffer)| wgpu::BindGroupEntry {
                    binding,
                    resource: buffer.as_entire_binding(),
                })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("test field bindings"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        };
        let history = std::array::from_fn(|side| {
            group(
                &update,
                &[
                    (0, &frame_params),
                    (1, &fields[side]),
                    (2, &fields[1 - side]),
                ],
            )
        });
        let queries = std::array::from_fn(|side| {
            group(
                &query,
                &[(1, &fields[side]), (3, &query_params), (4, &output)],
            )
        });
        Self {
            generation: surface.generation,
            queue: surface.queue.clone(),
            frame_params,
            query_params,
            output,
            history,
            queries,
            update,
            query,
            readbacks: std::array::from_fn(|_| {
                buffer(
                    "test field answer readback",
                    Usage::MAP_READ | Usage::COPY_DST,
                )
            }),
        }
    }
}
