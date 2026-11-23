//! Mac drawable and configuration receipts for the forced display arms.
use std::{fs::File, io::Write as _};
use thrum_render::{
    runtime::{CaptureLimits, DriveReport, FrameHost, FrameWorld, SceneSource},
    synthetic::CaptureSource,
};

pub(in crate::app) struct DisplayProof {
    log: File,
    tap: bool,
    pub(in crate::app) configures: u64,
    submitted: u64,
    collected: u64,
}

impl DisplayProof {
    pub(in crate::app) fn start<W: FrameWorld, V: SceneSource<W::Page>>(
        host: &mut FrameHost<W, V>,
    ) -> Result<Option<Self>, String> {
        let Some(path) = std::env::var_os("THRUM_MAC_EDR_FRAMES") else {
            return Ok(None);
        };
        let tap = std::env::var_os("THRUM_MAC_EDR_TAP").is_some();
        if tap {
            host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::Edr)?;
        }
        Ok(Some(Self {
            log: File::create(path).map_err(|error| error.to_string())?,
            tap,
            configures: 1,
            submitted: 0,
            collected: 0,
        }))
    }

    pub(in crate::app) fn frame<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
        tick: u64,
        frame: &DriveReport,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<(), String> {
        self.submitted += u64::from(frame.surface_presented);
        writeln!(self.log, "{}", serde_json::json!({
            "tick": tick, "presented": frame.surface_presented, "submission": host.last_submission(),
            "headroom": host.display_headroom(), "format": format!("{:?}", config.format),
            "color_space": format!("{:?}", config.color_space), "configures": self.configures,
        })).map_err(|error| error.to_string())?;
        self.collect(host)
    }

    fn collect<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
    ) -> Result<(), String> {
        while self.tap
            && let Some(image) = host.try_submitted_image()?
        {
            let max = image.edr_max_rgb().ok_or("invalid EDR drawable pixels")?;
            writeln!(
                self.log,
                "{}",
                serde_json::json!({
                    "capture_submission": image.sample.submission_order, "max_rgb": max,
                    "presented": image.sample.surface_presented,
                    "presenter_generation": image.presenter_generation,
                })
            )
            .map_err(|error| error.to_string())?;
            self.collected += 1;
            host.recycle_submitted_image(image);
        }
        Ok(())
    }

    pub(in crate::app) fn finish<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
    ) -> Result<(), String> {
        if self.tap {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while self.collected < self.submitted {
                self.collect(host)?;
                if host.submitted_capture_omissions() != 0 || std::time::Instant::now() >= deadline
                {
                    return Err(format!(
                        "collected {} of {} EDR submissions, {} omitted",
                        self.collected,
                        self.submitted,
                        host.submitted_capture_omissions()
                    ));
                }
                std::thread::yield_now();
            }
        }
        #[cfg(feature = "creation-counters")]
        let report = Some(host.creation_report());
        #[cfg(not(feature = "creation-counters"))]
        let report: Option<thrum_render::runtime::CreationReport> = None;
        let report = report.ok_or("EDR qualification requires creation counters")?;
        let pipelines: u64 = report
            .pipelines
            .iter()
            .map(|p| p.render + p.compute + p.mesh)
            .sum();
        writeln!(
            self.log,
            "{}",
            serde_json::json!({
                "created_pipelines": pipelines, "violations": report.violations,
                "creation_report": format!("{report:?}"), "collected": self.collected,
                "submitted": self.submitted,
            })
        )
        .map_err(|error| error.to_string())
    }
}
