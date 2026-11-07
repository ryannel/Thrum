//! Per-slot culling counts on the completion path. An untimed graph copy
//! moves the culled list's GPU-written instance count into the slot's own
//! map buffer; the host maps it when it submits the slot and reads it when
//! the slot's work is done, so nothing waits on the frame path and no count
//! depends on timestamps. Direct lists nothing and declares no copy.

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
    /// After the host submits this slot: map its counts, recording the
    /// frame's mode and active prefix. Nothing waits on the mapping.
    pub(crate) fn map_culling_stats(&mut self, mode: CullMode, active: u32) {
        let Some(stats) = self.culling_stats.as_mut() else {
            return;
        };
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

    /// Whether this slot's counts no longer wait on the GPU: absent, never
    /// mapped, read, or mapped (or failed) after completion.
    pub(crate) fn culling_stats_settled(&self) -> bool {
        self.culling_stats
            .as_ref()
            .is_none_or(|stats| stats.mapping.load(Ordering::Acquire) != PENDING)
    }

    /// The submitted frame's counts once mapped; unmaps for the slot's next
    /// submission. `None` for Direct and for a failed mapping.
    pub(crate) fn take_culling_counts(&mut self) -> Option<CullingCounts> {
        let stats = self.culling_stats.as_mut()?;
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
