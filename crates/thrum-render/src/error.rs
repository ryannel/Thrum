//! Errors shared across renderer features.

use std::{error::Error, fmt};

/// Expected failures before a view can be encoded or published ready.
#[derive(Debug)]
pub enum RenderError {
    /// Failure at the hardware recipe boundary.
    Recipe(crate::capabilities::RecipeError),
    /// Failure at the presentation boundary.
    Present(crate::present::PresentError),
    /// Failure at the scene ABI boundary.
    Abi(crate::abi::AbiError),
    /// Failure at the frame graph boundary.
    Graph(crate::graph::GraphError),
    /// Failure at the GPU timing boundary.
    Timing(crate::timing::TimingError),
    /// Failure at the effect graph boundary.
    EffectGraph(EffectGraphError),
    /// Failure at the mesh arena boundary.
    Arena(ArenaError),
    /// Failure at the GPU validation boundary.
    Gpu(wgpu::Error),
    /// Sprite scene has no authored sheets.
    MissingSpriteSheets,
    /// An authored sheet exceeds the supported sheet count.
    SpriteSheetLimit {
        /// First sheet beyond the limit.
        name: String,
        /// Supported number of sheets.
        limit: usize,
    },
    /// Sprite order band is nonfinite or negative.
    InvalidSpriteOrderBand,
    /// A sprite sheet has invalid texture, texel density or frame data.
    InvalidSpriteSheet(String),
    /// 2D capture supports only SDR and scene-linear output.
    UnsupportedSpriteCapture,
    /// The native preparation lock was poisoned.
    NativePreparationPoisoned,
    /// The native candidate is already reserved by an encoded frame.
    NativeCandidateReserved,
    /// Could not prepare the scene ray structure.
    RayPreparation,
    /// Synthetic grid requires between 1 and 1,000,000 instances.
    GridInstanceCount,
    /// Cloud optical depth field does not match the prepared scene.
    CloudFieldLayout,
    /// Cloud optical depth exceeds the device texture extent.
    CloudTextureExtent,
    /// Instance count does not match the prepared scene.
    InstanceCountMismatch,
    /// Local-light records exceed the supported storage buffer capacity.
    LocalLightBufferCapacity,
    /// Cluster heads exceed the supported storage buffer capacity.
    ClusterHeadBufferCapacity,
    /// Cluster indices exceed the supported storage buffer capacity.
    ClusterIndexBufferCapacity,
    /// Effect pass index exceeds the supported range.
    EffectPassCount,
    /// Sprite row count exceeds its supported index range.
    SpriteRowCapacity,
    /// Sprite rows exceed the device storage binding limit.
    SpriteRowStorageLimit,
    /// A sprite scene has no capacity for world or frame rows.
    EmptySpriteRowCapacity,
    /// The scene source did not implement sprite sampling.
    MissingSpriteSource,
    /// The contribution did not implement preparation for a sprite scene.
    UnsupportedSpriteContribution,
    /// Sprite frame table exceeds the supported count or storage capacity.
    SpriteFrameCapacity,
    /// An expected mesh arena row is missing.
    MissingMeshRow,
    /// The mesh arena has no prepared GPU buffers.
    MissingMeshBuffers,
    /// Local-light source count exceeds the CPU cluster profile.
    LocalLightCount,
    /// Local-light source count is outside the GPU cluster profile.
    GpuLocalLightCount,
    /// Cluster light indices exceed the supported index range.
    ClusterIndexCapacity,
    /// Cluster probe output exceeds the supported storage capacity.
    ClusterProbeCapacity,
    /// An aloft mesh index has no corresponding scene mesh.
    InvalidAloftMesh,
    /// Mesh selection must supply one slot per scene instance.
    MeshSlotCount,
    /// A scene mesh slot has no corresponding mesh.
    InvalidMeshSlot,
    /// The bucket table is empty or exceeds the shader's bucket count.
    BucketCount,
    /// The member list or scratch binding exceeds the device storage limit.
    BucketStorageLimit,
    /// The worst-case bucket layout needs too many compute workgroups.
    BucketDispatchLimit,
    /// Mesh bucket member storage exceeds its supported capacity.
    BucketMemberCapacity,
    /// Mesh draw indices exceed the supported range.
    MeshIndexRange,
    /// Material table exceeds the supported storage or index range.
    MaterialTableCapacity,
    /// Instance table exceeds the supported index range.
    InstanceTableCapacity,
    /// Transform history exceeds the device storage binding limit.
    TransformHistoryCapacity,
    /// Aligned view records exceed the supported buffer capacity.
    ViewBufferCapacity,
    /// Mesh exceeds the device ray primitive limit.
    RayPrimitiveLimit,
    /// Ray instance list exceeds the device or prepared structure capacity.
    RayInstanceLimit,
    /// Shadow caster table must supply one row per scene instance.
    ShadowCasterCount,
    /// A selected star catalogue has no records.
    EmptyStarTable,
    /// The star table exceeds the supported storage or index range.
    StarTableCapacity,
    /// Changed instance slots are out of bounds, duplicated or unordered.
    InvalidInstanceUpdate,
    /// Instance logical slot does not match its scene row.
    InvalidInstanceIdentity,
    /// Instance transform is nonfinite, degenerate or outside the supported range.
    InvalidInstanceTransform,
    /// Material or skinning is unsupported by the synthetic renderer.
    UnsupportedSyntheticMaterial,
    /// Lighting input is invalid or incompatible with the prepared scene.
    InvalidLighting,
    /// Camera projection is nonfinite or degenerate.
    InvalidView,
    /// Reduced render scale was requested on a target without portable TAAU.
    UnsupportedRenderScale,
    /// Exposure gain is unrepresentable or the incident EV is unsupported.
    InvalidExposure,
    /// Target or readback belongs to another generation, grid or exposure.
    StaleTarget,
    /// Target extent exceeds device or readback limits.
    TargetExtent,
    /// Compiled pass does not match its prepared recorder.
    GraphPassMismatch,
    /// Readback is already pending or failed.
    Readback,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recipe(error) => write!(f, "hardware recipe: {error}"),
            Self::Present(error) => write!(f, "presentation: {error}"),
            Self::Abi(error) => write!(f, "scene ABI: {error}"),
            Self::Graph(error) => write!(f, "frame graph: {error}"),
            Self::Timing(error) => write!(f, "GPU timing: {error}"),
            Self::EffectGraph(error) => write!(f, "effect graph: {error}"),
            Self::Arena(error) => write!(f, "mesh arena: {error}"),
            Self::Gpu(error) => write!(f, "GPU validation: {error}"),
            Self::MissingSpriteSheets => f.write_str("Sprite scene has no authored sheets"),
            Self::SpriteSheetLimit { name, limit } => {
                write!(f, "sprite sheet '{name}' exceeds the {limit} sheet slots")
            }
            Self::InvalidSpriteOrderBand => {
                f.write_str("Sprite order band is nonfinite or negative")
            }
            Self::InvalidSpriteSheet(name) => write!(f, "invalid embedded sprite sheet '{name}'"),
            Self::UnsupportedSpriteCapture => {
                f.write_str("2D capture supports SDR and scene-linear output")
            }
            Self::NativePreparationPoisoned => f.write_str("Native preparation lock poisoned"),
            Self::NativeCandidateReserved => f.write_str("Native candidate already encoded"),
            Self::RayPreparation => f.write_str("Could not prepare the scene ray structure"),
            Self::GridInstanceCount => {
                f.write_str("Synthetic grid requires between 1 and 1,000,000 instances")
            }
            Self::CloudFieldLayout => {
                f.write_str("Cloud optical depth field does not match the prepared scene")
            }
            Self::CloudTextureExtent => {
                f.write_str("Cloud optical depth exceeds the device texture extent")
            }
            Self::InstanceCountMismatch => {
                f.write_str("Instance count does not match the prepared scene")
            }
            Self::LocalLightBufferCapacity => {
                f.write_str("Local-light records exceed storage buffer capacity")
            }
            Self::ClusterHeadBufferCapacity => {
                f.write_str("Cluster heads exceed storage buffer capacity")
            }
            Self::ClusterIndexBufferCapacity => {
                f.write_str("Cluster indices exceed storage buffer capacity")
            }
            Self::EffectPassCount => f.write_str("Effect pass index exceeds the supported range"),
            Self::EmptySpriteRowCapacity => f.write_str("Sprite scene requires at least one row"),
            Self::MissingSpriteSource => f.write_str("Scene source does not supply sprite rows"),
            Self::UnsupportedSpriteContribution => {
                f.write_str("Contribution does not support 2D preparation")
            }
            Self::SpriteRowCapacity => f.write_str("Sprite row count exceeds the supported range"),
            Self::SpriteRowStorageLimit => {
                f.write_str("Sprite rows exceed device storage capacity")
            }
            Self::SpriteFrameCapacity => {
                f.write_str("Sprite frame table exceeds the supported count or storage capacity")
            }
            Self::MissingMeshRow => f.write_str("An expected mesh arena row is missing"),
            Self::MissingMeshBuffers => f.write_str("The mesh arena has no prepared GPU buffers"),
            Self::LocalLightCount => {
                f.write_str("Local-light source count exceeds the CPU cluster profile")
            }
            Self::GpuLocalLightCount => {
                f.write_str("Local-light source count is outside the GPU cluster profile")
            }
            Self::ClusterIndexCapacity => {
                f.write_str("Cluster light indices exceed the supported index range")
            }
            Self::ClusterProbeCapacity => {
                f.write_str("Cluster probe output exceeds the supported storage capacity")
            }
            Self::InvalidAloftMesh => {
                f.write_str("An aloft mesh index has no corresponding scene mesh")
            }
            Self::MeshSlotCount => {
                f.write_str("Mesh selection must supply one slot per scene instance")
            }
            Self::InvalidMeshSlot => f.write_str("A scene mesh slot has no corresponding mesh"),
            Self::BucketCount => {
                f.write_str("Mesh bucket count is outside the shader's supported range")
            }
            Self::BucketStorageLimit => {
                f.write_str("Bucket layout exceeds device storage capacity")
            }
            Self::BucketDispatchLimit => {
                f.write_str("Bucket layout exceeds the device workgroup limit")
            }
            Self::BucketMemberCapacity => {
                f.write_str("Mesh bucket member storage exceeds its supported capacity")
            }
            Self::MeshIndexRange => f.write_str("Mesh draw indices exceed the supported range"),
            Self::MaterialTableCapacity => {
                f.write_str("Material table exceeds the supported storage or index range")
            }
            Self::InstanceTableCapacity => {
                f.write_str("Instance table exceeds the supported index range")
            }
            Self::TransformHistoryCapacity => {
                f.write_str("Transform history exceeds the device storage binding limit")
            }
            Self::ViewBufferCapacity => {
                f.write_str("Aligned view records exceed the supported buffer capacity")
            }
            Self::RayPrimitiveLimit => f.write_str("Mesh exceeds the device ray primitive limit"),
            Self::RayInstanceLimit => {
                f.write_str("Ray instance list exceeds the device or prepared structure capacity")
            }
            Self::ShadowCasterCount => {
                f.write_str("Shadow caster table must supply one row per scene instance")
            }
            Self::EmptyStarTable => f.write_str("Star table requires at least one record"),
            Self::StarTableCapacity => {
                f.write_str("Star table exceeds the supported storage or index range")
            }
            Self::InvalidInstanceUpdate => {
                f.write_str("Changed instance slots are out of bounds, duplicated or unordered")
            }
            Self::InvalidInstanceIdentity => {
                f.write_str("Instance logical slot does not match its scene row")
            }
            Self::InvalidInstanceTransform => f.write_str(
                "Instance transform is nonfinite, degenerate or outside the supported range",
            ),
            Self::UnsupportedSyntheticMaterial => {
                f.write_str("Material or skinning is unsupported by the synthetic renderer")
            }
            Self::InvalidLighting => {
                f.write_str("Lighting input is invalid or incompatible with the prepared scene")
            }
            Self::InvalidView => f.write_str("Camera projection is nonfinite or degenerate"),
            Self::UnsupportedRenderScale => {
                f.write_str("Reduced render scale requires portable TAAU")
            }
            Self::InvalidExposure => {
                f.write_str("Exposure gain is unrepresentable or the incident EV is unsupported")
            }
            Self::StaleTarget => {
                f.write_str("Target or readback belongs to another generation, grid or exposure")
            }
            Self::TargetExtent => f.write_str("Target extent exceeds device or readback limits"),
            Self::GraphPassMismatch => {
                f.write_str("Compiled pass does not match its prepared recorder")
            }
            Self::Readback => f.write_str("Readback is already pending or failed"),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Recipe(error) => Some(error),
            Self::Present(error) => Some(error),
            Self::Abi(error) => Some(error),
            Self::Graph(error) => Some(error),
            Self::Timing(error) => Some(error),
            Self::EffectGraph(error) => Some(error),
            Self::Arena(error) => Some(error),
            Self::Gpu(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crate::abi::AbiError> for RenderError {
    fn from(error: crate::abi::AbiError) -> Self {
        Self::Abi(error)
    }
}

impl From<crate::graph::GraphError> for RenderError {
    fn from(error: crate::graph::GraphError) -> Self {
        Self::Graph(error)
    }
}

impl From<crate::timing::TimingError> for RenderError {
    fn from(error: crate::timing::TimingError) -> Self {
        Self::Timing(error)
    }
}

impl From<EffectGraphError> for RenderError {
    fn from(error: EffectGraphError) -> Self {
        Self::EffectGraph(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// A mesh failed admission before any geometry was written or drawn.
pub enum ArenaError {
    /// No triangles, or an index count that is not a multiple of three.
    NotTriangles,
    /// An index at or past the mesh's own vertex count.
    IndexOutOfRange {
        /// The first invalid index.
        index: u32,
        /// Number of available vertices.
        vertex_count: u32,
    },
    /// A non-finite vertex component.
    NonFinite,
    /// A referenced position exceeds the supported coordinate range.
    CoordinateLimit,
    /// The arena's vertex or index limit would be exceeded.
    Capacity,
    /// A `MeshId` this arena never allocated.
    UnknownMesh,
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotTriangles => f.write_str("Mesh indices must describe complete triangles"),
            Self::IndexOutOfRange {
                index,
                vertex_count,
            } => {
                write!(f, "Mesh index {index} is outside {vertex_count} vertices")
            }
            Self::NonFinite => f.write_str("A mesh vertex component is nonfinite"),
            Self::CoordinateLimit => f.write_str("A mesh position exceeds the coordinate limit"),
            Self::Capacity => f.write_str("Mesh exceeds the arena vertex or index capacity"),
            Self::UnknownMesh => f.write_str("Mesh has no allocated arena row"),
        }
    }
}

impl Error for ArenaError {}

/// An effect declaration cannot be scheduled or bound as requested.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectGraphError {
    /// After-opaque pass (named by the payload) cannot attach depth in that configuration.
    AfterOpaqueDepthConflict(String),
    /// After-opaque render pass (named by the payload) declares a scene attachment or copy access.
    AfterOpaqueUsesSceneAttachments(String),
    /// Pass (named by the payload) must write retained output to skip frames.
    CadenceRequiresRetainedOutput(String),
    /// After-opaque compute pass (named by the payload) declares an access it cannot encode.
    AfterOpaqueRequiresComputeAccess(String),
    /// Effect pass name (named by the payload) is used more than once.
    DuplicateEffectPass(String),
    /// Async effect pass (named by the payload) declares an access it cannot encode.
    AsyncRequiresCompute(String),
    /// Time-sliced pass (named by the payload) cannot write history.
    TimeSlicedHistory(String),
    /// Retained effect owner is declared more than once.
    DuplicateEffectOwner,
    /// Effect access names an unknown resource.
    UnknownEffectResource,
    /// Opaque effect inputs must declare read access.
    OpaqueReadRequiresReadAccess,
    /// Only one effect may declare the scene's GPU Sun-path target.
    DuplicateSunPath,
    /// The Sun-path target must have its complete texture and mapping reads declared.
    InvalidSunPathReads,
    /// View-volume medium declared without one complete producer.
    InvalidViewVolumeMedium,
    /// History write requires the next binding.
    HistoryWriteRequiresNext,
    /// Multiple passes produce the same effect history.
    DuplicateHistoryProducer,
    /// History read requires a history binding.
    HistoryBindingRequired,
    /// Non-history resource cannot use a history binding.
    HistoryBindingOnNonHistory,
    /// Effect history result is read before its producer.
    ResultBeforeProducer,
    /// A tick pass may only read persistent resources and histories.
    TickPassRequiresRetainedRead(String),
    /// A tick pass may only read the committed side of a history.
    TickPassRequiresPrevious(String),
    /// A tick read is incompatible with its resource descriptor.
    InvalidTickRead {
        /// Name of the tick pass.
        pass: String,
        /// Descriptor validation failure.
        source: crate::graph::GraphError,
    },
    /// A 2D scene does not support the named tick pass.
    TickPassIn2d(String),
}

impl fmt::Display for EffectGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TickPassRequiresRetainedRead(pass) => {
                write!(
                    f,
                    "Tick pass {pass} may only read retained compute resources"
                )
            }
            Self::TickPassRequiresPrevious(pass) => {
                write!(f, "Tick pass {pass} must read history by previous")
            }
            Self::InvalidTickRead { pass, source } => {
                write!(f, "Tick pass {pass} has an invalid read: {source}")
            }
            Self::TickPassIn2d(pass) => write!(f, "2D scene refuses tick pass {pass}"),
            Self::AfterOpaqueDepthConflict(pass) => write!(
                f,
                "After-opaque pass {pass} cannot attach depth in that configuration"
            ),
            Self::AfterOpaqueUsesSceneAttachments(pass) => write!(
                f,
                "After-opaque render pass {pass} declares a scene attachment or copy access"
            ),
            Self::CadenceRequiresRetainedOutput(pass) => {
                write!(f, "Pass {pass} must write retained output to skip frames")
            }
            Self::AfterOpaqueRequiresComputeAccess(pass) => write!(
                f,
                "After-opaque compute pass {pass} declares an access it cannot encode"
            ),
            Self::DuplicateEffectPass(pass) => {
                write!(f, "Effect pass name {pass} is used more than once")
            }
            Self::AsyncRequiresCompute(pass) => write!(
                f,
                "Async effect pass {pass} declares an access it cannot encode"
            ),
            Self::TimeSlicedHistory(pass) => {
                write!(f, "Time-sliced pass {pass} cannot write history")
            }
            Self::DuplicateEffectOwner => {
                f.write_str("Retained effect owner is declared more than once")
            }
            Self::UnknownEffectResource => f.write_str("Effect access names an unknown resource"),
            Self::OpaqueReadRequiresReadAccess => {
                f.write_str("Opaque effect inputs must declare read access")
            }
            Self::DuplicateSunPath => {
                f.write_str("Multiple effects declare the GPU Sun-path target")
            }
            Self::InvalidViewVolumeMedium => {
                write!(f, "expected one complete view-volume medium writer")
            }
            Self::InvalidSunPathReads => {
                f.write_str("GPU Sun-path target is missing a complete opaque read")
            }
            Self::HistoryWriteRequiresNext => {
                f.write_str("History write requires the next binding")
            }
            Self::DuplicateHistoryProducer => {
                f.write_str("Multiple passes produce the same effect history")
            }
            Self::HistoryBindingRequired => f.write_str("History read requires a history binding"),
            Self::HistoryBindingOnNonHistory => {
                f.write_str("Non-history resource cannot use a history binding")
            }
            Self::ResultBeforeProducer => {
                f.write_str("Effect history result is read before its producer")
            }
        }
    }
}

