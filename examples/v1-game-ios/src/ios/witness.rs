//! Raw function witnesses for the simulator checker: one key=value file in the
//! app's temporary directory, rewritten at transitions. Simulator only.

use std::{fmt::Write as _, fs, time::Duration};

use thrum_core::sim::WorldLifecycle;
use thrum_render::frame::TargetLeadStatus;

use super::render::{FrameReply, SurfaceStatus};
use crate::gesture::GestureCounters;

const NAME: &str = "thrum-v1-ios-state.txt";

#[derive(Default)]
pub(super) struct Witness {
    pub(super) state: String,
    pub(super) adapter: String,
    pub(super) format: String,
    pub(super) display_thread: String,
    pub(super) touch_thread: String,
    pub(super) lifecycle_thread: String,
    pub(super) prepare_requested_on: String,
    pub(super) prepare_thread: String,
    pub(super) frame_thread: String,
    pub(super) world_thread: String,
    pub(super) display_link_callbacks: u64,
    pub(super) targets_ignored: u64,
    pub(super) targets_coalesced: u64,
    pub(super) requests: u64,
    pub(super) replies: u64,
    pub(super) presented: u64,
    pub(super) completed_gpu: u64,
    pub(super) surface_samples: u64,
    pub(super) presentable_encodes: u64,
    pub(super) acquired: u64,
    pub(super) no_view: u64,
    pub(super) denied: u64,
    pub(super) no_lease: u64,
    pub(super) missing: u64,
    pub(super) reconfigure_waits: u64,
    pub(super) stale_dropped: u64,
    pub(super) configured: u64,
    pub(super) max_pending_frames: usize,
    pub(super) late_targets: u64,
    pub(super) invalid_targets: u64,
    pub(super) clamped_targets: u64,
    pub(super) epoch: u64,
    pub(super) presented_epoch: u64,
    pub(super) installs: u64,
    pub(super) release_immediate: u64,
    pub(super) release_deferred: u64,
    pub(super) release_completed: u64,
    pub(super) resizes: u64,
    pub(super) suspends: u64,
    pub(super) resumes: u64,
    pub(super) replacements: u64,
    pub(super) commands: u64,
    pub(super) commands_refused: u64,
    pub(super) last_command: String,
    pub(super) main_max_us: u128,
    pub(super) lifecycle_main_max_us: u128,
    pub(super) stall_main_max_us: u128,
    pub(super) stalls: u64,
    pub(super) stall_targets: u64,
    pub(super) stall_tick_start: u64,
    pub(super) stall_tick_end: u64,
    pub(super) stall_status: String,
    pub(super) world_tick: u64,
    pub(super) world_paused: bool,
    pub(super) pause_requests: u64,
    pub(super) pause_replies: u64,
    pub(super) resume_replies: u64,
    pub(super) pause_epoch: u128,
    pub(super) pause_tick: u64,
    pub(super) resume_epoch: u128,
    pub(super) resume_tick: u64,
    pub(super) overloads: u64,
    pub(super) refused_ticks: u64,
    pub(super) axes_sent: u64,
    pub(super) axes_moving: u64,
    pub(super) max_acquire_us: u128,
    pub(super) raced_presents: u64,
    pub(super) live_detached_windows: i32,
    pub(super) detached_windows: u64,
    pub(super) old_window_touches: u64,
    pub(super) last_axes: [f32; 3],
    pub(super) gesture: GestureCounters,
    pub(super) input_epoch: u64,
}

impl Witness {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn main_callback(&mut self, wall: Duration, stalled: bool, lifecycle: bool) {
        let micros = wall.as_micros();
        if lifecycle {
            self.lifecycle_main_max_us = self.lifecycle_main_max_us.max(micros);
        } else {
            self.main_max_us = self.main_max_us.max(micros);
        }
        if stalled {
            self.stall_main_max_us = self.stall_main_max_us.max(micros);
        }
    }

    pub(super) fn frame(&mut self, frame: &FrameReply, current_epoch: u64) {
        self.replies += 1;
        self.completed_gpu += u64::from(frame.completed);
        self.surface_samples += u64::from(frame.surface_samples);
        self.presentable_encodes += u64::from(frame.presentable_encode);
        self.acquired += u64::from(frame.acquired);
        self.configured += u64::from(frame.configured);
        self.max_pending_frames = self.max_pending_frames.max(frame.pending_frames);
        self.max_acquire_us = self.max_acquire_us.max(frame.acquire.as_micros());
        self.raced_presents += u64::from(frame.raced_present);
        match frame.target {
            TargetLeadStatus::Future => {}
            TargetLeadStatus::Late => self.late_targets += 1,
            TargetLeadStatus::Invalid => self.invalid_targets += 1,
            TargetLeadStatus::Clamped => self.clamped_targets += 1,
        }
        if let Some(report) = frame.report {
            self.world_tick = self.world_tick.max(report.tick);
        }
        match frame.status {
            SurfaceStatus::Presented => {
                self.presented += 1;
                self.presented_epoch = frame.epoch;
                debug_assert!(frame.epoch <= current_epoch);
            }
            SurfaceStatus::NoView => self.no_view += 1,
            SurfaceStatus::Denied => self.denied += 1,
            SurfaceStatus::NoLease => self.no_lease += 1,
            SurfaceStatus::Missing => self.missing += 1,
            SurfaceStatus::Reconfigure => self.reconfigure_waits += 1,
            SurfaceStatus::StaleDropped => self.stale_dropped += 1,
        }
    }

