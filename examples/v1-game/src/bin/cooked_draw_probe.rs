//! Fixed-time submitted-output diagnostic for the COOKED-DRAW parity command.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use thrum_render::{capabilities::DeviceCapabilities, present::SurfaceProfile};
use v1_game::{FrameConfig, FrameHost, SurfaceOutput};

fn main() -> Result<(), String> {
    pollster::block_on(run())
}

async fn run() -> Result<(), String> {
    let output = PathBuf::from(std::env::var("COOKED_DRAW_OUTPUT").map_err(|e| e.to_string())?);
    std::fs::create_dir_all(&output).map_err(|e| e.to_string())?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|e| e.to_string())?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_content_features(&adapter)
                | DeviceCapabilities::requested_timing_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    let mut capabilities = DeviceCapabilities::capture(&adapter, &device, 1);
    let captured_flags = capabilities.downlevel();
    let masked = std::env::var("COOKED_DRAW_MASK_INDIRECT").as_deref() == Ok("1");
    if masked {
        capabilities = capabilities.without_indirect_execution();
    }
    let adapter_less = std::env::var("COOKED_DRAW_ADAPTER_LESS").as_deref() == Ok("1");
    if adapter_less {
        capabilities = DeviceCapabilities::capture_device(&device, 1);
    }
    let extent = [1920, 1080];
    let config = FrameConfig {
        render_extent: extent,
        surface: Some(SurfaceProfile {
            format: wgpu::TextureFormat::Rgba8Unorm,
            extent,
        }),
    };
    let mut host = if adapter_less {
        FrameHost::new(&device, &queue, config).await?
    } else {
        FrameHost::new_with_capabilities(&device, &queue, &capabilities, config).await?
    };
    let mode = std::env::var("COOKED_DRAW_MODE").unwrap_or_else(|_| "production".into());
    let names: Vec<String> =
        serde_json::from_str(&std::env::var("COOKED_DRAW_MESHES").map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let store = PathBuf::from(std::env::var("THRUM_ASSET_STORE").map_err(|e| e.to_string())?);
    let reference =
        thrum_asset_gpu::draw_probe::reference_commands(&store, &names, mode == "partitioned")?;
    let probe = match mode.as_str() {
        "production" => None,
        "reference" | "partitioned" => Some(thrum_asset_gpu::draw_probe::Probe::Reference(
            reference.clone(),
        )),
        "omit" => Some(thrum_asset_gpu::draw_probe::Probe::Omit(0)),
        _ => return Err("unknown draw probe mode".into()),
    };
    if probe.is_some() && !masked {
        return Err("draw controls require explicit masked-record arm".into());
    }
    let stats = if adapter_less {
        v1_game::attach_cooked_scene(&mut host)?
    } else {
        v1_game::attach_cooked_scene_with_probe(&mut host, probe)?
    }
    .ok_or("no cooked content selected")?;
    host.pause()?;
    let yaw: f32 = std::env::var("COOKED_DRAW_YAW")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .map_err(|_| "yaw")?;
    let solar: f32 = std::env::var("COOKED_DRAW_SOLAR")
        .unwrap_or_else(|_| "12".into())
        .parse()
        .map_err(|_| "solar")?;
    host.set_time_of_day(solar / 24.0)?;
    host.set_camera_yaw(yaw.to_radians())?;
    host.set_display_overlay(thrum_render::synthetic::DisplayOverlay::default());
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cooked parity output"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let deadline = Instant::now() + Duration::from_secs(60);
    // Fixed world and view; warm history and atmosphere only after content has drawn.
    let mut warm = 0;
    let mut timing_samples = Vec::new();
    loop {
        if Instant::now() >= deadline {
            return Err("cooked warmup deadline".into());
        }
        let surface = SurfaceOutput {
            view: &view,
            extent,
        };
        host.drive(Duration::ZERO, Some(&surface))?;
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|e| e.to_string())?;
        while let Some(sample) = host.poll_completed()? {
            timing_samples.push(serde_json::json!({"submission": sample.submission_order, "encode_ns": sample.encode_cpu.as_nanos(), "forward_ns": sample.gpu_pass_ns.map(|times| times[3]), "contribution_ns":sample.gpu_contribution_ns}));
        }
        let stats = stats.lock().map_err(|e| e.to_string())?;
        if !stats.errors.is_empty() {
            return Err(format!("cooked errors: {:?}", stats.errors));
        }
        if stats.drawn.is_some() {
            warm += 1;
        }
        if warm >= 16 {
            break;
        }
    }
    timing_samples.clear();
    let measured_frames: u32 = std::env::var("COOKED_DRAW_SAMPLES")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .map_err(|_| "samples")?;
    for _ in 0..measured_frames {
        host.drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &view,
                extent,
            }),
        )?;
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|e| e.to_string())?;
        while let Some(sample) = host.poll_completed()? {
            timing_samples.push(serde_json::json!({"submission":sample.submission_order,"encode_ns":sample.encode_cpu.as_nanos(),"forward_ns":sample.gpu_pass_ns.map(|times| times[3]),"contribution_ns":sample.gpu_contribution_ns}));
        }
    }
    host.enable_submitted_capture(1)?;
    host.drive(
        Duration::ZERO,
        Some(&SurfaceOutput {
            view: &view,
            extent,
        }),
    )?;
    loop {
        if Instant::now() >= deadline {
            return Err("submitted image deadline".into());
        }
        device
            .poll(wgpu::PollType::Poll)
            .map_err(|e| e.to_string())?;
        let _ = host.poll_completed()?;
        if let Some(image) = host.try_submitted_image()? {
            std::fs::write(output.join("output.rgba"), &image.rgba).map_err(|e| e.to_string())?;
            let stats = stats.lock().map_err(|e| e.to_string())?;
            let report = serde_json::json!({"adapter":format!("{:?}", capabilities.adapter()), "flags":format!("{:?}",capabilities.downlevel()), "adapter_flags":format!("{captured_flags:?}"), "masked":masked, "diagnostic_mode":mode, "adapter_less_host":adapter_less, "reference_commands":reference, "encoded_commands": stats.probe_commands, "resource_observation":stats.probe_resources, "mesh_placements":names, "timing_samples":timing_samples, "path":format!("{:?}", stats.draw_path), "drawn":stats.drawn, "visible":stats.visible_meshlets, "extent":extent, "yaw":yaw, "solar":solar, "submission":image.sample.submission_order, "tick":image.sample.tick, "encode_ns":image.sample.encode_cpu.as_nanos(), "gpu_pass_ns":image.sample.gpu_pass_ns});
            std::fs::write(
                output.join("report.json"),
                serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            break;
        }
    }
    Ok(())
}
