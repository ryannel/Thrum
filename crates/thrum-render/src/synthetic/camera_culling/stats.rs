//! Per-slot culling counts beside the completion path. An untimed graph copy
//! moves the culled list's GPU-written instance count into the slot's own
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

/// Graph name of the untimed copy of the culled list's instance count.
pub(crate) const STATS_PASS: &str = "camera culling stats";
/// Byte offset of `instance_count` in indexed indirect arguments.
const COUNT_OFFSET: u64 = 4;
const COUNT_BYTES: u64 = 4;

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
    /// Instances the indirect draw drew.
    pub drawn: u32,
}

impl CullingCounts {
    /// Active slots the cull rejected.
    #[must_use]
    pub fn rejected(&self) -> u32 {
        self.active.saturating_sub(self.drawn)
    }
}

/// One frame slot's count buffer and its mapping state.
pub(in crate::synthetic) struct CullingStats {
    buffer: wgpu::Buffer,
    mapping: Arc<AtomicU8>,
    frame: Option<(CullMode, u32)>,
    /// This frame copies and maps: the buffer was free when it was encoded.
    armed: bool,
}

impl CullingStats {
    pub(in crate::synthetic) fn new(device: &wgpu::Device, mode: CullMode) -> Option<Self> {
        if mode == CullMode::Direct {
            return None;
        }
        Some(Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("camera culling counts"),
                size: COUNT_BYTES,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            mapping: Arc::new(AtomicU8::new(IDLE)),
            frame: None,
            armed: false,
        })
    }
}

impl PreparedSynthetic {
    /// Declare the untimed count copy from the culled list's arguments,
    /// after the pass that writes them. Only a culled mode calls this.
    pub(in crate::synthetic) fn append_camera_stats(
        graph: &mut FrameGraph,
        args: &ResourceId,
    ) -> Result<(), GraphError> {
        let destination = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: COUNT_BYTES,
                usage: wgpu::BufferUsages::COPY_DST,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        graph.pass(Pass {
            name: STATS_PASS.into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![
                Access {
                    resource: args.clone(),
                    range: COUNT_OFFSET..COUNT_OFFSET + COUNT_BYTES,
                    kind: AccessKind::CopyRead,
                    store: true,
                },
                Access {
                    resource: destination,
                    range: 0..COUNT_BYTES,
                    kind: AccessKind::CopyWrite,
                    store: true,
                },
            ],
            external_side_effect: true,
        });
        Ok(())
    }
}

impl SceneContext<'_> {
    /// Copy the culled list's instance count into the slot's count buffer.
    pub(in crate::synthetic) fn culling_stats(&mut self) -> Result<(), RenderError> {
        let target = self.target;
        let (Some(stats), Some(list)) = (&target.culling_stats, &target.visibility) else {
            return Err(RenderError::GraphPassMismatch);
        };
        if !stats.armed {
            // Still mapping an earlier frame, or a capture: no count.
            return Ok(());
        }
        self.encoder.copy_buffer_to_buffer(
            list.args(),
            COUNT_OFFSET,
            &stats.buffer,
            0,
            COUNT_BYTES,
        );
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
        let counts = stats
            .buffer
            .slice(..)
            .get_mapped_range()
            .ok()
            .and_then(|bytes| {
                let drawn = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?);
                frame.map(|(mode, active)| CullingCounts {
                    mode,
                    active,
                    drawn,
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
