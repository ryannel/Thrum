//! Release cost record on the ordinary Metal frame host, without a capture tap.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameConfig, SurfaceOutput},
};
use thrum_test_support::{Profile, Restriction};
use v1_game::{GameHost, GameScene, GameSetup, InterfaceText};

pub(crate) fn measure(store: &Path) -> serde_json::Value {
    let clock = Instant::now();
    let layout = GameScene::status_layout_costs(|| clock.elapsed().as_secs_f64() * 1000.0)
        .expect("CPU layout arms");
    let gpu = thrum_test_support::open(
        "status costs",
        Some(wgpu::Backend::Metal),
        Profile::Native,
        &Restriction::default(),
    )
    .expect("Mac Metal status cost target");
    assert_eq!(gpu.capabilities.adapter().backend, wgpu::Backend::Metal);
    let extent = [1920, 1080];
    let format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let drawable = gpu
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("status cost drawable"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
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
        view: &drawable,
        extent,
    };
    let setup = GameSetup {
        interface_scale: Some(2),
        interface_text: Some(InterfaceText::Status),
        sprite_scale: Some(2),
        sprite_rows: Some(10_000),
        ..GameSetup::default()
    };
    let mut host = pollster::block_on(setup.prepare_with_sheet_source(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile { extent, format }),
        },
        store.to_path_buf().into(),
    ))
    .expect("live Metal status workload");
    host.set_timing_requested(true);
    host.continuous_source_mut().measure_status_updates();
    let mut previous = None;
    let mut samples = Vec::new();
    for _ in 0..180 {
        let now = Duration::from_secs_f64(host.route_step_ms(0.5) / 1000.0);
        let frame = host.drive(now, Some(&output)).expect("drive sampled frame");
        assert!(frame.submitted && frame.surface_presented);
        let sample = thrum_test_support::wait::until(
            "Metal status cost sample",
            Duration::from_secs(10),
            || host.poll_completed().expect("healthy Metal device"),
        );
        let interface = sample.interface.expect("status stage");
        if interface.stamp.is_none() || host.source().status_stats().is_none() {
            continue;
        }
        assert_eq!(interface.dropped_rows, 0);
        assert!(!interface.extent_mismatch && !interface.stamp_mismatch);
        let stats = host.source().status_stats().expect("retained layout");
        if let Some(before) = previous.replace(stats) {
            let timings = sample.gpu_pass_ns.as_ref().expect("frame GPU timestamps");
            samples.push(serde_json::json!({
                "tick": sample.tick,
                "submission": sample.submission_order,
                "layouts": stats.layouts - before.layouts,
                "lists": stats.lists - before.lists,
                "rows": stats.rows - before.rows,
                "upload_bytes": interface.uploaded_bytes,
                "cpu_ns": sample.render_cpu.as_nanos(),
                "gpu_ns": timings.timed_pass_span_nanoseconds(),
                "stamp": interface.stamp.map(|stamp| stamp.scale),
            }));
        }
        if samples.len() == 40 {
            break;
        }
    }
    assert_eq!(samples.len(), 40, "complete status cost record");
    serde_json::json!({"extent":extent,"adapter":gpu.capabilities.adapter().name,"layout":layout,"samples": &samples[8..]})
}
