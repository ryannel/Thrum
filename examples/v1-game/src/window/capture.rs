//! Authored V1 scenarios on the shared unattended engine driver.

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
        writer::{ImageWriter, SpoolEncoding},
    },
};
use thrum_render::{
    capabilities::DeviceCapabilities, runtime::SubmittedImage, synthetic::CaptureSource,
};

/// Run the ordinary game on a dedicated render-owner thread. Stdout is raw RGBA;
/// scripts own the sequential spool, packaging and finalized evidence manifest.
pub(super) fn run(output: &Path, scenario: &str) -> Result<(), String> {
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
            | "reconstruction-stars"
            | "reconstruction-scrub"
    ) {
        return Err("unknown runtime capture scenario".to_owned());
    }
    let output = output.to_owned();
    let scenario = scenario.to_owned();
    std::thread::Builder::new()
        .name("thrum-headless-render".to_owned())
        .spawn(move || pollster::block_on(record(&output, &scenario)))
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "capture render owner panicked".to_owned())?
}

#[derive(Default)]
struct Trace {
    opportunities: Vec<Value>,
    images: Vec<Value>,
    gpu: Vec<Value>,
    stalls: Vec<Value>,
    controls: Vec<Value>,
    writer_omissions: u64,
    input_delay: Option<Duration>,
    input_delay_finished: bool,
    route_last_tick: Option<u64>,
    route_finished_at: Option<Duration>,
}

