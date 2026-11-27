//! Native window proof input. The scrub advances once per submitted frame.

use std::cell::Cell;

use super::{DayCycle, GameScene, SolarSweep};
use crate::environment::SunSample;

#[derive(Default)]
pub(super) struct ChartProof {
    submission: u64,
    at_refresh_boundary: bool,
    first: Cell<Option<u64>>,
    clock_anchor: Cell<Option<(u64, f32)>>,
}

impl ChartProof {
    pub(super) fn observe(&mut self, submission: u64, at_refresh_boundary: bool) {
        self.submission = submission;
        self.at_refresh_boundary = at_refresh_boundary;
    }

    fn offset(tick: u64, alpha: f32, hour: f32) -> f32 {
        // The author specifies the rendered hour, which precedes the clock's
        // committed endpoint by the unused fraction of the current tick.
        let phase = (hour / 24.0 + (1.0 - alpha) / 18_000.0).rem_euclid(1.0);
        DayCycle::offset_for_time(tick, phase).expect("chart proof hours are in the supported day")
    }

    pub(super) fn sample(
        &self,
        sweep: SolarSweep,
        tick: u64,
        alpha: f32,
    ) -> Option<(SunSample, bool)> {
        let ([start, _], [from, to]) = sweep;
        let first = if let Some(first) = self.first.get() {
            first
        } else if tick >= start && self.at_refresh_boundary {
            self.first.set(Some(self.submission));
            self.submission
        } else {
            return None;
        };
        let intervals = ((to - from).abs() * (15.0 / 1.2)).ceil().max(1.0);
        let elapsed =
            f32::from(u16::try_from(self.submission.saturating_sub(first)).unwrap_or(u16::MAX));
        if elapsed <= intervals {
            let hour = (to - from).mul_add(elapsed / intervals, from);
            if elapsed >= intervals {
                self.clock_anchor.set(Some((tick, alpha)));
            }
            return Some((
                DayCycle::sample(tick, alpha, Self::offset(tick, alpha, hour)),
                true,
            ));
        }
        let (anchor, fraction) = self
            .clock_anchor
            .get()
            .expect("the scrub reached the end hour");
        Some((
            DayCycle::sample(tick, alpha, Self::offset(anchor, fraction, to)),
            false,
        ))
    }
}

impl GameScene {
    pub(crate) fn observe_chart_submission(&mut self, submission: u64, at_refresh_boundary: bool) {
        self.chart_proof.observe(submission, at_refresh_boundary);
    }

    pub(super) fn chart_sweep_sample(&self, tick: u64, alpha: f32) -> Option<(SunSample, bool)> {
        self.chart_jump?;
        self.chart_proof.sample(self.solar_sweep?, tick, alpha)
    }
}
