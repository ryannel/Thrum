//! Optional pass-boundary GPU timestamp batches. Unsupported devices allocate
//! nothing and still encode the same rendering work. Readback is asynchronous;
//! the owning host polls its device/event loop and bounds outstanding batches.

use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use crate::capabilities::DeviceCapabilities;

/// The timing bucket a pass declares. Every graph pass names one, and the frame
/// timing feed sums each bucket by it, never by a pass's name, so a renamed pass
/// keeps its bucket and a new pass cannot be left out of every bucket.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum TimingCategory {
    /// Near Sun shadow map and its caster-list build.
    ShadowNear,
    /// Far Sun shadow map, horizon heights and atmospheric shadow heights.
    ShadowFar,
    /// Local-light cluster build.
    Clusters,
    /// Opaque-phase work: forward, contribution compute, stars and camera culling.
    Forward,
    /// Transform history publication.
    History,
    /// Exposure resolve and the display tone stage.
    Tone,
    /// Atmospheric transmittance preparation.
    AtmosphereTransmittance,
    /// Multiple scattering and ground irradiance preparation.
    AtmosphereMultipleGround,
    /// Sky-view, DFG and reflection preparation.
    AtmosphereSky,
    /// Diffuse irradiance preparation.
    AtmosphereDiffuse,
    /// Aerial perspective preparation.
    AtmosphereAerial,
    /// Temporal inputs and temporal upscaling.
    Temporal,
    /// Contributed effect passes, before and after opaque.
    Effects,
    /// Every pass of a secondary scene view.
    SecondaryViews,
    /// The 2D sprite scene's draws and compute.
    Sprites,
    /// The surface consumer, reported apart from the scene sum.
    Surface,
    /// Copies, readbacks, native segments and acceleration builds, which the
    /// graph records without timestamp writes and reports as unmeasured, and
    /// the readback copy's own diagnostic brackets.
    Transfer,
}

impl TimingCategory {
    /// Every category, in profiler order.
    pub const ALL: [Self; 17] = [
        Self::ShadowNear,
        Self::ShadowFar,
        Self::Clusters,
        Self::Forward,
        Self::History,
        Self::Tone,
        Self::AtmosphereTransmittance,
        Self::AtmosphereMultipleGround,
        Self::AtmosphereSky,
        Self::AtmosphereDiffuse,
        Self::AtmosphereAerial,
        Self::Temporal,
        Self::Effects,
        Self::SecondaryViews,
        Self::Sprites,
        Self::Surface,
        Self::Transfer,
    ];

    /// Whether this bucket is an atmosphere preparation stage.
    #[must_use]
    pub const fn is_atmosphere(self) -> bool {
        matches!(
            self,
            Self::AtmosphereTransmittance
                | Self::AtmosphereMultipleGround
                | Self::AtmosphereSky
                | Self::AtmosphereDiffuse
                | Self::AtmosphereAerial
        )
    }
}

/// One pass duration, separate from CPU encode time or display latency.
#[derive(Clone, Debug, PartialEq)]
pub struct PassTiming {
    /// Caller-assigned diagnostic name in encoding order.
    pub name: Arc<str>,
    /// The bucket the pass declared.
    pub category: TimingCategory,
    /// GPU timestamp delta in nanoseconds; may be zero at backend precision.
    pub nanoseconds: f64,
}

/// Completed scene timestamps with each pass's name and declared bucket. Missing
/// optional passes are absent; unsupported or faulted batches are `None` in
/// `FrameSample`. Buckets sum by declared category; no name or ordinal selects one.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneGpuTimings(Arc<TimingRecord>);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TimingRecord {
    frame: Option<TimingFrame>,
    passes: Vec<PassTiming>,
    unmeasured: Vec<Arc<str>>,
    not_run: Vec<Arc<str>>,
    effect_boundary: Option<f64>,
    timed_pass_span: Option<f64>,
}

/// Named pass execution, preserving the difference between a skip and a zero timestamp delta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PassExecution {
    /// Executed and measured in nanoseconds.
    Timed(f64),
    /// Declared in this graph but skipped at runtime.
    NotRun,
    /// Executed without timestamp counters.
    Unmeasured,
}

