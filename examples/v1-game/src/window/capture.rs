//! Authored V1 scenarios on the shared unattended engine driver.

mod admission;
mod cloud_take;
mod native_recovery;
pub(super) mod scene;
mod sheet_edits;

use super::{RuntimeCaptureOutput, WindowGame};
use crate::host::haze_capture::HazeTake;
use crate::{GameHost, hosted::HostedGame};

fn ambient_selection(selection: thrum_render::capabilities::AmbientOcclusionSelection) -> Value {
    json!({
        "requested_radius": selection.requested_radius,
        "radius": selection.radius,
        "fallback": selection.fallback.map(|reason| format!("{reason:?}")),
    })
}

fn shadow_selection(selection: thrum_render::synthetic::ShadowSelection) -> Value {
    json!({
        "requested_sun": format!("{:?}", selection.requested_sun),
        "sun": format!("{:?}", selection.sun),
        "requested_lamps": selection.requested_lamps,
        "lamps": selection.lamps,
        "fallback": selection.fallback.map(|reason| format!("{reason:?}")),
    })
}
use serde_json::{Value, json};
use std::io::Write;
use std::{
    os::fd::AsFd,
    path::Path,
    time::{Duration, Instant},
};
use thrum_app::{
    Game,
    offscreen::{
        Offscreen,
        clock::{OpportunityClock, Timebase},
        thread_policy::{self, RenderThreadPolicy, WorkClock},
        writer::{ImageWriter, SpoolEncoding, WriterReport},
    },
};
use thrum_render::{
    capabilities::{DeviceCapabilities, DeviceRecipe, FIRST_DEVICE_GENERATION, OpenedDevice},
    runtime::{FrameWorld, SubmittedImage},
    synthetic::CaptureSource,
};

