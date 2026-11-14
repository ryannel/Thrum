//! Reconstructible, every-frame wind waves. No clock, submission or ambient history is owned here.
//! A water contribution can embed [`Ambient`], append [`Ambient::graph`], stage an upload,
//! then forward each named simulate callback to [`Ambient::encode_pass`].
mod prepare;
mod upload;

#[cfg(test)]
mod tests {
    #[test]
    fn graph_compiles_with_no_ambient_history_or_uninitialized_reads() {
        use thrum_render::graph::{Access, FrameGraph, Pass, PassScope};
        let declaration = super::declaration(&super::prepare::sizes());
        declaration.validate().expect("every-frame declaration");
        let mut graph = FrameGraph::default();
        let resources: Vec<_> = declaration
            .resources
            .iter()
            .map(|(d, s)| graph.resource(d.clone(), *s).expect("real descriptors"))
            .collect();
        for pass in &declaration.passes {
            graph.pass(Pass {
                name: pass.name.clone(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: pass
                    .accesses
                    .iter()
                    .map(|a| Access {
                        resource: resources[a.resource].clone(),
                        range: a.range.clone(),
                        kind: a.kind,
                        view: a.view,
                        store: true,
                    })
                    .collect(),
                external_side_effect: true,
            });
        }
        let compiled = graph
            .compile(1, &std::collections::BTreeSet::new())
            .expect("complete producers");
        assert_eq!(compiled.pass_names().count(), 13);
    }
    #[test]
    fn portable_shader_validates_without_a_device() {
        let source = include_str!("ambient.wgsl");
        let module = wgpu::naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    }
}

use thrum_render::contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass};
use thrum_render::graph::{AccessKind, ResourceDescriptor, ResourceScope};
use thrum_sea_model::Spectrum;

/// Texel count of one physical cascade.
pub const PIXELS: u32 = 256 * 256;
/// All mip texels of one cascade, including its base.
pub const MIP_PIXELS: u32 = 87381;
/// Number of long-wave cache levels.
pub const LEVELS: u32 = 8;
/// Width and height of a cache level.
pub const CACHE_SIZE: u32 = 128;
/// Maximum query batch size. Queries and cache use the same shader evaluator.
pub const MAX_QUERIES: u32 = 4096;
const CHANNELS: u64 = 11;

/// Display seconds under C02: committed tick minus one plus interpolation alpha.
/// The initial tick is clamped to time zero. Captures supply their own clock.
pub fn display_seconds(time: thrum_render::contribution::EffectTime) -> f64 {
    (time.tick.saturating_sub(1) as f64 + f64::from(time.alpha)) * time.tick_duration.as_secs_f64()
}

/// Physical profile, independent of quality or selected recipe.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Must be 256; other transforms are not implemented by this profile.
    pub transform_size: u32,
    /// Must be three, matching the CPU spectrum partition.
    pub cascades: u32,
    /// Must be eight; cache texels double outward from one metre.
    pub cache_levels: u32,
    /// Gerstner horizontal choppiness; contraction is checked on upload.
    pub choppiness: f64,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            transform_size: 256,
            cascades: 3,
            cache_levels: LEVELS,
            choppiness: 0.1,
        }
    }
}

/// One snapped, world-addressed cache level. Unresolved components are excluded.
#[derive(Clone, Copy, Debug)]
pub struct CacheLevel {
    /// World x/z of the lower left texel edge, in metres.
    pub origin: [f64; 2],
    /// Metres per texel.
    pub texel: f64,
    /// Maximum represented wavenumber in radians/metre.
    pub max_wavenumber: f64,
}

/// Prepared generation resources. All products are overwritten, including on capture frames.
pub struct Ambient {
    profile: Profile,
    buffers: Vec<wgpu::Buffer>,
    pipelines: Vec<wgpu::ComputePipeline>,
    groups: Vec<wgpu::BindGroup>,
    mip_groups: Vec<wgpu::BindGroup>,
    mip_uniforms: Vec<wgpu::Buffer>,
    staging: wgpu::Buffer,
    upload: Vec<u8>,
    /// Exact world cache addresses for this frame's oracle and consumers.
    pub levels: [CacheLevel; LEVELS as usize],
}

impl Ambient {
    /// Prepare against the actual device limits. No queue or submission authority is retained.
    /// # Errors
    /// Names the first unsupported profile or resource limit.
    pub fn new(
        device: &thrum_render::runtime::DeviceSite,
        profile: Profile,
    ) -> Result<Self, String> {
        prepare::new(device, profile)
    }

    /// Every GPU resource and its bytes, including staging and mip uniforms.
    pub fn resource_bytes(&self) -> Vec<(String, u64)> {
        let mut rows: Vec<_> = self
            .buffers
            .iter()
            .zip(prepare::NAMES)
            .map(|(b, n)| (n.into(), b.size()))
            .collect();
        rows.push(("sea upload staging".into(), self.staging.size()));
        rows.extend(
            self.mip_uniforms
                .iter()
                .enumerate()
                .map(|(i, b)| (format!("sea mip {i} parameters"), b.size())),
        );
        rows
    }