impl TimingRecord {
    pub(crate) fn with_capacity(passes: usize, graph_passes: usize) -> Self {
        Self {
            frame: None,
            passes: Vec::with_capacity(passes),
            unmeasured: Vec::with_capacity(graph_passes),
            not_run: Vec::with_capacity(graph_passes),
            effect_boundary: None,
            timed_pass_span: None,
        }
    }

    pub(crate) fn unmeasured(&mut self, names: impl Iterator<Item = Arc<str>>) {
        self.unmeasured.extend(names);
    }

    pub(crate) fn effect_passes(&mut self, names: impl Iterator<Item = Arc<str>>) {
        for name in names {
            if !self.passes.iter().any(|pass| pass.name == name) {
                self.not_run.push(name);
            }
        }
    }

    pub(crate) fn set_effect_boundary(&mut self, value: Option<f64>) {
        self.effect_boundary = value;
    }

    fn get(&self, name: &str) -> Option<f64> {
        self.passes
            .iter()
            .find(|pass| pass.name.as_ref() == name)
            .map(|pass| pass.nanoseconds)
    }
}

impl SceneGpuTimings {
    pub(crate) fn new(passes: &[PassTiming]) -> Self {
        let mut record = TimingRecord::with_capacity(passes.len(), 0);
        record.passes.extend_from_slice(passes);
        Self(Arc::new(record))
    }

    pub(crate) fn detach(&mut self) {
        Arc::make_mut(&mut self.0);
    }

    /// Identity recorded by the timestamp batch when its frame was submitted.
    /// Standalone diagnostics that construct a sample from pass values have no identity.
    #[must_use]
    pub fn frame(&self) -> Option<TimingFrame> {
        self.0.frame
    }
    /// First timed pass begin to output consumer end, including intervening work.
    /// Queued writes before the first pass are outside the interval.
    #[must_use]
    pub fn timed_pass_span_nanoseconds(&self) -> Option<f64> {
        self.0.timed_pass_span
    }
    /// Upper bracket from the preceding timed pass's end through the first late pass.
    /// Includes a requested refraction copy, the pass boundary and that render pass's work.
    /// This overlaps `passes()` and must never be added to the scene sum.
    #[must_use]
    pub fn effect_boundary_nanoseconds(&self) -> Option<f64> {
        self.0.effect_boundary
    }
    /// Execution of a named pass, absent only when it is outside this frame's graph.
    #[must_use]
    pub fn execution(&self, name: &str) -> Option<PassExecution> {
        if let Some(value) = self.get(name) {
            Some(PassExecution::Timed(value))
        } else if self.0.not_run.iter().any(|n| n.as_ref() == name) {
            Some(PassExecution::NotRun)
        } else if (name == RESOLVE_PASS && self.0.frame.is_some())
            || self.0.unmeasured.iter().any(|n| n.as_ref() == name)
        {
            Some(PassExecution::Unmeasured)
        } else {
            None
        }
    }
    /// Declared compute passes skipped in this frame, under their executor names.
    #[must_use]
    pub fn not_run_passes(&self) -> &[Arc<str>] {
        &self.0.not_run
    }

    /// Actual executed, timed graph passes (including the surface consumer).
    pub fn passes(&self) -> &[PassTiming] {
        &self.0.passes
    }

