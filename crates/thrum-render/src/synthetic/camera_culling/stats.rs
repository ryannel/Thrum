//! Per-slot culling counts beside the completion path. An untimed graph copy
//! moves each culling list's GPU-written instance count into the slot's own
//! map buffer; the host maps it when it submits the slot and takes it if the
//! mapping has settled by the time the slot retires. Slot retirement never
//! waits on the mapping: a frame whose count is not ready reports none, and a
//! slot whose buffer is still mapping skips the copy and the map on its next
//! frame. No count depends on timestamps. Direct lists nothing and declares
//! no copy.

use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use super::super::{OffscreenTarget, PreparedSynthetic, RenderError, SceneContext};
use super::CullMode;
use crate::graph::{
    Access, AccessKind, FrameGraph, GraphError, Pass, PassScope, ResourceDescriptor, ResourceId,
    ResourceScope,
};
use crate::timing::GpuTimings;
use crate::visibility::ARGS_BYTES;

/// Graph name of the untimed copy of each culling list's instance count.
pub(crate) const STATS_PASS: &str = "camera culling stats";
/// Word index of `instance_count` in an indexed indirect argument record.
const COUNT_WORD: usize = 1;

const IDLE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

/// The instances camera culling drew in one submitted frame, read back on
/// the completion path without timestamps. A Direct frame has none.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CullingCounts {
    /// The frame's culling mode, as prepared by the renderer.
    pub mode: CullMode,
    /// Active instance slots the frame tested.
    pub active: u32,
    /// Instances the colour draw drew: the frustum list, or Occlusion's
    /// final list.
    pub drawn: u32,
    /// Occlusion's depth prepass list (last frame's visible set in view).
    /// Zero for Frustum.
    pub prepass: u32,
}

impl CullingCounts {
    /// Active slots the cull rejected: outside the view or, under Occlusion,
    /// hidden by this frame's depth.
    #[must_use]
    pub fn rejected(&self) -> u32 {
        self.active.saturating_sub(self.drawn)
    }
}

/// One frame slot's count buffer and its mapping state.
pub(in crate::synthetic) struct CullingStats {
    buffer: wgpu::Buffer,
    lists: u64,
    /// Bytes of each list's argument records: one per bucket on the bucket
    /// route, whose drawn count is their sum.
    args_bytes: u64,
    mapping: Arc<AtomicU8>,
    frame: Option<(CullMode, u32)>,
    /// This frame copies and maps: the buffer was free when it was encoded.
    armed: bool,
}

impl CullingStats {
    pub(in crate::synthetic) fn new(
        device: &wgpu::Device,
        mode: CullMode,
        args_bytes: u64,
    ) -> Option<Self> {
        let lists = match mode {
            CullMode::Direct => return None,
            CullMode::Frustum => 1,
            CullMode::Occlusion => 2,
        };
        Some(Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("camera culling counts"),
                size: lists * args_bytes,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            lists,
            args_bytes,
            mapping: Arc::new(AtomicU8::new(IDLE)),
            frame: None,
            armed: false,
        })
    }
}

