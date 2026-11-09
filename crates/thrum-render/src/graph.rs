//! Typed GPU pass declarations and preparation-time lifetime analysis.
//!
//! A prepared graph orders live passes by dependencies, then derives attachment
//! operations and transient lifetimes from that order. wgpu applies concrete
//! API transitions while encoding.
//! Exact-compatible, nonoverlapping transient texture lifetimes share a
//! physical allocation retained by the target until its GPU work completes.

use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Arc;

use crate::timing::{GpuTimings, TimingError};

/// A graph-local resource identity. Constructed only by its owning graph.
#[derive(Clone, Debug)]
pub struct ResourceId {
    index: usize,
    owner: Arc<()>,
}
impl PartialEq for ResourceId {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && Arc::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for ResourceId {}

/// Immutable physical properties required to reuse a transient allocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceDescriptor {
    /// A buffer, with ranges expressed in bytes.
    Buffer {
        /// Total byte capacity.
        bytes: u64,
        /// All permitted uses, including copy operations.
        usage: wgpu::BufferUsages,
    },
    /// A 2D texture. Ranges flatten `(mip * layers + layer)`; one unit is
    /// a complete mip/layer. Depth/stencil aspects are conservatively unified.
    Texture {
        /// Base level extent.
        width: u32,
        /// Base level extent.
        height: u32,
        /// Array layers.
        layers: u32,
        /// Mip count.
        mips: u32,
        /// Sample count.
        samples: u32,
        /// Exact format; differently encoded formats cannot alias.
        format: wgpu::TextureFormat,
        /// All permitted uses.
        usage: wgpu::TextureUsages,
    },
}

impl ResourceDescriptor {
    fn units(&self) -> u64 {
        match self {
            Self::Buffer { bytes, .. } => *bytes,
            Self::Texture {
                layers,
                mips,
                width,
                height,
                samples,
                ..
            } => {
                if *width == 0
                    || *height == 0
                    || *mips == 0
                    || !matches!(samples, 1 | 2 | 4 | 8 | 16)
                    || *mips > 32 - width.max(height).leading_zeros()
                    || (*samples != 1 && (*mips != 1 || *layers != 1))
                {
                    0
                } else {
                    u64::from(*layers) * u64::from(*mips)
                }
            }
        }
    }
    fn supports(&self, kind: AccessKind) -> bool {
        match self {
            Self::Buffer { usage, .. } => match kind {
                AccessKind::CopyRead => usage.contains(wgpu::BufferUsages::COPY_SRC),
                AccessKind::CopyWrite => usage.contains(wgpu::BufferUsages::COPY_DST),
                AccessKind::StorageRead
                | AccessKind::StorageWrite
                | AccessKind::StorageReadWrite => usage.contains(wgpu::BufferUsages::STORAGE),
                AccessKind::UniformRead => usage.contains(wgpu::BufferUsages::UNIFORM),
                AccessKind::VertexRead => usage.contains(wgpu::BufferUsages::VERTEX),
                AccessKind::IndexRead => usage.contains(wgpu::BufferUsages::INDEX),
                AccessKind::IndirectRead => usage.contains(wgpu::BufferUsages::INDIRECT),
                _ => false,
            },
            Self::Texture { usage, .. } => match kind {
                AccessKind::CopyRead => usage.contains(wgpu::TextureUsages::COPY_SRC),
                AccessKind::CopyWrite => usage.contains(wgpu::TextureUsages::COPY_DST),
                AccessKind::SampledRead => usage.contains(wgpu::TextureUsages::TEXTURE_BINDING),
                AccessKind::StorageRead
                | AccessKind::StorageWrite
                | AccessKind::StorageReadWrite => {
                    usage.contains(wgpu::TextureUsages::STORAGE_BINDING)
                }
                AccessKind::AttachmentClear | AccessKind::AttachmentLoad => {
                    usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
                }
                _ => false,
            },
        }
    }
}

/// Lifetime authority. Persistent resources never enter the transient alias pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceScope {
    /// Uninitialized allocation local to this graph execution.
    Transient,
    /// An external owner retains lifetime beyond this graph.
    Imported {
        /// Whether all declared units are initialized on entry.
        initialized: bool,
    },
    /// A named persistent owner and invalidation epoch.
    Persistent {
        /// Stable owner identity.
        owner: u64,
        /// Content/history epoch; pass removal cannot release the owner.
        epoch: u64,
        /// Whether all units contain valid data for this epoch.
        initialized: bool,
    },
    /// One side of a retained history pair. The executor binds the last
    /// submitted side for reads and the alternate side for publication.
    History {
        /// Stable owner identity for the pair.
        owner: u64,
        /// Invalidation epoch shared by both sides.
        epoch: u64,
        /// Whether the physical side has defined contents on entry.
        initialized: bool,
    },
}

/// Declared GPU usage. Attachment load both reads and writes its subresource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessKind {
    /// Copy source.
    CopyRead,
    /// Copy destination, defining the full selected range.
    CopyWrite,
    /// Sampled texture.
    SampledRead,
    /// Read-only storage.
    StorageRead,
    /// Storage write, defining the full selected range.
    StorageWrite,
    /// Storage operation consuming and replacing the range.
    StorageReadWrite,
    /// Uniform buffer.
    UniformRead,
    /// Vertex stream.
    VertexRead,
    /// Index stream.
    IndexRead,
    /// Indirect draw/dispatch arguments.
    IndirectRead,
    /// Attachment clear followed by writes.
    AttachmentClear,
    /// Attachment load followed by writes.
    AttachmentLoad,
}
impl AccessKind {
    fn writes(self) -> bool {
        matches!(
            self,
            Self::CopyWrite
                | Self::StorageWrite
                | Self::StorageReadWrite
                | Self::AttachmentClear
                | Self::AttachmentLoad
        )
    }
    fn reads(self) -> bool {
        !matches!(
            self,
            Self::CopyWrite | Self::StorageWrite | Self::AttachmentClear
        )
    }
    fn compatible_buffer_usage(self, other: Self) -> bool {
        // wgpu tracks pass usage for the whole buffer. Read-only usages may
        // combine, but an exclusive write usage cannot combine with another
        // usage even when the declared byte ranges are disjoint. Both storage
        // write declarations map to wgpu's STORAGE_READ_WRITE usage.
        (!self.writes() && !other.writes())
            || self == other
            || matches!(
                (self, other),
                (Self::StorageWrite, Self::StorageReadWrite)
                    | (Self::StorageReadWrite, Self::StorageWrite)
            )
    }
}

