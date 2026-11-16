//! Capture-off measurement of the selected real game, paired by the Python driver.
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use v1_game::{FrameConfig, FrameHost};

fn main() -> Result<(), String> {
    pollster::block_on(measure())
}

#[expect(
    clippy::print_stdout,
    reason = "measurement executable emits its one machine-readable record"
)]
async fn measure() -> Result<(), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|e| e.to_string())?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features:
                thrum_render::capabilities::DeviceCapabilities::requested_timing_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    let caps = thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
    let mut host = FrameHost::new_with_setup(
        &device,
        &queue,
        &caps,
        FrameConfig {
            render_extent: [2560, 1440],
            surface: None,
        },
        v1_game::selected_setup()?,
    )
    .await?;
    let content = v1_game::attach_cooked_scene(&mut host)?;
    // Settle ordinary asynchronous residency before the timed interval.
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last_tick = 0;
    let mut refill = 0.0_f64;
    for tick in 0..240_u32 {
        last_tick = tick;
        host.drive(Duration::from_secs_f64(f64::from(tick) / 60.0), None)?;
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        while let Some(sample) = host.poll_completed()? {
            let timings = sample.gpu_pass_ns.ok_or("GPU counters required")?;
            refill = refill.max(
                timings
                    .passes()
                    .iter()
                    .filter(|p| p.name.contains("cloud Sun path"))
                    .map(|p| p.nanoseconds)
                    .sum(),
            );
        }
        if content
            .as_ref()
            .is_none_or(|s| s.lock().is_ok_and(|s| s.drawn.is_some()))
            && tick > 30
        {
            break;
        }
        if Instant::now() > deadline {
            return Err("cooked scene failed to settle".into());
        }
    }
    let mut passes: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut whole = Vec::new();
    for tick in last_tick + 1..=last_tick + 120 {
        let report = host.drive(Duration::from_secs_f64(f64::from(tick) / 60.0), None)?;
        if !report.submitted {
            continue;
        }
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        while let Some(sample) = host.poll_completed()? {
            let timings = sample.gpu_pass_ns.ok_or("GPU counters required")?;
            whole.push(timings.passes().iter().map(|p| p.nanoseconds).sum::<f64>());
            let mut sun_sum = 0.0;
            for pass in timings.passes() {
                passes
                    .entry(pass.name.to_string())
                    .or_default()
                    .push(pass.nanoseconds);
                if pass.name.contains("cloud Sun path") {
                    sun_sum += pass.nanoseconds;
                }
            }
            refill = refill.max(sun_sum);
        }
    }
    if whole.len() < 30 {
        return Err("insufficient completed GPU frames".into());
    }
    let mean = |v: &[f64]| {
        v.iter().sum::<f64>() / f64::from(u32::try_from(v.len()).expect("bounded frame count"))
    };
    let pass_means: BTreeMap<_, _> = passes
        .into_iter()
        .map(|(name, values)| (name, mean(&values)))
        .collect();
    println!(
        "{}",
        serde_json::json!({
            "adapter":format!("{:?}",adapter.get_info()), "extent":[2560,1440], "capture":false,
            "frames":whole.len(), "whole_frame_gpu_ns":mean(&whole), "pass_gpu_ns":pass_means,
            "maximum_sun_refill_ns":refill, "resources":host.cloud_resource_bytes(),
            "storage":host.storage_report(), "view":host.scene_camera().map(v1_game::SceneCamera::eye),
        })
    );
    Ok(())
}
