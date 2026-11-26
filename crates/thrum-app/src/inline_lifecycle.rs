//! Reconcile an inline world on the thread that advances it.

use std::time::Duration;

use thrum_core::sim::{WorldLifecycle, WorldLifecyclePolicy};
use thrum_render::runtime::DriveReport;

use crate::{GameError, InlineWorld};

#[derive(Default)]
pub(crate) struct InlineLifecycle {
    policy: WorldLifecyclePolicy,
    queued_ticks: u32,
}

impl InlineLifecycle {
    pub(crate) fn observe(
        &mut self,
        world: &mut (impl InlineWorld + ?Sized),
        report: DriveReport,
        now: Duration,
    ) -> Result<(), GameError> {
        self.queued_ticks = report.queued_ticks;
        if let Some(refused) = report.overload {
            self.policy.overloaded(refused);
        }
        self.reconcile(world, now)
    }

    #[cfg(any(target_os = "ios", target_arch = "wasm32"))]
    pub(crate) fn set_paused(
        &mut self,
        world: &mut (impl InlineWorld + ?Sized),
        paused: bool,
        now: Duration,
    ) -> Result<(), GameError> {
        self.policy.request(paused);
        self.reconcile(world, now)
    }

    #[cfg(target_os = "ios")]
    pub(crate) fn restart_after_gap(
        &mut self,
        world: &mut (impl InlineWorld + ?Sized),
        now: Duration,
    ) -> Result<(), GameError> {
        // A dropped frame can have advanced the queue without returning a
        // report. Stop admission now and use the next report to certify drain.
        self.queued_ticks = self.queued_ticks.max(1);
        self.set_paused(world, true, now)?;
        self.policy.request(false);
        Ok(())
    }

    pub(crate) fn counts(&self) -> (u64, u64) {
        (self.policy.overload_count(), self.policy.refused_ticks())
    }

    fn reconcile(
        &mut self,
        world: &mut (impl InlineWorld + ?Sized),
        now: Duration,
    ) -> Result<(), GameError> {
        self.policy
            .drive_inline(self.queued_ticks, |request| match request {
                WorldLifecycle::Pause => world.pause(),
                WorldLifecycle::Resume => world.resume(now),
            })
    }
}
