//! Opt-in observations of the ordinary Mac render owner's surface changes.

use super::super::render_actor::{RenderOutcome, RenderReply};
use objc2_quartz_core::CACurrentMediaTime;
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};
use thrum_render::{
    capabilities::drawable_extent,
    runtime::{ExtentRequest, SurfaceSwitchRecord},
};

/// Arm of the ordinary window resize proof.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TransitionArm {
    /// Keep the window at its original size.
    Baseline,
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
    pub(in crate::app) fn resizes(self) -> bool {
        self != Self::Baseline
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
        media: f64,
        render_busy: bool,
    ) -> io::Result<()> {
        writeln!(
            self.0,
            "{}",
            json!({"sequence": sequence, "media": media, "render_busy": render_busy})
        )?;
        self.0.flush()
    }
}

pub(in crate::app) struct TransitionProof {
    output: BufWriter<File>,
    arm: TransitionArm,
    faulted: bool,
    drawable: Option<([u32; 2], [u32; 2])>,
    switch: Option<Value>,
}

impl TransitionProof {
    pub(in crate::app) fn start(options: &WindowTransitionProof) -> Result<Self, String> {
        let file =
            File::create(options.output.join("frames.jsonl")).map_err(|error| error.to_string())?;
        Ok(Self {
            output: BufWriter::new(file),
            arm: options.arm,
            faulted: false,
            drawable: None,
            switch: None,
        })
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

    pub(in crate::app) fn switch(
        &mut self,
        request: ExtentRequest,
        record: &SurfaceSwitchRecord,
        media_started: f64,
    ) {
        self.switch = Some(json!({
            "extent": request.extent,
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

    pub(in crate::app) fn frame<R>(
        &mut self,
        reply: &RenderReply<R>,
        extent: [u32; 2],
        scale: u64,
        submission: Option<u64>,
    ) -> io::Result<()> {
        let presented =
            matches!(&reply.outcome, RenderOutcome::Frame(Ok(report)) if report.surface_presented);
        writeln!(
            self.output,
            "{}",
            json!({
                "sequence": reply.sequence,
                "window": extent,
                "scale": f64::from_bits(scale),
                "surface": reply.surface_extent,
                "presented": presented,
                "status": format!("{:?}", reply.surface_status),
            "transition_error": reply.transition_error,
                "submission": submission,
                "wall_ms": reply.worker_wall.as_secs_f64() * 1000.0,
                "drawable": self.drawable.take(),
                "switch": self.switch.take(),
            })
        )?;
        self.output.flush()
    }
}
