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
    let gpu = thrum_test_support::gpu("cloud cost", Some(wgpu::Backend::Metal))
        .ok_or("Metal unavailable")?;
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let adapter = gpu.adapter;
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
    let mut first = None;
    for tick in 0..240_u32 {
        last_tick = tick;
        host.drive(Duration::from_secs_f64(f64::from(tick) / 60.0), None)?;
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        while let Some(sample) = host.poll_completed()? {
            let timings = sample.gpu_pass_ns.ok_or("GPU counters required")?;
            if first.is_none() {
                first = Some(
                    serde_json::json!({"whole_frame_gpu_ns":timings.passes().iter().map(|p|p.nanoseconds).sum::<f64>(),"pass_gpu_ns":timings.passes().iter().map(|p|(p.name.to_string(),p.nanoseconds)).collect::<BTreeMap<_,_>>()}),
                );
            }
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
    v1_game::apply_cloud_quality(&mut host).map_err(|e| e.to_string())?;
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
            "first_submission":first,"ray_statistics":ray_statistics(&device,&queue,host.cloud_sky_products().as_ref())?,
            "quality":host.quality_knobs().iter().map(|q|serde_json::json!({"name":q.name,"value":q.value})).collect::<Vec<_>>(),
        })
    );
    Ok(())
}

fn ray_statistics(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sky: Option<&thrum_clouds::SkyProducts>,
) -> Result<serde_json::Value, String> {
    let Some(sky) = sky else {
        return Ok(serde_json::Value::Null);
    };
    let texture = &sky.fresh_front;
    let stride = (texture.width() * 8).div_ceil(256) * 256;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cloud cost sample counts"),
        size: u64::from(stride) * u64::from(texture.height()),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &output,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&output, wgpu::MapMode::Read, .., move |r| {
        let _ = send.send(r);
    });
    queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    receive
        .recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mapped = output.get_mapped_range(..).map_err(|e| e.to_string())?;
    let refresh = sky.refresh_stride;
    let phase = sky.submitted_frames.wrapping_sub(1) % (refresh * refresh);
    let mut samples = Vec::new();
    for y in 0..texture.height() {
        for x in 0..texture.width() {
            if x % refresh != phase % refresh || y % refresh != phase / refresh {
                continue;
            }
            let at = (y * stride + x * 8 + 4) as usize;
            let word = u32::from(u16::from_le_bytes([mapped[at], mapped[at + 1]]));
            let sample = if word == 0 {
                0.0
            } else {
                f32::from_bits((((word >> 10) + 112) << 23) | ((word & 1023) << 13))
            };
            samples.push(sample);
        }
    }
    samples.sort_by(f32::total_cmp);
    let rays = u32::try_from(samples.len()).map_err(|e| e.to_string())?;
    if rays == 0 {
        return Err("no traced rays in selected phase".into());
    }
    Ok(
        serde_json::json!({"scope":"last completed capture-off frame, selected fresh texels, readback after timed interval","rays_traced":rays,"mean_samples":samples.iter().map(|&n|f64::from(n)).sum::<f64>()/f64::from(rays),"p95_samples":samples[(samples.len()*95/100).min(samples.len()-1)],"refresh_stride":refresh,"refresh_fraction":1.0/f64::from(refresh*refresh)}),
    )
}
