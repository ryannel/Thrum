//! Scratch-only display-link input and attribution for the M2 release diagnostic.

use std::{io::Write, path::PathBuf};

use thrum_controls::{Controls, Value, macos::MacPanel};

use super::pan_timeline::{PanAction, PanTimeline};

// The isolated C functions take no arguments and have no caller preconditions.
#[expect(unsafe_code, reason = "diagnostic-only bridge to the public Apple signpost macro")]
unsafe extern "C" {
    safe fn thrum_pan_start() -> u64;
    safe fn thrum_pan_end() -> u64;
}

struct Row {
    kind: &'static str,
    sequence: u64,
    target: f64,
    value: f64,
}

pub(super) struct Witness {
    output: PathBuf,
    arm: PathBuf,
    timeline: Option<PanTimeline>,
    active: bool,
    begun: bool,
    ended: bool,
    rows: Vec<Row>,
}

impl Witness {
    pub(super) fn from_env() -> Option<Self> {
        let output = PathBuf::from(std::env::var_os("THRUM_M2_DIR")?);
        Some(Self {
            arm: output.join("arm"),
            output,
            timeline: None,
            active: false,
            begun: false,
            ended: false,
            rows: Vec::with_capacity(2_048),
        })
    }

    pub(super) fn sample(
        &mut self,
        target: f64,
        sequence: u64,
        panel: Option<&MacPanel>,
        controls: &Controls,
    ) -> Result<(), String> {
        if self.ended {
            return Ok(());
        }
        if self.timeline.is_none() {
            if !self.arm.is_file() {
                return Ok(());
            }
            self.timeline = Some(PanTimeline::armed(target)?);
        }
        let action = self
            .timeline
            .as_mut()
            .ok_or("pan timeline unavailable")?
            .sample(target)?;
        let yaw = match action {
            PanAction::Hold => return Ok(()),
            PanAction::Begin => {
                self.active = true;
                self.begun = true;
                let mach = thrum_pan_start();
                eprintln!("m2 pan start mach={mach} sequence={sequence}");
                return Ok(());
            }
            PanAction::Move(yaw) | PanAction::End(yaw) => yaw,
        };
        let id = controls.find("camera.yaw").ok_or("missing yaw control")?;
        if !panel.ok_or("missing Mac panel")?.simulate(id, yaw) {
            return Err("Mac panel refused yaw injection".into());
        }
        self.record("action", sequence, target, yaw)?;
        if matches!(action, PanAction::End(_)) {
            self.ended = true;
            let mach = thrum_pan_end();
            eprintln!("m2 pan end mach={mach} sequence={sequence}");
            std::fs::write(self.output.join("end"), b"").map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(super) fn accepted(&mut self, sequence: u64, controls: &Controls) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        let id = controls.find("camera.yaw").ok_or("missing yaw control")?;
        let Value::Number(yaw) = controls.value(id) else {
            return Err("yaw control is not a number".into());
        };
        self.record("accepted", sequence, 0.0, f64::from(yaw.to_degrees()))
    }

    pub(super) fn presented(&mut self, sequence: u64, yaw: f32) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        self.record("present-called", sequence, 0.0, f64::from(yaw.to_degrees()))
    }

    fn record(
        &mut self,
        kind: &'static str,
        sequence: u64,
        target: f64,
        value: f64,
    ) -> Result<(), String> {
        if self.rows.len() == self.rows.capacity() {
            return Err("diagnostic pan log capacity reached".into());
        }
        self.rows.push(Row {
            kind,
            sequence,
            target,
            value,
        });
        Ok(())
    }

    pub(super) fn should_finish(&self) -> bool {
        self.output.join("finish").is_file()
    }

    pub(super) fn finish(&mut self) -> Result<(), String> {
        if !(self.begun && self.ended) {
            return Err("diagnostic pan did not complete".into());
        }
        self.active = false;
        let file = std::fs::File::create_new(self.output.join("pan.csv"))
            .map_err(|error| error.to_string())?;
        let mut out = std::io::BufWriter::new(file);
        writeln!(out, "kind,sequence,display_target_seconds,yaw_degrees")
            .map_err(|error| error.to_string())?;
        for row in &self.rows {
            writeln!(
                out,
                "{},{},{:.9},{:.6}",
                row.kind, row.sequence, row.target, row.value
            )
            .map_err(|error| error.to_string())?;
        }
        out.flush().map_err(|error| error.to_string())
    }
}
