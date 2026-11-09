//! Authored V1 scenarios on the shared unattended engine driver.

use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use thrum_app::{
    Game,
    offscreen::{
        Offscreen,
        clock::{OpportunityClock, Timebase},
        writer::ImageWriter,
    },
};
use thrum_render::{capabilities::DeviceCapabilities, runtime::SubmittedImage};

/// Run the ordinary game on a dedicated render-owner thread. Stdout is raw RGBA;
/// scripts own the sequential spool, packaging and finalized evidence manifest.
pub(super) fn run(output: &Path, scenario: &str) -> Result<(), String> {
    if !matches!(
        scenario,
        "pan" | "running" | "render-stall" | "input-stall" | "off" | "writer-slow"
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
    writer_omissions: u64,
    input_delay: Option<Duration>,
    input_delay_finished: bool,
}

impl Trace {
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
        self.images.push(json!({"submission_id":image.sample.submission_order,"tick":image.sample.tick,
            "epoch":image.source.epoch,"timeline":image.source.timeline,
            "previous_endpoint_tick":image.source.tick.checked_sub(1),"current_endpoint_tick":image.source.tick,
            "alpha":image.sample.presentation_alpha,"history_epoch":image.history_epoch,
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
                "gpu_pass_ns":sample.gpu_pass_ns,"render_cpu_ns":sample.render_cpu.as_nanos()}),
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
    slow: bool,
}
impl std::io::Write for ScenarioSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.slow {
            std::thread::sleep(Duration::from_millis(50));
        }
        std::io::Write::write(&mut std::io::stdout(), bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut std::io::stdout())
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

fn warmup<G: Game>(driver: &mut Offscreen<G>) -> Result<(), String> {
    let warmup = Instant::now();
    while warmup.elapsed() < Duration::from_secs(3) {
        driver.step()?;
        while driver.completed()?.is_some() {}
        std::thread::sleep(Duration::from_millis(8));
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while driver.pending_frames() != 0 {
        let _ = driver.completed()?;
        if Instant::now() >= deadline {
            return Err("warmup drain timeout".to_owned());
        }
    }
    Ok(())
}

async fn record(output: &Path, scenario: &str) -> Result<(), String> {
    let mode = capture_mode(scenario)?;
    let (adapter, device, queue, capabilities) = metal_device().await?;
    let mut driver = Offscreen::new(
        super::capture_game()?,
        &device,
        &capabilities,
        &queue,
        [1920, 1080],
    )?;
    let preset_requested = Instant::now();
    driver.parameters(include_str!("../../params/showcase-noon.params.json"))?;
    let preset_accepted = Instant::now();
    warmup(&mut driver)?;
    let capture = scenario != "off";
    if capture {
        driver.enable_capture_with_limits(
            1,
            thrum_render::runtime::CaptureLimits {
                staging_slots: 6,
                cpu_buffers: 96,
                byte_allowance: 1024 * 1024 * 1024,
            },
        )?;
    }
    let writer = if capture {
        Some(ImageWriter::start(
            ScenarioSink {
                slow: scenario == "writer-slow",
            },
            96,
            8 * 1024 * 1024 * 1024,
        )?)
    } else {
        None
    };
    let mut trace = Trace::default();
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
    if capture {
        let deadline = origin + Duration::from_secs(10);
        loop {
            let _ = driver.completed()?;
            if let Some(image) = driver.image()? {
                origin = image.submitted_at;
                trace.image(
                    &mut driver,
                    writer.as_ref().ok_or("missing capture writer")?,
                    image,
                    origin,
                );
                break;
            }
            if Instant::now() >= deadline {
                return Err("initial image deadline".to_owned());
            }
        }
    }
    trace.opportunities.push(
        json!({"id":0,"request_ns":relative_ns(origin,prime_requested),"accepted_ns":null,"scenario_ns":0,
        "submitted":true,"tick":primed.tick,"submission_id":driver.last_submission(),
        "requested_yaw_degrees":0.0,"accepted_yaw_degrees":accepted_yaw(&driver)?,"primed":true}),
    );
    let timebase = if mode == "fixed" {
        Timebase::Fixed
    } else {
        Timebase::Elapsed
    };
    let mut clock = OpportunityClock::new(origin, 120, timebase)?;
    let mut running = false;
    let mut stalled = false;
    let mut final_hold = None;
    let mut last_sample_time = Duration::ZERO;
    loop {
        if mode == "still" {
            break;
        }
        let opportunity = clock.next_opportunity();
        let t = opportunity.scenario_time.as_secs_f64();
        if mode == "realtime" && t >= 6.0 {
            break;
        }
        if (mode == "fixed" && opportunity.index > 480)
            || (mode == "realtime"
                && t >= 4.0
                && final_hold.is_some_and(|start: Duration| {
                    origin.elapsed() >= start + Duration::from_secs(1)
                }))
        {
            break;
        }
        let active_world = matches!(scenario, "running" | "writer-slow");
        if active_world && (1.0..3.0).contains(&t) && !running {
            driver.parameters(r#"{"world.paused":false}"#)?;
            running = true;
        } else if active_world && t >= 3.0 && running {
            driver.parameters(r#"{"world.paused":true}"#)?;
            running = false;
        }
        let yaw = ((t - 1.0) / 2.0).clamp(0.0, 1.0) * 36.0;
        let control_dispatched =
            !active_world && trace.yaw(&mut driver, scenario, t, yaw, origin)?;
        let accepted = origin.elapsed();
        let accepted_yaw = accepted_yaw(&driver)?;
        if scenario == "render-stall" && t >= 1.8 && !stalled {
            trace.delay("render-owner-before-ordinary-step", origin);
            stalled = true;
        }
        let report = driver.step_for(&opportunity)?;
        if t >= 3.0 && report.submitted && final_hold.is_none() {
            final_hold = Some(origin.elapsed());
        }
        trace.opportunities.push(json!({"id":opportunity.index,"request_ns":opportunity.requested.as_nanos(),
            "scenario_ns":opportunity.scenario_time.as_nanos(),"accepted_ns":if control_dispatched {Some(accepted.as_nanos())} else {None},
            "control_dispatched":control_dispatched,
            "skipped_before":opportunity.skipped_before,"skip_reason":"driver busy or host oversleep",
            "returned_ns":origin.elapsed().as_nanos(),"requested_yaw_degrees":yaw,"accepted_yaw_degrees":accepted_yaw,
            "submitted":report.submitted,"tick":report.tick,"dropped_view":report.dropped_view,
            "submission_id":if report.submitted {driver.last_submission()} else {None},"capture_usage":driver.capture_usage()}));
        trace.drain(&mut driver, writer.as_ref(), origin)?;
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
            && final_hold.is_some_and(|start| origin.elapsed() >= start + Duration::from_secs(1)));
    let complete = interval_complete
        && writer_result.is_ok()
        && (!capture
            || (trace.images.len() == submitted
                && driver.omissions() == 0
                && trace.writer_omissions == 0));
    let result = json!({"scenario":scenario,"adapter":adapter.get_info().name,"backend":"Metal","mode":mode,
        "presentation_lead_policy":"engine clock next opportunity target, shared display_target_lead at consumption; untargeted prime/warmup zero",
        "surface_generation_scope":"engine presenter incarnation: prepare, resize, device publish; not OS surface lease",
        "recipe":"current portable wgpu Metal offscreen ordinary presenter","enabled_features":format!("{:?}",capabilities.features()),
        "extent":[1920,1080],"requested_hz":120,"interval_end_ns":interval_end_ns,
        "last_scenario_ns":last_sample_time.as_nanos(),"duration_including_drain_ns":origin.elapsed().as_nanos(),
        "world_origin_before_run_ns":origin.saturating_duration_since(driver.origin()).as_nanos(),
        "initial_controls":{"request_ns":relative_ns(origin,preset_requested),"accepted_ns":relative_ns(origin,preset_accepted)},
        "opportunities":trace.opportunities,"images":trace.images,"gpu":trace.gpu,"diagnostic_actions":trace.stalls,
        "ring_omissions":driver.omissions(),"writer_omissions":trace.writer_omissions,
        "writer_result":writer_result.as_ref().map(|report| json!({"bytes":report.bytes,"frames":report.frames,
            "write_ns":report.write_time.as_nanos(),"flush_ns":report.flush_time.as_nanos()})),"complete":complete});
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

fn capture_mode(scenario: &str) -> Result<String, String> {
    let mode = std::env::var("THRUM_CAPTURE_MODE").unwrap_or_else(|_| "realtime".to_owned());
    if !matches!(mode.as_str(), "realtime" | "fixed" | "still") {
        return Err("invalid capture mode".to_owned());
    }
    if mode != "realtime"
        && matches!(
            scenario,
            "running" | "writer-slow" | "render-stall" | "input-stall"
        )
    {
        return Err("running-world and diagnostic controls require actual elapsed time".to_owned());
    }
    if mode != "realtime" && scenario == "off" {
        return Err("capture-off observer row requires elapsed time".to_owned());
    }
    Ok(mode)
}