/// One initialized range contract. A write must define every selected unit;
/// partial buffer updates must declare only their actually written byte range.
#[derive(Clone, Debug)]
pub struct Access {
    /// Resource from the same graph.
    pub resource: ResourceId,
    /// Half-open bytes or flattened mip/layers.
    pub range: Range<u64>,
    /// Usage.
    pub kind: AccessKind,
    /// Retain a published attachment on an external pass; otherwise attachment
    /// store is derived from later readers and persistent/history scope.
    pub store: bool,
}

/// Device-local prepared pipeline identity. The producer's revision is explicit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PipelineKey {
    /// Device generation, invalidated by device loss.
    pub device_generation: u64,
    /// Producer/content revision (synthetic fixture revision for M1).
    pub revision: u64,
    /// Finite role identity within that producer's closure.
    pub role: u32,
}

/// Pass ownership is distinct from a quality/profile identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassScope {
    /// Scene-wide work shared by views.
    Shared,
    /// Work for one stable view identity.
    View(u64),
}

/// Encoder family selected from a closed pass's declared accesses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PassKind {
    Render,
    Compute,
    Copy,
}

fn pass_kind(pass: &Pass) -> Option<PassKind> {
    let attachment = pass.accesses.iter().any(|access| {
        matches!(
            access.kind,
            AccessKind::AttachmentClear | AccessKind::AttachmentLoad
        )
    });
    let copy = pass
        .accesses
        .iter()
        .any(|access| matches!(access.kind, AccessKind::CopyRead | AccessKind::CopyWrite));
    if attachment && !copy {
        return Some(PassKind::Render);
    }
    if pass.pipeline.is_some() && !attachment && !copy {
        return Some(PassKind::Compute);
    }
    (pass.pipeline.is_none()
        && !pass.accesses.is_empty()
        && pass
            .accesses
            .iter()
            .all(|access| matches!(access.kind, AccessKind::CopyRead | AccessKind::CopyWrite)))
    .then_some(PassKind::Copy)
}

/// A pass declaration. Insertion order breaks ties between independent work;
/// `external_side_effect` retains presentation and readback outputs.
#[derive(Clone, Debug)]
pub struct Pass {
    /// Diagnostic name.
    pub name: String,
    /// View or shared ownership.
    pub scope: PassScope,
    /// Required prepared pipeline; copies need none.
    pub pipeline: Option<PipelineKey>,
    /// Exact resource accesses.
    pub accesses: Vec<Access>,
    /// Presentation, readback or another externally observed effect.
    pub external_side_effect: bool,
}

#[derive(Clone, Debug)]
struct Resource {
    descriptor: ResourceDescriptor,
    scope: ResourceScope,
}

struct PassSchedule {
    order: Vec<usize>,
    edges: Vec<(usize, usize)>,
}

/// Preparation-time graph; compile once per structure/generation change and
/// retain the plan across frames with the same resource and pipeline closure.
#[derive(Default)]
pub struct FrameGraph {
    resources: Vec<Resource>,
    passes: Vec<Pass>,
    explicit_dependencies: Vec<(usize, usize)>,
    owner: Arc<()>,
}

/// Why a declaration cannot execute safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphError {
    /// A zero extent/capacity or invalid texture descriptor.
    EmptyResource,
    /// A retained identity names more than one independent resource.
    DuplicatePersistentOwner {
        /// Conflicting stable owner identity.
        owner: u64,
    },
    /// Missing/empty/out-of-bounds resource range.
    InvalidRange {
        /// Offending pass.
        pass: usize,
    },
    /// Descriptor flags do not authorize this access.
    UsageMismatch {
        /// Offending pass.
        pass: usize,
    },
    /// A same-pass pair has overlapping writes, or incompatible whole-buffer
    /// usages even when its byte ranges are disjoint. Use separate passes for
    /// incompatible usages; merge an overlapping storage read/write declaration.
    ConflictingPassAccess {
        /// Offending pass.
        pass: usize,
    },
    /// Read/load consumes undefined or discarded contents.
    UninitializedRead {
        /// Offending pass.
        pass: usize,
        /// Resource being read.
        resource: ResourceId,
    },
    /// Required tuple is absent or belongs to an old device generation.
    PipelineNotReady {
        /// Offending pass.
        pass: usize,
    },
    /// An explicit dependency names a missing pass or points to itself.
    InvalidDependency,
    /// Overlapping writes leave a read's producer unspecified.
    AmbiguousProducer {
        /// Reading pass.
        pass: usize,
        /// Resource with multiple possible producers.
        resource: ResourceId,
    },
    /// Live pass dependencies form a cycle.
    DependencyCycle,
    /// A recorder names a pass that was removed or does not exist.
    UnknownRecorder {
        /// Offending declaration index.
        pass: usize,
    },
    /// More than one recorder was bound to the same live pass.
    DuplicateRecorder {
        /// Offending declaration index.
        pass: usize,
    },
    /// A live pass has no record closure.
    MissingRecorder {
        /// Offending declaration index.
        pass: usize,
    },
    /// A recorder's encoder family disagrees with the declared accesses.
    RecorderKindMismatch {
        /// Offending declaration index.
        pass: usize,
    },
}

/// A dependency with an overlapping write. These are API-independent reasons
/// for ordering, not additional explicit wgpu barriers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hazard {
    /// Earlier pass index.
    pub before: usize,
    /// Later pass index.
    pub after: usize,
    /// Shared physical resource identity.
    pub resource: ResourceId,
    /// Precisely intersecting range.
    pub range: Range<u64>,
}

/// Resource's first and last pass, and a compatible logical allocation slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lifetime {
    /// Graph resource.
    pub resource: ResourceId,
    /// First pass using it.
    pub first: usize,
    /// Last pass using it. Retirement also waits for GPU completion.
    pub last: usize,
    /// A reusable slot for transient resources only.
    pub transient_slot: Option<usize>,
}

/// Validated execution order, hazards and logical allocation plan.
#[derive(Clone, Debug)]
pub struct CompiledGraph {
    /// Device generation used to validate the closure. Recompile after loss.
    pub device_generation: u64,
    /// Unique live pass dependencies, using declaration indices.
    pub dependencies: Vec<(usize, usize)>,
    /// Detailed overlapping-use reasons.
    pub hazards: Vec<Hazard>,
    /// Used resources only; unused declarations require no allocation.
    pub lifetimes: Vec<Lifetime>,
    /// Number of transient slots after exact-descriptor reuse.
    pub transient_slots: usize,
    /// Declaration indices in actual execution order.
    pub pass_order: Vec<usize>,
    /// Passes removed because no output consumes them.
    pub culled_passes: usize,
    pipeline_pass_count: usize,
    passes: Vec<CompiledPass>,
    resources: Vec<Resource>,
    owner: Arc<()>,
}