/// Run the ordinary game on a dedicated render-owner thread. Stdout is raw RGBA;
/// scripts own the sequential spool, packaging and finalized evidence manifest.
pub(super) fn run(
    output: &Path,
    scenario: &str,
    wait: bool,
    hold_full: Duration,
    recovery: super::NativeRecovery,
    capture_output: RuntimeCaptureOutput,
    status: super::StatusCapture,
    hardware: Vec<super::HardwareRequest>,
) -> Result<(), String> {
    if !matches!(
        scenario,
        "pan"
            | "route"
            | "variants"
            | "running"
            | "render-stall"
            | "input-stall"
            | "off"
            | "writer-slow"
            | "reconstruction-pan"
            | "content-edits"
            | "reconstruction-stars"
            | "reconstruction-scrub"
            | "atmosphere-scrub"
            | "atmosphere-ground-pan"
            | "clouds-inspect"
            | "clouds-drift"
            | "clouds-wind"
            | "clouds-cycle"
            | "clouds-light-cycle"
            | "clouds-life"
            | "clouds-life-fast"
            | "clouds-placed"
            | "clouds-dusk"
            | "clouds-still"
            | "checkpoint"
            | "clouds-route"
            | "exit-route"
            | "exit-drift"
            | "exit-stop"
            | "sun-settle"
            | "sun-cut"
            | "sun-set"
            | "sun-motion"
            | "sprites-field"
            | "sprites-field-cost"
            | "sprites-scroll"
            | "sprites-sheets"
            | "sprites-status"
    ) {
        return Err("unknown runtime capture scenario".to_owned());
    }
    let output = output.to_owned();
    let scenario = scenario.to_owned();
    std::thread::Builder::new()
        .name("thrum-headless-render".to_owned())
        .spawn(move || {
            pollster::block_on(record(
                &output,
                &scenario,
                wait,
                hold_full,
                recovery,
                capture_output,
                status,
                &hardware,
            ))
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "capture render owner panicked".to_owned())?
}

#[derive(Default)]
struct Trace {
    opportunities: Vec<Value>,
    preview: Option<crate::content_preview::PreviewRecord>,
    images: Vec<Value>,
    gpu: Vec<Value>,
    stalls: Vec<Value>,
    controls: Vec<Value>,
    world_overload_recoveries: Vec<Value>,
    writer_omissions: u64,
    input_delay: Option<Duration>,
    input_delay_finished: bool,
    route_last_tick: Option<u64>,
    route_finished_at: Option<Duration>,
}

impl Trace {
    fn display_phase_counts(&self) -> Value {
        let mut statuses = [0_u64; 5];
        let mut bound = 0;
        let mut stopped = 0;
        let mut origin_changes = 0;
        let mut last_origin: Option<f64> = None;
        for image in &self.images {
            if let (Some(due), Some(tick)) =
                (image["tick_due_time_ns"].as_u64(), image["tick"].as_u64())
            {
                let origin = due as f64 / 1e9 - tick as f64 / 60.0;
                if let Some(previous) = last_origin
                    && (origin - previous).abs() > 2e-9
                {
                    origin_changes += 1;
                }
                last_origin = Some(origin);
            }
            let status = match image["lead_status"].as_str() {
                Some("future") => 0,
                Some("late") => 1,
                Some("invalid") => 2,
                Some("clamped") => 3,
                _ => 4,
            };
            statuses[status] += 1;
            stopped += u64::from(image["clock_interpolating"] == false);
            // This game's authoritative recipe is 60 Hz: 1 + .050 * 60.
            bound += u64::from(image["display_phase"].as_f64().is_some_and(|p| p >= 4.0));
        }
        json!({"future":statuses[0],"late":statuses[1],"invalid":statuses[2],
            "clamped":statuses[3],"untargeted":statuses[4],"bound":bound,"stopped":stopped,"origin_changes":origin_changes})
    }

    fn for_route(
        route: Option<&Value>,
        preview: Option<crate::content_preview::PreviewRecord>,
    ) -> Result<Self, String> {
        let route_last_tick = route
            .map(|route| {
                let origin = route["origin_tick"]
                    .as_u64()
                    .ok_or("missing route origin")?;
                let last = route["keys"]
                    .as_array()
                    .and_then(|keys| keys.last())
                    .and_then(|key| key["tick"].as_u64())
                    .ok_or("missing last route key")?;
                origin.checked_add(last).ok_or("route tick overflow")
            })
            .transpose()?;
        Ok(Self {
            preview,
            route_last_tick,
            ..Self::default()
        })
    }

    fn opportunity(&mut self, mut row: Value, report: &thrum_render::runtime::DriveReport) {
        if report.submitted
            && let Some(preview) = &self.preview
        {
            row["content"] = json!(preview.frame());
            row["presented_to_engine_surface"] = json!(report.surface_presented);
            row["drawable_presented"] = json!(false);
        }
        self.opportunities.push(row);
    }

    fn prime<G: Game>(
        &mut self,
        output: &Path,
        driver: &mut Offscreen<G>,
        writer: Option<&ImageWriter>,
    ) -> Result<Instant, String> {
        // A real initial submission anchors zero. This gives a full initial hold,
        // rather than losing the first render's latency from the required second.
        std::fs::write(
            output.join("measurement-ready.txt"),
            b"before prime; measured interval follows\n",
        )
        .map_err(|e| e.to_string())?;
        let prime_requested = Instant::now();
        let primed = driver.step().map_err(|error| error.to_string())?;
        if !primed.submitted {
            return Err("initial ordinary submission refused".to_owned());
        }
        let mut origin = Instant::now();
        if let Some(writer) = writer {
            let deadline = origin + Duration::from_secs(10);
            loop {
                drop(driver.completed()?);
                if let Some(image) = driver.image()? {
                    origin = image.submitted_at;
                    self.image(driver, writer, image, origin);
                    break;
                }
                if Instant::now() >= deadline {
                    return Err("initial image deadline".to_owned());
                }
            }
        }
        self.opportunity(json!({"id":0,"request_ns":relative_ns(origin,prime_requested),"accepted_ns":null,"scenario_ns":0,
        "submitted":true,"tick":primed.tick,"submission_id":driver.last_submission(),
        "requested_yaw_degrees":0.0,"accepted_yaw_degrees":accepted_yaw(driver)?,"primed":true}), &primed,
    );
        Ok(origin)
    }

    fn world_state<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        paused: bool,
        origin: Instant,
    ) -> Result<(), String> {
        let requested = origin.elapsed();
        driver
            .parameters(&json!({"world.paused":paused}).to_string())
            .map_err(|error| error.to_string())?;
        self.controls
            .push(json!({"kind":"world-pause", "paused":paused,
            "requested_ns":requested.as_nanos(),"accepted_ns":origin.elapsed().as_nanos()}));
        Ok(())
    }

    fn yaw<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        scenario: &str,
        t: f64,
        yaw: f64,
        origin: Instant,
    ) -> Result<bool, String> {
        if scenario == "input-stall" && t >= 1.8 && !self.input_delay_finished {
            let start = *self.input_delay.get_or_insert_with(|| origin.elapsed());
            if origin.elapsed() < start + Duration::from_millis(250) {
                return Ok(false);
            }
            self.stalls.push(json!({"kind":"input-dispatch-before-semantic-handler",
                "requested_ns":start.as_nanos(),"start_ns":start.as_nanos(),"end_ns":origin.elapsed().as_nanos()}));
            self.input_delay_finished = true;
        }
        driver
            .parameters(&json!({"camera.yaw":yaw}).to_string())
            .map_err(|error| error.to_string())?;
        Ok(true)
    }
    fn image<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        writer: &ImageWriter,
        image: SubmittedImage,
        origin: Instant,
    ) {
        self.observe_route_end(
            image.source.tick,
            image.submitted_at.saturating_duration_since(origin),
        );
        self.images.push(json!({"submission_id":image.sample.submission_order,"tick":image.sample.tick,
            "shadow_selection":shadow_selection(image.sample.shadow_selection),
            "ambient_occlusion":ambient_selection(image.sample.ambient_occlusion),
            "drawn_extent":image.sample.render_extent,"history":image.sample.history_identity,
            "jitter_phase":image.sample.jitter_phase,
            "epoch":image.source.epoch,"timeline":image.source.timeline,
            "previous_endpoint_tick":image.source.tick.checked_sub(1),"current_endpoint_tick":image.source.tick,
            "alpha":image.sample.presentation_alpha,"history_epoch":image.history_epoch,
            "observation_ns":image.sample.observation.as_nanos(),
            "display_phase":image.sample.display_phase,"clock_interpolating":image.sample.clock_interpolating,
            "tick_due_time_ns":image.sample.tick_due_time.map(|t| t.as_nanos()),
            "platform_target_ns":image.sample.platform_target.map(|t| t.as_nanos()),
            "lead_status":image.sample.lead_status.map(|s| format!("{s:?}").to_lowercase()),
            "pixel_source":format!("{:?}",image.pixel_source),"exposure_gpu_words":image.exposure_gpu_words,
            "visual_revision":image.sample.visual_revision,"device_generation":image.device_generation,"surface_generation":image.presenter_generation,
            "presentation_lead_ns":image.presentation_lead.as_nanos(),"readback_copy_gpu_bracket_ns":image.copy_gpu_ns,
            "sprites":crate::sprite_field::sample_stats(&image.sample),
            "interface":image.sample.interface.map(|stage| json!({"scale":stage.stamp.map(|stamp| stamp.scale),
                "stamp_mismatch":stage.stamp_mismatch,"extent_mismatch":stage.extent_mismatch,"dropped_rows":stage.dropped_rows})) ,
            "submit_ns":image.submitted_at.saturating_duration_since(origin).as_nanos(),
            "map_observed_ns":origin.elapsed().as_nanos()}));
        if let Some(image) = writer.submit(image) {
            self.writer_omissions += 1;
            driver.recycle(image);
        }
    }

    fn observe_route_end(&mut self, tick: u64, elapsed: Duration) {
        if self.route_last_tick.is_some_and(|last| tick >= last) {
            self.route_finished_at.get_or_insert(elapsed);
        }
    }

    fn observe_overload<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        origin: Instant,
        observed: Duration,
    ) -> Result<(), String> {
        if let Some(refused_ticks) = driver.recover_world_overload().map_err(|e| e.to_string())? {
            self.world_overload_recoveries.push(json!({
                "observed_ns": observed.as_nanos(),
                "resumed_ns": origin.elapsed().as_nanos(),
                "refused_ticks": refused_ticks,
            }));
        }
        Ok(())
    }

    fn finish_writer<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        writer: Option<ImageWriter>,
        origin: Instant,
        submitted: usize,
    ) -> Result<Result<WriterReport, String>, String> {
        let Some(writer) = writer else {
            return Ok(Ok(WriterReport::default()));
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.images.len() < submitted && Instant::now() < deadline {
            self.drain(driver, Some(&writer), origin, true)?;
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(writer
            .finish(Duration::from_secs(10))
            .map_err(|error| error.to_string()))
    }

    // The offscreen render owner otherwise sleeps between observations. Give
    // queued frames and their later timestamp resolves time to retire during
    // that idle period without waiting into the next render opportunity.
    fn progress_before<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        admission: &mut admission::Admission,
        writer: Option<&ImageWriter>,
        origin: Instant,
        next: Instant,
    ) -> Result<Value, String> {
        let mut polls = 0;
        let mut cpu = Duration::ZERO;
        let mut wall = Duration::ZERO;
        while driver.pending_frames() != 0
            && next.saturating_duration_since(Instant::now()) > Duration::from_micros(750)
        {
            std::thread::sleep(Duration::from_micros(250));
            let work = WorkClock::start().map_err(|error| error.to_string())?;
            let collect = admission.collect_images(driver, origin);
            self.drain(driver, writer, origin, collect)?;
            let work = work.finish().map_err(|error| error.to_string())?;
            polls += 1;
            cpu += work.cpu;
            wall += work.wall;
        }
        Ok(json!({"polls": polls, "cpu_ns": cpu.as_nanos(), "wall_ns": wall.as_nanos()}))
    }

    fn drain<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        writer: Option<&ImageWriter>,
        origin: Instant,
        collect_images: bool,
    ) -> Result<(), String> {
        while let Some(sample) = driver.completed()? {
            self.gpu.push(
                json!({"submission_id":sample.submission_order,"tick":sample.tick,
                "culling_selection":format!("{:?}",sample.culling_selection),
                "shadow_selection":shadow_selection(sample.shadow_selection),
                "ambient_occlusion":ambient_selection(sample.ambient_occlusion),
                "timed_pass_span_ns":sample.gpu_pass_ns.as_ref().and_then(thrum_render::timing::SceneGpuTimings::timed_pass_span_nanoseconds),
                "gpu_effect_boundary_ns":sample.gpu_pass_ns.as_ref().and_then(thrum_render::timing::SceneGpuTimings::effect_boundary_nanoseconds),
                "gpu_not_run":sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::not_run_passes),
                "native_segment":sample.native_segment.as_ref().map(|segment| json!({"pass":segment.pass_name(),"generation":segment.generation,"submission":segment.submission,"clock":"Metal host seconds; uncorrelated with wgpu queries","outcome":format!("{:?}",segment.outcome())})),
                "sprites":crate::sprite_field::sample_stats(&sample),"encode_cpu_ns":sample.encode_cpu.as_nanos(),"upload_cpu_ns":sample.upload_cpu.as_nanos(),"extract_cpu_ns":sample.extract_cpu.as_nanos(),"gpu_pass_ns":sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::values),"render_cpu_ns":sample.render_cpu.as_nanos(),"wall_ns":sample.wall.as_nanos(),"tick_cpu_ns":sample.tick_cpu.as_nanos(),"gpu_present_ns":sample.gpu_present_ns}),
            );
        }
        if let Some(writer) = writer {
            while let Some(image) = writer.recycled() {
                driver.recycle(image);
            }
            if collect_images {
                while let Some(image) = driver.image()? {
                    self.image(driver, writer, image, origin);
                }
            }
        }
        Ok(())
    }

    fn delay(&mut self, kind: &str, origin: Instant) {
        let start = origin.elapsed().as_nanos();
        std::thread::sleep(Duration::from_millis(250));
        self.stalls
            .push(json!({"kind":kind,"requested_ns":start,"start_ns":start,
                              "end_ns":origin.elapsed().as_nanos()}));
    }
}

