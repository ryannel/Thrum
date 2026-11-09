//! Raw function witnesses for the simulator checker: one key=value file in the
//! app's temporary directory, rewritten at transitions. Simulator only.

use std::{fmt::Write as _, fs, time::Duration};

use thrum_core::sim::WorldLifecycle;
use thrum_render::frame::TargetLeadStatus;

use super::render::{FrameReply, Recovered, SurfaceStatus};
use crate::gesture::GestureCounters;

const NAME: &str = "thrum-v1-ios-state.txt";

#[derive(Default)]
pub(super) struct Witness {
    pub(super) state: String,
    cooked: String,
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
    /// `Resized` events on the current window that changed the extent.
    pub(super) resize_events: u64,
    /// The surface epoch the latest resize created.
    pub(super) resize_epoch: u64,
    /// Waiting for the first present at the latest resize's extent.
    resize_open: bool,
    /// Frame replies since the latest resize, until a new-extent present.
    resize_replies: u64,
    /// Most frame replies any resize waited for a new-extent present.
    pub(super) max_resize_gap_replies: u64,
    /// Presents admitted on an epoch a resize had already revoked.
    pub(super) resize_raced_presents: u64,
    pub(super) extent_drops: u64,
    pub(super) old_extent_presents: u64,
    pub(super) presented_extent: [u32; 2],
    frame_extent: Option<thrum_render::runtime::FrameExtent>,
    drawable: [u32; 2],
    pub(super) install_retries: u64,
    pub(super) loss_replacements: u64,
    pub(super) device_generation: u64,
    pub(super) presented_generation: u64,
    pub(super) lost_generation: u64,
    pub(super) losses: u64,
    pub(super) stale_loss_wakes: u64,
    pub(super) lost_frames: u64,
    pub(super) lost_held_drops: u64,
    /// Recorded frames skipped on a healthy device, each flushing staging
    /// with an empty queue submit.
    pub(super) empty_submits: u64,
    /// Empty submits on a frame that replied `DeviceLost`; must stay zero.
    pub(super) lost_empty_submits: u64,
    /// Recorded frames abandoned on a lost device with no queue call.
    pub(super) lost_abandons: u64,
    pub(super) stale_generation_refusals: u64,
    pub(super) recover_requests: u64,
    pub(super) recoveries: u64,
    pub(super) recover_deferred: u64,
    pub(super) recover_candidate_lost: u64,
    pub(super) recover_failed: u64,
    pub(super) recover_stale: u64,
    pub(super) recover_held: u64,
    pub(super) recover_refitted: u64,
    pub(super) recover_wall_us: u128,
    pub(super) recovered_extent: [u32; 2],
    pub(super) recover_error: String,
    pub(super) suspends: u64,
    pub(super) resumes: u64,
    pub(super) replacements: u64,
    pub(super) commands: u64,
    pub(super) commands_refused: u64,
    pub(super) last_command: String,
    pub(super) main_max_us: u128,
    pub(super) lifecycle_main_max_us: u128,
    pub(super) main_max_kind: &'static str,
    pub(super) lifecycle_main_max_kind: &'static str,
    pub(super) stall_main_max_us: u128,
    pub(super) stalls: u64,
    pub(super) stall_targets: u64,
    pub(super) stall_tick_start: u64,
    pub(super) stall_tick_end: u64,
    pub(super) stall_status: String,
    pub(super) stall_active: bool,
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
    pub(super) actor_samples: u64,
    pub(super) actor_position: Option<[f64; 3]>,
    pub(super) gesture: GestureCounters,
    pub(super) input_epoch: u64,
}

impl Witness {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn main_callback(
        &mut self,
        wall: Duration,
        stalled: bool,
        lifecycle: bool,
        kind: &'static str,
    ) {
        let micros = wall.as_micros();
        if lifecycle {
            if micros > self.lifecycle_main_max_us {
                self.lifecycle_main_max_us = micros;
                self.lifecycle_main_max_kind = kind;
            }
        } else if micros > self.main_max_us {
            self.main_max_us = micros;
            self.main_max_kind = kind;
        }
        if stalled {
            self.stall_main_max_us = self.stall_main_max_us.max(micros);
        }
    }

    /// A resize created surface epoch `epoch`; a present at it closes the gap.
    pub(super) fn resized(&mut self, epoch: u64) {
        self.resizes += 1;
        self.resize_epoch = epoch;
        self.resize_open = true;
        self.resize_replies = 0;
    }

    pub(super) fn recovered(&mut self, recovered: &Recovered) {
        self.recoveries += 1;
        self.device_generation = recovered.generation;
        self.recover_refitted += u64::from(recovered.refitted);
        self.recover_wall_us = recovered.wall.as_micros();
        self.recovered_extent = recovered.extent;
    }

