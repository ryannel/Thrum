//! Reconstructible, every-frame wind waves. No clock, submission or ambient history is owned here.
//! A water contribution can embed [`Ambient`], append [`Ambient::graph`], stage an upload,
//! then forward each named simulate callback to [`Ambient::encode_pass`].
mod error;
pub use error::{AmbientError, ProfileField};
mod filtered;
mod prepare;
/// Host record reflection metadata for shader tooling.
pub mod records;
mod spec;
mod upload;
use spec::declaration;

/// Versioned WGSL evaluator shared by field-cache and body-query pipelines.
/// Declares `sample_long(point: vec2<f32>, max_k: f32) -> Field`. The consumer
/// supplies `components: array<Long>` in read-only storage, `p.time.y` (count)
/// and `p.physical.x` (choppiness), plus the `Long` and `Field` layouts from the
/// crate README. Phases must be reduced in f64 at the query's own time/anchor.
pub const LONG_WAVE_EVALUATOR: &str = include_str!("long_waves.wgsl");
/// Filtered-field packing and mip reduction source for reflection tooling.
pub const FILTERED_SHADER: &str = filtered::SHADER;
/// Uniform contraction bound admitted by the bounded GPU long-wave inversion.
/// The CPU oracle admits sums below one; 0.9 leaves a convergence margin for
/// the GPU's 256 iterations and metre-scale binary32 residual.
pub const LONG_WAVE_CONTRACTION_LIMIT: f64 = 0.9;
/// Complete ambient shader source, for layout and binding reflection tools.
pub const SHADER: &str = concat!(
    include_str!("field.wgsl"),
    "\n",
    include_str!("ambient.wgsl"),
    "\n",
    include_str!("long_waves.wgsl")
);

#[cfg(test)]
mod tests {
    #[test]
    fn initial_display_clock_has_no_extra_tick() {
        let mut time = thrum_render::contribution::EffectTime {
            epoch: 1,
            timeline: 1,
            tick: 0,
            tick_duration: std::time::Duration::from_millis(20),
            alpha: 1.0,
            display_phase: 1.0,
            capture: false,
        };
        assert_eq!(super::display_seconds(time), 0.0);
        time.alpha = 0.5;
        assert_eq!(super::display_seconds(time), -0.01);
        time.tick = 1;
        time.alpha = 0.0;
        assert_eq!(super::display_seconds(time), 0.0);
        time.alpha = 0.5;
        assert_eq!(super::display_seconds(time), 0.01);
    }
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
        for source in [super::SHADER, super::FILTERED_SHADER] {
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
}

use std::collections::BTreeMap;
use thrum_render::contribution::EffectGraph;
use thrum_sea_model::Spectrum;

/// Width of the supported physical transform.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the model's fixed 256-square grid fits u32"
)]
pub const TRANSFORM_SIZE: u32 = thrum_sea_model::GRID_SIZE as u32;

/// Texel count of one physical cascade.
pub const PIXELS: u32 = TRANSFORM_SIZE * TRANSFORM_SIZE;
/// Number of long-wave cache levels.
pub const LEVELS: u32 = 8;
/// Width and height of a cache level.
pub const CACHE_SIZE: u32 = 128;
/// Maximum query batch size. Queries and cache use the same shader evaluator.
pub const MAX_QUERIES: u32 = 4096;
// Eleven real fields share six complex transforms. The last lane is unused.
const PACKED_CHANNELS: u32 = 6;

/// Display seconds under C02: committed tick minus one plus interpolation alpha.
/// Captures supply their own clock. At tick zero the analytic field can be
/// sampled before zero; alpha one samples exactly zero, without a tick of advance.
pub fn display_seconds(time: thrum_render::contribution::EffectTime) -> f64 {
    (time.tick as f64 - 1.0 + f64::from(time.alpha)) * time.tick_duration.as_secs_f64()
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
    /// Gerstner horizontal choppiness in 0..=4; contraction is checked on upload.
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
    filtered: filtered::Filtered,
    profile: Profile,
    buffers: BTreeMap<spec::BufferKey, wgpu::Buffer>,
    passes: BTreeMap<spec::PassKey, prepare::PreparedPass>,
    specs: Vec<spec::PassSpec>,
    upload: Vec<u8>,
    modes: Vec<records::Bin>,
    cached_cascades: [Vec<thrum_sea_model::Component>; spec::CASCADES as usize],
    cached_lengths: [f64; spec::CASCADES as usize],
    modes_initialized: bool,
    last_upload: UploadBytes,
    /// Exact world cache addresses for this frame's oracle and consumers.
    pub levels: [CacheLevel; LEVELS as usize],
}

/// Bytes handed to the queue by the last successful upload, before encoder submission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UploadBytes {
    /// Sum of payload lengths passed to `Queue::write_buffer`.
    pub total: u64,
    /// Complete parameter record.
    pub parameters: u64,
    /// Static Fourier modes; zero unless the cascade spectrum changed.
    pub modes: u64,
    /// Time/anchor dependent long-wave records.
    pub long: u64,
    /// Active relative query points.
    pub queries: u64,
}

