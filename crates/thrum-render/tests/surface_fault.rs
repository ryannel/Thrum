//! Public drive and completed-frame telemetry for a stale drawable extent.
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
    let Some(gpu) = thrum_test_support::gpu("surface fault report", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let profile = SurfaceProfile {
        format: wgpu::TextureFormat::Rgba8Unorm,
        extent: [64, 64],
    };
    let mut host = pollster::block_on(support::host_with_config(
        &cap,
        &queue,
        SyntheticScene::grid(1).unwrap(),
        FrameLighting::Diagnostic(SyntheticLight::default()),
        FrameConfig {
            render_extent: profile.extent,
            surface: Some(profile),
        },
    ));
    host.set_timing_requested(false);
    let drawable = device.create_texture(&wgpu::TextureDescriptor {
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
    });
    let view = drawable.create_view(&wgpu::TextureViewDescriptor::default());
    let report = host
        .drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &view,
                extent: [32, 64],
            }),
        )
        .expect("the extent may change at acquisition");
    assert!(report.submitted && report.dropped_view && !report.surface_presented);
    assert_eq!(report.surface_fault, Some(SurfaceFault::ExtentMismatch));
    assert_eq!(host.dropped_surface(), 1);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let sample = host.poll_completed().unwrap().expect("scene completion");
    assert!(!sample.surface_presented);
    assert_eq!(sample.surface_fault, report.surface_fault);
}