impl PreparedSynthetic {
    /// Declare the untimed copy of each culling list's argument records,
    /// after the passes that write them. Direct declares nothing.
    pub(in crate::synthetic) fn append_camera_stats(
        graph: &mut FrameGraph,
        lists: &[ResourceId],
        args_bytes: u64,
    ) -> Result<(), GraphError> {
        if lists.is_empty() {
            return Ok(());
        }
        let bytes = u64::try_from(lists.len()).unwrap_or(u64::MAX) * args_bytes;
        let destination = graph.resource(
            ResourceDescriptor::Buffer {
                bytes,
                usage: wgpu::BufferUsages::COPY_DST,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        let mut accesses: Vec<Access> = lists
            .iter()
            .map(|args| Access {
                view: None,
                resource: args.clone(),
                range: 0..args_bytes,
                kind: AccessKind::CopyRead,
                store: true,
            })
            .collect();
        accesses.push(Access {
            view: None,
            resource: destination,
            range: 0..bytes,
            kind: AccessKind::CopyWrite,
            store: true,
        });
        graph.pass(Pass {
            name: STATS_PASS.into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses,
            external_side_effect: true,
        });
        Ok(())
    }
}

/// A frame `PreparedSynthetic::submit_counted` recorded, submitted and
/// mapped, holding its target until the counts are taken.
#[must_use = "take the counts, or the next counted frame discards them"]
pub struct CountedFrame<'target> {
    target: &'target mut OffscreenTarget,
    submission: wgpu::SubmissionIndex,
}

impl CountedFrame<'_> {
    /// The submission to wait on before taking the counts.
    #[must_use]
    pub fn submission(&self) -> &wgpu::SubmissionIndex {
        &self.submission
    }

    /// The frame's counts once the device has been polled past the
    /// submission and the mapping has settled. `None` for Direct, a failed
    /// or still pending mapping, or a frame whose earlier mapping had not
    /// settled when it was armed.
    #[must_use]
    pub fn take(self) -> Option<CullingCounts> {
        self.target.take_culling_counts()
    }
}

impl PreparedSynthetic {
    /// Record, submit and map one frame with its culling counts, in the order
    /// `FrameHost` uses: arm the count copy, encode the scene (closing the
    /// timestamp batch when `timings` is given), submit, then map with this
    /// prepared scene's mode and active prefix. Call after `update_frame`
    /// and commit the view afterwards, as for `encode`.
    /// # Errors
    /// Returns encoding and timestamp errors before anything is submitted.
    pub fn submit_counted<'target>(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        generation: u64,
        target: &'target mut OffscreenTarget,
        mut timings: Option<&mut GpuTimings>,
    ) -> Result<CountedFrame<'target>, RenderError> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("counted synthetic frame"),
        });
        target.arm_culling_stats();
        self.encode(&mut encoder, generation, target, timings.as_deref_mut())?;
        if let Some(timings) = timings {
            timings
                .finish_render(generation, &encoder)
                .map_err(RenderError::Timing)?;
        }
        let submission = queue.submit([encoder.finish()]);
        target.map_culling_stats(self.camera_culling(), self.active_instance_count());
        Ok(CountedFrame { target, submission })
    }
}

impl SceneContext<'_> {
    /// Copy each list's instance count into the slot's count buffer.
    pub(in crate::synthetic) fn culling_stats(&mut self) -> Result<(), RenderError> {
        let target = self.target;
        let (Some(stats), Some(drawn)) = (&target.culling_stats, &target.visibility) else {
            return Err(RenderError::GraphPassMismatch);
        };
        if !stats.armed {
            // Still mapping an earlier frame, or a capture: no count.
            return Ok(());
        }
        let prepass = target
            .occlusion
            .as_ref()
            .map(|prepass| prepass.target.prepass());
        let lists = std::iter::once(drawn).chain(prepass);
        for (index, list) in (0..stats.lists).zip(lists) {
            self.encoder.copy_buffer_to_buffer(
                list.args(),
                0,
                &stats.buffer,
                index * stats.args_bytes,
                stats.args_bytes,
            );
        }
        Ok(())
    }
}

impl OffscreenTarget {
    /// Before the host encodes this slot: arm the count copy unless the
    /// buffer is still mapping. Either way the earlier frame's metadata is
    /// forgotten, so a mapping that settles later is reaped without being
    /// attributed to this frame. A count the last frame left untaken is
    /// discarded and unmapped.
    pub(crate) fn arm_culling_stats(&mut self) {
        let Some(stats) = self.culling_stats.as_mut() else {
            return;
        };
        stats.armed = match stats.mapping.load(Ordering::Acquire) {
            PENDING => {
                stats.frame = None;
                false
            }
            state => {
                if state == READY {
                    stats.buffer.unmap();
                }
                stats.frame = None;
                stats.mapping.store(IDLE, Ordering::Release);
                true
            }
        };
    }