/// One closed pass handed to an encoder callback in compiled order.
#[derive(Clone, Debug)]
pub(crate) struct CompiledPass {
    /// Stable diagnostic label chosen at graph closure.
    pub(crate) name: String,
    /// Pipeline identity checked during closure; copy passes have none.
    pub(crate) pipeline: Option<PipelineKey>,
    pub(crate) kind: Option<PassKind>,
    attachments: Vec<CompiledAttachment>,
}

#[derive(Clone, Debug)]
struct CompiledAttachment {
    resource: ResourceId,
    range: Range<u64>,
    load: bool,
    store: bool,
}

pub(crate) struct GraphColorAttachment<'a> {
    pub(crate) resource: &'a ResourceId,
    pub(crate) range: Range<u64>,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) clear: wgpu::Color,
}

pub(crate) struct GraphDepthAttachment<'a> {
    pub(crate) resource: &'a ResourceId,
    pub(crate) range: Range<u64>,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) clear: f32,
}

impl CompiledPass {
    /// Open a declared compute pass with the graph's automatic timestamps.
    pub(crate) fn begin_compute<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        generation: u64,
        timings: Option<&mut GpuTimings>,
    ) -> Result<Option<wgpu::ComputePass<'a>>, TimingError> {
        if !self.attachments.is_empty() || self.pipeline.is_none() {
            return Ok(None);
        }
        let timestamp_writes = timings
            .map(|timings| timings.compute_writes(generation, &self.name))
            .transpose()?
            .flatten();
        Ok(Some(encoder.begin_compute_pass(
            &wgpu::ComputePassDescriptor {
                label: Some(&self.name),
                timestamp_writes,
            },
        )))
    }

    /// The executor, not a recorder, owns attachment load and store decisions.
    pub(crate) fn attachment_ops<T>(
        &self,
        resource: &ResourceId,
        range: Range<u64>,
        clear: T,
    ) -> Option<wgpu::Operations<T>> {
        self.attachments
            .iter()
            .find(|attachment| attachment.resource == *resource && attachment.range == range)
            .map(|attachment| wgpu::Operations {
                load: if attachment.load {
                    wgpu::LoadOp::Load
                } else {
                    wgpu::LoadOp::Clear(clear)
                },
                store: if attachment.store {
                    wgpu::StoreOp::Store
                } else {
                    wgpu::StoreOp::Discard
                },
            })
    }

    /// Open the declared render attachments without allocating on the frame path.
    /// Recorders receive a pass and cannot substitute load/store operations.
    pub(crate) fn begin_render<'a, const N: usize>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        color: [Option<GraphColorAttachment<'_>>; N],
        depth: Option<GraphDepthAttachment<'_>>,
        generation: u64,
        timings: Option<&mut GpuTimings>,
    ) -> Result<Option<wgpu::RenderPass<'a>>, TimingError> {
        let color_count = color
            .iter()
            .filter(|attachment| attachment.is_some())
            .count();
        if color_count == 0 && depth.is_none()
            || self.attachments.len() != color_count + usize::from(depth.is_some())
        {
            return Ok(None);
        }
        let colors = color.map(|attachment| {
            attachment.and_then(|attachment| {
                let ops =
                    self.attachment_ops(attachment.resource, attachment.range, attachment.clear)?;
                Some(wgpu::RenderPassColorAttachment {
                    view: attachment.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops,
                })
            })
        });
        if colors
            .iter()
            .filter(|attachment| attachment.is_some())
            .count()
            != color_count
        {
            return Ok(None);
        }
        let depth = match depth {
            Some(attachment) => {
                let Some(ops) =
                    self.attachment_ops(attachment.resource, attachment.range, attachment.clear)
                else {
                    return Ok(None);
                };
                Some(wgpu::RenderPassDepthStencilAttachment {
                    view: attachment.view,
                    depth_ops: Some(ops),
                    stencil_ops: None,
                })
            }
            None => None,
        };
        let color_attachments = &colors[..];
        let timestamp_writes = timings
            .map(|timings| timings.pass_writes(generation, &self.name))
            .transpose()?
            .flatten();
        Ok(Some(encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some(&self.name),
                color_attachments,
                depth_stencil_attachment: depth,
                timestamp_writes,
                ..Default::default()
            },
        )))
    }
}

/// Textures allocated once for the compiled graph's exact transient slots.
/// The target retains this lease across its frames and GPU submissions.
pub(crate) struct GraphTextures {
    owner: Arc<()>,
    slots: Vec<Option<wgpu::Texture>>,
    resource_slots: Vec<Option<usize>>,
}

impl GraphTextures {
    pub(crate) fn new(device: &wgpu::Device, graph: &CompiledGraph) -> Self {
        let mut slots: Vec<Option<wgpu::Texture>> =
            (0..graph.transient_slots).map(|_| None).collect();
        let mut resource_slots = vec![None; graph.resources.len()];
        for lifetime in &graph.lifetimes {
            let Some(slot) = lifetime.transient_slot else {
                continue;
            };
            let ResourceDescriptor::Texture {
                width,
                height,
                layers,
                mips,
                samples,
                format,
                usage,
            } = graph.resources[lifetime.resource.index].descriptor
            else {
                continue;
            };
            resource_slots[lifetime.resource.index] = Some(slot);
            if slots[slot].is_none() {
                slots[slot] = Some(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("thrum graph transient"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: layers,
                    },
                    mip_level_count: mips,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                }));
            }
        }
        Self {
            owner: Arc::clone(&graph.owner),
            slots,
            resource_slots,
        }
    }

    /// Count unique allocated textures and their unpadded texel storage.
    pub(crate) fn allocation(&self) -> (u32, u64) {
        let mut count = 0;
        let mut bytes = 0;
        for texture in self.slots.iter().flatten() {
            count += 1;
            let (bw, bh) = texture.format().block_dimensions();
            let block_bytes = u64::from(texture.format().block_copy_size(None).unwrap_or(0));
            for mip in 0..texture.mip_level_count() {
                bytes += u64::from((texture.width() >> mip).max(1).div_ceil(bw))
                    * u64::from((texture.height() >> mip).max(1).div_ceil(bh))
                    * u64::from(texture.depth_or_array_layers())
                    * u64::from(texture.sample_count())
                    * block_bytes;
            }
        }
        (count, bytes)
    }

    pub(crate) fn texture(&self, resource: &ResourceId) -> Option<&wgpu::Texture> {
        if !Arc::ptr_eq(&self.owner, &resource.owner) {
            return None;
        }
        self.resource_slots
            .get(resource.index)
            .and_then(|slot| slot.and_then(|index| self.slots[index].as_ref()))
    }
}