struct ScenarioSink {
    output: std::fs::File,
    next_write: Instant,
    limit: Option<u64>,
    slow: bool,
}
impl Write for ScenarioSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.slow {
            std::thread::sleep(Duration::from_millis(500));
        }
        if self.limit.is_some()
            && let Some(wait) = self.next_write.checked_duration_since(Instant::now())
        {
            std::thread::sleep(wait);
        }
        let begin = Instant::now();
        let n = self.output.write(bytes)?;
        if let Some(limit) = self.limit {
            self.next_write = begin + Duration::from_secs_f64(n as f64 / limit as f64);
        }
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()
    }
}

fn accepted_yaw<G: Game>(driver: &Offscreen<G>) -> Result<f32, String> {
    let controls = driver.controls();
    let id = controls.lookup("camera.yaw").map_err(|e| e.to_string())?;
    if let thrum_controls::Value::Number(yaw) = controls.value(id) {
        Ok(yaw.to_degrees())
    } else {
        Err("camera yaw is not numeric".to_owned())
    }
}

fn start_writer(
    scenario: &str,
    encoding: SpoolEncoding,
    limit: Option<u64>,
    recovery: super::NativeRecovery,
) -> Result<ImageWriter, String> {
    let output = std::fs::File::from(
        std::io::stdout()
            .as_fd()
            .try_clone_to_owned()
            .map_err(|e| e.to_string())?,
    );
    ImageWriter::start_with_frame_limit(
        std::io::BufWriter::with_capacity(
            4 * 1024 * 1024,
            ScenarioSink {
                output,
                next_write: Instant::now(),
                limit,
                slow: scenario == "writer-slow",
            },
        ),
        96,
        8 * 1024 * 1024 * 1024,
        encoding,
        if matches!(
            scenario,
            "clouds-cycle" | "clouds-life" | "clouds-light-cycle"
        ) {
            16_384
        } else if matches!(
            scenario,
            "clouds-drift"
                | "clouds-inspect"
                | "clouds-wind"
                | "clouds-life-fast"
                | "clouds-placed"
                | "clouds-route"
                | "exit-route"
                | "exit-drift"
                | "sun-motion"
                | "sprites-field"
        ) || recovery != super::NativeRecovery::Disabled
        {
            4096
        } else {
            1024
        },
    )
    .map_err(|e| e.to_string())
}

pub(super) fn prepare_parity_content(
    host: &mut crate::host::Runtime,
    content: Option<&std::sync::Mutex<thrum_asset_gpu::CookedStats>>,
) -> Result<(), String> {
    if !renderer_parity_requested() {
        return Ok(());
    }
    if crate::content::launch_value("THRUM_CAPTURE_START_PAUSED").as_deref() == Ok("1") {
        host.world_mut().pause()?;
    }
    // Submit endpoint draws until the valley is resident. These draws do not
    // commit view history or consume any of the held reconstruction warmup.
    let stats = content.ok_or("renderer parity needs cooked content")?;
    thrum_asset_gpu::await_drawn(host, stats, thrum_asset_gpu::Drawn::Any)?;
    Ok(())
}

fn renderer_parity_requested() -> bool {
    crate::content::launch_value("THRUM_CAPTURE_RENDERER_SIGNATURE").as_deref() == Ok("1")
}

fn warmup<G: Game>(driver: &mut Offscreen<G>) -> Result<(), String> {
    warmup_checkpoint(driver, false)
}

