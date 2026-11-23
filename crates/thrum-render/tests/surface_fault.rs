//! Presentation encode failures and the frame host's public drop telemetry.
mod support;

use std::time::Duration;
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{FrameConfig, SurfaceFault, SurfaceOutput},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
};

#[test]
fn extent_mismatch_reaches_the_drive_and_frame_sample() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "surface fault report",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let profile = SurfaceProfile {
        format: wgpu::TextureFormat::Rgba8Unorm,
        extent: [64, 64],
    };
    let scene = SyntheticScene::grid(1).unwrap();
    let mut host = pollster::block_on(support::host_with_config(
        &cap,
        &queue,
        scene,
        FrameLighting::Diagnostic(SyntheticLight::default()),
        FrameConfig {
            render_extent: profile.extent,
            surface: Some(profile),
        },
    ));
    host.set_timing_requested(false);
    let stale_drawable = wgpu::TextureDescriptor {
        label: Some("drawable from the earlier extent"),
        size: wgpu::Extent3d {
            width: 32,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: profile.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    let drawable = device.create_texture(&stale_drawable);
    let view = drawable.create_view(&wgpu::TextureViewDescriptor::default());
    for (index, extent) in [[32, 64], profile.extent].into_iter().enumerate() {
        let report = host
            .drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &view,
                    extent,
                }),
            )
            .expect("the extent may change at acquisition");
        assert!(report.submitted && report.dropped_view && !report.surface_presented);
        assert_eq!(report.surface_fault, Some(SurfaceFault::ExtentMismatch));
        assert_eq!(host.dropped_surface(), index as u64 + 1);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let sample = host.poll_completed().unwrap().expect("scene completion");
        assert!(!sample.surface_presented);
        assert_eq!(sample.surface_fault, report.surface_fault);
    }
}