impl CompiledGraph {
    /// Visit live passes and their stable declaration identities in execution order.
    pub(crate) fn pass_bindings(&self) -> impl Iterator<Item = (usize, &CompiledPass)> {
        self.pass_order.iter().copied().zip(&self.passes)
    }

    /// Number of render or compute pipelines requiring timestamp slots.
    pub(crate) fn pipeline_pass_count(&self) -> usize {
        self.pipeline_pass_count
    }
}

impl FrameGraph {
    fn attachment_store(&self, order: &[usize], position: usize, access: &Access) -> bool {
        if (self.passes[order[position]].external_side_effect && access.store)
            || matches!(
                self.resources[access.resource.index].scope,
                ResourceScope::Persistent { .. } | ResourceScope::History { .. }
            )
        {
            return true;
        }
        let mut live = vec![access.range.clone()];
        for &index in &order[position + 1..] {
            let pass = &self.passes[index];
            for next in &pass.accesses {
                if next.resource != access.resource {
                    continue;
                }
                if next.kind.reads() && live.iter().any(|range| overlap(range, &next.range)) {
                    return true;
                }
                if next.kind.writes() {
                    remove_range(&mut live, &next.range);
                    if live.is_empty() {
                        return false;
                    }
                }
            }
        }
        false
    }

    /// Add a resource; its identity is local to this graph.
    ///
    /// # Errors
    ///
    /// Returns an error if the resource has no valid units.
    pub fn resource(
        &mut self,
        descriptor: ResourceDescriptor,
        scope: ResourceScope,
    ) -> Result<ResourceId, GraphError> {
        if descriptor.units() == 0 {
            return Err(GraphError::EmptyResource);
        }
        let conflict = match scope {
            ResourceScope::Persistent { owner, .. } => self.resources.iter().any(|resource| {
                matches!(resource.scope, ResourceScope::Persistent { owner: existing, .. } | ResourceScope::History { owner: existing, .. } if existing == owner)
            }),
            ResourceScope::History { owner, .. } => self.resources.iter().any(|resource| {
                matches!(resource.scope, ResourceScope::Persistent { owner: existing, .. } if existing == owner)
            }),
            ResourceScope::Transient | ResourceScope::Imported { .. } => false,
        };
        if conflict {
            let owner = match scope {
                ResourceScope::Persistent { owner, .. } | ResourceScope::History { owner, .. } => {
                    owner
                }
                ResourceScope::Transient | ResourceScope::Imported { .. } => unreachable!(),
            };
            return Err(GraphError::DuplicatePersistentOwner { owner });
        }
        let id = ResourceId {
            index: self.resources.len(),
            owner: Arc::clone(&self.owner),
        };
        self.resources.push(Resource { descriptor, scope });
        Ok(id)
    }
    /// Append a pass and return its declaration index for explicit dependencies.
    pub fn pass(&mut self, pass: Pass) -> usize {
        let index = self.passes.len();
        self.passes.push(pass);
        index
    }
    /// Require `before` to execute before `after`. A direct writer-to-reader
    /// edge also selects a producer when several writers overlap that read.
    ///
    /// # Errors
    ///
    /// Returns an error for an absent pass or self-dependency. Cycles are
    /// checked when compiling the graph.
    pub fn dependency(&mut self, before: usize, after: usize) -> Result<(), GraphError> {
        if before == after || before >= self.passes.len() || after >= self.passes.len() {
            return Err(GraphError::InvalidDependency);
        }
        self.explicit_dependencies.push((before, after));
        Ok(())
    }
    /// Inspect the exact allocation properties for an identity from this graph.
    /// A foreign identity returns None even when its numeric slot matches.
    pub fn descriptor(&self, resource: &ResourceId) -> Option<&ResourceDescriptor> {
        if !Arc::ptr_eq(&resource.owner, &self.owner) {
            return None;
        }
        self.resources
            .get(resource.index)
            .map(|resource| &resource.descriptor)
    }
    /// Inspect the lifetime authority retained independently of pass presence.
    pub fn scope(&self, resource: &ResourceId) -> Option<ResourceScope> {
        if !Arc::ptr_eq(&resource.owner, &self.owner) {
            return None;
        }
        self.resources
            .get(resource.index)
            .map(|resource| resource.scope)
    }
    /// Inspect immutable pass declarations for the executor.
    pub fn passes(&self) -> &[Pass] {
        &self.passes
    }
    fn initial_contents(&self, resource: usize) -> bool {
        matches!(
            self.resources[resource].scope,
            ResourceScope::Imported { initialized: true }
                | ResourceScope::Persistent {
                    initialized: true,
                    ..
                }
                | ResourceScope::History {
                    initialized: true,
                    ..
                }
        )
    }