    pub(super) fn frame(&mut self, frame: &FrameReply) {
        self.cooked.clone_from(&frame.cooked);
        self.replies += 1;
        if self.resize_open {
            self.resize_replies += 1;
            self.max_resize_gap_replies = self.max_resize_gap_replies.max(self.resize_replies);
        }
        self.extent_drops += u64::from(frame.extent_dropped);
        self.old_extent_presents += u64::from(frame.old_extent_present);
        if frame.raced_present && frame.epoch < self.resize_epoch {
            self.resize_raced_presents += 1;
        }
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
        self.empty_submits += u64::from(frame.empty_submit);
        self.lost_abandons += u64::from(frame.abandoned);
        if frame.status == SurfaceStatus::DeviceLost {
            self.lost_empty_submits += u64::from(frame.empty_submit);
        }
        if let Some(report) = frame.report {
            self.world_tick = self.world_tick.max(report.tick);
        }
        match frame.status {
            SurfaceStatus::Presented => {
                self.frame_extent = frame.frame_extent;
                self.drawable = frame.drawable;
                self.presented += 1;
                self.presented_epoch = frame.epoch;
                self.presented_extent = frame.extent;
                self.presented_generation = frame.generation;
                if frame.epoch >= self.resize_epoch {
                    self.resize_open = false;
                }
            }
            SurfaceStatus::NoView => self.no_view += 1,
            SurfaceStatus::Denied => self.denied += 1,
            SurfaceStatus::NoLease => self.no_lease += 1,
            SurfaceStatus::Missing => self.missing += 1,
            SurfaceStatus::Reconfigure => self.reconfigure_waits += 1,
            SurfaceStatus::StaleDropped => self.stale_dropped += 1,
            SurfaceStatus::DeviceLost => {
                self.lost_frames += 1;
                self.lost_held_drops += u64::from(frame.acquired);
            }
            SurfaceStatus::StaleGeneration => self.stale_generation_refusals += 1,
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

    fn extent_text(&self) -> String {
        self.frame_extent.map_or_else(
            || "none".to_owned(),
            |frame| {
                let attachments = frame
                    .attachments
                    .map(|extent| format!("{}x{}", extent[0], extent[1]))
                    .join(",");
                let matrix = frame
                    .view_projection
                    .iter()
                    .flatten()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{};{};{}x{};{}x{};{},{},{},{};{};{};{};{}",
                    attachments,
                    matrix,
                    frame.source[0],
                    frame.source[1],
                    frame.destination[0],
                    frame.destination[1],
                    frame.viewport[0],
                    frame.viewport[1],
                    frame.viewport[2],
                    frame.viewport[3],
                    frame.submission,
                    frame.revision,
                    frame.target_textures,
                    frame.target_bytes
                )
            },
        )
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
            (
                "display_link_callbacks",
                self.display_link_callbacks.to_string(),
            ),
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
        let actor = self
            .actor_position
            .map_or_else(|| "none".to_owned(), |[x, y, z]| format!("{x},{y},{z}"));
        let tail: [(&str, String); 47] = [
            ("actor_samples", self.actor_samples.to_string()),
            ("actor_position", actor),
            ("stall_active", self.stall_active.to_string()),
            ("main_max_kind", self.main_max_kind.to_owned()),
            (
                "lifecycle_main_max_kind",
                self.lifecycle_main_max_kind.to_owned(),
            ),
            ("last_axes", format!("{x},{y},{z}")),
            (
                "lifecycle_main_max_us",
                self.lifecycle_main_max_us.to_string(),
            ),
            ("touch_sends", g.sends.to_string()),
            ("input_epoch", self.input_epoch.to_string()),
            ("max_acquire_us", self.max_acquire_us.to_string()),
            ("raced_presents", self.raced_presents.to_string()),
            (
                "live_detached_windows",
                self.live_detached_windows.to_string(),
            ),
            ("detached_windows", self.detached_windows.to_string()),
            ("old_window_touches", self.old_window_touches.to_string()),
            ("resize_events", self.resize_events.to_string()),
            ("resize_epoch", self.resize_epoch.to_string()),
            (
                "max_resize_gap_replies",
                self.max_resize_gap_replies.to_string(),
            ),
            (
                "resize_raced_presents",
                self.resize_raced_presents.to_string(),
            ),
            ("extent_drops", self.extent_drops.to_string()),
            ("install_retries", self.install_retries.to_string()),
            ("loss_replacements", self.loss_replacements.to_string()),
            ("device_generation", self.device_generation.to_string()),
            (
                "presented_generation",
                self.presented_generation.to_string(),
            ),
            ("lost_generation", self.lost_generation.to_string()),
            ("losses", self.losses.to_string()),
            ("stale_loss_wakes", self.stale_loss_wakes.to_string()),
            ("lost_frames", self.lost_frames.to_string()),
            ("lost_held_drops", self.lost_held_drops.to_string()),
            ("empty_submits", self.empty_submits.to_string()),
            ("lost_empty_submits", self.lost_empty_submits.to_string()),
            ("lost_abandons", self.lost_abandons.to_string()),
            (
                "stale_generation_refusals",
                self.stale_generation_refusals.to_string(),
            ),
            ("recover_requests", self.recover_requests.to_string()),
            ("recoveries", self.recoveries.to_string()),
            ("recover_deferred", self.recover_deferred.to_string()),
            (
                "recover_candidate_lost",
                self.recover_candidate_lost.to_string(),
            ),
            ("recover_failed", self.recover_failed.to_string()),
            ("recover_stale", self.recover_stale.to_string()),
            ("recover_held", self.recover_held.to_string()),
            ("recover_refitted", self.recover_refitted.to_string()),
            ("recover_wall_us", self.recover_wall_us.to_string()),
            (
                "recovered_extent",
                format!("{}x{}", self.recovered_extent[0], self.recovered_extent[1]),
            ),
            ("recover_error", self.recover_error.clone()),
            ("frame_extent", self.extent_text()),
            (
                "drawable",
                format!("{}x{}", self.drawable[0], self.drawable[1]),
            ),
            ("old_extent_presents", self.old_extent_presents.to_string()),
            (
                "presented_extent",
                format!("{}x{}", self.presented_extent[0], self.presented_extent[1]),
            ),
        ];
        for (key, value) in tail {
            let _ = writeln!(out, "{key}={value}");
        }
        let _ = writeln!(out, "{}", self.cooked);
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
