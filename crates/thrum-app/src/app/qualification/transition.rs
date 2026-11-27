//! Opt-in observations of the ordinary Mac render owner's surface changes.

use super::super::{
    output_scale::OutputRequest,
    render_actor::{RenderOutcome, RenderReply},
};
use objc2_quartz_core::CACurrentMediaTime;
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};
use thrum_render::{
    capabilities::{configure_surface_once, drawable_extent},
    runtime::{ExtentRequest, FrameExtent, SurfaceSwitchRecord},
};

/// Arm of the ordinary window resize proof.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TransitionArm {
    /// Keep the window at its original size.
    Baseline,
    /// Keep the window at its monitor's fullscreen extent.
    Fullscreen,
    /// Launch in simple fullscreen and record the scaled drawable and layer.
    OutputScaleFullscreen,
    /// Toggle native AppKit fullscreen in both directions during play.
    OutputScaleMode,
    /// Keep the window at its initial size during the presentation comparison.
    Route,
    /// Capture a status display across a series of real window sizes.
    Status,
    /// Sweep the window and inject a display backing-scale observation.
    Sweep,
    /// Force generation preparation and configuration on every size change.
    Synchronous,
    /// Cause one real configure failure and test restoration.
    Restore,
    /// Show the descriptor discrepancy when the restoring call is omitted.
    SkipRestore,
}

impl TransitionArm {
    pub(in crate::app) fn window_step(self, frame: usize) -> Option<[u32; 2]> {
        match (self, frame) {
            (Self::Status, 24) => Some([640, 360]),
            (Self::Status, 48) => Some([320, 568]),
            (Self::Status, 72) => Some([568, 320]),
            (Self::Status, 96) => Some([1280, 720]),
            (
                Self::Status
                | Self::Baseline
                | Self::Fullscreen
                | Self::Route
                | Self::OutputScaleFullscreen
                | Self::OutputScaleMode,
                _,
            ) => None,
            (_, 24..48) => {
                let step = u32::try_from(frame - 24).expect("resize sweep step fits u32");
                Some([960 - step * 8, 540 - step * 4])
            }
            _ => None,
        }
    }

    pub(in crate::app) fn backing_scale(self, actual: f64, frame: usize) -> f64 {
        if self == Self::Sweep && frame >= 100 {
            // Inject a changed scale observation while AppKit still supplies the real grid.
            actual * 0.75
        } else {
            actual
        }
    }
}

/// Inputs for a scripted transition in the real Mac window.
pub struct WindowTransitionProof {
    /// Existing directory for window and submission records.
    pub output: PathBuf,
    /// Scripted sequence or fault control.
    pub arm: TransitionArm,
    /// Retain the ordinary submitted T2 images with their extents.
    pub tap: bool,
    /// Optional inclusive world-tick cue and its preallocated capture limits.
    pub capture_cue: Option<([u64; 2], thrum_render::runtime::CaptureLimits)>,
    /// Record governor samples and optionally inject a platform thermal excursion.
    pub governor: Option<GovernorProof>,
    /// Authored scale replay for a comparison, indexed by the published world tick.
    pub scale_replay: Option<ScaleReplay>,
    /// Substitute this value for the registry default in the fullscreen output policy.
    pub fullscreen_output_scale: Option<thrum_render::synthetic::RenderScale>,
    /// Stage this chart cadence for the presented proof arm.
    pub chart_cadence: Option<u32>,
}

/// Scale trace used in place of the live governor for a paired comparison.
#[derive(Clone, Copy)]
pub struct ScaleReplay {
    /// First tick of the authored route in the published world.
    pub origin: u64,
    /// Drawn scale at a tick relative to the route's origin.
    pub scale_at: fn(u64) -> thrum_render::synthetic::RenderScale,
}

/// Governor observations on the same real surface and display link as the transition proof.
#[derive(Clone, Copy)]
pub enum GovernorProof {
    /// Record the actual platform thermal state.
    Observed,
    /// Supply serious pressure for this half-open range of display-link ticks.
    Thermal {
        /// First display-link tick with forced pressure.
        from: u64,
        /// First display-link tick with forced nominal state.
        until: u64,
    },
}

impl GovernorProof {
    /// Apply a proof's scoped thermal pressure to the platform record for this opportunity.
    #[must_use]
    pub fn display_record(
        self,
        mut display: thrum_render::governor::DisplayRecord,
        tick: u64,
    ) -> thrum_render::governor::DisplayRecord {
        use thrum_render::governor::ThermalState;
        if let Self::Thermal { from, until } = self {
            display.thermal = if (from..until).contains(&tick) {
                ThermalState::Serious
            } else {
                ThermalState::Nominal
            };
        }
        display
    }
}