/// One actual prepared binding, for graph/shader diagnostic tooling.
#[derive(Debug)]
pub struct BindingAccess {
    /// Graph pass name.
    pub pass: String,
    /// Shader entry point used by this pass.
    pub entry: &'static str,
    /// Shader bind group.
    pub group: u32,
    /// Shader binding slot.
    pub binding: u32,
    /// Resource ordinal at the public effect-graph boundary.
    pub resource: usize,
    /// Access performed by this entry point.
    pub access: thrum_render::graph::AccessKind,
}

impl Ambient {
    /// Byte accounting from the actual queue-write path; rejected uploads leave it unchanged.
    pub fn upload_bytes(&self) -> UploadBytes {
        self.last_upload
    }

    /// Bindings used to create this generation's bind groups and graph declarations.
    pub fn binding_accesses(&self) -> Vec<BindingAccess> {
        let resources: BTreeMap<_, _> = spec::resources()
            .iter()
            .enumerate()
            .map(|(i, r)| (r.key, i))
            .collect();
        let mut bindings: Vec<_> = self
            .specs
            .iter()
            .flat_map(|p| {
                p.bindings.iter().map(|b| BindingAccess {
                    pass: p.name.clone(),
                    entry: p.entry,
                    group: b.group.slot(),
                    binding: b.slot,
                    resource: resources[&b.buffer],
                    access: b.access,
                })
            })
            .collect();
        for pass in self.graph().passes.iter().skip(self.specs.len()) {
            let pack = pass.name == filtered::PACK_PASS;
            let slots = if pack { &[0, 1, 2, 3][..] } else { &[4, 5][..] };
            for (access, &slot) in pass.accesses.iter().zip(slots) {
                bindings.push(BindingAccess {
                    pass: pass.name.clone(),
                    entry: if pack { "pack" } else { "reduce_texture" },
                    group: 0,
                    binding: slot,
                    resource: access.resource,
                    access: access.kind,
                });
            }
        }
        bindings
    }
    /// Prepare against the actual device limits. No queue or submission authority is retained.
    /// # Errors
    /// Names the first unsupported profile or resource limit.
    pub fn new(
        device: &thrum_render::runtime::DeviceSite,
        profile: Profile,
    ) -> Result<Self, AmbientError> {
        prepare::new(device, profile)
    }

    /// Every GPU resource and its bytes, including staging and mip uniforms.
    pub fn resource_bytes(&self) -> Vec<(String, u64)> {
        let mut resources: Vec<_> = spec::resources()
            .into_iter()
            .map(|r| (r.name, self.buffers[&r.key].size()))
            .collect();
        resources.extend(self.filtered.resource_bytes());
        resources
    }

    /// Declare actual hazards. All resources are transient in content, even though allocation is reused.
    pub fn graph(&self) -> EffectGraph {
        debug_assert_eq!(
            prepare::sizes().iter().sum::<u64>(),
            self.buffers.values().map(wgpu::Buffer::size).sum::<u64>()
        );
        let mut graph = declaration(&prepare::sizes());
        self.filtered.append_graph(&mut graph);
        graph
    }

    /// Record one named graph pass, under the host's encoder and timing scope.
    pub fn encode_pass(&self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        if index >= self.specs.len() {
            self.filtered.encode(index - self.specs.len(), pass);
            return;
        }
        // The contribution API supplies a graph ordinal; translate once, then use typed keys.
        let Some(spec) = self.specs.get(index) else {
            return;
        };
        let prepared = &self.passes[&spec.key];
        pass.set_pipeline(&prepared.pipeline);
        for (group, binding) in &prepared.groups {
            pass.set_bind_group(group.slot(), binding, &[]);
        }
        let [x, y, z] = spec.dispatch;
        pass.dispatch_workgroups(x, y, z);
    }

    /// Read-only lending of cascade displacement/Jacobian, slopes/differentials and velocity.
    /// Each texel is three vec4<f32>, in that order; see the crate README for channels.
    pub fn cascade_buffer(&self) -> &wgpu::Buffer {
        &self.buffers[&spec::BufferKey::Cascades]
    }
    /// Full LEAN mip chains, five array planes per cascade or long-cache level.
    /// Planes contain displacement/J, rest derivatives, velocity, slope second
    /// moments/dZdz, and J squared/displacement residual. Moments and decoded
    /// displacements filter linearly, including through every mip.
    pub fn filtered_views(&self) -> &[wgpu::TextureView; 2] {
        &self.filtered.views
    }
    /// Read-only texture lending for forward-field readback and tooling.
    /// Index zero is the three cascades; index one is the eight cache levels.
    pub fn filtered_texture(&self, index: usize) -> Option<&wgpu::Texture> {
        self.filtered.textures.get(index)
    }
    /// Sampled texture hazards for the complete forward field and its mips.
    pub fn filtered_accesses(&self) -> Vec<thrum_render::contribution::EffectAccess> {
        self.filtered.surface_accesses(spec::resources().len())
    }
    /// Query samples followed by eight row-major 128-square cache levels, three vec4s per sample.
    pub fn long_buffer(&self) -> &wgpu::Buffer {
        &self.buffers[&spec::BufferKey::LongField]
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
    ) -> Result<(), AmbientError> {
        upload::frame(self, queue, encoder, spectrum, time, anchor, queries)
    }
}
