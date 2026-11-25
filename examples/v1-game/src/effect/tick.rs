//! The diagnostic's world posts and asynchronous trail readback.

use std::sync::{Arc, Mutex};
use thrum_render::{contribution::SceneSurface, extract::SourceTick};

pub(super) const PASS: &str = "diagnostic trail query";
const CAPACITY: usize = 8;
const OUTPUT_BYTES: u64 = (CAPACITY * size_of::<u32>()) as u64;

// The shader writes one u32 per point; an unmatched world never consumes that word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Post {
    pub(super) endpoint: SourceTick,
    pub(super) use_tick: u64,
    pub(super) point: [u32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Answer {
    pub(super) post: Post,
    pub(super) value: Option<u32>,
}

#[derive(Default)]
pub(super) struct Mailbox {
    generation: u64,
    busy: bool,
    posts: Vec<Post>,
    answers: Vec<Answer>,
}

impl Mailbox {
    pub(super) fn rebuild(&mut self, generation: u64) {
        self.generation = generation;
        self.busy = false;
        self.answers.clear();
    }

    #[cfg(all(feature = "proving", target_arch = "wasm32"))]
    pub(super) fn post(&mut self, post: Post) -> bool {
        if self.posts.len() + self.answers.len() >= CAPACITY {
            return false;
        }
        self.posts.push(post);
        true
    }

    #[cfg(all(feature = "proving", target_arch = "wasm32"))]
    pub(super) fn drain(&mut self, endpoint: SourceTick) -> Vec<Answer> {
        let mut answers = std::mem::take(&mut self.answers);
        answers.retain(|answer| {
            let asked = answer.post.endpoint;
            asked.epoch == endpoint.epoch && asked.timeline == endpoint.timeline
        });
        answers
    }

    pub(super) fn has_work(&self) -> bool {
        !self.busy && !self.posts.is_empty()
    }
}

pub(super) struct Query {
    params: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    sides: Vec<wgpu::BindGroup>,
    states: Vec<bool>,
    side: usize,
}

impl Query {
    pub(super) fn new(surface: &SceneSurface<'_>, history: &[wgpu::Texture]) -> Self {
        use wgpu::BufferUsages as Usage;
        let buffer = |label, size, usage| {
            surface.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = buffer(
            "trail query points",
            16 + OUTPUT_BYTES * 4,
            Usage::UNIFORM | Usage::COPY_DST,
        );
        let output = buffer(
            "trail query results",
            OUTPUT_BYTES,
            Usage::STORAGE | Usage::COPY_SRC,
        );
        let readback = buffer(
            "trail query readback",
            OUTPUT_BYTES,
            Usage::MAP_READ | Usage::COPY_DST,
        );
        let module = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(PASS),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "const CAPACITY: u32 = {CAPACITY}u;\n{}",
                        include_str!("tick.wgsl")
                    )
                    .into(),
                ),
            });
        let pipeline = surface.device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let sides = history
            .iter()
            .map(|texture| {
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                surface
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(PASS),
                        layout: &layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: params.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: output.as_entire_binding(),
                            },
                        ],
                    })
            })
            .collect();
        Self {
            params,
            output,
            readback,
            pipeline,
            sides,
            states: Vec::with_capacity(CAPACITY),
            side: 0,
        }
    }

    pub(super) fn stage(
        &mut self,
        queue: &wgpu::Queue,
        posts: &Mailbox,
        state: &thrum_render::contribution::EffectState,
    ) {
        use thrum_render::contribution::HistoryBinding::Previous;
        self.side = state.side(0, Previous);
        self.states.clear();
        let count = posts.posts.len().min(CAPACITY);
        let mut words = [[0_u32; 4]; CAPACITY + 1];
        words[0][0] = u32::try_from(count).expect("bounded posts");
        for (post, words) in posts.posts.iter().zip(&mut words[1..]) {
            let has_state = state.committed_for(post.endpoint).is_some();
            self.states.push(has_state);
            *words = [post.point[0], post.point[1], u32::from(has_state), 0];
        }
        let mut bytes = [0; (CAPACITY + 1) * 16];
        for (chunk, word) in bytes
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(words.as_flattened())
        {
            *chunk = word.to_le_bytes();
        }
        queue.write_buffer(&self.params, 0, &bytes[..(count + 1) * 16]);
    }

    pub(super) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.sides[self.side], &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    pub(super) fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, OUTPUT_BYTES);
    }

    pub(super) fn submitted(&mut self, mailbox: &Arc<Mutex<Mailbox>>) {
        let (generation, posts) = {
            let mut mailbox = mailbox.lock().expect("trail queries");
            mailbox.busy = true;
            let posts: Vec<_> = mailbox
                .posts
                .drain(..self.states.len())
                .zip(self.states.drain(..))
                .collect();
            (mailbox.generation, posts)
        };
        let mailbox = Arc::clone(mailbox);
        let readback = self.readback.clone();
        self.readback
            .map_async(wgpu::MapMode::Read, .., move |result| {
                let answers = if result.is_ok() {
                    let answers = readback.get_mapped_range(..).ok().map(|mapped| {
                        posts
                            .into_iter()
                            .zip(mapped.as_chunks::<4>().0)
                            .map(|((post, has_state), word)| {
                                let value = has_state.then(|| u32::from_le_bytes(*word));
                                Answer { post, value }
                            })
                            .collect::<Vec<_>>()
                    });
                    readback.unmap();
                    answers
                } else {
                    None
                };
                let mut mailbox = mailbox.lock().expect("trail queries");
                if mailbox.generation == generation {
                    mailbox.answers.extend(answers.into_iter().flatten());
                    mailbox.busy = false;
                }
            });
    }
}