pub(in crate::app) struct TransitionTicks(BufWriter<File>);

impl TransitionTicks {
    pub(in crate::app) fn start(options: &WindowTransitionProof) -> Result<Self, String> {
        let file = File::create(options.output.join("display.jsonl"))
            .map_err(|error| error.to_string())?;
        Ok(Self(BufWriter::new(file)))
    }

    pub(in crate::app) fn record(
        &mut self,
        sequence: u64,
        callback: u64,
        media: f64,
        target: f64,
        render_busy: bool,
    ) -> io::Result<()> {
        writeln!(
            self.0,
            "{}",
            json!({
                "sequence": sequence, "callback": callback, "media": media, "target": target,
                "dispatched": CACurrentMediaTime(), "render_busy": render_busy,
            })
        )?;
        self.0.flush()
    }
}

pub(in crate::app) struct TransitionProof {
    output: BufWriter<File>,
    configure_log: BufWriter<File>,
    configure_error: Option<io::Error>,
    sequence: Option<u64>,
    work: Vec<(&'static str, f64, f64)>,
    stage: (&'static str, f64),
    present_media: Option<f64>,
    arm: TransitionArm,
    faulted: bool,
    governor: Option<GovernorProof>,
    drawable: Option<([u32; 2], [u32; 2])>,
    frame_extent: Option<FrameExtent>,
    switch: Option<Value>,
    build: Option<Value>,
    identity: Option<PathBuf>,
    #[cfg(feature = "creation-counters")]
    pub(in crate::app) creation_record: Option<PathBuf>,
    pub(in crate::app) scale_replay: Option<ScaleReplay>,
    pub(in crate::app) chart_cadence: Option<u32>,
}

impl TransitionProof {
    pub(in crate::app) fn start(options: &WindowTransitionProof) -> Result<Self, String> {
        let file =
            File::create(options.output.join("frames.jsonl")).map_err(|error| error.to_string())?;
        let configure = File::create(options.output.join("configure.jsonl"))
            .map_err(|error| error.to_string())?;
        let closing_route = matches!(
            options.arm,
            TransitionArm::Fullscreen
                | TransitionArm::Route
                | TransitionArm::OutputScaleFullscreen
                | TransitionArm::OutputScaleMode
        );
        Ok(Self {
            identity: closing_route.then(|| options.output.join("prepared.json")),
            #[cfg(feature = "creation-counters")]
            creation_record: closing_route.then(|| options.output.join("creations.txt")),
            scale_replay: options.scale_replay,
            chart_cadence: options.chart_cadence,
            output: BufWriter::new(file),
            configure_log: BufWriter::new(configure),
            configure_error: None,
            sequence: None,
            work: Vec::new(),
            stage: ("idle", 0.0),
            present_media: None,
            arm: options.arm,
            faulted: false,
            governor: options.governor,
            drawable: None,
            frame_extent: None,
            switch: None,
            build: None,
        })
    }

    pub(in crate::app) fn begin_frame(&mut self, sequence: u64) {
        self.sequence = Some(sequence);
        self.stage = ("frame policy and extent coordination", CACurrentMediaTime());
    }

    pub(in crate::app) fn stage(&mut self, next: &'static str) {
        let now = CACurrentMediaTime();
        self.work.push((self.stage.0, self.stage.1, now));
        self.stage = (next, now);
    }

    pub(in crate::app) fn presented(&mut self, extent: Option<&FrameExtent>) {
        self.present_media = Some(CACurrentMediaTime());
        self.frame_extent = extent.copied();
    }

    pub(in crate::app) fn configure(
        proof: Option<&mut Self>,
        path: &'static str,
        surface: &wgpu::Surface<'_>,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<(), wgpu::Error> {
        let Some(proof) = proof else {
            return configure_surface_once(surface, device, config);
        };
        let started = CACurrentMediaTime();
        let result = configure_surface_once(surface, device, config);
        let ended = CACurrentMediaTime();
        let write = writeln!(
            proof.configure_log,
            "{}",
            json!({
                "sequence": proof.sequence, "path": path, "extent": [config.width, config.height],
                "started": started, "ended": ended, "wall_ms": (ended - started) * 1000.0,
                "succeeded": result.is_ok(),
            })
        )
        .and_then(|()| proof.configure_log.flush());
        if let Err(error) = write {
            proof.configure_error = Some(error);
        }
        result
    }

    pub(in crate::app) fn synchronous(&self) -> bool {
        self.arm == TransitionArm::Synchronous
    }

    pub(in crate::app) fn skip_restore(&self) -> bool {
        self.arm == TransitionArm::SkipRestore
    }

    pub(in crate::app) fn fault_configure(&mut self, call: u32) -> bool {
        if call == 1
            && !self.faulted
            && matches!(
                self.arm,
                TransitionArm::Restore | TransitionArm::SkipRestore
            )
        {
            self.faulted = true;
            return true;
        }
        false
    }

    pub(in crate::app) fn build(&mut self, request: ExtentRequest) {
        self.build = Some(json!({
            "media": CACurrentMediaTime(),
            "backing": request.extent,
            "output_scale": request.output_scale.percent(),
            "extent": request.output_extent(),
        }));
    }

    pub(in crate::app) fn switch(
        &mut self,
        request: ExtentRequest,
        record: &SurfaceSwitchRecord,
        media_started: f64,
    ) {
        self.switch = Some(json!({
            "extent": request.output_extent(),
            "backing": request.extent,
            "output_scale": request.output_scale.percent(),
            "scale": f64::from_bits(request.backing_scale),
            "deferrals": record.deferrals,
            "configure_calls": record.configure_calls,
            "taken": record.generations_taken,
            "restores": if self.skip_restore() { 0 } else { record.restore_calls },
            "restored": if self.skip_restore() { None } else { record.restored },
            "restore_skipped": self.skip_restore(),
            "configure_error": record.configure_error,
            "restore_error": record.restore_error,
            "configure_started": media_started,
            "configure_ended": CACurrentMediaTime(),
            "configure_ms": record.configure_wall.as_secs_f64() * 1000.0,
            "restore_ms": record.restore_wall.as_secs_f64() * 1000.0,
        }));
    }

    pub(in crate::app) fn drawable(
        &mut self,
        texture: &wgpu::Texture,
    ) -> Result<bool, &'static str> {
        let size = [texture.width(), texture.height()];
        let metal = drawable_extent(texture).ok_or("acquired drawable has no Metal texture")?;
        self.drawable = Some((size, metal));
        Ok(self.skip_restore() && size != metal)
    }

    pub(in crate::app) fn display_record(
        &self,
        display: thrum_render::governor::DisplayRecord,
        tick: u64,
    ) -> thrum_render::governor::DisplayRecord {
        self.governor
            .map_or(display, |proof| proof.display_record(display, tick))
    }

    pub(in crate::app) fn frame<
        R,
        W: thrum_render::runtime::FrameWorld,
        V: thrum_render::runtime::SceneSource<W::Page>,
    >(
        &mut self,
        reply: &RenderReply<R>,
        request: OutputRequest,
        scale: u64,
        display: thrum_render::governor::DisplayRecord,
        host: &thrum_render::runtime::FrameHost<W, V>,
    ) -> io::Result<()> {
        if let Some(error) = self.configure_error.take() {
            return Err(error);
        }
        if let Some(path) = self.identity.take() {
            let caps = host.device_capabilities();
            let graph = host.scene_graph().map_err(io::Error::other)?;
            std::fs::write(path, json!({
                "adapter": caps.adapter().name,
                "recipe": format!("{:?}", caps.recipe()),
                "registry_digest": caps.registry_digest(),
                "reconstruction": format!("{:?}", host.reconstruction()),
                "registry_default_reconstructor": format!("{:?}", caps.reconstructor()),
                "storage": host.storage_report(),
                "graph_attachment_load_store_bytes": graph.declared_attachment_bytes().collect::<Vec<_>>(),
                "graph": format!("{graph:?}"),
            }).to_string())?;
        }
        self.stage("idle");
        self.sequence = None;
        let presented =
            matches!(&reply.outcome, RenderOutcome::Frame(Ok(report)) if report.surface_presented);
        let governor = self.governor.map(|_| {
            let samples = reply.samples.iter().flatten().map(|sample| json!({
                "submission": sample.submission_order,
                "tick": sample.tick, "alpha": sample.presentation_alpha,
                "observation_ns": sample.observation.as_nanos(),
                "render_extent": sample.render_extent,
                "gpu_ns": sample.gpu_pass_ns.as_ref().map(|gpu| gpu.sum() + sample.gpu_present_ns.unwrap_or_default()),
                "gpu_pass_ns": sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::values),
                "gpu_pass_order": sample.gpu_pass_ns.as_ref().map(|timing|
                    timing.passes().iter().map(|pass| pass.name.as_ref()).collect::<Vec<_>>()),
                "gpu_present_ns": sample.gpu_present_ns,
                "native_segment": sample.native_segment.as_ref().map(|segment| json!({
                    "pass": segment.pass_name(),
                    "outcome": format!("{:?}", segment.outcome()),
                })),
                "gpu_span_ns": sample.gpu_pass_ns.as_ref().and_then(thrum_render::timing::SceneGpuTimings::timed_pass_span_nanoseconds),
                "tick_cpu_ns": sample.tick_cpu.as_nanos(),
                "extract_cpu_ns": sample.extract_cpu.as_nanos(),
                "upload_cpu_ns": sample.upload_cpu.as_nanos(),
                "encode_cpu_ns": sample.encode_cpu.as_nanos(),
                "render_cpu_ns": sample.render_cpu.as_nanos(),
                "culling": format!("{:?}", sample.culling_mode),
                "debug_load_ns": sample.gpu_pass_ns.as_ref().and_then(|gpu| gpu.passes().iter().find(|pass| pass.name.ends_with("debug load")).map(|pass| pass.nanoseconds)),
                "quality": sample.governor.as_ref().map(governor_record),
            })).collect::<Vec<_>>();
            #[cfg(feature = "creation-counters")]
            let pipelines = {
                let report = host.creation_report();
                Some((report.pipelines.iter().map(|row| row.render + row.compute + row.mesh).sum::<u64>(), report.violations))
            };
            #[cfg(not(feature = "creation-counters"))]
            let pipelines: Option<(u64, u64)> = None;
            json!({
                "state": host.governor_frame().as_ref().map(governor_record),
                "platform_thermal": format!("{:?}", display.thermal),
                "missed_present_proxy": host.governor_missed_ticks(),
                "samples": samples,
                "pipelines": pipelines,
            })
        });
        let output = reply.platform.presented.map(|record| record.output);
        writeln!(
            self.output,
            "{}",
            json!({
                "sequence": reply.sequence,
                "window": request.backing,
                "requested_output": {
                    "extent": request.extent(),
                    "scale": request.scale.step.percent(),
                    "forced": request.scale.forced,
                },
                "backing_extent": output.map(|output| output.backing_extent),
                "output_extent": output.map(|output| output.output_extent),
                "output_scale": output.map(|output| output.output_scale.percent()),
                "output_scale_forced": output.map(|output| output.output_scale_forced),
                "scale": f64::from_bits(scale),
                "surface": reply.surface_extent,
                "presented": presented,
                "present_media": self.present_media.take(),
                "status": format!("{:?}", reply.platform.surface_status),
                "transition_error": reply.platform.transition_error,
                "submission": host.last_submission(),
                "world_tick": host.world().source().tick,
                "governor": governor,
                "wall_ms": reply.platform.worker_wall.as_secs_f64() * 1000.0,
                "work": std::mem::take(&mut self.work),
                "drawable": self.drawable.take(),
                "frame_extent": self.frame_extent.take().map(|frame| json!({
                    "submission": frame.submission,
                    "source": frame.source,
                    "destination": frame.destination,
                    "viewport": frame.viewport,
                    "reconstruction": frame.reconstruction,
                    "revision": frame.revision,
                    "attachment_extents": frame.attachments,
                })),
                "switch": self.switch.take(),
                "build": self.build.take(),
            })
        )?;
        self.output.flush()
    }
}

fn governor_record(frame: &thrum_render::runtime::GovernorFrame) -> Value {
    json!({
        "scale": frame.state.scale.percent(),
        "tier": frame.state.tier,
        "reason": format!("{:?}", frame.state.reason),
        "source": frame.source.map(|source| format!("{source:?}")),
        "budget_ns": frame.budget.as_nanos(),
        "display_interval_ns": frame.display.interval.as_nanos(),
        "thermal": format!("{:?}", frame.display.thermal),
        "held": frame.mark.held(),
        "aerial_age": frame.aerial_age,
        "chart_update": frame.chart_update,
        "chart_position": frame.chart_position,
        "knobs": frame.knobs.iter().map(|knob| (&knob.name, knob.value)).collect::<std::collections::BTreeMap<_, _>>(),
    })
}