    fn selected_producers(
        &self,
        consumer: usize,
        access: &Access,
        require_explicit_overlap: bool,
    ) -> Result<Vec<(usize, Range<u64>)>, GraphError> {
        let mut cuts = vec![access.range.start, access.range.end];
        for pass in &self.passes {
            for write in &pass.accesses {
                if write.resource == access.resource
                    && write.kind.writes()
                    && overlap(&write.range, &access.range)
                {
                    cuts.push(write.range.start.max(access.range.start));
                    cuts.push(write.range.end.min(access.range.end));
                }
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        let mut selected = Vec::new();
        for pair in cuts.windows(2) {
            let range = pair[0]..pair[1];
            let writers = self
                .passes
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != consumer)
                .filter(|(_, pass)| {
                    pass.accesses.iter().any(|write| {
                        write.resource == access.resource
                            && write.kind.writes()
                            && write.range.start <= range.start
                            && write.range.end >= range.end
                    })
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let explicit = writers
                .iter()
                .copied()
                .filter(|&writer| self.explicit_dependencies.contains(&(writer, consumer)))
                .collect::<Vec<_>>();
            let prior = writers
                .iter()
                .copied()
                .filter(|&writer| writer < consumer)
                .max();
            let producer = if explicit.len() > 1 {
                return Err(GraphError::AmbiguousProducer {
                    pass: consumer,
                    resource: access.resource.clone(),
                });
            } else if let Some(&chosen) = explicit.first() {
                if chosen < consumer
                    && writers.iter().any(|&writer| {
                        writer > chosen
                            && writer < consumer
                            && !self.explicit_dependencies.contains(&(consumer, writer))
                    })
                {
                    return Err(GraphError::AmbiguousProducer {
                        pass: consumer,
                        resource: access.resource.clone(),
                    });
                }
                Some(chosen)
            } else if prior.is_none() && self.initial_contents(access.resource.index) {
                None
            } else if require_explicit_overlap && writers.len() > 1 {
                return Err(GraphError::AmbiguousProducer {
                    pass: consumer,
                    resource: access.resource.clone(),
                });
            } else if let Some(prior) = prior {
                Some(prior)
            } else if writers.len() == 1 {
                Some(writers[0])
            } else if writers.is_empty() {
                return Err(GraphError::UninitializedRead {
                    pass: consumer,
                    resource: access.resource.clone(),
                });
            } else {
                return Err(GraphError::AmbiguousProducer {
                    pass: consumer,
                    resource: access.resource.clone(),
                });
            };
            if let Some(writer) = producer {
                selected.push((writer, range));
            }
        }
        Ok(selected)
    }

    fn validate_declarations(&self) -> Result<(), GraphError> {
        for (index, pass) in self.passes.iter().enumerate() {
            for (a_index, access) in pass.accesses.iter().enumerate() {
                if !Arc::ptr_eq(&access.resource.owner, &self.owner) {
                    return Err(GraphError::InvalidRange { pass: index });
                }
                let Some(resource) = self.resources.get(access.resource.index) else {
                    return Err(GraphError::InvalidRange { pass: index });
                };
                if access.range.start >= access.range.end
                    || access.range.end > resource.descriptor.units()
                {
                    return Err(GraphError::InvalidRange { pass: index });
                }
                if !resource.descriptor.supports(access.kind) {
                    return Err(GraphError::UsageMismatch { pass: index });
                }
                if pass.accesses[..a_index].iter().any(|other| {
                    other.resource == access.resource
                        && ((overlap(&other.range, &access.range)
                            && (other.kind.writes() || access.kind.writes()))
                            || (matches!(resource.descriptor, ResourceDescriptor::Buffer { .. })
                                && !other.kind.compatible_buffer_usage(access.kind)))
                }) {
                    return Err(GraphError::ConflictingPassAccess { pass: index });
                }
            }
        }
        Ok(())
    }

    fn schedule(&self) -> Result<PassSchedule, GraphError> {
        // Graphs with no published output remain all-pass diagnostic plans for
        // existing callers. The game graph declares present/readback roots.
        let mut live = self
            .passes
            .iter()
            .map(|pass| {
                pass.external_side_effect
                    || pass.accesses.iter().any(|access| {
                        access.kind.writes()
                            && matches!(
                                self.resources[access.resource.index].scope,
                                ResourceScope::Persistent { .. } | ResourceScope::History { .. }
                            )
                    })
            })
            .collect::<Vec<_>>();
        let diagnostic = !live.iter().any(|&root| root);
        if diagnostic {
            live.fill(true);
        }
        let mut data_edges = Vec::new();
        let mut selected = Vec::new();
        let mut queue = live
            .iter()
            .enumerate()
            .filter_map(|(index, &is_live)| is_live.then_some(index))
            .collect::<Vec<_>>();
        while let Some(consumer) = queue.pop() {
            for &(producer, after) in &self.explicit_dependencies {
                if after == consumer && !live[producer] {
                    live[producer] = true;
                    queue.push(producer);
                }
            }
            for access in &self.passes[consumer].accesses {
                if !access.kind.reads() {
                    continue;
                }
                for (producer, range) in self.selected_producers(consumer, access, !diagnostic)? {
                    data_edges.push((producer, consumer));
                    selected.push((consumer, producer, access.resource.clone(), range));
                    if !live[producer] {
                        live[producer] = true;
                        queue.push(producer);
                    }
                }
            }
        }
        let mut edges = BTreeSet::new();
        edges.extend(data_edges);
        edges.extend(
            self.explicit_dependencies
                .iter()
                .copied()
                .filter(|&(before, after)| live[before] && live[after]),
        );
        for (before, prior) in self.passes.iter().enumerate() {
            if !live[before] {
                continue;
            }
            for (after, next) in self.passes.iter().enumerate().skip(before + 1) {
                if !live[after] || edges.contains(&(after, before)) {
                    continue;
                }
                let conflicts = prior.accesses.iter().any(|a| {
                    next.accesses.iter().any(|b| {
                        a.resource == b.resource
                            && overlap(&a.range, &b.range)
                            && (a.kind.writes() || b.kind.writes())
                    })
                });
                if !conflicts {
                    continue;
                }
                let before_read_waits_for_later_write = prior.accesses.iter().any(|a| {
                    a.kind.reads()
                        && next.accesses.iter().any(|b| {
                            a.resource == b.resource
                                && b.kind.writes()
                                && overlap(&a.range, &b.range)
                                && selected.iter().any(|(reader, writer, resource, range)| {
                                    *reader == before
                                        && *writer >= after
                                        && *resource == a.resource
                                        && overlap(range, &b.range)
                                })
                        })
                });
                if !before_read_waits_for_later_write {
                    edges.insert((before, after));
                }
            }
        }
        let mut order = Vec::with_capacity(live.iter().filter(|&&value| value).count());
        let mut remaining = live;
        loop {
            let next = remaining.iter().enumerate().find_map(|(index, &is_live)| {
                (is_live
                    && !edges
                        .iter()
                        .any(|&(before, after)| after == index && remaining[before]))
                .then_some(index)
            });
            let Some(next) = next else {
                if remaining.iter().any(|&value| value) {
                    return Err(GraphError::DependencyCycle);
                }
                break;
            };
            remaining[next] = false;
            order.push(next);
        }
        Ok(PassSchedule {
            order,
            edges: edges.into_iter().collect(),
        })
    }
    /// Compile a variant with one terminal pass while preserving shared declaration IDs.
    pub(crate) fn compile_with_terminal_pass(
        &mut self,
        pass: Pass,
        generation: u64,
        prepared: &BTreeSet<PipelineKey>,
    ) -> Result<CompiledGraph, GraphError> {
        self.pass(pass);
        let compiled = self.compile(generation, prepared);
        drop(self.passes.pop());
        compiled
    }

    /// Validate closure and initialization, derive hazards and reuse. This is
    /// preparation work (quadratic in declared accesses), never per-instance work.
    ///
    /// # Errors
    ///
    /// Returns a graph error for invalid resource identity, access, pipeline
    /// closure or uninitialized reads.
    pub fn compile(
        &self,
        generation: u64,
        prepared: &BTreeSet<PipelineKey>,
    ) -> Result<CompiledGraph, GraphError> {
        self.validate_declarations()?;
        let PassSchedule {
            order,
            edges: schedule_edges,
        } = self.schedule()?;
        let mut initialized: Vec<Vec<Range<u64>>> = self
            .resources
            .iter()
            .map(|r| match r.scope {
                ResourceScope::Transient
                | ResourceScope::Imported { initialized: false }
                | ResourceScope::Persistent {
                    initialized: false, ..
                }
                | ResourceScope::History {
                    initialized: false, ..
                } => Vec::new(),
                _ => std::iter::once(0..r.descriptor.units()).collect(),
            })
            .collect();
        let mut prior: Vec<(usize, &Access)> = Vec::new();
        let mut hazards = Vec::new();
        let mut edges = BTreeSet::from_iter(schedule_edges);
        let mut bounds: Vec<Option<(usize, usize)>> = vec![None; self.resources.len()];
        for (position, &index) in order.iter().enumerate() {
            let pass = &self.passes[index];
            if pass
                .pipeline
                .is_some_and(|key| key.device_generation != generation || !prepared.contains(&key))
            {
                return Err(GraphError::PipelineNotReady { pass: index });
            }
            for access in &pass.accesses {
                if access.kind.reads()
                    && !covered(&initialized[access.resource.index], &access.range)
                {
                    return Err(GraphError::UninitializedRead {
                        pass: index,
                        resource: access.resource.clone(),
                    });
                }
                for (before, previous) in &prior {
                    if previous.resource == access.resource
                        && overlap(&previous.range, &access.range)
                        && (previous.kind.writes() || access.kind.writes())
                    {
                        edges.insert((*before, index));
                        hazards.push(Hazard {
                            before: *before,
                            after: index,
                            resource: access.resource.clone(),
                            range: previous.range.start.max(access.range.start)
                                ..previous.range.end.min(access.range.end),
                        });
                    }
                }
                if access.kind.writes() {
                    remove_range(&mut initialized[access.resource.index], &access.range);
                    let store = if matches!(
                        access.kind,
                        AccessKind::AttachmentClear | AccessKind::AttachmentLoad
                    ) {
                        self.attachment_store(&order, position, access)
                    } else {
                        access.store
                    };
                    if store {
                        initialized[access.resource.index].push(access.range.clone());
                    }
                }
                bounds[access.resource.index] = Some(
                    bounds[access.resource.index]
                        .map_or((position, position), |(first, _)| (first, position)),
                );
            }
            prior.extend(pass.accesses.iter().map(|a| (index, a)));
        }
        let mut lifetimes: Vec<Lifetime> = bounds
            .into_iter()
            .enumerate()
            .filter_map(|(id, bounds)| {
                bounds.map(|(first, last)| Lifetime {
                    resource: ResourceId {
                        index: id,
                        owner: Arc::clone(&self.owner),
                    },
                    first,
                    last,
                    transient_slot: None,
                })
            })
            .collect();
        lifetimes.sort_by_key(|l| (l.first, l.resource.index));
        let mut slots: Vec<(ResourceDescriptor, usize)> = Vec::new();
        for life in &mut lifetimes {
            let resource = &self.resources[life.resource.index];
            if resource.scope != ResourceScope::Transient {
                continue;
            }
            if let Some((slot, (_, last))) = slots
                .iter_mut()
                .enumerate()
                .find(|(_, (desc, last))| *desc == resource.descriptor && *last < life.first)
            {
                *last = life.last;
                life.transient_slot = Some(slot);
            } else {
                life.transient_slot = Some(slots.len());
                slots.push((resource.descriptor.clone(), life.last));
            }
        }
        Ok(CompiledGraph {
            device_generation: generation,
            dependencies: edges.into_iter().collect(),
            hazards,
            lifetimes,
            transient_slots: slots.len(),
            pass_order: order.clone(),
            culled_passes: self.passes.len() - order.len(),
            pipeline_pass_count: order
                .iter()
                .filter(|&&index| self.passes[index].pipeline.is_some())
                .count(),
            passes: order
                .iter()
                .enumerate()
                .map(|(position, &index)| CompiledPass {
                    name: self.passes[index].name.clone(),
                    pipeline: self.passes[index].pipeline,
                    kind: pass_kind(&self.passes[index]),
                    attachments: self.passes[index]
                        .accesses
                        .iter()
                        .filter(|access| {
                            matches!(
                                access.kind,
                                AccessKind::AttachmentClear | AccessKind::AttachmentLoad
                            )
                        })
                        .map(|access| CompiledAttachment {
                            resource: access.resource.clone(),
                            range: access.range.clone(),
                            load: access.kind == AccessKind::AttachmentLoad,
                            store: self.attachment_store(&order, position, access),
                        })
                        .collect(),
                })
                .collect(),
            resources: self.resources.clone(),
            owner: Arc::clone(&self.owner),
        })
    }
}
fn overlap(a: &Range<u64>, b: &Range<u64>) -> bool {
    a.start < b.end && b.start < a.end
}
fn covered(ranges: &[Range<u64>], target: &Range<u64>) -> bool {
    let mut cursor = target.start;
    loop {
        let next = ranges
            .iter()
            .filter(|r| r.start <= cursor && r.end > cursor)
            .map(|r| r.end)
            .max();
        match next {
            Some(end) if end >= target.end => return true,
            Some(end) => cursor = end,
            None => return false,
        }
    }
}
fn remove_range(ranges: &mut Vec<Range<u64>>, cut: &Range<u64>) {
    let mut remaining = Vec::with_capacity(ranges.len() + 1);
    for range in ranges.drain(..) {
        if !overlap(&range, cut) {
            remaining.push(range);
            continue;
        }
        if range.start < cut.start {
            remaining.push(range.start..cut.start);
        }
        if range.end > cut.end {
            remaining.push(cut.end..range.end);
        }
    }
    *ranges = remaining;
}

#[cfg(test)]
mod tests {
    use super::{
        Access, AccessKind, FrameGraph, Pass, PassScope, ResourceDescriptor, ResourceScope,
    };
    use std::collections::BTreeSet;

    #[test]
    fn attachment_store_follows_the_first_live_consumer_and_overwrite() {
        let mut graph = FrameGraph::default();
        let texture = graph
            .resource(
                ResourceDescriptor::Texture {
                    width: 4,
                    height: 4,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                },
                ResourceScope::Transient,
            )
            .expect("valid target");
        for (name, kind) in [
            ("discarded clear", AccessKind::AttachmentClear),
            ("sampled clear", AccessKind::AttachmentClear),
            ("sample", AccessKind::SampledRead),
        ] {
            graph.pass(Pass {
                name: name.into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![Access {
                    resource: texture.clone(),
                    range: 0..1,
                    kind,
                    store: false,
                }],
                external_side_effect: name == "sample",
            });
        }
        assert!(matches!(
            graph.compile(1, &BTreeSet::new()),
            Err(super::GraphError::AmbiguousProducer { pass: 2, .. })
        ));
        graph.dependency(1, 2).expect("select sampled clear");
        let compiled = graph.compile(1, &BTreeSet::new()).expect("valid graph");
        assert_eq!(compiled.pass_order, [1, 2]);
        let mut stores = Vec::new();
        for (_, pass) in compiled.pass_bindings() {
            if let Some(ops) = pass.attachment_ops(&texture, 0..1, wgpu::Color::BLACK) {
                stores.push(ops.store);
                assert!(matches!(ops.load, wgpu::LoadOp::Clear(_)));
            }
        }
        assert_eq!(compiled.culled_passes, 1);
        assert_eq!(stores, [wgpu::StoreOp::Store]);
    }

    #[test]
    fn compiled_order_keeps_the_producer_before_its_consumer() {
        let mut graph = FrameGraph::default();
        let buffer = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 16,
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Transient,
            )
            .expect("buffer");
        for (name, kind) in [
            ("producer", AccessKind::CopyWrite),
            ("consumer", AccessKind::CopyRead),
            ("present", AccessKind::CopyRead),
        ] {
            graph.pass(Pass {
                name: name.into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![Access {
                    resource: buffer.clone(),
                    range: 0..16,
                    kind,
                    store: true,
                }],
                external_side_effect: name != "producer",
            });
        }
        let compiled = graph.compile(1, &BTreeSet::new()).expect("valid closure");
        assert_eq!(compiled.dependencies, vec![(0, 1), (0, 2)]);
        assert_eq!(
            compiled
                .pass_bindings()
                .map(|(_, pass)| pass.name.as_str())
                .collect::<Vec<_>>(),
            ["producer", "consumer", "present"]
        );
    }

    #[test]
    fn later_declared_producer_executes_before_its_live_consumer() {
        let mut graph = FrameGraph::default();
        let buffer = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 16,
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Transient,
            )
            .expect("buffer");
        for (name, kind, external_side_effect) in [
            ("consumer", AccessKind::CopyRead, true),
            ("producer", AccessKind::CopyWrite, false),
        ] {
            graph.pass(Pass {
                name: name.into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![Access {
                    resource: buffer.clone(),
                    range: 0..16,
                    kind,
                    store: true,
                }],
                external_side_effect,
            });
        }
        let compiled = graph.compile(1, &BTreeSet::new()).expect("topology");
        assert_eq!(compiled.pass_order, [1, 0]);
        assert_eq!(compiled.dependencies, [(1, 0)]);
        assert_eq!(
            (compiled.lifetimes[0].first, compiled.lifetimes[0].last),
            (0, 1)
        );
        let visited = compiled
            .pass_bindings()
            .map(|(_, pass)| pass.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(visited, ["producer", "consumer"]);
    }

    #[test]
    fn live_outputs_cull_dead_work_and_reuse_disjoint_transients() {
        let mut graph = FrameGraph::default();
        let mut resources = Vec::new();
        for _ in 0..3 {
            resources.push(
                graph
                    .resource(
                        ResourceDescriptor::Buffer {
                            bytes: 16,
                            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                        },
                        ResourceScope::Transient,
                    )
                    .expect("buffer"),
            );
        }
        let write = |name: &str, resource: &super::ResourceId| Pass {
            name: name.into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![Access {
                resource: resource.clone(),
                range: 0..16,
                kind: AccessKind::CopyWrite,
                store: true,
            }],
            external_side_effect: false,
        };
        let read = |name: &str, resource: &super::ResourceId| Pass {
            name: name.into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![Access {
                resource: resource.clone(),
                range: 0..16,
                kind: AccessKind::CopyRead,
                store: true,
            }],
            external_side_effect: true,
        };
        graph.pass(write("dead", &resources[0]));
        graph.pass(write("first", &resources[1]));
        graph.pass(read("first output", &resources[1]));
        graph.pass(write("second", &resources[2]));
        graph.pass(read("second output", &resources[2]));
        let compiled = graph.compile(1, &BTreeSet::new()).expect("live outputs");
        assert_eq!(compiled.culled_passes, 1);
        assert_eq!(compiled.pass_order, [1, 2, 3, 4]);
        assert_eq!(compiled.lifetimes.len(), 2);
        assert_eq!(compiled.transient_slots, 1);
    }

    #[test]
    fn cycle_and_invalid_dependency_refuse_before_execution() {
        let mut graph = FrameGraph::default();
        let a = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 16,
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Transient,
            )
            .expect("first buffer");
        let b = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 16,
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Transient,
            )
            .expect("second buffer");
        assert_eq!(
            graph.dependency(0, 0),
            Err(super::GraphError::InvalidDependency)
        );
        for (read, write) in [(&b, &a), (&a, &b)] {
            graph.pass(Pass {
                name: "cycle".into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![
                    Access {
                        resource: read.clone(),
                        range: 0..16,
                        kind: AccessKind::CopyRead,
                        store: true,
                    },
                    Access {
                        resource: write.clone(),
                        range: 0..16,
                        kind: AccessKind::CopyWrite,
                        store: true,
                    },
                ],
                external_side_effect: read == &b,
            });
        }
        assert_eq!(
            graph.compile(1, &BTreeSet::new()).unwrap_err(),
            super::GraphError::DependencyCycle
        );
    }

