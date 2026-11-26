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

    pub(crate) fn set_paused(
        &mut self,
        world: &mut (impl InlineWorld + ?Sized),
        paused: bool,
        now: Duration,
    ) -> Result<(), GameError> {
        self.policy.request(paused);
        self.reconcile(world, now)
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
