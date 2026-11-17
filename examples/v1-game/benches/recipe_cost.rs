//! Capture-OFF/ON cost rows through the real 10k game facade, on Metal only.
//! Run under the orchestrate GPU lock; no presentation or speed qualification.
use std::{
    io::Write,
    time::{Duration, Instant},
};
use thrum_render::{capabilities::DeviceCapabilities, synthetic::CullMode};
use v1_game::{FrameConfig, FrameHost, FrameSample};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}

fn number(name: &str, default: u64) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(std::env::var(name).map_or(Ok(default), |v| v.parse())?)
}

fn complete(
    host: &mut FrameHost,
    device: &wgpu::Device,
) -> Result<FrameSample, Box<dyn std::error::Error>> {
    loop {
        if let Some(sample) = host.poll_completed()? {
            return Ok(sample);
        }
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })?;
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let extent = [
        u32::try_from(number("RECIPE_WIDTH", 1280)?)?,
        u32::try_from(number("RECIPE_HEIGHT", 720)?)?,
    ];
    let seconds = number("RECIPE_SECONDS", 0)?;
    let samples = number("RECIPE_SAMPLES", 180)?;
    let capture = number("RECIPE_CAPTURE", 0)? != 0;
    let warmup = number("RECIPE_WARMUP", 120)?;
    let gpu = thrum_test_support::gpu("integrated recipe cost", Some(wgpu::Backend::Metal))
        .ok_or("Metal is required for this cost record")?;
    let adapter = &gpu.adapter;
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(adapter, &device, 1);
    let mut setup = v1_game::selected_setup()?;
    setup.camera_culling = match std::env::var("RECIPE_CULL").as_deref() {
        Ok("frustum") => CullMode::Frustum,
        Ok("occlusion") => CullMode::Occlusion,
        _ => CullMode::Direct,
    };
    let mut host = FrameHost::new_with_setup(
        &device,
        &queue,
        &caps,
        FrameConfig {
            render_extent: extent,
            surface: None,
        },
        setup,
    )
    .await?;
    let cooked = v1_game::attach_cooked_scene(&mut host)?;
    if capture {
        host.enable_submitted_capture(1)?;
    }
    let mut output = std::io::BufWriter::new(std::io::stdout().lock());
    writeln!(
        output,
        "{}",
        serde_json::json!({"header":true,"adapter":format!("{:?}",adapter.get_info()),
        "registry":caps.registry_digest(),"recipe":format!("{:?}",caps.recipe()),
        "extent":extent,"capture":capture,"culling":format!("{:?}",setup.camera_culling),
        "warmup":warmup,"minimum_seconds":seconds,"minimum_samples":samples,
        "storage":host.storage_report(),"resident_prepared":caps.resident_device_bytes(),
        "pipelines_prepared":host.pipeline_creation_count(),
        "cpu_scope":"upload includes view/light validation, packing and queue writes; encode includes validation/finish/submit; no independent validation timer",
        "observer_scope":"ON drains submitted readback without compression or filesystem writer; OFF has no tap",
        "clock":"synthetic 60Hz authoritative ticks, unpaced GPU completion; duration is monotonic host wall"})
    )?;
    let mut tick = 0_u64;
    let mut measured = 0_u64;
    let mut start = Instant::now();
    loop {
        if tick == warmup {
            start = Instant::now();
        }
        // Repeat the same bounded visibility stimulus on both recipes.
        host.set_camera_yaw(if tick % 240 < 120 { 0.0 } else { 0.6 })?;
        let drive_start = Instant::now();
        let report = host.drive(Duration::from_nanos((tick + 1) * 16_666_667 + 1), None)?;
        if !report.submitted {
            return Err("cost drive did not submit a frame".into());
        }
        let drive_ns = drive_start.elapsed().as_nanos();
        let sample = complete(&mut host, &device)?;
        if capture {
            loop {
                if let Some(image) = host.try_submitted_image()? {
                    host.recycle_submitted_image(image);
                    break;
                }
                device.poll(wgpu::PollType::wait_indefinitely())?;
            }
        }
        if tick >= warmup {
            measured += 1;
            let passes = sample.gpu_pass_ns.as_ref().map(|t| t.values());
            writeln!(
                output,
                "{}",
                serde_json::json!({"tick":sample.tick,"submission":sample.submission_order,
                "elapsed_s":start.elapsed().as_secs_f64(),"drive_ns":drive_ns,
                "upload_ns":sample.upload_cpu.as_nanos(),"encode_ns":sample.encode_cpu.as_nanos(),
                "view_update_ns":sample.view_update_ns,"extract_ns":sample.extract_cpu.as_nanos(),
                "tick_cpu_ns":sample.tick_cpu.as_nanos(),"render_ns":sample.render_cpu.as_nanos(),
                "gpu_pass_ns":passes,"gpu_sum_ns":sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::sum),
                "native_segment":sample.native_segment.as_ref().map(|s|format!("{:?}",s.outcome())),
                "uploaded_rows":sample.uploaded_rows,"uploaded_bytes":sample.uploaded_bytes,"moving":sample.moving,
                "dropped_light_references":sample.dropped_light_references,
                "culling_counts":sample.camera_culling.map(|c|serde_json::json!({"active":c.active,"drawn":c.drawn,"prepass":c.prepass})),
                "queued_ticks":sample.queued_ticks,"pending_frames":host.pending_frames()})
            )?;
        }
        tick += 1;
        if measured >= samples && start.elapsed() >= Duration::from_secs(seconds) {
            break;
        }
    }
    writeln!(
        output,
        "{}",
        serde_json::json!({"footer":true,"samples":measured,"actual_seconds":start.elapsed().as_secs_f64(),
        "pipelines_end":host.pipeline_creation_count(),"resident_end":caps.resident_device_bytes(),
        "storage_end":host.storage_report(),"native_witness":format!("{:?}",host.native_witness_report()),
        "pending_frames":host.pending_frames(),"capture_drained_frames":if capture {Some(tick)} else {None},
        "cooked_stats":cooked.as_ref().map(|s|format!("{:?}",s.lock())),
        "retirement":"host dropped after completed last frame; driver bytes sampled afterward below"})
    )?;
    drop(host);
    device.poll(wgpu::PollType::wait_indefinitely())?;
    writeln!(
        output,
        "{}",
        serde_json::json!({"retired":true,"resident_after_retire":caps.resident_device_bytes()})
    )?;
    Ok(())
}