    /// After the host submits this slot: map an armed frame's count,
    /// recording its mode and active prefix. Nothing waits on the mapping.
    pub(crate) fn map_culling_stats(&mut self, mode: CullMode, active: u32) {
        let Some(stats) = self.culling_stats.as_mut() else {
            return;
        };
        if !std::mem::take(&mut stats.armed) {
            return;
        }
        stats.frame = Some((mode, active));
        stats.mapping.store(PENDING, Ordering::Release);
        let mapping = Arc::clone(&stats.mapping);
        stats
            .buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let state = if result.is_ok() { READY } else { FAILED };
                mapping.store(state, Ordering::Release);
            });
    }

    /// The submitted frame's counts if its mapping has settled, unmapping
    /// the buffer. `None` for Direct, a failed mapping, a mapping still
    /// pending (left for the next arm), and an unarmed frame, which reaps
    /// an earlier frame's settled mapping without reporting it.
    pub(crate) fn take_culling_counts(&mut self) -> Option<CullingCounts> {
        let stats = self.culling_stats.as_mut()?;
        if stats.mapping.load(Ordering::Acquire) == PENDING {
            return None;
        }
        let state = stats.mapping.swap(IDLE, Ordering::AcqRel);
        let frame = stats.frame.take();
        if state != READY {
            return None;
        }
        let records = usize::try_from(stats.args_bytes / ARGS_BYTES).unwrap_or(0);
        let counts = stats
            .buffer
            .slice(..)
            .get_mapped_range()
            .ok()
            .and_then(|bytes| {
                let word = |index: usize| {
                    bytes
                        .get(index * 4..index * 4 + 4)
                        .and_then(|word| word.try_into().ok())
                        .map_or(0, u32::from_le_bytes)
                };
                // Each list's records, summed over buckets.
                let count = |list: usize| {
                    (0..records)
                        .map(|record| word((list * records + record) * 5 + COUNT_WORD))
                        .fold(0_u32, u32::saturating_add)
                };
                frame.map(|(mode, active)| CullingCounts {
                    mode,
                    active,
                    drawn: count(0),
                    prepass: count(1),
                })
            });
        stats.buffer.unmap();
        counts
    }
}

/// Test hooks for a mapping callback that has not arrived.
#[cfg(test)]
mod hooks {
    use super::{AtomicU8, IDLE, OffscreenTarget, Ordering, PENDING};
    use std::sync::Arc;

    impl OffscreenTarget {
        /// Hide this slot's real mapping state behind a pending one, and
        /// return the state the real callback writes.
        pub(crate) fn defer_culling_mapping(&mut self) -> Option<Arc<AtomicU8>> {
            let stats = self.culling_stats.as_mut()?;
            Some(std::mem::replace(
                &mut stats.mapping,
                Arc::new(AtomicU8::new(PENDING)),
            ))
        }

        /// Make a deferred mapping's real state visible to the host.
        pub(crate) fn reveal_culling_mapping(&mut self, real: &AtomicU8) {
            if let Some(stats) = self.culling_stats.as_mut() {
                stats
                    .mapping
                    .store(real.load(Ordering::Acquire), Ordering::Release);
            }
        }

        /// Whether a deferred mapping's real callback has not arrived yet.
        pub(crate) fn culling_mapping_pending(real: &AtomicU8) -> bool {
            real.load(Ordering::Acquire) == PENDING
        }

        /// Hold this slot's count buffer as if its mapping were pending.
        pub(crate) fn stall_culling_stats(&mut self, stalled: bool) {
            if let Some(stats) = self.culling_stats.as_mut() {
                let state = if stalled { PENDING } else { IDLE };
                stats.mapping.store(state, Ordering::Release);
            }
        }
    }
}
