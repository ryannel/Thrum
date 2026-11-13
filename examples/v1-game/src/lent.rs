//! Opt-in CONTENT-LOOP diagnostic. Its only product authority is a public lease.

use std::sync::{Arc, Mutex};
use thrum_asset_gpu::{LentSet, ProductLease};
use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface,
};
use wgpu::util::DeviceExt;

#[cfg(target_arch = "wasm32")]
pub(crate) mod browser;

/// Diagnostic outputs, in lease request order. Each mip is tightly packed RGBA8 words.
#[derive(Default)]
pub struct LentWitness {
    /// Publication the most recent recording sampled.
    pub offered: Option<Arc<LentSet>>,
    /// Per-product outputs, usable only after that recording is submitted.
    pub outputs: Vec<wgpu::Buffer>,
    /// Products invalidated by this recording, retained through capture/discard.
    pub changed: Vec<bool>,
    /// Number of history-advancing submissions that consumed each product's change.
    pub invalidations: Vec<u64>,
}

struct Binding {
    key: (u64, u64),
    groups: Vec<(wgpu::BindGroup, u32)>,
}

/// Samples cooked products with compute passes, creating pipelines only on scene rebuild.
pub struct LentDiagnostic {
    lease: ProductLease,
    witness: Arc<Mutex<LentWitness>>,
    device: Option<thrum_render::runtime::DeviceSite>,
    pipelines: Vec<wgpu::ComputePipeline>,
    bindings: Vec<Binding>,
    committed: Vec<(u64, u64)>,
    recorded: Vec<(u64, u64)>,
}

impl LentDiagnostic {
    /// Construct explicitly; ordinary game scenes never attach this diagnostic.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "the same diagnostic handle is shared on native and used on the single browser GPU thread"
        )
    )]
    pub fn new(lease: ProductLease) -> (Self, Arc<Mutex<LentWitness>>) {
        let witness = Arc::new(Mutex::new(LentWitness::default()));
        (
            Self {
                lease,
                witness: Arc::clone(&witness),
                device: None,
                pipelines: Vec::new(),
                bindings: Vec::new(),
                committed: Vec::new(),
                recorded: Vec::new(),
            },
            witness,
        )
    }

    fn bind(&mut self, set: &LentSet) {
        let Some(device) = &self.device else {
            return;
        };
        let Ok(mut witness) = self.witness.lock() else {
            return;
        };
        for (index, product) in set.products.iter().enumerate() {
            let key = (product.device_generation, product.serial);
            if self
                .bindings
                .get(index)
                .is_some_and(|binding| binding.key == key)
            {
                continue;
            }
            let texture = &product.texture;
            let mut levels = Vec::new();
            let mut offset = 0u32;
            for mip in 0..texture.mip_level_count() {
                let width = (texture.width() >> mip).max(1);
                let height = (texture.height() >> mip).max(1);
                let depth = if texture.dimension() == wgpu::TextureDimension::D3 {
                    (texture.depth_or_array_layers() >> mip).max(1)
                } else {
                    1
                };
                let count = width * height * depth;
                levels.push((
                    [width, height, depth, offset, mip, count, 0, 0],
                    count.div_ceil(64),
                ));
                offset += count;
            }
            let output = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lent decoded texels"),
                size: u64::from(offset) * 4,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let pipeline =
                &self.pipelines[usize::from(texture.dimension() == wgpu::TextureDimension::D3)];
            let groups = levels
                .into_iter()
                .map(|(params, dispatch)| {
                    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("lent mip"),
                        contents: &params
                            .into_iter()
                            .flat_map(u32::to_le_bytes)
                            .collect::<Vec<_>>(),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("lent mip"),
                        layout: &pipeline.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&product.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: output.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: uniform.as_entire_binding(),
                            },
                        ],
                    });
                    (group, dispatch)
                })
                .collect();
            let binding = Binding { key, groups };
            if index == self.bindings.len() {
                self.bindings.push(binding);
                witness.outputs.push(output);
            } else {
                self.bindings[index] = binding;
                witness.outputs[index] = output;
            }
        }
    }
}

impl SceneContribution for LentDiagnostic {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: true,
        }
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.device = Some(surface.device.clone());
        self.bindings.clear();
        self.committed.clear();
        self.recorded.clear();
        if let Ok(mut witness) = self.witness.lock() {
            *witness = LentWitness::default();
        }
        self.pipelines = [false, true].into_iter().map(|volume| {
            let (kind, position) = if volume { ("3d", "vec3<i32>(vec3<u32>(i % p.a.x, (i / p.a.x) % p.a.y, i / (p.a.x * p.a.y)))") }
                else { ("2d", "vec2<i32>(vec2<u32>(i % p.a.x, i / p.a.x))") };
            let source = format!("@group(0) @binding(0) var source: texture_{kind}<f32>;\n@group(0) @binding(1) var<storage, read_write> out: array<u32>;\nstruct Params {{ a: vec4<u32>, b: vec4<u32> }}\n@group(0) @binding(2) var<uniform> p: Params;\n@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id: vec3<u32>) {{ let i = id.x; if i >= p.b.y {{ return; }} out[p.a.w + i] = pack4x8unorm(textureLoad(source, {position}, i32(p.b.x))); }}");
            let shader = surface.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("lent reader"), source: wgpu::ShaderSource::Wgsl(source.into()) });
            surface.device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("lent reader"), layout: None, module: &shader, entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(), cache: None,
            })
        }).collect();
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.recorded.clear();
        let Some(set) = self.lease.offered().filter(|set| {
            set.products
                .iter()
                .all(|product| product.device_generation == frame.generation)
        }) else {
            return;
        };
        self.bind(&set);
        self.recorded.extend(
            set.products
                .iter()
                .map(|product| (product.device_generation, product.serial)),
        );
        if let Ok(mut witness) = self.witness.lock() {
            witness.changed.clear();
            witness.changed.extend(
                self.recorded
                    .iter()
                    .enumerate()
                    .map(|(i, key)| self.committed.get(i) != Some(key)),
            );
            witness.invalidations.resize(self.recorded.len(), 0);
            witness.offered = Some(set);
        }
    }

    fn computes(&self, _: &ContributionFrame) -> bool {
        !self.recorded.is_empty()
    }

    fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
    fn draw(&self, _: &mut wgpu::RenderPass<'_>) {}

    fn encode_compute(&mut self, pass: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {
        let Some(set) = self.lease.offered() else {
            return;
        };
        for (product, binding) in set.products.iter().zip(&self.bindings) {
            pass.set_pipeline(
                &self.pipelines
                    [usize::from(product.texture.dimension() == wgpu::TextureDimension::D3)],
            );
            for (group, dispatch) in &binding.groups {
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(*dispatch, 1, 1);
            }
        }
    }

    fn submitted(&mut self, history_advanced: bool) {
        if history_advanced && !self.recorded.is_empty() {
            if let Ok(mut witness) = self.witness.lock() {
                for (i, key) in self.recorded.iter().enumerate() {
                    if self.committed.get(i) != Some(key) {
                        witness.invalidations[i] += 1;
                    }
                }
            }
            self.committed.clone_from(&self.recorded);
        }
    }
}