impl Error for EffectGraphError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTickRead { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Typed cause of a GPU-facing frame operation failure.
#[derive(Debug)]
pub enum FrameGpuError {
    /// Scene or sprite renderer refused an operation.
    Render(RenderError),
    /// Surface preparation or recording failed.
    Present(crate::present::PresentError),
    /// GPU timestamp operation failed.
    Timing(crate::timing::TimingError),
    /// Device polling failed.
    Poll(wgpu::PollError),
    /// The device refused resource preparation.
    Device(wgpu::Error),
    /// Secondary view exceeds device limits.
    SecondaryView(crate::contribution::SecondaryViewError),
    /// The extent preparation thread could not be started.
    ThreadStart(std::io::Error),
    /// The extent preparation thread panicked.
    ExtentWorkerStopped,
}

impl fmt::Display for FrameGpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => fmt::Display::fmt(error, f),
            Self::Present(error) => fmt::Display::fmt(error, f),
            Self::Timing(error) => fmt::Display::fmt(error, f),
            Self::Poll(error) => fmt::Display::fmt(error, f),
            Self::Device(error) => fmt::Display::fmt(error, f),
            Self::SecondaryView(error) => fmt::Display::fmt(error, f),
            Self::ThreadStart(error) => fmt::Display::fmt(error, f),
            Self::ExtentWorkerStopped => f.write_str("Extent preparation thread stopped"),
        }
    }
}

