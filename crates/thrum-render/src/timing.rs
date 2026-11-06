//! Optional pass-boundary GPU timestamp batches. Unsupported devices allocate
//! nothing and still encode the same rendering work. Readback is asynchronous;
//! the owning host polls its device/event loop and bounds outstanding batches.

use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use crate::capabilities::DeviceCapabilities;

/// One pass duration, separate from CPU encode time or display latency.
#[derive(Clone, Debug, PartialEq)]
pub struct PassTiming {
    /// Caller-assigned diagnostic name in encoding order.
    pub name: Arc<str>,
    /// GPU timestamp delta in nanoseconds; may be zero at backend precision.
    pub nanoseconds: f64,
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
        /// Raw beginning counter.
        start: u64,
        /// Raw ending counter.
        end: u64,
    },
}

struct Enabled {
    query: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    period: f64,
    capacity: u32,
    names: PassNames,
    results: Vec<PassTiming>,
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
    active: usize,
}
impl PassNames {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            active: 0,
        }
    }

    fn push(&mut self, name: &str) -> usize {
        let index = self.active;
        if let Some(slot) = self.slots.get_mut(index) {
            if slot.as_ref() != name {
                *slot = Arc::from(name);
            }
        } else {
            self.slots.push(Arc::from(name));
        }
        self.active += 1;
        index
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

    fn iter(&self) -> impl Iterator<Item = &Arc<str>> {
        self.slots[..self.active].iter()
    }
}

/// A single bounded reusable timestamp batch; it may be reused after results
/// are collected. Disabled batches produce `None` rather than fake zero timings.
pub struct GpuTimings {
    generation: u64,
    enabled: Option<Enabled>,
}
impl GpuTimings {
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
                results: Vec::with_capacity(max_passes as usize),
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
    ) -> Result<Option<wgpu::RenderPassTimestampWrites<'_>>, TimingError> {
        Ok(self
            .reserve_pass(generation, name)?
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
    ) -> Result<Option<wgpu::ComputePassTimestampWrites<'_>>, TimingError> {
        Ok(self
            .reserve_pass(generation, name)?
            .map(|(query_set, index)| wgpu::ComputePassTimestampWrites {
                query_set,
                beginning_of_pass_write_index: Some(index * 2),
                end_of_pass_write_index: Some(index * 2 + 1),
            }))
    }

    fn reserve_pass(
        &mut self,
        generation: u64,
        name: impl AsRef<str>,
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
        enabled.names.push(name.as_ref());
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
        self.check_generation(generation)?;
        let Some(enabled) = &mut self.enabled else {
            return Ok(());
        };
        if !matches!(enabled.phase, Phase::Recording) {
            return Err(TimingError::Busy);
        }
        enabled.names.clear();
        Ok(())
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
        let mut invalid = None;
        for (index, name) in enabled.names.iter().enumerate() {
            let base = index * 16;
            let start =
                u64::from_le_bytes(values[base..base + 8].try_into().expect("query scalar"));
            let end = u64::from_le_bytes(
                values[base + 8..base + 16]
                    .try_into()
                    .expect("query scalar"),
            );
            if start != 0 && start != u64::MAX && end != u64::MAX && end >= start {
                let delta = end - start;
                enabled.results.push(PassTiming {
                    name: Arc::clone(name),
                    nanoseconds: delta as f64 * enabled.period,
                });
            } else {
                invalid = Some((start, end));
            }
        }
        drop(values);
        enabled.readback.unmap();
        enabled.names.clear();
        if let Some((start, end)) = invalid {
            enabled.results.clear();
            return Err(TimingError::InvalidTimestamp { start, end });
        }
        Ok(Some(&enabled.results))
    }
}

#[cfg(test)]
mod tests;
