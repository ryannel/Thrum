//! Opt-in Mac qualification observer; ordinary presentation remains the only producer.

use crate::{
    Game,
    offscreen::writer::{ImageWriter, SpoolEncoding},
};
use objc2_quartz_core::CACurrentMediaTime;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thrum_controls::{Controls, Value, macos::MacPanel};
use thrum_render::runtime::{
    CaptureLimits, FrameError, FrameExtent, FrameHost, FrameWorld, SceneSource, SubmittedImage,
};

const RECORDS: usize = 2048;

/// One identified submission from the ordinary Mac window.
pub(super) struct ExtentCapture {
    output: PathBuf,
    frame: Option<(FrameExtent, [u32; 2])>,
}

/// Requested AppKit bounds for a one-submission extent proof.
#[derive(Clone, Copy)]
pub enum ExtentWindow {
    /// Content size in logical points.
    Points([u32; 2]),
    /// Fullscreen on the window's monitor.
    Fullscreen,
}

/// Inputs for one window extent capture.
pub struct WindowExtentCapture {
    /// Window size to prepare.
    pub window: ExtentWindow,
    /// Existing output directory for the image and its extent witnesses.
    pub output: PathBuf,
}

impl WindowExtentCapture {
    pub(super) fn attributes(&self) -> winit::window::WindowAttributes {
        let attributes = winit::window::Window::default_attributes();
        match self.window {
            ExtentWindow::Points([width, height]) => {
                attributes.with_inner_size(winit::dpi::LogicalSize::new(width, height))
            }
            ExtentWindow::Fullscreen => attributes,
        }
    }

    pub(super) fn prepare_window(&self, window: &winit::window::Window) -> Result<(), String> {
        use winit::platform::macos::WindowExtMacOS;
        match self.window {
            ExtentWindow::Points([width, height]) => {
                // An oversized qualification window can extend past the monitor.
                // AppKit otherwise clamps its height to the visible frame.
                let points = winit::dpi::LogicalSize::new(width, height);
                window.set_min_inner_size(Some(points));
                let _ = window.request_inner_size(points);
            }
            ExtentWindow::Fullscreen => {
                // Establish the fullscreen backing before preparing the surface and host.
                window.set_borderless_game(true);
                if !window.set_simple_fullscreen(true) {
                    return Err("could not enter fullscreen for the extent proof".to_owned());
                }
            }
        }
        let backing = window.inner_size();
        let scale = window.scale_factor();
        let logical = backing.to_logical::<f64>(scale);
        let monitor = window.current_monitor().map(|monitor| {
            let size = monitor.size();
            [size.width, size.height]
        });
        let record = serde_json::json!({
            "backing": [backing.width, backing.height],
            "points": [logical.width, logical.height],
            "scale": scale,
            "fullscreen": window.simple_fullscreen(),
            "monitor_backing": monitor,
        });
        std::fs::write(self.output.join("window.json"), record.to_string())
            .map_err(|error| error.to_string())
    }
}