impl Error for FrameGpuError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Render(error) => Some(error),
            Self::Present(error) => Some(error),
            Self::Timing(error) => Some(error),
            Self::Poll(error) => Some(error),
            Self::Device(error) => Some(error),
            Self::SecondaryView(error) => Some(error),
            Self::ThreadStart(error) => Some(error),
            Self::ExtentWorkerStopped => None,
        }
    }
}

impl From<RenderError> for FrameGpuError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl From<crate::present::PresentError> for FrameGpuError {
    fn from(error: crate::present::PresentError) -> Self {
        Self::Present(error)
    }
}

impl From<crate::timing::TimingError> for FrameGpuError {
    fn from(error: crate::timing::TimingError) -> Self {
        Self::Timing(error)
    }
}

impl From<wgpu::PollError> for FrameGpuError {
    fn from(error: wgpu::PollError) -> Self {
        Self::Poll(error)
    }
}

impl From<wgpu::Error> for FrameGpuError {
    fn from(error: wgpu::Error) -> Self {
        Self::Device(error)
    }
}

impl From<crate::contribution::SecondaryViewError> for FrameGpuError {
    fn from(error: crate::contribution::SecondaryViewError) -> Self {
        Self::SecondaryView(error)
    }
}

impl From<std::io::Error> for FrameGpuError {
    fn from(error: std::io::Error) -> Self {
        Self::ThreadStart(error)
    }
}