    /// A measured pass, or `None` when absent. Zero is a valid measured delta.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.0.get(name)
    }

    /// Scene pass sum; surface is reported separately. Untimed commands are
    /// outside this sum, never inferred as zero.
    pub fn sum(&self) -> f64 {
        self.0
            .passes
            .iter()
            .filter(|p| p.category != TimingCategory::Surface)
            .map(|p| p.nanoseconds)
            .sum()
    }

    /// Every measured pass that declared `category`; zero when none executed.
    #[must_use]
    pub fn bucket(&self, category: TimingCategory) -> f64 {
        self.sum_where(|c| c == category)
    }

    fn sum_where(&self, predicate: impl Fn(TimingCategory) -> bool) -> f64 {
        self.0
            .passes
            .iter()
            .filter(|p| predicate(p.category))
            .map(|p| p.nanoseconds)
            .sum()
    }

    /// Near shadow map and its optional caster-list build.
    pub fn shadow_near(&self) -> f64 {
        self.bucket(TimingCategory::ShadowNear)
    }
    /// Far shadow map, horizon heights and atmospheric shadow heights.
    pub fn shadow_far(&self) -> f64 {
        self.bucket(TimingCategory::ShadowFar)
    }
    /// Local-light cluster build, zero when not executed.
    pub fn clusters(&self) -> f64 {
        self.bucket(TimingCategory::Clusters)
    }
    /// Opaque-phase work including contribution, stars and camera culling.
    pub fn forward(&self) -> f64 {
        self.bucket(TimingCategory::Forward)
    }
    /// Transform history publication, zero for an empty prefix.
    pub fn history(&self) -> f64 {
        self.bucket(TimingCategory::History)
    }
    /// Exposure resolve and SDR tone stage.
    pub fn tone(&self) -> f64 {
        self.bucket(TimingCategory::Tone)
    }
    /// All executed atmosphere preparation stages.
    pub fn atmosphere(&self) -> f64 {
        self.sum_where(TimingCategory::is_atmosphere)
    }
    /// Atmospheric transmittance preparation.
    pub fn transmittance(&self) -> f64 {
        self.bucket(TimingCategory::AtmosphereTransmittance)
    }
    /// Multiple scattering and ground irradiance preparation.
    pub fn multiple_ground(&self) -> f64 {
        self.bucket(TimingCategory::AtmosphereMultipleGround)
    }
    /// Sky-view and reflection preparation.
    pub fn sky(&self) -> f64 {
        self.bucket(TimingCategory::AtmosphereSky)
    }
    /// Diffuse irradiance preparation.
    pub fn diffuse(&self) -> f64 {
        self.bucket(TimingCategory::AtmosphereDiffuse)
    }
    /// Aerial perspective preparation.
    pub fn aerial(&self) -> f64 {
        self.bucket(TimingCategory::AtmosphereAerial)
    }
    /// Named values for serializers, without imposing a serialization dependency.
    /// Executed transfers and query resolution without counters have `None` values.
    pub fn values(&self) -> std::collections::BTreeMap<&str, Option<f64>> {
        self.0
            .passes
            .iter()
            .map(|p| (p.name.as_ref(), Some(p.nanoseconds)))
            .chain(self.0.unmeasured.iter().map(|n| (n.as_ref(), None)))
            .chain(self.0.frame.map(|_| (RESOLVE_PASS, None)))
            .collect()
    }
}

const RESOLVE_PASS: &str = "timestamp query resolve/readback";

/// Identity of a frame whose GPU pass timestamps were recorded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimingFrame {
    /// The renderer's device generation.
    pub device_generation: u64,
    /// The host's frame submission order.
    pub submission_order: u64,
}

/// Why a gathered GPU timestamp batch produced no timing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingRefusal {
    /// A counter was zero.
    Zero,
    /// A counter was the backend error sentinel.
    Sentinel,
    /// A pass ended before it began.
    Inverted,
    /// A counter predates the last valid batch from this query set.
    Repeated,
    /// The batch belonged to a device that was replaced.
    RetiredDevice,
    /// The frame encoder was abandoned before it was submitted.
    Discarded,
    /// Render completion or query readback failed.
    Readback,
    /// The recorded pass layout or frame identity did not agree with the frame.
    Layout,
    /// Consumers still hold every prepared timing record for this slot.
    ReportsHeld,
}

/// Cumulative outcomes for GPU frame timing in one host.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimingStats {
    /// Batches submitted or discarded after reserving timestamp queries.
    pub gathered: u64,
    /// Batches published to a completed frame sample.
    pub published: u64,
    /// Submitted batches still in render or readback.
    pub pending: usize,
    refused: [u64; TimingRefusal::ReportsHeld as usize + 1],
}

impl TimingStats {
    /// Number of batches refused for this reason.
    #[must_use]
    pub fn refused(&self, reason: TimingRefusal) -> u64 {
        self.refused[reason as usize]
    }

    /// Number of batches refused across all reasons.
    #[must_use]
    pub fn total_refused(&self) -> u64 {
        self.refused.iter().sum()
    }

