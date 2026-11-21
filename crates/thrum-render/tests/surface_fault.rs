//! Presentation encode failures and the frame host's public drop telemetry.
mod support;

use std::time::Duration;
use thrum_render::timing::TimingCategory;
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::{PresentError, SurfacePresenter, SurfaceProfile},
    runtime::{FrameConfig, SurfaceFault, SurfaceOutput},
    synthetic::{
        FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticScene,
    },
    timing::{GpuTimings, TimingError},
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
    let scene = SyntheticScene::grid(1).unwrap();
    let mut host = pollster::block_on(support::host_with_config(
        &cap,
        &queue,
        scene.clone(),
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

    pollster::block_on(async {
        let prepared = PreparedSynthetic::prepare(&device, &queue, 1, &scene)
            .await
            .unwrap();
        let target = OffscreenTarget::new(&device, &prepared, 64, 64).unwrap();
        let presenter = SurfacePresenter::prepare(&cap, profile.extent, profile)
            .await
            .unwrap();
        let retired = SurfacePresenter::prepare(&cap, profile.extent, profile)
            .await
            .unwrap();
        let binding = presenter.bind_target(&device, &target).unwrap();
        let old_binding = retired.bind_target(&device, &target).unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert!(matches!(
            presenter.encode(&mut encoder, &old_binding, &view, profile.extent, None),
            Err(PresentError::StaleTarget)
        ));
        assert!(matches!(
            presenter.encode(&mut encoder, &binding, &view, [32, 64], None),
            Err(PresentError::Extent)
        ));
        let mut timings = GpuTimings::new(&cap, &queue, 1).unwrap();
        assert!(
            timings
                .pass_writes(1, "earlier scene pass", TimingCategory::Effects)
                .unwrap()
                .is_some()
        );
        assert!(matches!(
            presenter.encode(
                &mut encoder,
                &binding,
                &view,
                profile.extent,
                Some(&mut timings)
            ),
            Err(PresentError::Timing(TimingError::Capacity))
        ));
    });
}