impl Trace {
    fn for_route(route: Option<&Value>) -> Result<Self, String> {
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
            route_last_tick,
            ..Self::default()
        })
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
        let primed = driver.step()?;
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
        self.opportunities.push(
        json!({"id":0,"request_ns":relative_ns(origin,prime_requested),"accepted_ns":null,"scenario_ns":0,
        "submitted":true,"tick":primed.tick,"submission_id":driver.last_submission(),
        "requested_yaw_degrees":0.0,"accepted_yaw_degrees":accepted_yaw(driver)?,"primed":true}),
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
        driver.parameters(&json!({"world.paused":paused}).to_string())?;
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
        driver.parameters(&json!({"camera.yaw":yaw}).to_string())?;
        Ok(true)
    }
    fn image<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        writer: &ImageWriter,
        image: SubmittedImage,
        origin: Instant,
    ) {
        if self
            .route_last_tick
            .is_some_and(|tick| image.source.tick >= tick)
            && self.route_finished_at.is_none()
        {
            self.route_finished_at = Some(image.submitted_at.saturating_duration_since(origin));
        }
        self.images.push(json!({"submission_id":image.sample.submission_order,"tick":image.sample.tick,
            "epoch":image.source.epoch,"timeline":image.source.timeline,
            "previous_endpoint_tick":image.source.tick.checked_sub(1),"current_endpoint_tick":image.source.tick,
            "alpha":image.sample.presentation_alpha,"history_epoch":image.history_epoch,
            "pixel_source":format!("{:?}",image.pixel_source),"exposure_gpu_words":image.exposure_gpu_words,
            "visual_revision":image.sample.visual_revision,"device_generation":image.device_generation,"surface_generation":image.presenter_generation,
            "presentation_lead_ns":image.presentation_lead.as_nanos(),"readback_copy_gpu_bracket_ns":image.copy_gpu_ns,
            "submit_ns":image.submitted_at.saturating_duration_since(origin).as_nanos(),
            "map_observed_ns":origin.elapsed().as_nanos()}));
        if let Some(image) = writer.submit(image) {
            self.writer_omissions += 1;
            driver.recycle(image);
        }
    }

    fn drain<G: Game>(
        &mut self,
        driver: &mut Offscreen<G>,
        writer: Option<&ImageWriter>,
        origin: Instant,
    ) -> Result<(), String> {
        while let Some(sample) = driver.completed()? {
            self.gpu.push(
                json!({"submission_id":sample.submission_order,"tick":sample.tick,
                "encode_cpu_ns":sample.encode_cpu.as_nanos(),"upload_cpu_ns":sample.upload_cpu.as_nanos(),"extract_cpu_ns":sample.extract_cpu.as_nanos(),"gpu_pass_ns":sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::values),"render_cpu_ns":sample.render_cpu.as_nanos(),"wall_ns":sample.wall.as_nanos(),"tick_cpu_ns":sample.tick_cpu.as_nanos(),"gpu_present_ns":sample.gpu_present_ns}),
            );
        }
        if let Some(writer) = writer {
            while let Some(image) = writer.recycled() {
                driver.recycle(image);
            }
            while let Some(image) = driver.image()? {
                self.image(driver, writer, image, origin);
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
) -> Result<ImageWriter, String> {
    let output = std::fs::File::from(
        std::io::stdout()
            .as_fd()
            .try_clone_to_owned()
            .map_err(|e| e.to_string())?,
    );
    ImageWriter::start(
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
    )
}

fn warmup<G: Game>(driver: &mut Offscreen<G>) -> Result<(), String> {
    let warmup = Instant::now();
    while warmup.elapsed() < Duration::from_secs(3) {
        driver.step()?;
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
    Ok(if scenario == "route" {
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

fn prepare_parameters<G: Game>(
    driver: &mut Offscreen<G>,
    scenario: &str,
) -> Result<(Instant, Instant), String> {
    let preset_requested = Instant::now();
    let parameters = std::env::var_os("THRUM_CAPTURE_RUNTIME_PARAMS")
        .map(std::fs::read_to_string)
        .transpose()
        .map_err(|error| error.to_string())?;
    if scenario == "route" {
        driver.parameters(r#"{"world.paused":true}"#)?;
    } else {
        driver.parameters(
            parameters
                .as_deref()
                .unwrap_or(include_str!("../../params/showcase-noon.params.json")),
        )?;
    }
    let preset_accepted = Instant::now();
    warmup(driver)?;
    Ok((preset_requested, preset_accepted))
}

async fn record(output: &Path, scenario: &str) -> Result<(), String> {
    let mode = capture_mode(scenario)?;
    let source = match std::env::var("THRUM_CAPTURE_SOURCE").as_deref() {
        Ok("sdr") | Err(_) => CaptureSource::Sdr,
        Ok("scene-linear") => CaptureSource::SceneLinear,
        _ => return Err("unknown capture source".to_owned()),
    };
    let (qos, encoding, disk_limit) = capture_settings()?;
    let active_pacing = active_pacing()?;
    let (adapter, device, queue, capabilities) = metal_device().await?;
    let mut driver = Offscreen::new(
        super::capture_game()?,
        &device,
        &capabilities,
        &queue,
        [1920, 1080],
    )?;
    let (preset_requested, preset_accepted) = prepare_parameters(&mut driver, scenario)?;
    let capture = scenario != "off";
    if capture {
        driver.enable_capture_from(
            1,
            thrum_render::runtime::CaptureLimits {
                staging_slots: 6,
                cpu_buffers: if source == CaptureSource::Sdr { 96 } else { 32 },
                byte_allowance: 1024 * 1024 * 1024,
            },
            source,
        )?;
    }
    let writer = if capture {
        Some(start_writer(scenario, encoding, disk_limit)?)
    } else {
        None
    };
    let route = requested_route(scenario)?;
    let mut trace = Trace::for_route(route.as_ref())?;
    let origin = trace.prime(output, &mut driver, writer.as_ref())?;
    let timebase = if mode == "fixed" {
        Timebase::Fixed
    } else {
        Timebase::Elapsed
    };
    let mut clock = OpportunityClock::new(origin, 120, timebase)?;
    let mut running = false;
    let mut night_variant = false;
    let mut stalled = false;
    let mut final_hold = None;
    let mut last_sample_time = Duration::ZERO;
    loop {
        if mode == "still" {
            break;
        }
        let opportunity = if active_pacing {
            clock.next_active_opportunity()
        } else {
            clock.next_opportunity()
        };
        let t = opportunity.scenario_time.as_secs_f64();
        if mode == "realtime" && t >= if scenario == "route" { 8.4 } else { 6.0 } {
            break;
        }
        if (mode == "fixed" && opportunity.index > 480)
            || (mode == "realtime"
                && t >= if scenario == "route" { 8.0 } else { 4.0 }
                && if scenario == "route" {
                    trace.route_finished_at
                } else {
                    final_hold
                }
                .is_some_and(|start: Duration| origin.elapsed() >= start + Duration::from_secs(1)))
        {
            break;
        }
        let work = WorkClock::start().map_err(|e| e.to_string())?;
        let active_world = matches!(
            scenario,
            "running" | "writer-slow" | "route" | "reconstruction-pan"
        );
        if active_world && t >= 1.0 && (scenario == "route" || t < 3.0) && !running {
            trace.world_state(&mut driver, false, origin)?;
            running = true;
        } else if active_world && scenario != "route" && t >= 3.0 && running {
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
        )?;
        let yaw = ((t - 1.0) / 2.0).clamp(0.0, 1.0) * 36.0;
        let control_dispatched = (!active_world || scenario == "reconstruction-pan")
            && !matches!(scenario, "reconstruction-stars" | "reconstruction-scrub")
            && trace.yaw(&mut driver, scenario, t, yaw, origin)?;
        let accepted = origin.elapsed();
        let accepted_yaw = accepted_yaw(&driver)?;
        if scenario == "render-stall" && t >= 1.8 && !stalled {
            trace.delay("render-owner-before-ordinary-step", origin);
            stalled = true;
        }
        let step = WorkClock::start().map_err(|e| e.to_string())?;
        let report = driver.step_for(&opportunity)?;
        let step = step.finish().map_err(|e| e.to_string())?;
        let returned = origin.elapsed();
        if t >= 3.0 && report.submitted && final_hold.is_none() {
            final_hold = Some(origin.elapsed());
        }
        let drain = Instant::now();
        trace.drain(&mut driver, writer.as_ref(), origin)?;
        let drain = drain.elapsed();
        let work = work.finish().map_err(|e| e.to_string())?;
        trace.opportunities.push(json!({"step_cpu_start":step.cpu_start,"step_cpu_end":step.cpu_end,"loop_cpu_start":work.cpu_start,"loop_cpu_end":work.cpu_end,"step_thread_cpu_ns":step.cpu.as_nanos(),"step_wall_ns":step.wall.as_nanos(),"loop_thread_cpu_ns":work.cpu.as_nanos(),"loop_work_wall_ns":work.wall.as_nanos(),"drain_wall_ns":drain.as_nanos(),"id":opportunity.index,"request_ns":opportunity.requested.as_nanos(),
            "scenario_ns":opportunity.scenario_time.as_nanos(),"accepted_ns":if control_dispatched {Some(accepted.as_nanos())} else {None},
            "control_dispatched":control_dispatched,
            "skipped_before":opportunity.skipped_before,"skip_reason":"driver busy or host oversleep",
            "returned_ns":returned.as_nanos(),"requested_yaw_degrees":yaw,"accepted_yaw_degrees":accepted_yaw,
            "submitted":report.submitted,"tick":report.tick,"dropped_view":report.dropped_view,
            "submission_id":if report.submitted {driver.last_submission()} else {None},"capture_usage":driver.capture_usage()}));
        last_sample_time = opportunity.scenario_time;
        if mode == "fixed" {
            let deadline = Instant::now() + Duration::from_secs(10);
            let expected = trace
                .opportunities
                .iter()
                .filter(|row| row["submitted"] == true)
                .count();
            while trace.images.len() < expected {
                trace.drain(&mut driver, writer.as_ref(), origin)?;
                if Instant::now() >= deadline {
                    return Err("fixed sample readback deadline".to_owned());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    let interval_end_ns = origin.elapsed().as_nanos();
    let submitted = trace
        .opportunities
        .iter()
        .filter(|row| row["submitted"] == true)
        .count();
    let deadline = Instant::now() + Duration::from_secs(10);
    while capture && trace.images.len() < submitted && Instant::now() < deadline {
        trace.drain(&mut driver, writer.as_ref(), origin)?;
        std::thread::sleep(Duration::from_millis(1));
    }
    let writer_result = writer.map_or(
        Ok(thrum_app::offscreen::writer::WriterReport::default()),
        |writer| writer.finish(Duration::from_secs(10)),
    );
    let interval_complete = mode == "still"
        || (mode == "fixed" && last_sample_time >= Duration::from_secs(4))
        || (mode == "realtime"
            && if scenario == "route" {
                trace.route_finished_at
            } else {
                final_hold
            }
            .is_some_and(|start| origin.elapsed() >= start + Duration::from_secs(1)));
    let complete = interval_complete
        && writer_result.is_ok()
        && (!capture
            || (trace.images.len() == submitted
                && driver.omissions() == 0
                && trace.writer_omissions == 0));
    let mut result = json!({"active_pacing":active_pacing,"thread_qos_before":qos.before,"thread_qos_after":qos.after,"spool_encoding":format!("{encoding:?}"),"pixel_source":format!("{source:?}"),"disk_limit_Bps":disk_limit,
        "scenario":scenario,"adapter":adapter.get_info().name,"backend":"Metal","mode":mode,
        "presentation_lead_policy":"engine clock next opportunity target, shared display_target_lead at consumption; untargeted prime/warmup zero",
        "surface_generation_scope":"engine presenter incarnation: prepare, resize, device publish; not OS surface lease",
        "recipe":format!("{:?}",capabilities.recipe()),"registry_digest":capabilities.registry_digest(),"unified_memory":capabilities.unified_memory(),"enabled_features":format!("{:?}",capabilities.features()),
        "extent":[1920,1080],"requested_hz":120,"interval_end_ns":interval_end_ns,
        "last_scenario_ns":last_sample_time.as_nanos(),"duration_including_drain_ns":origin.elapsed().as_nanos(),
        "world_origin_before_run_ns":origin.saturating_duration_since(driver.origin()).as_nanos(),
        "route":route,
        "initial_controls":{"request_ns":relative_ns(origin,preset_requested),"accepted_ns":relative_ns(origin,preset_accepted)},
        "opportunities":trace.opportunities,"images":trace.images,"gpu":trace.gpu,"diagnostic_actions":trace.stalls,"control_actions":trace.controls,
        "ring_omissions":driver.omissions(),"writer_omissions":trace.writer_omissions,
        "writer_result":writer_result.as_ref().map(|report| json!({"bytes":report.bytes,"frames":report.frames,
            "write_ns":report.write_time.as_nanos(),"flush_ns":report.flush_time.as_nanos(),"io_ns":report.io_time.as_nanos(),"encoded_queue_peak":report.encoded_queue_peak,"encoded_bytes":report.encoded_bytes,"frame_sha256":report.frame_hashes})),"complete":complete});
    result["max_sampled_textures_per_shader_stage"] =
        json!(capabilities.limits().max_sampled_textures_per_shader_stage);
    result["diagnostic_clouds"] = json!(crate::selected_setup()?.diagnostic_clouds);
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

async fn metal_device()
-> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue, DeviceCapabilities), String> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|e| e.to_string())?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter)
                | DeviceCapabilities::requested_content_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    let capabilities = DeviceCapabilities::capture(&adapter, &device, 1);
    Ok((adapter, device, queue, capabilities))
}

fn lighting_edits<G: Game>(
    driver: &mut Offscreen<G>,
    scenario: &str,
    scenario_time: Duration,
    night_variant: &mut bool,
    trace: &mut Trace,
    origin: Instant,
) -> Result<(), String> {
    let t = scenario_time.as_secs_f64();
    let night = (1.0..3.0).contains(&t);
    let solar = if scenario == "variants" && night != *night_variant {
        *night_variant = night;
        Some(if night { 0.0 } else { 12.0 })
    } else if scenario == "reconstruction-scrub" && t >= 1.0 {
        Some(6.0 + 12.0 * ((t - 1.0) / 2.0).clamp(0.0, 1.0))
    } else {
        None
    };
    if let Some(solar) = solar {
        driver.parameters(&json!({"sky.solar_time":solar}).to_string())?;
        trace.controls.push(json!({"kind":if scenario == "variants" {"prepared-star-variant-switch"} else {"solar-scrub"},"scenario_ns":scenario_time.as_nanos(),"accepted_ns":origin.elapsed().as_nanos(),"solar_hours":solar}));
    }
    Ok(())
}

fn capture_mode(scenario: &str) -> Result<String, String> {
    let mode = std::env::var("THRUM_CAPTURE_MODE").unwrap_or_else(|_| "realtime".to_owned());
    if !matches!(mode.as_str(), "realtime" | "fixed" | "still") {
        return Err("invalid capture mode".to_owned());
    }
    if mode != "realtime"
        && matches!(
            scenario,
            "running" | "writer-slow" | "render-stall" | "input-stall" | "route"
        )
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
    host: &mut crate::FrameHost,
    content: Option<std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>>,
    text: &str,
    render_extent: [u32; 2],
) -> Result<Duration, String> {
    let camera = host
        .scene_camera()
        .ok_or("routed capture needs the cooked valley camera")?;
    let route = crate::CameraRoute::parse(
        text,
        camera,
        thrum_core::ClockConfig::default().tick_hz.get(),
    )?;
    if route.render_extent != render_extent {
        return Err("runtime route extent differs from the prepared game grid".to_owned());
    }
    let stats = content.ok_or("routed capture needs cooked content")?;
    host.pause()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        {
            let stats = stats
                .lock()
                .map_err(|_| "routed capture content observations poisoned")?;
            if !stats.errors.is_empty() {
                return Err(format!("routed capture content failed: {:?}", stats.errors));
            }
            if let Some(generation) = stats.drawn {
                if generation != route.cooked_generation {
                    return Err("routed capture generation mismatch".to_owned());
                }
                break;
            }
        }
        if Instant::now() >= deadline {
            return Err("routed capture content preparation deadline".to_owned());
        }
        drop(host.capture_pixels()?);
        std::thread::sleep(Duration::from_millis(2));
    }
    host.resume(Duration::ZERO)?;
    let mut elapsed = Duration::ZERO;
    while host.tick() < route.origin_tick - 1 {
        elapsed = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
        host.drive(elapsed, None)?;
    }
    host.pause()?;
    host.set_camera_route(route)?;
    Ok(elapsed)
}