    #[test]
    fn persistent_identity_cannot_name_a_second_resource_or_history_pair() {
        let mut graph = FrameGraph::default();
        let descriptor = ResourceDescriptor::Buffer {
            bytes: 16,
            usage: wgpu::BufferUsages::STORAGE,
        };
        let persistent = ResourceScope::Persistent {
            owner: 7,
            epoch: 1,
            initialized: false,
        };
        let history = ResourceScope::History {
            owner: 7,
            epoch: 1,
            initialized: false,
        };
        graph
            .resource(descriptor.clone(), persistent)
            .expect("first retained owner");
        for scope in [persistent, history] {
            assert!(matches!(
                graph.resource(descriptor.clone(), scope),
                Err(super::GraphError::DuplicatePersistentOwner { owner: 7 })
            ));
        }
        let mut other = FrameGraph::default();
        other
            .resource(descriptor.clone(), history)
            .expect("first history side");
        other
            .resource(descriptor, history)
            .expect("the same history owner has two physical sides");
    }

    #[test]
    fn persistent_history_write_is_live_and_stored_for_the_next_frame() {
        let mut graph = FrameGraph::default();
        let history = graph
            .resource(
                ResourceDescriptor::Texture {
                    width: 4,
                    height: 4,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                },
                ResourceScope::Persistent {
                    owner: 7,
                    epoch: 2,
                    initialized: false,
                },
            )
            .expect("history texture");
        graph.pass(Pass {
            name: "history publish".into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![Access {
                resource: history.clone(),
                range: 0..1,
                kind: AccessKind::AttachmentClear,
                store: false,
            }],
            external_side_effect: false,
        });
        let compiled = graph.compile(1, &BTreeSet::new()).expect("history root");
        assert_eq!(compiled.pass_order, [0]);
        assert_eq!(compiled.culled_passes, 0);
        let ops = compiled
            .pass_bindings()
            .next()
            .expect("history pass")
            .1
            .attachment_ops(&history, 0..1, wgpu::Color::BLACK)
            .expect("history attachment");
        assert_eq!(ops.store, wgpu::StoreOp::Store);
    }