    pub(super) fn world_reply(&mut self, request: WorldLifecycle, epoch: u128, tick: u64) {
        match request {
            WorldLifecycle::Pause => {
                self.pause_replies += 1;
                self.pause_epoch = epoch;
                self.pause_tick = tick;
            }
            WorldLifecycle::Resume => {
                self.resume_replies += 1;
                self.resume_epoch = epoch;
                self.resume_tick = tick;
            }
        }
    }

    pub(super) fn write(&self) {
        let mut out = String::with_capacity(2048);
        let g = self.gesture;
        let rows: [(&str, String); 71] = [
            ("pid", std::process::id().to_string()),
            ("state", self.state.clone()),
            ("adapter", self.adapter.replace(' ', "_")),
            ("format", self.format.clone()),
            ("display_thread", self.display_thread.clone()),
            ("touch_thread", self.touch_thread.clone()),
            ("lifecycle_thread", self.lifecycle_thread.clone()),
            ("prepare_requested_on", self.prepare_requested_on.clone()),
            ("prepare_thread", self.prepare_thread.clone()),
            ("frame_thread", self.frame_thread.clone()),
            ("world_thread", self.world_thread.clone()),
            ("display_link_callbacks", self.display_link_callbacks.to_string()),
            ("targets_ignored", self.targets_ignored.to_string()),
            ("targets_coalesced", self.targets_coalesced.to_string()),
            ("requests", self.requests.to_string()),
            ("replies", self.replies.to_string()),
            ("presented", self.presented.to_string()),
            ("completed_gpu", self.completed_gpu.to_string()),
            ("surface_samples", self.surface_samples.to_string()),
            ("presentable_encodes", self.presentable_encodes.to_string()),
            ("acquired", self.acquired.to_string()),
            ("no_view", self.no_view.to_string()),
            ("denied", self.denied.to_string()),
            ("no_lease", self.no_lease.to_string()),
            ("missing", self.missing.to_string()),
            ("reconfigure_waits", self.reconfigure_waits.to_string()),
            ("stale_dropped", self.stale_dropped.to_string()),
            ("configured", self.configured.to_string()),
            ("max_pending_frames", self.max_pending_frames.to_string()),
            ("late_targets", self.late_targets.to_string()),
            ("invalid_targets", self.invalid_targets.to_string()),
            ("clamped_targets", self.clamped_targets.to_string()),
            ("epoch", self.epoch.to_string()),
            ("presented_epoch", self.presented_epoch.to_string()),
            ("installs", self.installs.to_string()),
            ("release_immediate", self.release_immediate.to_string()),
            ("release_deferred", self.release_deferred.to_string()),
            ("release_completed", self.release_completed.to_string()),
            ("resizes", self.resizes.to_string()),
            ("suspends", self.suspends.to_string()),
            ("resumes", self.resumes.to_string()),
            ("replacements", self.replacements.to_string()),
            ("commands", self.commands.to_string()),
            ("commands_refused", self.commands_refused.to_string()),
            ("last_command", self.last_command.clone()),
            ("main_max_us", self.main_max_us.to_string()),
            ("stall_main_max_us", self.stall_main_max_us.to_string()),
            ("stalls", self.stalls.to_string()),
            ("stall_targets", self.stall_targets.to_string()),
            ("stall_tick_start", self.stall_tick_start.to_string()),
            ("stall_tick_end", self.stall_tick_end.to_string()),
            ("stall_status", self.stall_status.clone()),
            ("world_tick", self.world_tick.to_string()),
            ("world_paused", self.world_paused.to_string()),
            ("pause_requests", self.pause_requests.to_string()),
            ("pause_replies", self.pause_replies.to_string()),
            ("resume_replies", self.resume_replies.to_string()),
            ("pause_epoch", self.pause_epoch.to_string()),
            ("pause_tick", self.pause_tick.to_string()),
            ("resume_epoch", self.resume_epoch.to_string()),
            ("resume_tick", self.resume_tick.to_string()),
            ("overloads", self.overloads.to_string()),
            ("refused_ticks", self.refused_ticks.to_string()),
            ("axes_sent", self.axes_sent.to_string()),
            ("axes_moving", self.axes_moving.to_string()),
            ("touch_starts", g.starts.to_string()),
            ("touch_releases", g.releases.to_string()),
            ("touch_cancels", g.cancels.to_string()),
            ("touch_clears", g.clears.to_string()),
            ("touch_stale", g.stale.to_string()),
            ("touch_ignored_fingers", g.ignored_fingers.to_string()),
        ];
        for (key, value) in rows {
            let _ = writeln!(out, "{key}={value}");
        }
        let [x, y, z] = self.last_axes;
        let tail: [(&str, String); 9] = [
            ("last_axes", format!("{x},{y},{z}")),
            ("lifecycle_main_max_us", self.lifecycle_main_max_us.to_string()),
            ("touch_sends", g.sends.to_string()),
            ("input_epoch", self.input_epoch.to_string()),
            ("max_acquire_us", self.max_acquire_us.to_string()),
            ("raced_presents", self.raced_presents.to_string()),
            ("live_detached_windows", self.live_detached_windows.to_string()),
            ("detached_windows", self.detached_windows.to_string()),
            ("old_window_touches", self.old_window_touches.to_string()),
        ];
        for (key, value) in tail {
            let _ = writeln!(out, "{key}={value}");
        }
        replace(&out);
    }
}

pub(super) fn write_fault(error: &str) {
    replace(&format!("fault={}\n", error.replace('\n', " ")));
}

/// Write beside, then rename, so the checker never reads a partial file.
fn replace(text: &str) {
    let dir = std::env::temp_dir();
    let partial = dir.join(format!("{NAME}.partial"));
    if fs::write(&partial, text).is_ok() {
        drop(fs::rename(&partial, dir.join(NAME)));
    }
}
