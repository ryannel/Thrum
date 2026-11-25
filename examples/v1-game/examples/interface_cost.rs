//! Deterministic Metal timing take of the sprite field or the authored valley route.

#[cfg(target_os = "macos")]
mod support {
    pub(super) mod interface_cost;
}

#[cfg(target_os = "macos")]
use std::{io::Write as _, path::PathBuf, time::Duration};
#[cfg(target_os = "macos")]
use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameConfig, SurfaceOutput},
};
#[cfg(target_os = "macos")]
use v1_game::{CameraRoute, GameHost, GameSetup};

#[cfg(target_os = "macos")]
fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("8-bit or float");
    let width = args
        .next()
        .expect("output width")
        .parse()
        .expect("pixel width");
    let height = args
        .next()
        .expect("output height")
        .parse()
        .expect("pixel height");
    let extent = [width, height];
    let route = args
        .next()
        .map(PathBuf::from)
        .map(|path| std::fs::read_to_string(path).expect("authored route"));
    assert!(args.next().is_none());
    let format = match mode.as_str() {
        "8-bit" => wgpu::TextureFormat::Bgra8Unorm,
        "float" => wgpu::TextureFormat::Rgba16Float,
        _ => panic!("unknown display mode"),
    };
    let surface = SurfaceProfile { extent, format };
    let opened = thrum_test_support::open(
        "interface cost",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Native,
        &thrum_test_support::Restriction::default(),
    )
    .expect("Metal cost target");
    let caps = opened
        .capabilities
        .with_recipe_override("portable")
        .expect("portable Metal");
    let mut setup: GameSetup = v1_game::selected_setup().expect("select the authored cost scene");
    setup.valley_stars = false;
    if let Some(text) = &route {
        setup.route_overlay = CameraRoute::declared(text).expect("route declaration").1;
    }
    let mut host = pollster::block_on(setup.prepare(
        &opened.device,
        &opened.queue,
        &caps,
        FrameConfig {
            render_extent: extent,
            surface: Some(surface),
        },
    ))
    .expect("prepare the real game");
    host.set_display_headroom(4.0);
    let view = opened
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("interface cost drawable"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent,
    };
    let route = route.map(|text| {
        let stats = v1_game::attach_cooked_scene(&mut host)
            .expect("attach the authored valley")
            .expect("valley scene contribution");
        host.world_mut().pause().expect("pause during residency");
        thrum_asset_gpu::await_drawn(&mut host, &stats, thrum_asset_gpu::Drawn::Generation(1))
            .expect("valley placements drawn");
        host.world_mut()
            .resume(Duration::ZERO)
            .expect("resume deterministic game clock");
        let camera = host.scene_camera().expect("authored valley camera");
        let route = CameraRoute::parse(&text, camera, 60).expect("load the authored valley route");
        while host.world().tick() + 1 < route.origin_tick {
            let now = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
            host.advance_without_rendering(now)
                .expect("advance to the route origin");
        }
        host.set_camera_route(route.clone())
            .expect("arm the authored route");
        route
    });
    let graph = host.scene_graph().expect("prepared cost graph");
    let mut rows = Vec::new();
    for _ in 0..40 {
        let now = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
        let report = host
            .drive(now, Some(&output))
            .expect("record the cost frame");
        assert!(report.submitted && report.surface_presented);
        let sample = thrum_test_support::wait::until(
            "Metal cost frame retirement",
            Duration::from_secs(10),
            || host.poll_completed().expect("healthy Metal cost device"),
        );
        let timing = sample.gpu_pass_ns.as_ref().expect("Metal frame timestamps");
        rows.push(serde_json::json!({
            "tick": sample.tick,
            "cpu_ns": sample.render_cpu.as_nanos(),
            "gpu_pass_span_ns": timing.timed_pass_span_nanoseconds(),
            "present_ns": sample.gpu_present_ns,
            "interface_ns": timing.passes().iter().find(|p| p.name.as_ref() == "interface stage").map(|p| p.nanoseconds),
            "interface": support::interface_cost::stats(&sample),
        }));
    }
    let json = serde_json::json!({
        "mode": mode, "extent": extent, "adapter": caps.adapter().name, "recipe": "portable",
        "passes": graph.pass_names().collect::<Vec<_>>(),
        "route_origin": route.map(|route| route.origin_tick), "samples": &rows[8..],
    });
    writeln!(std::io::stdout().lock(), "{json}").expect("write the cost record");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