fn warmup_checkpoint<G: Game>(
    driver: &mut Offscreen<G>,
    parity_checkpoint: bool,
) -> Result<(), String> {
    let warmup = Instant::now();
    let last_warmup_submission = driver.host().last_submission().map_or(127, |id| id + 128);
    while if parity_checkpoint {
        driver
            .host()
            .last_submission()
            .is_none_or(|id| id < last_warmup_submission)
    } else {
        warmup.elapsed() < Duration::from_secs(3)
    } {
        if warmup.elapsed() >= Duration::from_secs(30) {
            return Err("fixed checkpoint warmup submission timeout".to_owned());
        }
        driver.step().map_err(|error| error.to_string())?;
        while driver.completed()?.is_some() {}
        std::thread::sleep(Duration::from_millis(8));
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while driver.pending_frames() != 0 {
        drop(driver.completed()?);
        if Instant::now() >= deadline {
            return Err("warmup drain timeout".to_owned());
        }
    }
    Ok(())
}

fn capture_settings() -> Result<
    (
        thread_policy::ThreadPolicyReport,
        SpoolEncoding,
        Option<u64>,
    ),
    String,
> {
    let qos = match std::env::var("THRUM_CAPTURE_QOS").as_deref() {
        Ok("inherited") => RenderThreadPolicy::Inherited,
        Ok("interactive") | Err(_) => RenderThreadPolicy::Interactive,
        _ => return Err("invalid capture QoS".to_owned()),
    };
    let qos = thread_policy::configure(qos).map_err(|e| e.to_string())?;
    let encoding = match std::env::var("THRUM_CAPTURE_ENCODING").as_deref() {
        Ok("raw") => SpoolEncoding::Raw,
        Ok("zstd-delta") | Err(_) => SpoolEncoding::ZstdDelta,
        _ => return Err("invalid spool encoding".to_owned()),
    };
    let disk_limit = std::env::var("THRUM_CAPTURE_DISK_BPS")
        .ok()
        .map(|v| v.parse::<u64>())
        .transpose()
        .map_err(|e| e.to_string())?;
    if disk_limit == Some(0) {
        return Err("disk rate must be positive".to_owned());
    }
    Ok((qos, encoding, disk_limit))
}

fn active_pacing() -> Result<bool, String> {
    match std::env::var("THRUM_CAPTURE_PACING").as_deref() {
        Ok("active") => Ok(true),
        Ok("sleep") | Err(_) => Ok(false),
        _ => Err("capture pacing must be sleep or active".to_owned()),
    }
}

fn requested_route(scenario: &str) -> Result<Option<Value>, String> {
    Ok(if is_route(scenario) {
        let path = std::env::var_os("THRUM_CAPTURE_RUNTIME_ROUTE")
            .ok_or("route capture requires its route file")?;
        Some(
            serde_json::from_str(
                &std::fs::read_to_string(path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?,
        )
    } else {
        None
    })
}

fn is_route(scenario: &str) -> bool {
    matches!(scenario, "route" | "clouds-route" | "exit-route")
}

fn world_moves(scenario: &str) -> bool {
    is_route(scenario)
        || weather_motion(scenario)
        || matches!(
            scenario,
            "running"
                | "writer-slow"
                | "reconstruction-pan"
                | "content-edits"
                | "clouds-drift"
                | "clouds-placed"
                | "exit-drift"
                | "sun-motion"
                | "sprites-field"
                | "sprites-field-cost"
                | "sprites-scroll"
                | "sprites-sheets"
        )
}

fn applies_yaw(scenario: &str, active_world: bool) -> bool {
    (!active_world || matches!(scenario, "reconstruction-pan" | "content-edits"))
        && !matches!(
            scenario,
            "reconstruction-stars"
                | "reconstruction-scrub"
                | "clouds-dusk"
                | "clouds-placed"
                | "exit-stop"
                | "sprites-field"
                | "sprites-field-cost"
                | "sprites-sheets"
                | "sprites-status"
                | "atmosphere-scrub"
                | "atmosphere-ground-pan"
                | "sun-settle"
                | "sun-cut"
                | "sun-set"
                | "sun-motion"
        )
}

fn requested_yaw(scenario: &str, time: f64, recovery: super::NativeRecovery) -> f64 {
    if scenario == "content-edits" {
        3.0 * (time * 0.7).sin()
    } else {
        native_recovery::yaw(time, recovery)
    }
}

fn capture_hz(scenario: &str, motion_capture: bool) -> u32 {
    if (motion_capture && scenario != "clouds-wind")
        || matches!(scenario, "clouds-dusk" | "clouds-placed")
    {
        60
    } else {
        120
    }
}

fn prepare_parameters(
    driver: &mut Offscreen<HostedGame<WindowGame>>,
    scenario: &str,
    mode: &str,
    haze_take: Option<HazeTake>,
    parity_checkpoint: bool,
) -> Result<(Instant, Instant), String> {
    let preset_requested = Instant::now();
    let parameters = std::env::var_os("THRUM_CAPTURE_RUNTIME_PARAMS")
        .map(std::fs::read_to_string)
        .transpose()
        .map_err(|error| error.to_string())?;
    if is_route(scenario) || scenario.starts_with("sprites-") {
        driver
            .parameters(r#"{"world.paused":true}"#)
            .map_err(|error| error.to_string())?;
    } else {
        driver
            .parameters(
                parameters
                    .as_deref()
                    .unwrap_or(include_str!("../../params/showcase-noon.params.json")),
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(take) = haze_take {
        let pose = take.at(0.0);
        let host = driver.host_mut();
        crate::host::set_haze_eye(host, pose.eye, true)?;
        crate::host::scrub_haze_sun(host, pose.solar_hours / 24.0)?;
        host.set_camera_yaw(pose.yaw_degrees.to_radians())?;
    }
    if scenario == "sun-set" {
        crate::host::scrub_haze_sun(driver.host_mut(), SUNSET_START_HOURS / 24.0)?;
    }
    if weather_motion(scenario) || scenario == "clouds-inspect" {
        crate::host::hold_capture_sun(driver.host_mut());
    }
    let overlay_before = crate::content::launch_value("THRUM_CAPTURE_CLOUD_OVERLAY_BEFORE").ok();
    if let Some(before) = &overlay_before {
        if scenario != "clouds-still" || mode != "fixed" {
            return Err("a restored cloud overlay needs a fixed clouds-still capture".to_owned());
        }
        let before: u8 = before
            .parse()
            .map_err(|_| "invalid initial cloud overlay")?;
        if !(1..=7).contains(&before) {
            return Err("the initial cloud overlay needs an enabled field".to_owned());
        }
        driver
            .parameters(&json!({"clouds.overlay": before}).to_string())
            .map_err(|error| error.to_string())?;
    }
    let preset_accepted = Instant::now();
    warmup_checkpoint(driver, parity_checkpoint)?;
    settle_placed_capture(driver, mode, parameters.as_deref())?;
    if overlay_before.is_some() {
        driver
            .parameters(r#"{"clouds.overlay": 0}"#)
            .map_err(|error| error.to_string())?;
    }
    Ok((preset_requested, preset_accepted))
}

fn settle_placed_capture(
    driver: &mut Offscreen<HostedGame<WindowGame>>,
    mode: &str,
    parameters: Option<&str>,
) -> Result<(), String> {
    if driver.host().cloud_placed_input().is_none() {
        return Ok(());
    }
    driver
        .parameters(r#"{"world.paused":true}"#)
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(20);
    while !driver
        .host()
        .cloud_resource_bytes()
        .iter()
        .any(|(name, _)| name.starts_with("cloud-growth "))
    {
        driver.step().map_err(|error| error.to_string())?;
        while driver.completed()?.is_some() {}
        if Instant::now() >= deadline {
            return Err("placed stack did not become resident before capture".to_owned());
        }
    }
    if mode != "realtime" {
        driver
            .parameters(r#"{"world.paused":false}"#)
            .map_err(|error| error.to_string())?;
        let start = driver.host().world().tick();
        while driver.host().world().tick() < start + 300 {
            driver.step().map_err(|error| error.to_string())?;
            while driver.completed()?.is_some() {}
            if Instant::now() >= deadline {
                return Err("placed clouds did not form before the fixed checkpoint".to_owned());
            }
        }
        if let Some(parameters) = parameters {
            driver
                .parameters(parameters)
                .map_err(|error| error.to_string())?;
        }
        driver
            .parameters(r#"{"world.paused":true}"#)
            .map_err(|error| error.to_string())?;
    }
    while driver.pending_frames() != 0 {
        while driver.completed()?.is_some() {}
        if Instant::now() >= deadline {
            return Err("placed capture did not drain its setup frames".to_owned());
        }
    }
    Ok(())
}

fn capture_source() -> Result<CaptureSource, String> {
    match std::env::var("THRUM_CAPTURE_SOURCE").as_deref() {
        Ok("sdr") | Err(_) => Ok(CaptureSource::Sdr),
        Ok("scene-linear") => Ok(CaptureSource::SceneLinear),
        Ok("native-output") => Ok(CaptureSource::NativeOutput),
        Ok("native-history") => Ok(CaptureSource::NativeHistory),
        _ => Err("unknown capture source".to_owned()),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one capture interval owns the clock, submission and drain order"
)]
async fn record(
    output: &Path,
    scenario: &str,
    wait: bool,
    hold_full: Duration,
    recovery_mode: super::NativeRecovery,
    capture_output: RuntimeCaptureOutput,
    status: super::StatusCapture,
    hardware: &[super::HardwareRequest],
) -> Result<(), String> {
    let mode = capture_mode(scenario)?;
    let mut admission = admission::Admission::new(wait, hold_full, &mode)?;
    let (source, extent) = (capture_source()?, capture_extent()?);
    let (qos, encoding, disk_limit) = capture_settings()?;
    let parity_checkpoint = renderer_parity_requested();
    let active_pacing = active_pacing()?;
    let instance = metal_instance();
    let (adapter, device, queue, capabilities) =
        metal_device(&instance, FIRST_DEVICE_GENERATION, hardware).await?;
    let (game, preview) = capture_game(output, scenario, capture_output)?;
    let mut driver = Offscreen::new(game, &device, &capabilities, &queue, extent)
        .map_err(|error| error.to_string())?;
    let haze_take = crate::content::launch_value("THRUM_HAZE_TAKE")
        .ok()
        .map(|text| HazeTake::parse(&text))
        .transpose()?;
    let preview = preview.and_then(|record| record.lock().ok()?.clone());
    let governed = matches!(capture_output, RuntimeCaptureOutput::Governor { .. });
    if governed && (scenario != "route" || mode != "realtime") {
        return Err("governor capture requires the real-time moving route".to_owned());
    }
    let motion_capture = mode == "realtime" && (governed || is_motion_capture(scenario));
    let target_wind = scenario == "clouds-wind";
    let elapsed_observation = (motion_capture && !target_wind) || sprite_motion(scenario);
    let sprite_scroll_timings = scenario == "sprites-scroll"
        && crate::content::launch_value("THRUM_SPRITE_PASS_TIMING").as_deref() == Ok("1");
    let measure_passes = !(elapsed_observation || target_wind) || sprite_scroll_timings;
    if !measure_passes {
        driver.set_timing_requested(false);
    }
    if scenario == "sprites-status" {
        driver
            .host_mut()
            .continuous_source_mut()
            .hold_status_capture(status);
    }
    let (preset_requested, preset_accepted) =
        prepare_parameters(&mut driver, scenario, &mode, haze_take, parity_checkpoint)?;
    reset_native_capture(&mut driver);
    let sprite_cost = scenario == "sprites-field-cost";
    let observer_only = sprite_cost
        || matches!(
            capture_output,
            RuntimeCaptureOutput::ObserverOnly | RuntimeCaptureOutput::Governor { images: false }
        );
    if observer_only && !motion_capture && !sprite_cost {
        return Err(
            "observer-only control requires realtime exit or content-edit motion".to_owned(),
        );
    }
    let capture = scenario != "off" && !observer_only;
    if capture {
        enable_stage_capture(&mut driver, 1, source)?;
    }
    let writer = capture
        .then(|| start_writer(scenario, encoding, disk_limit, recovery_mode))
        .transpose()?;
    let route = requested_route(scenario)?;
    let scale_origin = if capture_output == RuntimeCaptureOutput::ScaleTraceImages {
        Some(
            route
                .as_ref()
                .and_then(|route| route["origin_tick"].as_u64())
                .ok_or("scripted render scale needs a route")?,
        )
    } else {
        None
    };
    let mut trace = Trace::for_route(route.as_ref(), preview)?;
    let origin = trace.prime(output, &mut driver, writer.as_ref())?;
    let sheet_saves = (scenario == "sprites-sheets")
        .then(|| sheet_edits::start(origin))
        .transpose()?;
    let timebase = if mode == "fixed" {
        Timebase::Fixed
    } else {
        Timebase::Elapsed
    };
    // Match the 60 Hz world and Chrome motion take. The native window and
    // existing 120 Hz capture scenarios keep their own opportunity cadence.
    let requested_hz = capture_hz(scenario, motion_capture);
    let mut quality_trace = Vec::new();
    if governed
        && !driver
            .host_mut()
            .enable_governor(thrum_render::governor::DisplayRecord {
                interval: Duration::from_secs(1) / requested_hz,
                thermal: thrum_render::governor::ThermalState::Nominal,
            })
    {
        return Err("governor capture requires a prepared TAAU scene".to_owned());
    }
    let mut clock =
        OpportunityClock::new(origin, requested_hz, timebase).map_err(|error| error.to_string())?;
    let mut running = false;
    let mut night_variant = false;
    let mut stalled = false;
    let mut recovery =
        native_recovery::Recovery::new(recovery_mode, capabilities.generation(), hardware);
    let cloud_take = cloud_take::Take::requested(scenario)?;
    let mut cloud_edit_done = false;
    let cloud_eye = cloud_take
        .as_ref()
        .map(|_| {
            driver
                .host()
                .scene_camera()
                .map(crate::SceneCamera::eye)
                .ok_or("clouds-inspect needs a valley camera")
        })
        .transpose()?;
    let mut final_hold = None;
    let mut last_sample_time = Duration::ZERO;
    let bank_mover = haze_take.is_some_and(HazeTake::runs_world);
    let (hard_end, soft_end, world_end) = if governed {
        (12.0, 10.0, f64::INFINITY)
    } else if let Some(take) = &cloud_take {
        take.bounds()
    } else if bank_mover {
        (14.0, 13.0, f64::INFINITY)
    } else {
        scenario_bounds(scenario, recovery_mode)
    };
    loop {
        if mode == "still"
            || held_sample(scenario)
            || scenario == "sprites-status" && mode == "fixed"
        {
            break;
        }
        admission.wait(&mut driver, &mut trace, writer.as_ref(), origin, &mut clock)?;
        let opportunity = if active_pacing {
            clock.next_active_opportunity()
        } else {
            clock.next_opportunity()
        };
        let t = opportunity.scenario_time.as_secs_f64();
        if mode == "realtime" && t >= hard_end {
            break;
        }
        let fixed_end = if scenario == "exit-stop" { 24 } else { 480 };
        if (mode == "fixed" && opportunity.index > fixed_end)
            || (mode == "realtime"
                && t >= soft_end
                && if is_route(scenario) {
                    trace.route_finished_at
                } else {
                    final_hold
                }
                .is_some_and(|start: Duration| origin.elapsed() >= start + Duration::from_secs(1)))
        {
            break;
        }
        let work = WorkClock::start().map_err(|e| e.to_string())?;
        let active_world = world_moves(scenario)
            || bank_mover
            || scenario == "sprites-status" && mode == "realtime";
        if active_world
            && t >= 1.0
            && (bank_mover || is_route(scenario) || t < world_end)
            && !running
        {
            trace.world_state(&mut driver, false, origin)?;
            running = true;
        } else if active_world && !bank_mover && !is_route(scenario) && t >= world_end && running {
            trace.world_state(&mut driver, true, origin)?;
            running = false;
        }
        lighting_edits(
            &mut driver,
            scenario,
            opportunity.scenario_time,
            &mut night_variant,
            &mut trace,
            origin,
            haze_take,
        )?;
        recovery
            .checkpoint(
                &instance,
                &mut driver,
                &mut trace,
                writer.as_ref(),
                origin,
                opportunity.scenario_time,
                source,
            )
            .await?;
        let cloud_edit = match (&cloud_take, cloud_eye) {
            (Some(take), Some(eye)) => take.apply(&mut driver, eye, t, &mut cloud_edit_done)?,
            _ => None,
        };
        let yaw = requested_yaw(scenario, t, recovery_mode);
        let control_dispatched = cloud_take.is_none()
            && applies_yaw(scenario, active_world)
            && trace.yaw(&mut driver, scenario, t, yaw, origin)?;
        let accepted = origin.elapsed();
        let accepted_yaw = accepted_yaw(&driver)?;
        if scenario == "render-stall" && t >= 1.8 && !stalled {
            trace.delay("render-owner-before-ordinary-step", origin);
            stalled = true;
        }
        if let Some(origin_tick) = scale_origin {
            let host = driver.host_mut();
            let tick = host
                .world()
                .source()
                .tick
                .saturating_add(1)
                .saturating_sub(origin_tick);
            host.set_render_scale(crate::camera::route_render_scale(tick))
                .map_err(|error| error.to_string())?;
        }
        // A frame can retire after this request becomes due and still leave
        // time to encode before the same display target. Keep that bounded
        // interval available for completions without changing the clock.
        let slot_wait = if (target_wind || governed) && !driver.host().has_free_slot() {
            let started = Instant::now();
            while !driver.host().has_free_slot()
                && opportunity.target.saturating_duration_since(Instant::now())
                    > Duration::from_millis(3)
            {
                let collect = admission.collect_images(&driver, origin);
                trace.drain(&mut driver, writer.as_ref(), origin, collect)?;
                if !driver.host().has_free_slot() {
                    std::thread::sleep(Duration::from_micros(250));
                }
            }
            Some(started.elapsed())
        } else {
            None
        };
        let step = WorkClock::start().map_err(|e| e.to_string())?;
        let admission_usage = driver.capture_usage();
        let omissions_before = driver.omissions();
        let report = if elapsed_observation {
            // The elapsed-time capture has no future display to target.
            driver.step().map_err(|error| error.to_string())?
        } else {
            driver
                .step_for(&opportunity)
                .map_err(|error| error.to_string())?
        };
        let step = step.finish().map_err(|e| e.to_string())?;
        let returned = origin.elapsed();
        if (elapsed_observation || target_wind || sprite_cost)
            && running
            && report.overload.is_some()
        {
            trace.observe_overload(&mut driver, origin, returned)?;
        }
        if observer_only && report.submitted {
            trace.observe_route_end(report.tick, returned);
        }
        if governed
            && report.submitted
            && let Some(quality) = driver.host().governor_frame()
        {
            quality_trace.push(json!({
                "submission": driver.last_submission(), "tick": report.tick,
                "scale": quality.state.scale.percent(), "tier": quality.state.tier,
                "reason": format!("{:?}", quality.state.reason),
                "source": quality.source.map(|source| format!("{source:?}")),
                "knobs": quality.knobs.iter().map(|knob| (&knob.name, knob.value))
                    .collect::<std::collections::BTreeMap<_, _>>(),
                "aerial_age": quality.aerial_age,
            }));
        }
        if t >= 3.0 && report.submitted && final_hold.is_none() {
            final_hold = Some(origin.elapsed());
        }
        let drain = Instant::now();
        let collect = admission.collect_images(&driver, origin);
        trace.drain(&mut driver, writer.as_ref(), origin, collect)?;
        let drain = drain.elapsed();
        let work = work.finish().map_err(|e| e.to_string())?;
        let idle_gpu_progress = if (governed || target_wind) && !active_pacing {
            Some(trace.progress_before(
                &mut driver,
                &mut admission,
                writer.as_ref(),
                origin,
                opportunity.target,
            )?)
        } else {
            None
        };
        let mut record = json!({"step_cpu_start":step.cpu_start,"step_cpu_end":step.cpu_end,"loop_cpu_start":work.cpu_start,"loop_cpu_end":work.cpu_end,"step_thread_cpu_ns":step.cpu.as_nanos(),"step_wall_ns":step.wall.as_nanos(),"loop_thread_cpu_ns":work.cpu.as_nanos(),"loop_work_wall_ns":work.wall.as_nanos(),"drain_wall_ns":drain.as_nanos(),"id":opportunity.index,"request_ns":opportunity.requested.as_nanos(),
            "scenario_ns":opportunity.scenario_time.as_nanos(),"accepted_ns":if control_dispatched {Some(accepted.as_nanos())} else {None},
            "control_dispatched":control_dispatched,
            "skipped_before":opportunity.skipped_before,"skip_reason":"driver busy or host oversleep",
            "returned_ns":returned.as_nanos(),"requested_yaw_degrees":yaw,"accepted_yaw_degrees":accepted_yaw,
            "submitted":report.submitted,"tick":report.tick,"dropped_view":report.dropped_view,
            "admission_usage":admission_usage,"ring_omissions":driver.omissions()-omissions_before,
            "submission_id":if report.submitted {driver.last_submission()} else {None},"capture_usage":driver.capture_usage()});
        if cloud_take.is_some() {
            record["cloud_edit"] = json!(cloud_edit);
            if report.submitted {
                record["cloud_history_valid"] = json!(
                    driver
                        .host()
                        .cloud_sky_products()
                        .map(|layer| layer.history_valid)
                );
            }
        }
        if let Some(idle_gpu_progress) = idle_gpu_progress {
            record["idle_gpu_progress"] = idle_gpu_progress;
        }
        if let Some(wait) = slot_wait {
            record["slot_wait_ns"] = json!(wait.as_nanos());
        }
        trace.opportunity(record, &report);
        last_sample_time = opportunity.scenario_time;
        if mode == "fixed" {
            drain_fixed(&mut driver, &mut trace, writer.as_ref(), origin)?;
        }
    }
    let interval_end_ns = origin.elapsed().as_nanos();
    let submitted = trace
        .opportunities
        .iter()
        .filter(|row| row["submitted"] == true)
        .count();
    let writer_result = trace.finish_writer(&mut driver, writer, origin, submitted)?;
    let interval_complete = mode == "still"
        || held_sample(scenario)
        || scenario == "sprites-status" && mode == "fixed"
        || (scenario == "exit-stop" && mode == "fixed" && trace.images.len() >= 24)
        || (mode == "fixed" && last_sample_time >= Duration::from_secs(4))
        || (mode == "realtime"
            && if is_route(scenario) {
                trace.route_finished_at
            } else {
                final_hold
            }
            .is_some_and(|start| origin.elapsed() >= start + Duration::from_secs(1)));
    let complete = interval_complete
        && (sprite_cost || trace.world_overload_recoveries.is_empty())
        && writer_result.is_ok()
        && (!capture
            || (trace.images.len() == submitted
                && driver.omissions() == 0
                && trace.writer_omissions == 0));
    let phase_counts = trace.display_phase_counts();
    let mut result = json!({"active_pacing":active_pacing,"thread_qos_before":qos.before,"thread_qos_after":qos.after,"spool_encoding":format!("{encoding:?}"),"pixel_source":format!("{source:?}"),"disk_limit_Bps":disk_limit,
        "scenario":scenario,"adapter":adapter.get_info().name,"backend":"Metal","mode":mode,
        "presentation_lead_policy":if elapsed_observation {"actual elapsed observation without a future target"} else {"engine clock next opportunity target, shared display_target_lead at consumption; untargeted prime/warmup zero"},
        "surface_generation_scope":"engine presenter incarnation: prepare, resize, device publish; not OS surface lease",
        "recipe":format!("{:?}",capabilities.recipe()),"registry_digest":capabilities.registry_digest(),"unified_memory":capabilities.unified_memory(),"enabled_features":format!("{:?}",capabilities.features()),
        "extent":extent,"requested_hz":requested_hz,"interval_end_ns":interval_end_ns,
        "last_scenario_ns":last_sample_time.as_nanos(),"duration_including_drain_ns":origin.elapsed().as_nanos(),
        "world_origin_before_run_ns":origin.saturating_duration_since(driver.origin()).as_nanos(),
        "route":route,
        "initial_controls":{"request_ns":relative_ns(origin,preset_requested),"accepted_ns":relative_ns(origin,preset_accepted)},
        "opportunities":trace.opportunities,"images":trace.images,"gpu":trace.gpu,"diagnostic_actions":trace.stalls,"control_actions":trace.controls,
        "ring_omissions":driver.omissions(),"writer_omissions":trace.writer_omissions,
        "writer_result":writer_result.as_ref().map(|report| json!({"bytes":report.bytes,"frames":report.frames,
            "write_ns":report.write_time.as_nanos(),"flush_ns":report.flush_time.as_nanos(),"io_ns":report.io_time.as_nanos(),"encoded_queue_peak":report.encoded_queue_peak,"encoded_bytes":report.encoded_bytes,"frame_sha256":report.frame_hashes})),"complete":complete});
    let penumbra = driver.host().quality_knobs();
    if let Some(knob) = penumbra.iter().find(|knob| knob.name == "Sun penumbra") {
        result["sun_penumbra"] = json!({
            "enabled": knob.value != 0,
            "settle_frames": penumbra.iter().find(|knob| knob.name == "Sun penumbra settle length").map(|knob| knob.value),
            "dive_limit_degrees": penumbra.iter().find(|knob| knob.name == "Sun penumbra dive limit").map(|knob| knob.value),
            "rim_rays": penumbra.iter().find(|knob| knob.name == "Sun penumbra rim rays").map(|knob| knob.value),
            "history_bytes": driver.host().sun_penumbra_history_bytes(),
        });
    }
    if let Some(knob) = penumbra
        .iter()
        .find(|knob| knob.name == "Ambient occlusion extent")
    {
        result["ambient_occlusion"] = json!({
            "extent": knob.value,
            "settle_frames": penumbra.iter().find(|knob| knob.name == "Ambient occlusion settle length").map(|knob| knob.value),
        });
    }
    result["display_phase_counts"] = phase_counts;
    result["gpu_pass_timing"] = json!(governed || measure_passes);
    if governed {
        result["governor"] = json!(quality_trace);
    }
    if let Some(saves) = sheet_saves {
        result["sheet_saves"] = json!(saves.join().map_err(|_| "sheet-save thread panicked")??);
    }
    result["observer_only"] = json!(observer_only);
    result["world_overload_recoveries"] = json!(trace.world_overload_recoveries);
    result["interval_complete"] = json!(interval_complete);
    write_runtime_report(
        output,
        result,
        &driver,
        &capabilities,
        &admission,
        parity_checkpoint,
    )
}

fn is_motion_capture(scenario: &str) -> bool {
    weather_motion(scenario)
        || matches!(
            scenario,
            "exit-route"
                | "sprites-status"
                | "sprites-sheets"
                | "sun-settle"
                | "sun-cut"
                | "sun-motion"
                | "exit-drift"
                | "content-edits"
                | "atmosphere-scrub"
                | "atmosphere-ground-pan"
        )
}

fn weather_motion(scenario: &str) -> bool {
    matches!(
        scenario,
        "clouds-inspect"
            | "clouds-wind"
            | "clouds-cycle"
            | "clouds-light-cycle"
            | "clouds-life"
            | "clouds-life-fast"
    )
}

fn capture_game(
    output: &Path,
    scenario: &str,
    capture_output: RuntimeCaptureOutput,
) -> Result<(HostedGame<WindowGame>, Option<super::ContentPreviewCapture>), String> {
    let mut cooked = super::CaptureCooked::new(output, capture_output);
    let preview = (scenario == "content-edits").then(super::ContentPreviewCapture::default);
    cooked.preview.clone_from(&preview);
    let game = super::capture_game_with_route_limit(
        matches!(scenario, "clouds-route" | "exit-route"),
        Some(cooked),
    )?;
    Ok((game, preview))
}

fn write_runtime_report(
    output: &Path,
    mut result: Value,
    driver: &Offscreen<HostedGame<WindowGame>>,
    capabilities: &DeviceCapabilities,
    admission: &admission::Admission,
    parity_checkpoint: bool,
) -> Result<(), String> {
    result["admission_policy"] = json!(admission.policy);
    result["admission_waits"] = json!(admission.waits);
    result["admission_skipped_opportunities"] = json!(admission.skipped);
    result["admission_diagnostic"] = admission.diagnostic();
    result["max_sampled_textures_per_shader_stage"] =
        json!(capabilities.limits().max_sampled_textures_per_shader_stage);
    let setup = crate::selected_setup()?;
    result["diagnostic_clouds"] = json!(setup.diagnostic_clouds);
    if setup.clouds != crate::CloudSelection::Off {
        result["cloud_selection"] = json!({
            "mode": format!("{:?}", setup.clouds),
            "coverage": setup.cloud_weather.coverage,
            "kind": setup.cloud_weather.kind,
            "resource_bytes": driver.host().cloud_resource_bytes(),
        });
    }
    if parity_checkpoint {
        let host = driver.host();
        result["renderer_signature"] = json!({
            "shader": host.scene_shader_source(),
            "graph": format!("{:?}", host.scene_graph().map_err(|err| err.to_string())?),
        });
    }
    let (scene_kind, present_target_bytes, storage) = driver.scene_storage();
    if scene_kind == thrum_render::runtime::SceneKind::TwoD {
        result["scene_kind"] = json!("TwoD");
        result["scene_pass_names"] = json!(
            driver
                .scene_pass_names()
                .map_err(|error| error.to_string())?
        );
        result["present_target_bytes"] = json!(present_target_bytes);
        result["storage"] = json!(storage);
        if let Some(stats) = driver.host().source().sprite_sheet_stats()
            && let Ok(stats) = stats.lock()
        {
            result["cooked_sheets"] = json!({
                "drawn": stats.drawn,
                "errors": stats.errors,
                "swaps": stats.swaps.iter().map(|swap| json!({
                    "generation": swap.generation,
                    "uploaded_bytes": swap.uploaded_bytes,
                    "upload_frames": swap.upload_frames,
                })).collect::<Vec<_>>(),
            });
        }
    }
    if !capabilities.hardware().is_empty() {
        result["hardware"] = json!(
            capabilities
                .hardware()
                .iter()
                .map(|entry| json!({
                    "name": entry.request.feature.name(), "required": entry.request.required,
                    "enabled": entry.declined.is_none(), "declined": entry.declined,
                }))
                .collect::<Vec<_>>()
        );
    }
    let reconstruction = driver.host().reconstruction();
    result["reconstruction"] = json!(format!("{reconstruction:?}"));
    if reconstruction == thrum_render::synthetic::Reconstruction::MetalFx {
        result["exposure_path"] = json!("exposure-texture");
    }
    if let Some(provider) = driver.host().native_provider_report() {
        result["native_provider"] = json!(format!("{provider:?}"));
    }
    if let Some(native) = driver.native_witness_report() {
        result["native_witness"] = json!({"selection":format!("{:?}",native.selection),"encoded":native.encoded,"submitted":native.submitted,"discarded":native.discarded,"resets":native.resets,"completed":native.completed,"resident_bytes":native.resident_bytes});
    }
    std::fs::write(
        output.join("runtime.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn relative_ns(origin: Instant, stamp: Instant) -> i128 {
    if stamp >= origin {
        i128::try_from(stamp.duration_since(origin).as_nanos())
            .expect("bounded session time fits i128")
    } else {
        -i128::try_from(origin.duration_since(stamp).as_nanos())
            .expect("bounded warmup time fits i128")
    }
}

fn enable_stage_capture<G: Game>(
    driver: &mut Offscreen<G>,
    session: u64,
    source: CaptureSource,
) -> Result<(), String> {
    let bytes_per_pixel = if source == CaptureSource::Sdr {
        4_u64
    } else {
        8
    };
    let image_bytes =
        u64::from(driver.output().width()) * u64::from(driver.output().height()) * bytes_per_pixel;
    let allowance = 1024 * 1024 * 1024;
    let capacity = (allowance / image_bytes).saturating_sub(7);
    let requested = if source == CaptureSource::Sdr { 96 } else { 32 };
    let cpu_buffers = usize::try_from(capacity.min(requested)).map_err(|e| e.to_string())?;
    driver
        .enable_capture_from(
            session,
            thrum_render::runtime::CaptureLimits {
                staging_slots: 6,
                cpu_buffers,
                byte_allowance: allowance,
            },
            source,
        )
        .map_err(String::from)
}

fn reset_native_capture<G: Game>(driver: &mut Offscreen<G>) {
    // Warmup has a variable submission count. Begin the labelled diagnostic at
    // a reset-relative phase without changing the ordinary scene capture path.
    if driver.native_witness_report().is_some() {
        driver.invalidate_history();
    }
}

async fn metal_device(
    instance: &wgpu::Instance,
    generation: u64,
    hardware: &[super::HardwareRequest],
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue, DeviceCapabilities), String> {
    let OpenedDevice {
        adapter,
        device,
        queue,
        capabilities,
        ..
    } = DeviceRecipe {
        generation,
        hardware,
        ..Default::default()
    }
    .open(instance, None)
    .await
    .map_err(|error| error.to_string())?;
    Ok((adapter, device, queue, capabilities))
}

fn metal_instance() -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    wgpu::Instance::new(descriptor)
}

const SUNSET_START_HOURS: f32 = 17.45;

fn lighting_edits(
    driver: &mut Offscreen<HostedGame<WindowGame>>,
    scenario: &str,
    scenario_time: Duration,
    night_variant: &mut bool,
    trace: &mut Trace,
    origin: Instant,
    haze_take: Option<HazeTake>,
) -> Result<(), String> {
    if scenario == "sun-cut" {
        let phase = scenario_time.as_secs_f64();
        let (yaw, cut) = if phase >= 2.5 && !*night_variant {
            *night_variant = true;
            (-50.0_f32, true)
        } else if (1.0..2.5).contains(&phase) {
            (42.0_f32, false)
        } else {
            return Ok(());
        };
        let host = driver.host_mut();
        if (host.camera_orientation()[1] - yaw.to_radians()).abs() > 1e-6 {
            host.set_camera_yaw(yaw.to_radians())?;
            if cut {
                host.invalidate_history();
            }
            trace
                .controls
                .push(json!({"kind":if cut {"sun-cut"} else {"sun-snap"},
                "scenario_ns":scenario_time.as_nanos(),"accepted_ns":origin.elapsed().as_nanos(),
                "yaw_degrees":yaw}));
        }
        return Ok(());
    }
    if scenario == "sun-set" {
        let progress = (scenario_time.as_secs_f32() / 4.0).clamp(0.0, 1.0);
        let solar = SUNSET_START_HOURS + 0.65 * progress;
        let host = driver.host_mut();
        crate::host::scrub_haze_sun(host, solar / 24.0)?;
        trace.controls.push(
            json!({"kind":"sun-set","scenario_ns":scenario_time.as_nanos(),
            "accepted_ns":origin.elapsed().as_nanos(),"solar_hours":crate::host::time_of_day(host) * 24.0}),
        );
        return Ok(());
    }
    if matches!(scenario, "atmosphere-scrub" | "atmosphere-ground-pan") {
        let t = scenario_time.as_secs_f32();
        if let Some(take) = haze_take {
            let pose = take.at(t);
            let host = driver.host_mut();
            crate::host::set_haze_eye(host, pose.eye, pose.cut)?;
            crate::host::scrub_haze_sun(host, pose.solar_hours / 24.0)?;
            host.set_camera_yaw(pose.yaw_degrees.to_radians())?;
            trace.controls.push(
                json!({"kind": "haze", "scenario_ns": scenario_time.as_nanos(), "pose": pose}),
            );
            return Ok(());
        }
        let (solar, yaw) = if scenario == "atmosphere-scrub" {
            let progress = ((t - 1.0) / 8.0).clamp(0.0, 1.0);
            (
                5.0 + 19.0 * progress,
                -45.0 + 22.5 * (std::f32::consts::TAU * progress).sin(),
            )
        } else {
            let progress = ((t - 1.0) / 5.0).clamp(0.0, 1.0);
            (17.4 + 0.4 * progress, -110.0 + 40.0 * progress)
        };
        let host = driver.host_mut();
        host.set_time_of_day(solar / 24.0)?;
        host.set_camera_yaw(yaw.to_radians())?;
        trace.controls.push(
            json!({"kind":scenario,"scenario_ns":scenario_time.as_nanos(),
            "accepted_ns":origin.elapsed().as_nanos(),"solar_hours":host.time_of_day()*24.0,
            "yaw_degrees":host.camera_orientation()[1].to_degrees()}),
        );
        return Ok(());
    }
    let t = scenario_time.as_secs_f64();
    let night = (1.0..3.0).contains(&t);
    let solar = if scenario == "variants" && night != *night_variant {
        *night_variant = night;
        Some(if night { 0.0 } else { 12.0 })
    } else if scenario == "clouds-dusk" && t >= 1.0 {
        let elevation = 3.0 - 21.0 * ((t - 1.0) / 12.0).clamp(0.0, 1.0);
        let hour_angle = (elevation.to_radians().sin() * std::f64::consts::SQRT_2).acos();
        Some(12.0 + hour_angle.to_degrees() / 15.0)
    } else if scenario == "reconstruction-scrub" && t >= 1.0 {
        Some(6.0 + 12.0 * ((t - 1.0) / 2.0).clamp(0.0, 1.0))
    } else {
        None
    };
    if let Some(solar) = solar {
        driver
            .parameters(&json!({"sky.solar_time":solar}).to_string())
            .map_err(|error| error.to_string())?;
        trace.controls.push(json!({"kind":if scenario == "variants" {"prepared-star-variant-switch"} else {"solar-scrub"},"scenario_ns":scenario_time.as_nanos(),"accepted_ns":origin.elapsed().as_nanos(),"solar_hours":solar}));
    }
    Ok(())
}

fn drain_fixed<G: Game>(
    driver: &mut Offscreen<G>,
    trace: &mut Trace,
    writer: Option<&ImageWriter>,
    origin: Instant,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let expected = trace
        .opportunities
        .iter()
        .filter(|row| row["submitted"] == true)
        .count();
    while trace.images.len() < expected {
        trace.drain(driver, writer, origin, true)?;
        if Instant::now() >= deadline {
            return Err("fixed sample readback deadline".to_owned());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn sprite_motion(scenario: &str) -> bool {
    matches!(
        scenario,
        "sprites-field" | "sprites-scroll" | "sprites-sheets"
    )
}

fn held_sample(scenario: &str) -> bool {
    matches!(scenario, "clouds-still" | "checkpoint")
}

fn scenario_bounds(scenario: &str, recovery: super::NativeRecovery) -> (f64, f64, f64) {
    if recovery != super::NativeRecovery::Disabled {
        return (24.0, 20.0, 3.0);
    }
    match scenario {
        "sprites-field-cost" => (11.5, 10.0, 10.0),
        "sprites-status" => (6.75, 5.75, 5.75),
        "sprites-scroll" => (5.5, 4.75, 4.25),
        "sprites-sheets" => (7.25, 6.25, 6.0),
        "clouds-drift" => (22.5, 22.0, 21.0),
        "clouds-wind" => (22.5, 22.0, 22.0),
        "clouds-cycle" | "clouds-life" => (244.0, 243.0, 243.0),
        "clouds-light-cycle" => (126.0, 125.0, 125.0),
        "clouds-life-fast" => (48.0, 47.0, 47.0),
        "clouds-placed" => (31.1, 31.0, 31.0),
        "clouds-dusk" => (14.5, 14.0, f64::INFINITY),
        "clouds-route" => (34.5, 0.0, f64::INFINITY),
        "exit-route" => (32.0, 0.0, f64::INFINITY),
        "sun-settle" | "sun-cut" | "sun-motion" => (5.0, 4.0, 4.0),
        "exit-drift" | "sprites-field" => (25.0, 23.0, 23.0),
        "content-edits" => (13.0, 12.0, 12.0),
        "atmosphere-scrub" => (11.0, 10.0, 0.0),
        "atmosphere-ground-pan" => (8.0, 7.0, 0.0),
        "route" => (8.4, 8.0, f64::INFINITY),
        _ => (6.0, 4.0, 3.0),
    }
}

pub(super) fn capture_extent() -> Result<[u32; 2], String> {
    let Ok(value) = crate::content::launch_value("THRUM_CAPTURE_EXTENT") else {
        return Ok([1920, 1080]);
    };
    let (width, height) = value
        .split_once(',')
        .ok_or("capture extent must be width,height")?;
    let extent = [
        width.parse::<u32>().map_err(|e| e.to_string())?,
        height.parse::<u32>().map_err(|e| e.to_string())?,
    ];
    if extent.iter().any(|n| !(2..=4096).contains(n) || n % 2 != 0) {
        return Err("capture extent must be even dimensions in 2..4096".into());
    }
    Ok(extent)
}

fn capture_mode(scenario: &str) -> Result<String, String> {
    let mode = std::env::var("THRUM_CAPTURE_MODE").unwrap_or_else(|_| "realtime".to_owned());
    if !matches!(mode.as_str(), "realtime" | "fixed" | "still") {
        return Err("invalid capture mode".to_owned());
    }
    if matches!(scenario, "exit-stop" | "checkpoint") && mode != "fixed" {
        return Err(format!("a {scenario} capture uses fixed mode"));
    }
    if mode != "realtime"
        && (weather_motion(scenario)
            || matches!(
                scenario,
                "running"
                    | "writer-slow"
                    | "render-stall"
                    | "input-stall"
                    | "content-edits"
                    | "route"
                    | "clouds-route"
                    | "clouds-placed"
                    | "clouds-drift"
                    | "clouds-dusk"
                    | "exit-route"
                    | "exit-drift"
                    | "sprites-field"
                    | "sprites-field-cost"
                    | "atmosphere-scrub"
                    | "atmosphere-ground-pan"
                    | "sprites-scroll"
                    | "sprites-sheets"
            ))
    {
        return Err("running-world and diagnostic controls require actual elapsed time".to_owned());
    }
    if mode != "realtime" && scenario == "off" {
        return Err("capture-off observer row requires elapsed time".to_owned());
    }
    if mode != "realtime" && scenario.starts_with("reconstruction-") {
        return Err("reconstruction motion witnesses require actual elapsed time".to_owned());
    }
    Ok(mode)
}

pub(super) fn prepare_route(
    host: &mut crate::host::Runtime,
    content: Option<std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>>,
    text: &str,
    render_extent: [u32; 2],
    gpu: &thrum_app::Gpu<'_>,
    cloud_capture: bool,
    report_folder: Option<&Path>,
) -> Result<Duration, String> {
    #[cfg(not(feature = "draw-probe"))]
    let _ = report_folder;
    let camera = host
        .scene_camera()
        .ok_or("routed capture needs the cooked valley camera")?;
    let tick_hz = thrum_core::ClockConfig::default().tick_hz.get();
    let route = if cloud_capture {
        crate::CameraRoute::parse_long_capture(text, camera, tick_hz)
            .map_err(|error| error.to_string())
    } else {
        crate::CameraRoute::parse(text, camera, tick_hz)
    }?;
    if route.render_extent != render_extent {
        return Err("runtime route extent differs from the prepared game grid".to_owned());
    }
    if cloud_capture && crate::selected_setup()?.clouds != crate::CloudSelection::Off {
        host.set_scene_camera(camera.at_route_key(route.keys[0])?)?;
    }
    let stats = content.ok_or("routed capture needs cooked content")?;
    // The native provider retains the ordinary presentation contract during
    // preparation too. Use an engine texture as the headless surface consumer;
    // never submit native work without its prepared presenter.
    let preparation_output = host.native_provider_report().map(|_| {
        gpu.device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("native route preparation virtual surface"),
                size: wgpu::Extent3d {
                    width: gpu.surface.extent[0],
                    height: gpu.surface.extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: gpu.surface.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    });
    host.world_mut().pause()?;
    let generation = if let Some(view) = &preparation_output {
        thrum_asset_gpu::await_drawn_on_surface(
            host,
            &stats,
            thrum_asset_gpu::Drawn::Any,
            &thrum_render::runtime::SurfaceOutput {
                view,
                extent: gpu.surface.extent,
            },
        )?
    } else {
        thrum_asset_gpu::await_drawn(host, &stats, thrum_asset_gpu::Drawn::Any)?
    };
    if generation != route.cooked_generation {
        return Err("routed capture generation mismatch".to_owned());
    }
    #[cfg(feature = "draw-probe")]
    if let Some(output) = report_folder {
        let stats = stats
            .lock()
            .map_err(|_| "routed capture content observations poisoned")?;
        let report = json!({
            "generation": generation,
            "recipe": format!("{:?}", stats.recipe),
            "requested_path": format!("{:?}", stats.requested_draw_path),
            "resolved_path": format!("{:?}", stats.draw_path),
            "fallback": stats.draw_fallback,
            "mesh_stage_draws": stats.mesh_stage_draws,
            "indexed_indirect_draws": stats.indexed_indirect_draws,
            "mesh_stage_exclusions": stats.mesh_stage_exclusions,
        });
        std::fs::write(output.join("cooked-path.json"), report.to_string())
            .map_err(|error| error.to_string())?;
    }
    host.world_mut().resume(Duration::ZERO)?;
    let mut elapsed = Duration::ZERO;
    while host.world().tick() < route.origin_tick - 1 {
        elapsed = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
        host.drive(elapsed, None)
            .map_err(|error| error.to_string())?;
    }
    host.world_mut().pause()?;
    host.set_camera_route(route)?;
    Ok(elapsed)
}