    /// Declare actual hazards. All resources are transient in content, even though allocation is reused.
    pub fn graph(&self) -> EffectGraph {
        declaration(
            &self
                .buffers
                .iter()
                .map(wgpu::Buffer::size)
                .collect::<Vec<_>>(),
        )
    }

    /// Record one named graph pass, under the host's encoder and timing scope.
    pub fn encode_pass(&self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        let pipeline = if index < 5 { index } else { 5 };
        pass.set_pipeline(&self.pipelines[pipeline]);
        let group = match index {
            3 | 4 => 1,
            5..=12 => index - 3,
            _ => 0,
        };
        pass.set_bind_group(0, &self.groups[group], &[]);
        pass.set_bind_group(1, &self.mip_groups[0], &[]);
        match index {
            0 => pass.dispatch_workgroups(
                (MAX_QUERIES + LEVELS * CACHE_SIZE * CACHE_SIZE).div_ceil(64),
                1,
                1,
            ),
            1 | 4 => pass.dispatch_workgroups((3 * PIXELS).div_ceil(64), 1, 1),
            2 | 3 => pass.dispatch_workgroups(256, 33, 1),
            5..=12 => {
                pass.set_bind_group(1, &self.mip_groups[index - 5], &[]);
                let width = 128_u32 >> (index - 5);
                pass.dispatch_workgroups((3 * width * width).div_ceil(64), 1, 1);
            }
            _ => {}
        }
    }

    /// Read-only lending of cascade displacement/Jacobian, slopes/differentials and velocity.
    /// Each texel is three vec4<f32>, in that order; see the crate README for channels.
    pub fn cascade_buffer(&self) -> &wgpu::Buffer {
        &self.buffers[6]
    }
    /// Raw filtered moments. Covariance and variance are recovered from second moments.
    pub fn moments_buffer(&self, mip: usize) -> Option<&wgpu::Buffer> {
        if mip == 0 {
            self.buffers.get(7)
        } else if mip <= 8 {
            self.buffers.get(8 + mip)
        } else {
            None
        }
    }
    /// Query samples followed by eight row-major 128-square cache levels, three vec4s per sample.
    pub fn long_buffer(&self) -> &wgpu::Buffer {
        &self.buffers[8]
    }

    /// Stage one frame's CPU-owned spectrum at the supplied display time. Copies are encoded in
    /// this frame's encoder, so discarding that encoder cannot publish a partial upload.
    /// # Errors
    /// Rejects nonfinite time/coordinates, incompatible counts or noncontractive long waves.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        spectrum: &Spectrum,
        time: f64,
        anchor: [f64; 2],
        queries: &[[f64; 2]],
    ) -> Result<(), String> {
        upload::frame(self, queue, encoder, spectrum, time, anchor, queries)
    }
}

fn declaration(sizes: &[u64]) -> EffectGraph {
    let resources = sizes
        .iter()
        .enumerate()
        .map(|(i, b)| {
            (
                ResourceDescriptor::Buffer {
                    bytes: *b,
                    usage: prepare::usage(i),
                },
                if i < 4 {
                    ResourceScope::Imported { initialized: true }
                } else {
                    ResourceScope::Transient
                },
            )
        })
        .collect();
    let specs: &[(&str, &[(usize, AccessKind)])] = &[
        (
            "sea long waves",
            &[
                (0, AccessKind::UniformRead),
                (2, AccessKind::StorageRead),
                (3, AccessKind::StorageRead),
                (8, AccessKind::StorageWrite),
            ],
        ),
        (
            "sea cascade evolve",
            &[
                (0, AccessKind::UniformRead),
                (1, AccessKind::StorageRead),
                (4, AccessKind::StorageWrite),
            ],
        ),
        (
            "sea cascade rows",
            &[(4, AccessKind::StorageRead), (5, AccessKind::StorageWrite)],
        ),
        (
            "sea cascade columns",
            &[(5, AccessKind::StorageRead), (1, AccessKind::StorageWrite)],
        ),
        (
            "sea cascade unpack",
            &[
                (1, AccessKind::StorageRead),
                (6, AccessKind::StorageWrite),
                (7, AccessKind::StorageWrite),
            ],
        ),
    ];
    let mut passes: Vec<_> = specs
        .iter()
        .map(|(name, accesses)| EffectPass {
            name: (*name).into(),
            cadence: EffectCadence::EveryFrame,
            accesses: accesses
                .iter()
                .map(|&(resource, kind)| EffectAccess {
                    resource,
                    range: 0..sizes[resource],
                    kind,
                    view: None,
                    history: None,
                })
                .collect(),
        })
        .collect();
    for level in 1..=8 {
        let child = 8 + level;
        let parent = if level == 1 { 7 } else { child - 1 };
        passes.push(EffectPass {
            name: format!("sea moments mip {level}"),
            cadence: EffectCadence::EveryFrame,
            accesses: vec![
                EffectAccess {
                    resource: parent,
                    range: 0..sizes[parent],
                    kind: AccessKind::StorageRead,
                    view: None,
                    history: None,
                },
                EffectAccess {
                    resource: child,
                    range: 0..sizes[child],
                    kind: AccessKind::StorageWrite,
                    view: None,
                    history: None,
                },
            ],
        });
    }
    EffectGraph { resources, passes }
}