    pub(crate) fn refuse(&mut self, reason: TimingRefusal) {
        self.refused[reason as usize] += 1;
    }
}

/// Expected timing admission or lifecycle failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingError {
    /// Zero/overflowing query count or invalid timestamp period.
    InvalidCapacity,
    /// All reserved pass pairs are used.
    Capacity,
    /// Pending readback cannot be overwritten or resolved twice.
    Busy,
    /// Mapping failed, e.g. on device loss. Recreate on the next generation.
    MapFailed,
    /// An old device callback/batch cannot publish into the current generation.
    StaleGeneration,
    /// Timestamp order or sentinel was invalid; never fabricate a duration.
    InvalidTimestamp {
        /// Reason the raw pair is unusable.
        reason: TimingRefusal,
        /// Raw beginning counter.
        start: u64,
        /// Raw ending counter.
        end: u64,
    },
}

impl std::fmt::Display for TimingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCapacity => f.write_str("Timestamp query capacity or period is invalid"),
            Self::Capacity => f.write_str("All timestamp query pairs are in use"),
            Self::Busy => f.write_str("A timestamp readback is already pending"),
            Self::MapFailed => f.write_str("Timestamp readback mapping failed"),
            Self::StaleGeneration => {
                f.write_str("Timestamp batch belongs to a retired device generation")
            }
            Self::InvalidTimestamp { reason, start, end } => {
                write!(
                    f,
                    "Invalid timestamp interval from {start} to {end}: {reason:?}"
                )
            }
        }
    }
}

impl std::error::Error for TimingError {}

struct Enabled {
    query: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    period: f64,
    capacity: u32,
    names: PassNames,
    frame: Option<TimingFrame>,
    last_counter: u64,
    results: Vec<PassTiming>,
    preceding_gaps: Vec<Option<f64>>,
    timed_pass_span: Option<f64>,
    completion: Completion,
    phase: Phase,
}

enum Phase {
    Recording,
    Rendering,
    Mapping,
}

const WAITING: u8 = 0;
const READY: u8 = 1;
const FAILED: u8 = 2;

// One sequential producer at a time: render completion, then map completion.
// Each timing owner has its own mailbox, retained across batches.
struct Completion(Arc<AtomicU8>);

impl Completion {
    fn new() -> Self {
        Self(Arc::new(AtomicU8::new(WAITING)))
    }

    fn arm(&self) -> CompletionTicket {
        self.0.store(WAITING, Ordering::Release);
        CompletionTicket(Some(Arc::clone(&self.0)))
    }

    fn ready(&self) -> Result<bool, TimingError> {
        match self.0.load(Ordering::Acquire) {
            WAITING => Ok(false),
            READY => Ok(true),
            _ => Err(TimingError::MapFailed),
        }
    }
}

// A callback dropped without delivery reports the same failure as a disconnected
// channel. Taking its handle MUST precede the terminal store: the owner may
// immediately rearm while this disarmed ticket is still being destroyed.
struct CompletionTicket(Option<Arc<AtomicU8>>);

impl CompletionTicket {
    fn complete(&mut self, succeeded: bool) {
        let state = self.0.take().expect("completion is delivered once");
        state.store(if succeeded { READY } else { FAILED }, Ordering::Release);
    }
}

impl Drop for CompletionTicket {
    fn drop(&mut self) {
        if let Some(state) = &self.0 {
            state.store(FAILED, Ordering::Release);
        }
    }
}

// Keep the owned labels across batches. A static graph records the same pass
// names every frame, so only its first batch allocates label storage.
struct PassNames {
    slots: Vec<Arc<str>>,
    categories: Vec<TimingCategory>,
    active: usize,
}
impl PassNames {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            categories: Vec::with_capacity(capacity),
            active: 0,
        }
    }

    fn push(&mut self, name: &str, category: TimingCategory, own: impl FnOnce() -> Arc<str>) {
        let index = self.active;
        if let Some(slot) = self.slots.get_mut(index) {
            if slot.as_ref() != name {
                *slot = own();
            }
            self.categories[index] = category;
        } else {
            self.slots.push(own());
            self.categories.push(category);
        }
        self.active += 1;
    }

    fn clear(&mut self) {
        self.active = 0;
    }

    fn is_empty(&self) -> bool {
        self.active == 0
    }

    fn len(&self) -> usize {
        self.active
    }

    fn iter(&self) -> impl Iterator<Item = (&Arc<str>, TimingCategory)> {
        self.slots[..self.active]
            .iter()
            .zip(self.categories[..self.active].iter().copied())
    }
}

