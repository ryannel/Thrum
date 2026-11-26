//! Opt-in Mac qualification observer; ordinary presentation remains the only producer.

mod transition;
pub use transition::{GovernorProof, TransitionArm, WindowTransitionProof};
pub(super) use transition::{TransitionProof, TransitionTicks};

use crate::{
    Game,
    offscreen::writer::{ImageWriter, SpoolEncoding},
};
use objc2_quartz_core::CACurrentMediaTime;
use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thrum_controls::{Controls, Value, macos::MacPanel};
use thrum_render::runtime::{
    CaptureLimits, FrameError, FrameExtent, FrameHost, FrameSample, FrameWorld, SceneSource,
    SubmittedImage,
};
use thrum_render::synthetic::CaptureSource;

const RECORDS: usize = 4096;

/// One identified submission from the ordinary Mac window.
pub(super) struct ExtentCapture {
    output: PathBuf,
    frames: VecDeque<(FrameExtent, [u32; 2])>,
    status: bool,
    resize: Option<[u32; 2]>,
    resized_extent: Option<[u32; 2]>,
    first_resized: Option<u64>,
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
    /// Wait until the extent capture can see a resident interface publication.
    pub status: bool,
    /// Optional second window size, requested after the first image reaches the capture.
    pub resize: Option<[u32; 2]>,
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
        #[cfg(feature = "interface")]
        let insets = super::display::safe_area(window);
        #[cfg(not(feature = "interface"))]
        let insets = [0.0; 4];
        let record = serde_json::json!({
            "backing": [backing.width, backing.height],
            "points": [logical.width, logical.height],
            "scale": scale,
            "insets": insets,
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
        capture: WindowExtentCapture,
        backing_scale: f64,
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
        let resized_extent = capture.resize.map(|extent| {
            let scale = winit::dpi::LogicalSize::new(extent[0], extent[1]);
            let size = scale.to_physical::<u32>(backing_scale);
            [size.width, size.height]
        });
        Ok(Self {
            output: capture.output,
            frames: VecDeque::new(),
            status: capture.status,
            resize: capture.resize,
            resized_extent,
            first_resized: None,
        })
    }

    pub(super) fn frame(&mut self, frame: FrameExtent, drawable: [u32; 2]) {
        if self.frames.len() == 64 {
            self.frames.pop_front();
        }
        if self.resize.is_none() && self.resized_extent == Some(drawable) {
            self.first_resized.get_or_insert(frame.submission);
        }
        self.frames.push_back((frame, drawable));
    }

    pub(super) fn collect<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
        recipe: &str,
        wake: &winit::event_loop::EventLoopProxy<super::UserEvent>,
    ) -> Result<bool, FrameError> {
        while let Some(image) = host.try_submitted_image()? {
            let order = image.sample.submission_order;
            while self
                .frames
                .front()
                .is_some_and(|(frame, _)| frame.submission < order)
            {
                self.frames.pop_front();
            }
            let frame = self
                .frames
                .front()
                .filter(|(frame, _)| frame.submission == order)
                .copied();
            let paint = image.sample.interface.and_then(|interface| interface.stamp);
            let resized = self.resized_extent.is_some() && self.resize.is_none();
            let matches_extent = !resized || self.resized_extent == Some(image.extent);
            if let Some((frame, drawable)) = frame
                && image.sample.surface_presented
                && matches_extent
                && (!self.status || paint.is_some())
            {
                if resized && self.first_resized != Some(order) {
                    return Err(FrameError::InvalidState(
                        "the first resized submission was omitted",
                    ));
                }
                let record = serde_json::json!({
                    "mode": "still", "first_resized_submission": self.first_resized, "recipe": recipe, "submission": frame.submission,
                    "tick": image.sample.tick, "paint_scale": paint.map(|stamp| stamp.scale),
                    "capture_extent": image.extent, "render_extent": image.sample.render_extent,
                    "attachment_extents": frame.attachments, "source": frame.source,
                    "destination": frame.destination, "drawable": drawable, "viewport": frame.viewport,
                    "device_generation": image.device_generation,
                });
                let stem = if resized { "resized" } else { "submitted" };
                std::fs::write(self.output.join(format!("{stem}.rgba")), &image.rgba)
                    .map_err(|error| FrameError::Scene(error.to_string()))?;
                std::fs::write(self.output.join(format!("{stem}.json")), record.to_string())
                    .map_err(|error| FrameError::Scene(error.to_string()))?;
                if let Some(next) = self.resize.take() {
                    wake.send_event(super::UserEvent::ExtentResize(next))
                        .map_err(|error| FrameError::Scene(error.to_string()))?;
                } else {
                    host.disable_submitted_capture();
                    return Ok(true);
                }
            }
            host.recycle_submitted_image(image);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests;

/// An opt-in observation of the ordinary window and render owner.
pub struct WindowQualification {
    /// Directory for the control, window and submitted-frame records.
    pub output: PathBuf,
    /// The game-declared numeric control to drive through the panel.
    pub control: &'static str,
    /// Name of the display-unit column in the qualification records.
    pub control_column: &'static str,
    /// Signpost boundaries in elapsed seconds; the last ends the run.
    pub boundaries: [f64; 4],
    /// Measure refreshes for this duration starting at the second signpost.
    /// The outer signposts calibrate the witness and leave room for bracketing submissions.
    pub refresh_measurement: Option<Duration>,
    /// Control value in display units at the given elapsed seconds.
    /// With no sample function, observe the control without driving the panel.
    pub sample: Option<fn(f64) -> f64>,
    /// Translate an accepted model value to the sweep's display units.
    pub accepted: fn(f32) -> f64,
    /// Record submitted images alongside submission records.
    pub capture: bool,
    /// Request per-pass GPU timestamps in addition to the window's normal timing policy.
    pub gpu_timestamps: bool,
    /// Exercise replacement with a readback pending.
    pub replace_surface: bool,
}

pub(super) struct Pan {
    pub(super) options: WindowQualification,
    start: Option<Instant>,
    rows: Vec<(u128, f64, f64, f64)>,
    requests: Vec<(u64, f64, f64, f64)>,
    signposts: Vec<(u32, f64, f64)>,
}
impl Pan {
    pub(super) fn new(options: WindowQualification) -> Self {
        Self {
            options,
            start: None,
            rows: Vec::with_capacity(RECORDS),
            requests: Vec::with_capacity(RECORDS),
            signposts: Vec::with_capacity(4),
        }
    }

    pub(super) fn measured_interval(&self) -> Option<(f64, f64)> {
        let length = self.options.refresh_measurement?.as_secs_f64();
        self.signposts
            .get(1)
            .map(|&(_, before, after)| (f64::midpoint(before, after), length))
    }

    pub(super) fn step(
        &mut self,
        panel: &mut MacPanel,
        controls: &mut Controls,
    ) -> Result<bool, String> {
        let now = Instant::now();
        let start = *self.start.get_or_insert(now);
        let elapsed = now.duration_since(start).as_secs_f64();
        let boundaries = self.options.boundaries;
        while self.signposts.len() < boundaries.len() && elapsed >= boundaries[self.signposts.len()]
        {
            let stage = u32::try_from(self.signposts.len()).expect("four signpost stages");
            let before = CACurrentMediaTime();
            super::qualification_signpost::mark(stage);
            self.signposts.push((stage, before, CACurrentMediaTime()));
        }
        let id = controls
            .lookup(self.options.control)
            .map_err(|e| e.to_string())?;
        let requested = if let Some(sample) = self.options.sample {
            let requested = sample(elapsed);
            if !panel.simulate(id, requested) {
                return Err("qualification slider action unavailable".to_owned());
            }
            panel.apply_edits(controls);
            if !panel.take_refused().is_empty() {
                return Err("AppKit qualification action refused".to_owned());
            }
            Some(requested)
        } else {
            None
        };
        let Value::Number(accepted) = controls.value(id) else {
            return Err("qualification control is not numeric".to_owned());
        };
        let accepted = (self.options.accepted)(*accepted);
        if self.rows.len() == RECORDS {
            return Err("pan marker capacity exhausted".to_owned());
        }
        self.rows.push((
            unix_ns()?,
            CACurrentMediaTime(),
            requested.unwrap_or(accepted),
            accepted,
        ));
        let witness_tail = if self.options.refresh_measurement.is_some() {
            0.25
        } else {
            0.0
        };
        Ok(elapsed >= boundaries[3] + witness_tail)
    }
    pub(super) fn request(
        &mut self,
        sequence: u64,
        target: f64,
        controls: &Controls,
    ) -> Result<(), String> {
        let id = controls
            .lookup(self.options.control)
            .map_err(|e| e.to_string())?;
        let Value::Number(accepted) = controls.value(id) else {
            return Err("qualification control is not numeric".to_owned());
        };
        if self.requests.len() == RECORDS {
            return Err("request marker capacity exhausted".to_owned());
        }
        self.requests.push((
            sequence,
            CACurrentMediaTime(),
            target,
            (self.options.accepted)(*accepted),
        ));
        Ok(())
    }
    pub(super) fn finish(&self) -> Result<(), String> {
        let mut markers = BufWriter::new(
            File::create(self.options.output.join("signposts.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(markers, "stage\tmedia_before\tmedia_after").map_err(|e| e.to_string())?;
        for (stage, before, after) in &self.signposts {
            writeln!(markers, "{stage}\t{before:.9}\t{after:.9}").map_err(|e| e.to_string())?;
        }
        markers.flush().map_err(|e| e.to_string())?;
        let mut file = BufWriter::new(
            File::create(self.options.output.join("controls.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(
            file,
            "unix_ns\tmedia_seconds\trequested_{0}\taccepted_{0}",
            self.options.control_column,
        )
        .map_err(|e| e.to_string())?;
        for (unix, media, requested, accepted) in &self.rows {
            writeln!(file, "{unix}\t{media:.9}\t{requested:.9}\t{accepted:.9}")
                .map_err(|e| e.to_string())?;
        }
        file.flush().map_err(|e| e.to_string())?;
        let mut file = BufWriter::new(
            File::create(self.options.output.join("requests.tsv")).map_err(|e| e.to_string())?,
        );
        writeln!(
            file,
            "sequence\tmedia_seconds\ttarget_media_seconds\taccepted_{}",
            self.options.control_column,
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
    extent_markers: Option<BufWriter<File>>,
    status_markers: Option<BufWriter<File>>,
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
        transition: Option<&WindowTransitionProof>,
        qualification: Option<&WindowQualification>,
    ) -> Result<Option<Self>, String> {
        let transition = transition.filter(|options| options.tap);
        let Some(output) = transition
            .map(|options| options.output.clone())
            .or_else(|| qualification.map(|options| options.output.clone()))
        else {
            return Ok(None);
        };
        let before = CACurrentMediaTime();
        let origin_elapsed = origin.elapsed().as_nanos();
        let unix = unix_ns()?;
        let after = CACurrentMediaTime();
        std::fs::write(output.join("clock-anchor.tsv"), format!("pid\torigin_elapsed_ns\tunix_ns\tmedia_before\tmedia_after\n{}\t{origin_elapsed}\t{unix}\t{before:.9}\t{after:.9}\n", std::process::id())).map_err(|e| e.to_string())?;
        let extent_markers = if transition.is_some() {
            let mut marks = BufWriter::new(
                File::create(output.join("extents.tsv")).map_err(|error| error.to_string())?,
            );
            writeln!(marks, "submission_id	width	height").map_err(|error| error.to_string())?;
            Some(marks)
        } else {
            None
        };
        let status_markers = if transition.is_some_and(|proof| proof.arm == TransitionArm::Status) {
            let file =
                File::create(output.join("status.jsonl")).map_err(|error| error.to_string())?;
            Some(BufWriter::new(file))
        } else {
            None
        };
        let capture = transition.is_some() || qualification.is_some_and(|options| options.capture);
        let writer = if capture {
            // The qualification spool and its readers exchange RGBA8.
            // Request that engine stage without changing the window's display mode.
            host.enable_submitted_capture_from(
                1,
                CaptureLimits {
                    staging_slots: 6,
                    cpu_buffers: 96,
                    byte_allowance: 1024 * 1024 * 1024,
                },
                CaptureSource::Sdr,
            )
            .map_err(|e| e.to_string())?;
            let (name, encoding) = if transition.is_some() {
                ("frames.rgba", SpoolEncoding::Raw)
            } else {
                ("frames.zst", SpoolEncoding::ZstdDelta)
            };
            let file = File::create(output.join(name)).map_err(|e| e.to_string())?;
            Some(
                ImageWriter::start(
                    BufWriter::with_capacity(4 * 1024 * 1024, file),
                    96,
                    8 * 1024 * 1024 * 1024,
                    encoding,
                )
                .map_err(|error| error.to_string())?,
            )
        } else {
            None
        };
        Ok(Some(Self {
            output,
            extent_markers,
            status_markers,
            writer,
            images: Vec::with_capacity(RECORDS),
            frames: Vec::with_capacity(RECORDS),
            refused: 0,
            origin,
            replace_surface: qualification.is_some_and(|options| options.replace_surface),
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
        host.enable_submitted_capture_from(
            self.session,
            CaptureLimits {
                staging_slots: 6,
                cpu_buffers: 96,
                byte_allowance: 1024 * 1024 * 1024,
            },
            CaptureSource::Sdr,
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
            if let Some(marks) = &mut self.extent_markers {
                writeln!(
                    marks,
                    "{}	{}	{}",
                    image.sample.submission_order, image.extent[0], image.extent[1]
                )
                .map_err(|error| FrameError::Scene(error.to_string()))?;
            }
            if let Some(marks) = &mut self.status_markers {
                let interface = image.sample.interface;
                let record = serde_json::json!({
                    "submission": image.sample.submission_order, "tick": image.sample.tick,
                    "extent": image.extent, "surface_presented": image.sample.surface_presented,
                    "scale": interface.and_then(|stage| stage.stamp).map(|stamp| stamp.scale),
                    "extent_mismatch": interface.is_some_and(|stage| stage.extent_mismatch),
                    "stamp_mismatch": interface.is_some_and(|stage| stage.stamp_mismatch),
                    "dropped_rows": interface.map(|stage| stage.dropped_rows),
                });
                writeln!(marks, "{record}")
                    .map_err(|error| FrameError::Scene(error.to_string()))?;
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
        if let Some(marks) = &mut self.status_markers {
            marks.flush().map_err(|error| error.to_string())?;
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
        if let Some(marks) = &mut self.extent_markers {
            marks.flush().map_err(|error| error.to_string())?;
        }
        if let Some(writer) = self.writer.take() {
            let report = writer
                .finish(Duration::from_secs(10))
                .map_err(|error| error.to_string())?;
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
pub(super) fn write_cadence_samples(
    output: &std::path::Path,
    samples: &[FrameSample],
) -> std::io::Result<()> {
    let mut frames = BufWriter::new(File::create(output.join("cadence-frames.tsv"))?);
    let mut passes = BufWriter::new(File::create(output.join("cadence-passes.tsv"))?);
    writeln!(
        frames,
        "submission_id\ttick\tdisplay_phase\tpresented\ttiming_requested\ttiming_fault"
    )?;
    writeln!(passes, "submission_id\tpass\tnanoseconds")?;
    for sample in samples {
        writeln!(
            frames,
            "{}\t{}\t{}\t{}\t{}\t{}",
            sample.submission_order,
            sample.tick,
            sample.display_phase,
            sample.surface_presented,
            sample.timing_requested,
            sample.timing_fault,
        )?;
        if let Some(timings) = &sample.gpu_pass_ns {
            for pass in timings.passes() {
                writeln!(
                    passes,
                    "{}\t{}\t{}",
                    sample.submission_order, pass.name, pass.nanoseconds,
                )?;
            }
        }
    }
    frames.flush()?;
    passes.flush()
}

fn unix_ns() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .map_err(|e| e.to_string())
}
