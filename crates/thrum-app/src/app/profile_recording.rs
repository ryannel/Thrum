//! Optional developer evidence sink. It observes the shell's samples and actual panel.
use std::{fs::OpenOptions, io::Write, path::PathBuf, time::Duration};
use thrum_controls::{Controls, macos::MacPanel};
use thrum_render::runtime::FrameSample;

pub(super) struct Recording {
    directory: PathBuf,
    saved: Duration,
}

impl Recording {
    pub(super) fn from_environment() -> Option<Self> {
        std::env::var_os("THRUM_PROFILE_OUTPUT").map(|path| Self {
            directory: path.into(),
            saved: Duration::ZERO,
        })
    }

    pub(super) fn observe(
        &mut self,
        now: Duration,
        samples: &[FrameSample],
        controls: &Controls,
        panel: Option<&MacPanel>,
    ) -> Result<(), String> {
        if now.saturating_sub(self.saved) < Duration::from_millis(250) {
            return Ok(());
        }
        self.saved = now;
        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        let frames: Vec<_> = samples.iter().rev().take(120).map(|s| serde_json::json!({
            "submission": s.submission_order, "timed": s.timing_requested,
            "tick_us": s.tick_cpu.as_secs_f64() * 1e6,
            "extract_us": s.extract_cpu.as_secs_f64() * 1e6,
            "encode_us": s.encode_cpu.as_secs_f64() * 1e6,
            "render_us": s.render_cpu.as_secs_f64() * 1e6,
            "gpu": s.gpu_pass_ns.as_ref().map(|p| p.passes().iter().map(|p| (p.name.to_string(), p.nanoseconds / 1e6)).collect::<Vec<_>>()),
        })).collect();
        let record = serde_json::json!({"elapsed":now.as_secs_f64(),"model":serde_json::from_str::<serde_json::Value>(&controls.describe_json()).map_err(|e| e.to_string())?,"frames":frames});
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.directory.join("samples.jsonl"))
            .map_err(|e| e.to_string())?;
        writeln!(log, "{record}").map_err(|e| e.to_string())?;
        let request = self.directory.join("capture");
        if let Ok(label) = std::fs::read_to_string(&request)
            && !label.is_empty()
            && label
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            let png = panel
                .ok_or("profiler recording needs a panel")?
                .render_png()?;
            std::fs::write(self.directory.join(format!("{label}.png")), png)
                .map_err(|e| e.to_string())?;
            std::fs::remove_file(request).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