/// A single bounded reusable timestamp batch; it may be reused after results
/// are collected. Disabled batches produce `None` rather than fake zero timings.
pub struct GpuTimings {
    generation: u64,
    enabled: Option<Enabled>,
}
impl GpuTimings {
    /// The passes one batch can time; zero for a disabled batch.
    pub(crate) fn capacity(&self) -> u32 {
        self.enabled.as_ref().map_or(0, |enabled| enabled.capacity)
    }

    pub(crate) fn effect_boundary(&self, name: &str) -> Option<f64> {
        let enabled = self.enabled.as_ref()?;
        let (index, pass) = enabled
            .results
            .iter()
            .enumerate()
            .find(|(_, pass)| pass.name.as_ref() == name)?;
        Some(enabled.preceding_gaps[index]? + pass.nanoseconds)
    }

    pub(crate) fn ready_to_resolve(&self) -> bool {
        self.enabled.as_ref().is_some_and(|e| {
            matches!(e.phase, Phase::Rendering) && e.completion.ready() == Ok(true)
        })
    }

    /// Nanoseconds from the end of one pass to the start of the next in the
    /// last collected batch. Overlapping intervals produce a negative gap.
    pub fn gap_between(&self, before: &str, after: &str) -> Option<f64> {
        let enabled = self.enabled.as_ref()?;
        let index =
            enabled.results.windows(2).position(|pair| {
                pair[0].name.as_ref() == before && pair[1].name.as_ref() == after
            })? + 1;
        enabled.preceding_gaps[index]
    }