impl ExtentCapture {
    pub(super) fn start<W: FrameWorld, V: SceneSource<W::Page>>(
        host: &mut FrameHost<W, V>,
        output: PathBuf,
    ) -> Result<Self, String> {
        host.enable_submitted_capture_with_limits(
            1,
            CaptureLimits {
                staging_slots: 2,
                cpu_buffers: 2,
                byte_allowance: 256 * 1024 * 1024,
            },
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            output,
            frame: None,
        })
    }

    pub(super) fn frame(&mut self, frame: FrameExtent, drawable: [u32; 2]) {
        self.frame.get_or_insert((frame, drawable));
    }

    pub(super) fn collect<W: FrameWorld, V: SceneSource<W::Page>>(
        &self,
        host: &mut FrameHost<W, V>,
        recipe: &str,
    ) -> Result<bool, FrameError> {
        let Some((frame, drawable)) = self.frame else {
            return Ok(false);
        };
        while let Some(image) = host.try_submitted_image()? {
            if image.sample.surface_presented && image.sample.submission_order == frame.submission {
                let record = serde_json::json!({
                    "mode": "still", "recipe": recipe, "submission": frame.submission,
                    "capture_extent": image.extent, "render_extent": image.sample.render_extent,
                    "attachment_extents": frame.attachments, "source": frame.source,
                    "destination": frame.destination, "drawable": drawable, "viewport": frame.viewport,
                    "device_generation": image.device_generation,
                });
                std::fs::write(self.output.join("submitted.rgba"), &image.rgba)
                    .map_err(|error| error.to_string())?;
                std::fs::write(self.output.join("submitted.json"), record.to_string())
                    .map_err(|error| error.to_string())?;
                host.disable_submitted_capture();
                return Ok(true);
            }
            host.recycle_submitted_image(image);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests;

pub(super) struct Pan {
    output: PathBuf,
    start: Option<Instant>,
    rows: Vec<(u128, f64, f64, f64)>,
    requests: Vec<(u64, f64, f64, f64)>,
    signposts: Vec<(u32, f64, f64)>,
}
impl Pan {
    pub(super) fn from_environment() -> Option<Self> {
        std::env::var_os("THRUM_MAC_QUALIFY").map(|output| Self {
            output: output.into(),
            start: None,
            rows: Vec::with_capacity(RECORDS),
            requests: Vec::with_capacity(RECORDS),
            signposts: Vec::with_capacity(4),
        })
    }
    pub(super) fn step(
        &mut self,
        panel: &mut MacPanel,
        controls: &mut Controls,
    ) -> Result<bool, String> {
        let now = Instant::now();
        let start = *self.start.get_or_insert(now);
        let elapsed = now.duration_since(start).as_secs_f64();
        let boundaries = [3.0, 4.0, 6.0, 7.1];
        while self.signposts.len() < boundaries.len() && elapsed >= boundaries[self.signposts.len()]
        {
            let stage = u32::try_from(self.signposts.len()).expect("four signpost stages");
            let before = CACurrentMediaTime();
            super::qualification_signpost::mark(stage);
            self.signposts.push((stage, before, CACurrentMediaTime()));
        }
        // Warm up for three seconds, then one-second holds around the two-second pan.
        let yaw = ((elapsed - 4.0) / 2.0).clamp(0.0, 1.0) * 36.0;
        let id = controls.lookup("camera.yaw").map_err(|e| e.to_string())?;
        if !panel.simulate(id, yaw) {
            return Err("yaw slider action unavailable".to_owned());
        }
        panel.apply_edits(controls);
        if !panel.take_refused().is_empty() {
            return Err("AppKit yaw action refused".to_owned());
        }
        let Value::Number(accepted) = controls.value(id) else {
            return Err("yaw is not numeric".to_owned());
        };
        if self.rows.len() == RECORDS {
            return Err("pan marker capacity exhausted".to_owned());
        }
        self.rows.push((
            unix_ns()?,
            CACurrentMediaTime(),
            yaw,
            f64::from(accepted.to_degrees()),
        ));
        Ok(elapsed >= 7.1)
    }
    pub(super) fn request(
        &mut self,
        sequence: u64,
        target: f64,
        controls: &Controls,
    ) -> Result<(), String> {
        let id = controls.lookup("camera.yaw").map_err(|e| e.to_string())?;
        let Value::Number(yaw) = controls.value(id) else {
            return Err("yaw is not numeric".to_owned());
        };
        if self.requests.len() == RECORDS {
            return Err("request marker capacity exhausted".to_owned());
        }
        self.requests.push((
            sequence,
            CACurrentMediaTime(),
            target,
            f64::from(yaw.to_degrees()),
        ));
        Ok(())
    }
    pub(super) fn finish(&self) -> Result<(), String> {
        let mut markers = BufWriter::new(
            File::create(self.output.join("signposts.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(markers, "stage\tmedia_before\tmedia_after").map_err(|e| e.to_string())?;
        for (stage, before, after) in &self.signposts {
            writeln!(markers, "{stage}\t{before:.9}\t{after:.9}").map_err(|e| e.to_string())?;
        }
        markers.flush().map_err(|e| e.to_string())?;
        let mut file = BufWriter::new(
            File::create(self.output.join("controls.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(
            file,
            "unix_ns\tmedia_seconds\trequested_yaw_degrees\taccepted_yaw_degrees"
        )
        .map_err(|e| e.to_string())?;
        for (unix, media, requested, accepted) in &self.rows {
            writeln!(file, "{unix}\t{media:.9}\t{requested:.9}\t{accepted:.9}")
                .map_err(|e| e.to_string())?;
        }
        file.flush().map_err(|e| e.to_string())?;
        let mut file = BufWriter::new(
            File::create(self.output.join("requests.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(
            file,
            "sequence\tmedia_seconds\ttarget_media_seconds\taccepted_yaw_degrees"
        )
        .map_err(|e| e.to_string())?;
        for (seq, media, target, yaw) in &self.requests {
            writeln!(file, "{seq}\t{media:.9}\t{target:.9}\t{yaw:.9}")
                .map_err(|e| e.to_string())?;
        }
        file.flush().map_err(|e| e.to_string())
    }
}

type ImageMarker = (u64, u128, f64, u64, u64, u64, u64, f32);

pub(super) struct Observer {
    output: PathBuf,
    writer: Option<ImageWriter>,
    images: Vec<ImageMarker>,
    frames: Vec<(u64, u128, f64, u64, u64, bool, bool)>,
    refused: u64,
    origin: Instant,
    replace_surface: bool,
    replacement_pending: bool,
    session: u64,
    retired: usize,
    retirements: Vec<(u64, usize, usize, u128)>,
    stale_image: Option<SubmittedImage>,
    stale_recycle: Option<(usize, usize)>,
}
impl Observer {
    pub(super) fn start<W: FrameWorld, V: SceneSource<W::Page>>(
        host: &mut FrameHost<W, V>,
        origin: Instant,
    ) -> Result<Option<Self>, String> {
        let Some(path) = std::env::var_os("THRUM_MAC_QUALIFY") else {
            return Ok(None);
        };
        let output = PathBuf::from(path);
        let before = CACurrentMediaTime();
        let origin_elapsed = origin.elapsed().as_nanos();
        let unix = unix_ns()?;
        let after = CACurrentMediaTime();
        std::fs::write(output.join("clock-anchor.tsv"), format!("pid\torigin_elapsed_ns\tunix_ns\tmedia_before\tmedia_after\n{}\t{origin_elapsed}\t{unix}\t{before:.9}\t{after:.9}\n", std::process::id())).map_err(|e| e.to_string())?;
        let writer = match std::env::var("THRUM_MAC_TAP").as_deref() {
            Ok("on") => {
                host.enable_submitted_capture_with_limits(
                    1,
                    CaptureLimits {
                        staging_slots: 6,
                        cpu_buffers: 96,
                        byte_allowance: 1024 * 1024 * 1024,
                    },
                )
                .map_err(|e| e.to_string())?;
                let file = File::create(output.join("frames.zst")).map_err(|e| e.to_string())?;
                Some(ImageWriter::start(
                    BufWriter::with_capacity(4 * 1024 * 1024, file),
                    96,
                    8 * 1024 * 1024 * 1024,
                    SpoolEncoding::ZstdDelta,
                )?)
            }
            Ok("off") => None,
            _ => return Err("qualification requires THRUM_MAC_TAP=on or off".to_owned()),
        };
        Ok(Some(Self {
            output,
            writer,
            images: Vec::with_capacity(RECORDS),
            frames: Vec::with_capacity(RECORDS),
            refused: 0,
            origin,
            replace_surface: std::env::var("THRUM_MAC_REPLACE").as_deref() == Ok("1"),
            replacement_pending: false,
            session: 1,
            retired: 0,
            retirements: Vec::with_capacity(1),
            stale_image: None,
            stale_recycle: None,
        }))
    }
    pub(super) fn frame<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &FrameHost<W, V>,
        sequence: u64,
        surface_generation: u64,
        presented: bool,
    ) -> Result<(), String> {
        if self.frames.len() == RECORDS {
            return Err("frame marker capacity exhausted".to_owned());
        }
        self.frames.push((
            sequence,
            unix_ns()?,
            CACurrentMediaTime(),
            host.last_submission().ok_or("missing submission")?,
            surface_generation,
            presented,
            host.submitted_capture_usage().is_some(),
        ));
        Ok(())
    }
    pub(super) fn replace_after_present(&mut self) -> bool {
        if self.replace_surface && self.frames.len() == 360 && self.session == 1 {
            self.replacement_pending = true;
            return true;
        }
        false
    }
    pub(super) fn replace<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
    ) -> Result<(), FrameError> {
        if self.writer.is_none() {
            return Ok(());
        }
        self.retire_tap(host)?;
        self.stale_image = host.try_submitted_image()?;
        self.restart_tap(host)
    }
    /// A published device abandons old tap mailboxes without polling the old queue.
    pub(super) fn replace_device<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
    ) -> Result<(), FrameError> {
        if self.writer.is_none() {
            return Ok(());
        }
        self.retire_tap(host)?;
        self.stale_image = None;
        self.restart_tap(host)
    }
    fn retire_tap<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &FrameHost<W, V>,
    ) -> Result<(), FrameError> {
        let (pending, leases) = host
            .submitted_capture_usage()
            .ok_or(FrameError::InvalidState("missing old tap"))?;
        self.retirements.push((
            self.session,
            pending,
            leases,
            self.origin.elapsed().as_nanos(),
        ));
        self.retired = self.frames.len() - self.images.len();
        Ok(())
    }
    fn restart_tap<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
    ) -> Result<(), FrameError> {
        host.disable_submitted_capture();
        self.session += 1;
        host.enable_submitted_capture_with_limits(
            self.session,
            CaptureLimits {
                staging_slots: 6,
                cpu_buffers: 96,
                byte_allowance: 1024 * 1024 * 1024,
            },
        )?;
        self.replacement_pending = false;
        Ok(())
    }
    pub(super) fn collect<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
    ) -> Result<(), FrameError> {
        if self.replacement_pending {
            return Ok(());
        }
        let Some(writer) = &self.writer else {
            return Ok(());
        };
        while let Some(image) = writer.recycled() {
            host.recycle_submitted_image(image);
        }
        while let Some(image) = host.try_submitted_image()? {
            if self.images.len() == RECORDS {
                return Err(FrameError::InvalidState("image marker capacity exhausted"));
            }
            if let Some(stale) = self.stale_image.take() {
                let before = host
                    .submitted_capture_usage()
                    .ok_or(FrameError::InvalidState("fresh tap missing"))?
                    .1;
                host.recycle_submitted_image(stale);
                let after = host
                    .submitted_capture_usage()
                    .ok_or(FrameError::InvalidState("fresh tap missing"))?
                    .1;
                self.stale_recycle = Some((before, after));
                if before == 0 || before != after {
                    return Err(FrameError::InvalidState(
                        "stale CPU image entered the replacement tap",
                    ));
                }
            }
            self.images.push((
                image.sample.submission_order,
                image
                    .submitted_at
                    .saturating_duration_since(self.origin)
                    .as_nanos(),
                CACurrentMediaTime(),
                image.device_generation,
                image.presenter_generation,
                image.source.tick,
                image.history_epoch,
                image.sample.presentation_alpha,
            ));
            if let Some(image) = writer.submit(image) {
                self.refused += 1;
                host.recycle_submitted_image(image);
            }
        }
        Ok(())
    }
    pub(super) fn finish<G: Game>(
        &mut self,
        host: &mut FrameHost<G::World, G::Scene>,
    ) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.writer.is_some()
            && self.images.len() + self.retired < self.frames.len()
            && Instant::now() < deadline
        {
            host.poll_completed().map_err(|e| e.to_string())?;
            self.collect(host).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(1));
        }
        let mut file = BufWriter::new(
            File::create(self.output.join("surface.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(file,"sequence\tunix_ns\tmedia_seconds\tsubmission_id\tlease_generation\tpresented\ttap_enabled").map_err(|e| e.to_string())?;
        for (seq, unix, media, id, generation, presented, tap) in &self.frames {
            writeln!(
                file,
                "{seq}\t{unix}\t{media:.9}\t{id}\t{generation}\t{presented}\t{tap}"
            )
            .map_err(|e| e.to_string())?;
        }
        file.flush().map_err(|e| e.to_string())?;
        let mut file = BufWriter::new(
            File::create(self.output.join("images.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(file,"submission_id\tsubmit_origin_ns\tobserved_media_seconds\tdevice_generation\tpresenter_generation\ttick\thistory_epoch\talpha").map_err(|e| e.to_string())?;
        for (id, unix, media, device, presenter, tick, history, alpha) in &self.images {
            writeln!(
                file,
                "{id}\t{unix}\t{media:.9}\t{device}\t{presenter}\t{tick}\t{history}\t{alpha}"
            )
            .map_err(|e| e.to_string())?;
        }
        file.flush().map_err(|e| e.to_string())?;
        let mut retirement = BufWriter::new(
            File::create(self.output.join("retirements.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(
            retirement,
            "session\tpending_staging\toutstanding_cpu\torigin_ns"
        )
        .map_err(|e| e.to_string())?;
        for (session, pending, leases, time) in &self.retirements {
            writeln!(retirement, "{session}\t{pending}\t{leases}\t{time}")
                .map_err(|e| e.to_string())?;
        }
        retirement.flush().map_err(|e| e.to_string())?;
        std::fs::write(
            self.output.join("stale-recycle.txt"),
            format!("{:?}\n", self.stale_recycle),
        )
        .map_err(|e| e.to_string())?;
        if self.replace_surface
            && (self.retirements.len() != 1
                || self.retirements[0].1 == 0
                || self.stale_recycle.is_none())
        {
            return Err("surface replacement did not retain a pending readback".to_owned());
        }
        if let Some(writer) = self.writer.take() {
            let report = writer.finish(Duration::from_secs(10))?;
            std::fs::write(
                self.output.join("hashes.txt"),
                report.frame_hashes.join("\n"),
            )
            .map_err(|e| e.to_string())?;
            if usize::try_from(report.frames).ok() != Some(self.frames.len() - self.retired)
                || self.refused != 0
                || host.submitted_capture_omissions() != 0
            {
                return Err("incomplete real-window capture".to_owned());
            }
        }
        Ok(())
    }
}
fn unix_ns() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .map_err(|e| e.to_string())
}
