//! Mac drawable and configuration receipts for the forced display arms.
use std::{fs::File, io::Write as _};
use thrum_render::runtime::{DriveReport, FrameHost, FrameWorld, SceneSource};

pub(in crate::app) struct DisplayProof {
    log: File,
    tap: bool,
    first_pixels: Option<std::path::PathBuf>,
    pub(in crate::app) configures: u64,
    submitted: u64,
    collected: u64,
}

impl DisplayProof {
    pub(in crate::app) fn start<W: FrameWorld, V: SceneSource<W::Page>>(
        host: &mut FrameHost<W, V>,
        options: Option<&super::WindowDisplayOptions>,
    ) -> Result<Option<Self>, String> {
        let Some((options, path)) =
            options.and_then(|options| options.output.as_ref().map(|path| (options, path)))
        else {
            return Ok(None);
        };
        let tap = options.capture;
        if tap {
            host.enable_submitted_capture_with_limits(1, options.capture_limits)?;
        }
        Ok(Some(Self {
            log: File::create(path.join("frames.jsonl")).map_err(|error| error.to_string())?,
            tap,
            first_pixels: options
                .capture_first_pixels
                .then(|| path.join("submitted.rgba")),
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
            "scene_kind": format!("{:?}", host.scene_kind()),
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
            let max = if image.pixel_source == thrum_render::synthetic::CaptureSource::Edr {
                Some(image.edr_max_rgb().ok_or("invalid EDR drawable pixels")?)
            } else {
                None
            };
            if image.sample.surface_presented
                && let Some(path) = self.first_pixels.take()
            {
                std::fs::write(path, &image.rgba).map_err(|error| error.to_string())?;
            }
            writeln!(
                self.log,
                "{}",
                serde_json::json!({
                    "capture_submission": image.sample.submission_order, "max_rgb": max,
                    "capture_source": format!("{:?}", image.pixel_source),
                    "capture_session": image.session, "pixel_bytes": image.rgba.len(),
                    "extent": image.extent,
                    "encoding": if max.is_some() { "rgba16float-le" } else { "rgba8-srgb" },
                    "presented": image.sample.surface_presented,
                    "presenter_generation": image.presenter_generation,
                })
            )
            .map_err(|error| error.to_string())?;
            self.collected += u64::from(image.sample.surface_presented);
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
                drop(host.poll_completed()?);
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
        {
            self.write_creation_receipt(host)
        }
        #[cfg(not(feature = "creation-counters"))]
        {
            Err("EDR qualification requires creation counters".to_owned())
        }
    }

    #[cfg(feature = "creation-counters")]
    fn write_creation_receipt<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &FrameHost<W, V>,
    ) -> Result<(), String> {
        let report = host.creation_report();
        let pipelines: u64 = report
            .pipelines
            .iter()
            .map(|p| p.render + p.compute + p.mesh)
            .sum();
        let owner = std::thread::current();
        let owner = owner.name().unwrap_or_default();
        let capture_pipelines: u64 = report
            .pipelines
            .iter()
            .filter(|row| row.thread == owner && row.label == "capture-copy-timing")
            .map(|row| row.render + row.compute + row.mesh)
            .sum();
        let (capture_groups, capture_queries) = report
            .rows
            .iter()
            .filter(|row| row.thread == owner && row.label == "capture-copy-timing")
            .fold((0, 0), |(groups, queries), row| {
                (
                    groups + row.counts.bind_groups.max(0),
                    queries + row.counts.query_sets.max(0),
                )
            });
        writeln!(
            self.log,
            "{}",
            serde_json::json!({
                "created_pipelines": pipelines, "violations": report.violations,
                "creation_report": format!("{report:?}"), "collected": self.collected,
                "submitted": self.submitted,
                "render_owner": owner, "render_owner_capture_pipelines": capture_pipelines,
                "render_owner_capture_bind_groups": capture_groups,
                "render_owner_capture_query_sets": capture_queries,
            })
        )
        .map_err(|error| error.to_string())
    }
}
