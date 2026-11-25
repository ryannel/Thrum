//! View-independent tick submission on the frame host's queue.

use super::{FrameError, FrameHost, FrameWorld, IN_FLIGHT, SceneSource, SlotState};
use crate::timing::{GpuTimings, TimingCategory};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// A shell's revocable right to submit a tick encoder.
pub trait TickSubmissionPermission: wgpu::WasmNotSend {
    /// Enter the shell's permission section or return without calling `record`.
    /// Call it once while the section is held. Its argument rechecks the right
    /// to submit while still inside that section.
    fn with_entry(&self, record: &mut dyn FnMut(&dyn Fn() -> bool));
}

/// View-independent submissions and backpressure, counted over the host's lifetime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickSubmissionStats {
    /// Tick encoders submitted.
    pub submitted: u64,
    /// Askings with work refused at the outstanding submission bound.
    pub refused_bound: u64,
    /// Askings with work refused by the platform.
    pub refused_permission: u64,
}

#[derive(Default)]
pub(super) struct TickStage {
    permission: Option<Box<dyn TickSubmissionPermission>>,
    outstanding: Vec<Arc<AtomicBool>>,
    stats: TickSubmissionStats,
    pub(super) timing_slot: Option<usize>,
}

impl TickStage {
    pub(super) fn retire_device(&mut self) {
        self.outstanding.clear();
        self.timing_slot = None;
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Install the shell's submission permission. It survives device replacement.
    pub fn set_tick_submission_permission(
        &mut self,
        permission: Box<dyn TickSubmissionPermission>,
    ) {
        self.ticks.permission = Some(permission);
    }

    /// Maximum tick submissions in flight on this host, independent of view slots.
    pub const MAX_TICK_SUBMISSIONS: usize = IN_FLIGHT;

    /// View-independent submission counts.
    pub const fn tick_submission_stats(&self) -> TickSubmissionStats {
        self.ticks.stats
    }

    /// Ask for posted tick work before waiting for optional rendering.
    /// This performs no clock observation, publication read or device poll.
    ///
    /// # Errors
    /// Returns a device fault if the active generation cannot submit.
    pub fn ask_tick_stage(&mut self) -> Result<(), FrameError> {
        self.submit_tick_work(false)
    }

    pub(super) fn ask_tick_stage_for_view(&mut self) -> Result<(), FrameError> {
        self.submit_tick_work(self.timing_requested)
    }

    fn submit_tick_work(&mut self, time_with_view: bool) -> Result<(), FrameError> {
        let Some(scene) = self.prepared.three_d() else {
            return Ok(());
        };
        let passes = &scene.effect_graph().tick_passes;
        if passes.is_empty() {
            return Ok(());
        }
        let Some(contribution) = &self.contribution else {
            return Ok(());
        };
        let active: Vec<_> = (0..passes.len())
            .filter(|&index| contribution.tick_has_work(index))
            .collect();
        if active.is_empty() {
            return Ok(());
        }
        self.ticks
            .outstanding
            .retain(|done| !done.load(Ordering::Acquire));
        if self.ticks.outstanding.len() >= Self::MAX_TICK_SUBMISSIONS {
            self.ticks.stats.refused_bound += 1;
            return Ok(());
        }
        self.check_native_completion()?;
        let scene = self
            .prepared
            .three_d()
            .expect("tick passes belong to 3D scenes");
        let passes = &scene.effect_graph().tick_passes;
        let contribution = self.contribution.as_deref_mut().expect("tick contribution");
        let tick_duration = self.world.tick_duration();
        // The first free slot is the one this advance would use for a view.
        // Reserve alongside that view's queries without claiming its slot.
        let timing_slot = if time_with_view {
            self.slots
                .iter()
                .position(|slot| matches!(slot.state, SlotState::Free))
        } else {
            None
        };
        let mut recording = Some((
            contribution,
            timing_slot.map(|index| &mut self.slots[index].timing.queries),
        ));
        let mut submitted = false;
        let mut error = None;
        let mut record = |confirm: &dyn Fn() -> bool| {
            let Some((contribution, timing)) = recording.take() else {
                return;
            };
            match record_tick_passes(
                &self.device,
                contribution,
                passes,
                &active,
                timing,
                self.generation,
                tick_duration,
            ) {
                Ok(encoder) => {
                    if confirm() {
                        self.queue.submit([encoder]);
                        submitted = true;
                    }
                }
                Err(fault) => error = Some(fault),
            }
        };
        if let Some(permission) = &self.ticks.permission {
            permission.with_entry(&mut record);
        } else {
            record(&|| true);
        }
        self.ticks.timing_slot = timing_slot;
        if !submitted {
            self.discard_tick_timing_without_view();
            if let Some(error) = error {
                return Err(error);
            }
            self.ticks.stats.refused_permission += 1;
            return Ok(());
        }
        let done = Arc::new(AtomicBool::new(false));
        let completion = Arc::clone(&done);
        self.queue
            .on_submitted_work_done(move || completion.store(true, Ordering::Release));
        self.ticks.outstanding.push(done);
        self.ticks.stats.submitted += 1;
        self.contribution
            .as_deref_mut()
            .expect("submitted tick contribution")
            .tick_submitted(&active);
        Ok(())
    }

    pub(super) fn discard_tick_timing_without_view(&mut self) {
        if let Some(index) = self.ticks.timing_slot.take()
            && let Some(slot) = self.slots.get_mut(index)
            && matches!(slot.state, SlotState::Free)
        {
            let _ = slot.timing.queries.discard_unsubmitted(self.generation);
        }
    }
}

fn record_tick_passes(
    device: &wgpu::Device,
    contribution: &mut dyn crate::contribution::SceneContribution,
    passes: &[crate::contribution::TickPass],
    active: &[usize],
    mut timing: Option<&mut GpuTimings>,
    generation: u64,
    tick_duration: std::time::Duration,
) -> Result<wgpu::CommandBuffer, FrameError> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("tick submission"),
    });
    for &index in active {
        let name = &passes[index].name;
        contribution.encode_tick_upload(index, &mut encoder, tick_duration);
        let timestamp_writes = timing
            .as_deref_mut()
            .map(|queries| queries.compute_writes(generation, name, TimingCategory::Effects))
            .transpose()
            .map_err(|error| FrameError::gpu("tick timestamps", error))?
            .flatten();
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(name),
                timestamp_writes,
            });
            contribution.encode_tick(index, &mut pass);
        }
        contribution.encode_tick_readback(index, &mut encoder);
    }
    Ok(encoder.finish())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
