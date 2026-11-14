//! M2 target proof harness: ordinary Game/FrameHost, elapsed offscreen opportunities,
//! and the shipping profile-window and AppKit-view sources. No OS presentation claim.
use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};
use thrum_app::{Game, Gpu};
use thrum_controls::{Value, macos::MacPanel};
use thrum_core::sim::WorldLifecycle;
use thrum_render::{
    capabilities::DeviceCapabilities, present::SurfaceProfile, runtime::SurfaceOutput,
    synthetic::OUTPUT_FORMAT,
};

fn apply<G: Game>(
    game: &mut G,
    host: &mut thrum_render::runtime::FrameHost<G::World, G::Scene>,
    path: &PathBuf,
) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let report = game
        .controls_mut()
        .apply_file(&text)
        .map_err(|e| e.to_string())?;
    if !report.problems.is_empty() {
        return Err(format!("{report:?}"));
    }
    let (edits, paused) = game.take_edits();
    G::apply(host, edits)?;
    if let Some(paused) = paused {
        let world = game.world().ok_or("no world for pause")?;
        let request = if paused {
            WorldLifecycle::Pause
        } else {
            WorldLifecycle::Resume
        };
        world
            .request_lifecycle(request)
            .map_err(|e| format!("{e:?}"))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if world
                .poll_lifecycle_reply()
                .map_err(|e| format!("{e:?}"))?
                .is_some_and(|reply| reply.request == request)
            {
                break;
            }
            if Instant::now() > deadline {
                return Err("pause acknowledgement timed out".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        game.observe_paused(paused);
    }
    Ok(())
}

fn stamp(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

pub fn run<G: Game>(mut game: G, extent: [u32; 2]) -> Result<(), String> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .map_err(|e| e.to_string())?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: DeviceCapabilities::requested_timing_features(&adapter)
            | DeviceCapabilities::requested_content_features(&adapter),
        ..Default::default()
    }))
    .map_err(|e| e.to_string())?;
    let capabilities = DeviceCapabilities::capture(&adapter, &device, 1);
    let prepared = game.prepare(
        Gpu {
            device: &device,
            queue: &queue,
            capabilities: &capabilities,
            surface: SurfaceProfile {
                format: OUTPUT_FORMAT,
                extent,
            },
        },
        Box::new(|| {}),
    )?;
    let mut host = prepared.host;
    let origin = prepared.origin;
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("CREATOR ordinary offscreen output"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OUTPUT_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = output.create_view(&wgpu::TextureViewDescriptor::default());
    let params = PathBuf::from(std::env::var_os("THRUM_PARAMS").ok_or("THRUM_PARAMS missing")?);
    apply(&mut game, &mut host, &params)?;
    let mut params_stamp = stamp(&params);
    let material = std::env::var_os("THRUM_MATERIAL").map(PathBuf::from);
    let mut material_stamp = material.as_ref().and_then(stamp);
    game.reload();
    let mut panel = MacPanel::new(game.controls(), "Thrum frame profiler", None)?;
    let mut window = crate::window::ProfileWindow::new();
    let mut recording =
        crate::recording::Recording::from_environment().ok_or("THRUM_PROFILE_OUTPUT missing")?;
    let directory =
        PathBuf::from(std::env::var_os("THRUM_PROFILE_OUTPUT").ok_or("output missing")?);
    std::fs::write(
        directory.join("provenance.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "adapter":format!("{:?}",adapter.get_info()),"extent":extent,
            "callback_source":"120 Hz elapsed engine opportunity clock; not CADisplayLink",
            "present_calls":"zero OS present calls; engine surface pass writes a texture",
            "profile_window_source":"crates/thrum-app/src/profile/window.rs",
            "panel_source":"crates/thrum-controls/src/macos.rs",
            "timing_source":"ordinary Game::prepare FrameHost completed samples"
        }))
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut clock = thrum_app::offscreen::clock::OpportunityClock::new(
        origin,
        120,
        thrum_app::offscreen::clock::Timebase::Elapsed,
    )?;
    let mut count = 0u64;
    let mut samples = std::collections::VecDeque::with_capacity(120);
    let mut minimum = 0;
    let mut enabled = false;
    let mut checked = Duration::ZERO;
    loop {
        let opportunity = clock.next_opportunity();
        let now = origin.elapsed();
        if now.saturating_sub(checked) >= Duration::from_millis(250) {
            checked = now;
            let fresh = stamp(&params);
            if fresh != params_stamp {
                apply(&mut game, &mut host, &params)?;
                params_stamp = fresh;
            }
            let fresh = material.as_ref().and_then(stamp);
            if fresh != material_stamp {
                game.reload();
                material_stamp = fresh;
            }
        }
        let on = game
            .controls()
            .find("profiler.enabled")
            .is_some_and(|id| matches!(game.controls().value(id), Value::Flag(true)));
        if on != enabled {
            minimum = host.last_submission().map_or(0, |order| order + 1);
            enabled = on;
        }
        host.set_timing_requested(enabled);
        window.configure(enabled, 1, minimum);
        window.callback(opportunity.requested);
        count += 1;
        host.set_presentation_lead(Duration::ZERO);
        host.drive(
            now,
            Some(&SurfaceOutput {
                view: &view,
                extent,
            }),
        )
        .map_err(|e| e.to_string())?;
        game.observe(G::readback(&host));
        while let Some(sample) = host.poll_completed().map_err(|e| e.to_string())? {
            window.push(&sample, now);
            if samples.len() == 120 {
                samples.pop_front();
            }
            samples.push_back(sample);
        }
        window.publish(game.controls_mut(), now, count, 0, false);
        panel.sync(game.controls());
        recording.observe(
            now,
            samples.make_contiguous(),
            game.controls(),
            Some(&panel),
            [0, count, 0],
        )?;
    }
}
