//! Typed, ordered GPU pass declarations and preparation-time lifetime analysis.
//!
//! The graph preserves declared order. Overlapping accesses involving a write
//! become hazard edges; wgpu applies concrete API transitions while encoding.
//! Aliases are a logical plan only, never an assertion of physical residency.

use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Arc;

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
    /// False models an attachment discard; later reads require another writer.
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

/// A pass declaration, ordered by insertion. No automatic pass culling hides
/// external side effects or unmodelled ownership.
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

/// Preparation-time graph; compile once per structure/generation change and
/// retain the plan across frames with the same resource and pipeline closure.
#[derive(Default)]
pub struct FrameGraph {
    resources: Vec<Resource>,
    passes: Vec<Pass>,
    owner: Arc<()>,
}

/// Why a declaration cannot execute safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphError {
    /// A zero extent/capacity or invalid texture descriptor.
    EmptyResource,
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
    /// Unique pass-to-pass hazard edges in declaration order.
    pub dependencies: Vec<(usize, usize)>,
    /// Detailed overlapping-use reasons.
    pub hazards: Vec<Hazard>,
    /// Used resources only; unused declarations require no allocation.
    pub lifetimes: Vec<Lifetime>,
    /// Number of transient slots after exact-descriptor reuse.
    pub transient_slots: usize,
}

impl FrameGraph {
    /// Add a resource; its identity is local to this graph.
    pub fn resource(
        &mut self,
        descriptor: ResourceDescriptor,
        scope: ResourceScope,
    ) -> Result<ResourceId, GraphError> {
        if descriptor.units() == 0 {
            return Err(GraphError::EmptyResource);
        }
        let id = ResourceId {
            index: self.resources.len(),
            owner: self.owner.clone(),
        };
        self.resources.push(Resource { descriptor, scope });
        Ok(id)
    }
    /// Append one pass to the intended execution order.
    pub fn pass(&mut self, pass: Pass) {
        self.passes.push(pass);
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
    /// Validate closure and initialization, derive hazards and reuse. This is
    /// preparation work (quadratic in declared accesses), never per-instance work.
    pub fn compile(
        &self,
        generation: u64,
        prepared: &BTreeSet<PipelineKey>,
    ) -> Result<CompiledGraph, GraphError> {
        let mut initialized: Vec<Vec<Range<u64>>> = self
            .resources
            .iter()
            .map(|r| match r.scope {
                ResourceScope::Transient
                | ResourceScope::Imported { initialized: false }
                | ResourceScope::Persistent {
                    initialized: false, ..
                } => Vec::new(),
                _ => std::iter::once(0..r.descriptor.units()).collect(),
            })
            .collect();
        let mut prior: Vec<(usize, &Access)> = Vec::new();
        let mut hazards = Vec::new();
        let mut edges = BTreeSet::new();
        let mut bounds: Vec<Option<(usize, usize)>> = vec![None; self.resources.len()];
        for (index, pass) in self.passes.iter().enumerate() {
            if pass
                .pipeline
                .is_some_and(|key| key.device_generation != generation || !prepared.contains(&key))
            {
                return Err(GraphError::PipelineNotReady { pass: index });
            }
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
                    if access.store {
                        initialized[access.resource.index].push(access.range.clone());
                    }
                }
                bounds[access.resource.index] = Some(
                    bounds[access.resource.index]
                        .map_or((index, index), |(first, _)| (first, index)),
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
                        owner: self.owner.clone(),
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