    #[test]
    fn history_pair_reads_prior_side_and_publishes_alternate_without_aliasing() {
        let mut graph = FrameGraph::default();
        let descriptor = ResourceDescriptor::Buffer {
            bytes: 80,
            usage: wgpu::BufferUsages::STORAGE,
        };
        let scope = ResourceScope::History {
            owner: 7,
            epoch: 2,
            initialized: true,
        };
        let prior = graph
            .resource(descriptor.clone(), scope)
            .expect("prior side");
        let next = graph.resource(descriptor, scope).expect("next side");
        let read = graph.pass(Pass {
            name: "consume prior transform".into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![Access {
                resource: prior.clone(),
                range: 0..80,
                kind: AccessKind::StorageRead,
                store: true,
            }],
            external_side_effect: false,
        });
        let publish = graph.pass(Pass {
            name: "publish next transform".into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![Access {
                resource: next.clone(),
                range: 0..80,
                kind: AccessKind::StorageWrite,
                store: true,
            }],
            external_side_effect: false,
        });
        graph.dependency(read, publish).expect("history order");
        let compiled = graph.compile(1, &BTreeSet::new()).expect("history closure");
        assert_eq!(compiled.pass_order, [read, publish]);
        assert_eq!(compiled.dependencies, [(read, publish)]);
        assert!(
            compiled
                .lifetimes
                .iter()
                .all(|item| item.transient_slot.is_none())
        );
        assert_eq!(graph.scope(&prior), Some(scope));
        assert_eq!(graph.scope(&next), Some(scope));
    }