    /// Allocate bounded query/resolve/readback storage on the executor's device.
    /// Feature absent from the *enabled device* is a successful disabled path.
    ///
    /// # Errors
    ///
    /// Returns an error if the query count or timestamp period is invalid.
    pub fn new(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        max_passes: u32,
    ) -> Result<Self, TimingError> {
        if !capabilities
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            return Ok(Self {
                generation: capabilities.generation(),
                enabled: None,
            });
        }
        let device = capabilities.device();
        let count = max_passes
            .checked_mul(2)
            .filter(|n| *n > 0 && *n <= wgpu::QUERY_SET_MAX_QUERIES)
            .ok_or(TimingError::InvalidCapacity)?;
        let period = f64::from(queue.get_timestamp_period());
        if !period.is_finite() || period <= 0.0 {
            return Err(TimingError::InvalidCapacity);
        }
        let size = u64::from(count) * u64::from(wgpu::QUERY_SIZE);
        let query = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("thrum pass timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("thrum timestamp resolve"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("thrum timestamp readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            generation: capabilities.generation(),
            enabled: Some(Enabled {
                query,
                resolve,
                readback,
                period,
                capacity: max_passes,
                names: PassNames::with_capacity(max_passes as usize),
                frame: None,
                last_counter: 0,
                results: Vec::with_capacity(max_passes as usize),
                preceding_gaps: Vec::with_capacity(max_passes as usize),
                timed_pass_span: None,
                completion: Completion::new(),
                phase: Phase::Recording,
            }),
        })
    }
    fn check_generation(&self, generation: u64) -> Result<(), TimingError> {
        if self.generation != generation {
            return Err(TimingError::StaleGeneration);
        }
        Ok(())
    }
    /// Whether actual timestamp-query resources exist.
    pub fn is_enabled(&self) -> bool {
        self.enabled.is_some()
    }
    /// Check that a whole prepared pass group fits before encoding any pass.
    /// Disabled timestamp batches need no query capacity.
    pub(crate) fn check_passes(&self, generation: u64, additional: u32) -> Result<(), TimingError> {
        self.check_generation(generation)?;
        let Some(enabled) = &self.enabled else {
            return Ok(());
        };
        if !matches!(enabled.phase, Phase::Recording) {
            return Err(TimingError::Busy);
        }
        let additional = usize::try_from(additional).map_err(|_| TimingError::Capacity)?;
        let needed = enabled
            .names
            .len()
            .checked_add(additional)
            .ok_or(TimingError::Capacity)?;
        if needed > usize::try_from(enabled.capacity).map_err(|_| TimingError::Capacity)? {
            return Err(TimingError::Capacity);
        }
        Ok(())
    }
    /// Reserve the next pair and attach its writes to exactly one encoded pass.
    /// Caller must end that pass before resolving this batch.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, busy batch or full capacity.
    pub fn pass_writes(
        &mut self,
        generation: u64,
        name: impl AsRef<str>,
        category: TimingCategory,
    ) -> Result<Option<wgpu::RenderPassTimestampWrites<'_>>, TimingError> {
        Ok(self
            .reserve_pass(generation, name.as_ref(), category, || {
                Arc::from(name.as_ref())
            })?
            .map(|(query_set, index)| wgpu::RenderPassTimestampWrites {
                query_set,
                beginning_of_pass_write_index: Some(index * 2),
                end_of_pass_write_index: Some(index * 2 + 1),
            }))
    }

    /// Reserve automatic timing writes for one compute pass.
    /// # Errors
    /// Returns an error for a stale generation, busy batch or full capacity.
    pub fn compute_writes(
        &mut self,
        generation: u64,
        name: impl AsRef<str>,
        category: TimingCategory,
    ) -> Result<Option<wgpu::ComputePassTimestampWrites<'_>>, TimingError> {
        Ok(self
            .reserve_pass(generation, name.as_ref(), category, || {
                Arc::from(name.as_ref())
            })?
            .map(|(query_set, index)| wgpu::ComputePassTimestampWrites {
                query_set,
                beginning_of_pass_write_index: Some(index * 2),
                end_of_pass_write_index: Some(index * 2 + 1),
            }))
    }

    pub(crate) fn reserve_compiled_pass(
        &mut self,
        generation: u64,
        name: &Arc<str>,
        category: TimingCategory,
    ) -> Result<Option<(&wgpu::QuerySet, u32)>, TimingError> {
        self.reserve_pass(generation, name, category, || Arc::clone(name))
    }

    fn reserve_pass(
        &mut self,
        generation: u64,
        name: &str,
        category: TimingCategory,
        own: impl FnOnce() -> Arc<str>,
    ) -> Result<Option<(&wgpu::QuerySet, u32)>, TimingError> {
        self.check_generation(generation)?;
        let Some(enabled) = &mut self.enabled else {
            return Ok(None);
        };
        if !matches!(enabled.phase, Phase::Recording) {
            return Err(TimingError::Busy);
        }
        let index = u32::try_from(enabled.names.len()).map_err(|_| TimingError::Capacity)?;
        if index >= enabled.capacity {
            return Err(TimingError::Capacity);
        }
        enabled.names.push(name, category, own);
        Ok(Some((&enabled.query, index)))
    }
    /// Release pass reservations after dropping their unsubmitted encoder.
    /// Call before `finish_render`; submitted work must complete and be collected
    /// normally. Stored labels are retained for allocation-free reuse when the
    /// next batch records the same pass names.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation or pending render/readback batch,
    /// leaving its reservations unchanged.
    pub fn discard_unsubmitted(&mut self, generation: u64) -> Result<(), TimingError> {
        self.discard_recorded(generation).map(|_| ())
    }

    pub(crate) fn discard_recorded(&mut self, generation: u64) -> Result<bool, TimingError> {
        self.check_generation(generation)?;
        let Some(enabled) = &mut self.enabled else {
            return Ok(false);
        };
        if !matches!(enabled.phase, Phase::Recording) {
            return Err(TimingError::Busy);
        }
        let recorded = !enabled.names.is_empty();
        enabled.names.clear();
        Ok(recorded)
    }
    /// Finish recording timestamps on the render command buffer. The executor
    /// submits normally; completion notification is only a nonblocking message.
    /// This certifies counter readiness, never temporal-history acceptance.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation or unresolved prior batch.
    pub fn finish_render(
        &mut self,
        generation: u64,
        encoder: &wgpu::CommandEncoder,
    ) -> Result<(), TimingError> {
        self.check_generation(generation)?;
        let Some(enabled) = &mut self.enabled else {
            return Ok(());
        };
        if !matches!(enabled.phase, Phase::Recording) {
            return Err(TimingError::Busy);
        }
        if enabled.names.is_empty() {
            return Ok(());
        }
        let mut ticket = enabled.completion.arm();
        enabled.phase = Phase::Rendering;
        encoder.on_submitted_work_done(move || {
            ticket.complete(true);
        });
        Ok(())
    }
    pub(crate) fn finish_frame(
        &mut self,
        frame: TimingFrame,
        encoder: &wgpu::CommandEncoder,
    ) -> Result<bool, TimingError> {
        self.finish_render(frame.device_generation, encoder)?;
        if let Some(enabled) = &mut self.enabled
            && matches!(enabled.phase, Phase::Rendering)
        {
            enabled.frame = Some(frame);
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn pending_frame(&self) -> Option<TimingFrame> {
        self.enabled.as_ref()?.frame
    }

    pub(crate) fn clear_frame(&mut self) {
        if let Some(enabled) = &mut self.enabled {
            enabled.frame = None;
        }
    }

    /// Encode resolve/copy in a *later* host-owned command buffer, only after
    /// the render completion callback. Returns false without encoding or waiting
    /// while completion is pending (or timing is disabled). This avoids the
    /// documented Metal stale-counter fault from same-submission resolution.
    /// Mapping is deferred until the host submits this resolve command buffer.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, busy batch or failed completion.
    pub fn resolve_if_ready(
        &mut self,
        generation: u64,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<bool, TimingError> {
        self.check_generation(generation)?;
        let Some(enabled) = &mut self.enabled else {
            return Ok(false);
        };
        if matches!(enabled.phase, Phase::Mapping) {
            return Err(TimingError::Busy);
        }
        if enabled.names.is_empty() {
            return Ok(false);
        }
        let Phase::Rendering = enabled.phase else {
            return Err(TimingError::Busy);
        };
        match enabled.completion.ready() {
            Ok(false) => return Ok(false),
            Err(error) => {
                enabled.phase = Phase::Recording;
                enabled.names.clear();
                return Err(error);
            }
            Ok(true) => {}
        }
        enabled.phase = Phase::Recording;
        let count = u32::try_from(enabled.names.len())
            .ok()
            .and_then(|n| n.checked_mul(2))
            .ok_or(TimingError::InvalidCapacity)?;
        let bytes = u64::from(count) * u64::from(wgpu::QUERY_SIZE);
        encoder.resolve_query_set(&enabled.query, 0..count, &enabled.resolve, 0);
        encoder.copy_buffer_to_buffer(&enabled.resolve, 0, &enabled.readback, 0, bytes);
        let mut ticket = enabled.completion.arm();
        enabled.phase = Phase::Mapping;
        encoder.map_buffer_on_submit(
            &enabled.readback,
            wgpu::MapMode::Read,
            0..bytes,
            move |result| {
                ticket.complete(result.is_ok());
            },
        );
        Ok(true)
    }
    /// Nonblocking collection. `None` means unsupported, no batch, or not ready;
    /// use `is_enabled` to distinguish unsupported. The host polls its device.
    /// Results borrow reusable storage until the next mutable timing operation.
    /// The current generation is checked before old callbacks can publish data.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation, readback fault or invalid timestamp.
    ///
    /// # Panics
    ///
    /// Panics only if the bounded query buffer no longer contains the recorded
    /// 64-bit timestamp pairs, which would violate the internal buffer invariant.
    pub fn try_collect(&mut self, generation: u64) -> Result<Option<&[PassTiming]>, TimingError> {
        self.check_generation(generation)?;
        let Some(enabled) = &mut self.enabled else {
            return Ok(None);
        };
        let Phase::Mapping = enabled.phase else {
            return Ok(None);
        };
        match enabled.completion.ready() {
            Ok(false) => return Ok(None),
            Err(error) => {
                enabled.phase = Phase::Recording;
                enabled.names.clear();
                return Err(error);
            }
            Ok(true) => {}
        }
        enabled.phase = Phase::Recording;
        let bytes = u64::try_from(enabled.names.len())
            .ok()
            .and_then(|n| n.checked_mul(2 * u64::from(wgpu::QUERY_SIZE)))
            .ok_or(TimingError::InvalidCapacity)?;
        let Ok(values) = enabled.readback.slice(0..bytes).get_mapped_range() else {
            enabled.names.clear();
            return Err(TimingError::MapFailed);
        };
        enabled.results.clear();
        enabled.preceding_gaps.clear();
        enabled.timed_pass_span = None;
        let mut first_start = None;
        let mut previous_end = None;
        let mut latest = enabled.last_counter;
        let mut result = Ok(());
        for (index, (name, category)) in enabled.names.iter().enumerate() {
            let base = index * 16;
            let start =
                u64::from_le_bytes(values[base..base + 8].try_into().expect("query scalar"));
            let end = u64::from_le_bytes(
                values[base + 8..base + 16]
                    .try_into()
                    .expect("query scalar"),
            );
            if let Err(error) = validate_pair(start, end, enabled.last_counter) {
                result = Err(error);
                break;
            }
            latest = latest.max(end);
            first_start.get_or_insert(start);
            if category == TimingCategory::Surface {
                enabled.timed_pass_span = first_start
                    .and_then(|first| end.checked_sub(first))
                    .map(|ticks| ticks as f64 * enabled.period);
            }
            enabled.preceding_gaps.push(previous_end.map(|previous| {
                let ticks = if start >= previous {
                    (start - previous) as f64
                } else {
                    -((previous - start) as f64)
                };
                ticks * enabled.period
            }));
            previous_end = Some(end);
            enabled.results.push(PassTiming {
                name: Arc::clone(name),
                category,
                nanoseconds: (end - start) as f64 * enabled.period,
            });
        }
        drop(values);
        enabled.readback.unmap();
        enabled.names.clear();
        if let Err(error) = result {
            enabled.results.clear();
            enabled.preceding_gaps.clear();
            enabled.timed_pass_span = None;
            return Err(error);
        }
        enabled.last_counter = latest;
        Ok(Some(&enabled.results))
    }

    pub(crate) fn move_frame_results(&mut self, record: &mut TimingRecord) {
        let enabled = self.enabled.as_mut().expect("collected a timed frame");
        record.frame = enabled.frame.take();
        record.unmeasured.clear();
        record.not_run.clear();
        record.effect_boundary = None;
        record.timed_pass_span = enabled.timed_pass_span;
        std::mem::swap(&mut enabled.results, &mut record.passes);
        enabled.results.clear();
    }
}

/// Own reusable records independently of the frame samples that borrow them.
/// A retained sample keeps its record immutable until its last clone is dropped.
pub(crate) struct TimingRecords(Vec<Arc<TimingRecord>>);

impl TimingRecords {
    pub(crate) fn new(count: usize, passes: usize, graph_passes: usize) -> Self {
        Self(
            (0..count)
                .map(|_| Arc::new(TimingRecord::with_capacity(passes, graph_passes)))
                .collect(),
        )
    }

    pub(crate) fn publish(
        &mut self,
        populate: impl FnOnce(&mut TimingRecord),
    ) -> Option<SceneGpuTimings> {
        let entry = self
            .0
            .iter_mut()
            .find(|entry| Arc::strong_count(entry) == 1)?;
        let record = Arc::get_mut(entry).expect("only the pool owns this record");
        populate(record);
        Some(SceneGpuTimings(Arc::clone(entry)))
    }
}

pub(crate) fn validate_pair(start: u64, end: u64, last_counter: u64) -> Result<(), TimingError> {
    let reason = if start == 0 || end == 0 {
        TimingRefusal::Zero
    } else if start == u64::MAX || end == u64::MAX {
        TimingRefusal::Sentinel
    } else if start > end {
        TimingRefusal::Inverted
    } else if last_counter != 0 && (start < last_counter || end <= last_counter) {
        TimingRefusal::Repeated
    } else {
        return Ok(());
    };
    Err(TimingError::InvalidTimestamp { reason, start, end })
}

#[cfg(test)]
mod tests;