    #[test]
    fn published_motion_does_not_force_unused_depth_to_store() {
        let mut graph = FrameGraph::default();
        let texture = |graph: &mut FrameGraph, format| {
            graph
                .resource(
                    ResourceDescriptor::Texture {
                        width: 4,
                        height: 4,
                        layers: 1,
                        mips: 1,
                        samples: 1,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    },
                    ResourceScope::Transient,
                )
                .expect("attachment")
        };
        let motion = texture(&mut graph, wgpu::TextureFormat::Rg16Float);
        let depth = texture(&mut graph, wgpu::TextureFormat::Depth32Float);
        graph.pass(Pass {
            name: "published motion with local depth".into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: [(&motion, true), (&depth, false)]
                .map(|(resource, store)| Access {
                    resource: resource.clone(),
                    range: 0..1,
                    kind: AccessKind::AttachmentClear,
                    store,
                })
                .into(),
            external_side_effect: true,
        });
        let compiled = graph.compile(1, &BTreeSet::new()).expect("frame output");
        let pass = compiled.pass_bindings().next().expect("render pass").1;
        assert_eq!(
            pass.attachment_ops(&motion, 0..1, wgpu::Color::BLACK)
                .expect("motion ops")
                .store,
            wgpu::StoreOp::Store
        );
        assert_eq!(
            pass.attachment_ops(&depth, 0..1, 0.0)
                .expect("depth ops")
                .store,
            wgpu::StoreOp::Discard
        );
    }

    #[test]
    fn explicit_prerequisite_survives_culling_with_its_output() {
        let mut graph = FrameGraph::default();
        let pass = |name: &str, external_side_effect| Pass {
            name: name.into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: Vec::new(),
            external_side_effect,
        };
        let prerequisite = graph.pass(pass("prerequisite", false));
        graph.pass(pass("unrelated", false));
        let output = graph.pass(pass("output", true));
        graph
            .dependency(prerequisite, output)
            .expect("valid prerequisite");
        let compiled = graph
            .compile(1, &BTreeSet::new())
            .expect("live prerequisite");
        assert_eq!(compiled.pass_order, [prerequisite, output]);
        assert_eq!(compiled.culled_passes, 1);
        assert_eq!(compiled.dependencies, [(prerequisite, output)]);
        let recorded = compiled
            .pass_bindings()
            .map(|(_, pass)| pass.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(recorded, ["prerequisite", "output"]);
    }

    #[test]
    fn selecting_an_older_writer_requires_moving_later_writes_after_the_read() {
        let mut graph = FrameGraph::default();
        let buffer = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 16,
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Transient,
            )
            .expect("buffer");
        for (name, kind, external_side_effect) in [
            ("first write", AccessKind::CopyWrite, false),
            ("second write", AccessKind::CopyWrite, true),
            ("read first", AccessKind::CopyRead, true),
        ] {
            graph.pass(Pass {
                name: name.into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![Access {
                    resource: buffer.clone(),
                    range: 0..16,
                    kind,
                    store: true,
                }],
                external_side_effect,
            });
        }
        graph.dependency(0, 2).expect("select first writer");
        assert_eq!(
            graph.compile(1, &BTreeSet::new()).unwrap_err(),
            super::GraphError::AmbiguousProducer {
                pass: 2,
                resource: buffer.clone(),
            }
        );
        graph.dependency(2, 1).expect("read before second writer");
        let compiled = graph
            .compile(1, &BTreeSet::new())
            .expect("ordered versions");
        assert_eq!(compiled.pass_order, [0, 2, 1]);
    }
}
