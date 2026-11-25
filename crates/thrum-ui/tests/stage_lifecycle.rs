//! Reuse of prepared paints across the real host's frame and device transitions.

#[expect(
    dead_code,
    reason = "individual proof binaries select different fixture helpers"
)]
mod support;

#[cfg(target_os = "macos")]
use std::{sync::atomic::AtomicBool, time::Duration};
#[cfg(target_os = "macos")]
use support::{FLOAT, SDR};
#[cfg(target_os = "macos")]
use thrum_render::{present::SurfaceProfile, runtime::SurfaceOutput};

thrum_test_support::macos_gpu_test! {
    fn interface_recovers_and_preserves_top_left_pixels() {
        let Some(gpu) = support::device("interface stage lifecycle", "portable") else { return };
        let initial = [16, 12];
        let mut host = support::host(&gpu.capabilities, &gpu.queue, initial, SDR, Some(support::interface(3)), 0.3)
            .expect("prepare interface");
        host.source_mut().paint = Some(support::paint(initial, 1));

        let extent = [19, 13];
        assert!(host.resize_scene_surface(extent).expect("resize output"));
        let view = support::drawable(&gpu.device, extent, SDR);
        let mut fresh = support::host(&gpu.capabilities, &gpu.queue, extent, SDR, Some(support::interface(3)), 0.3)
            .expect("prepare resized control");
        fresh.source_mut().paint = Some(support::paint(extent, 1));
        let expected = support::capture(&mut fresh, &view, extent);
        let resized = support::capture(&mut host, &view, extent);
        assert!(resized.sample.interface.expect("interface record").extent_mismatch);
        assert_eq!(resized.rgba, expected.rgba);

        drop(host.advance(Duration::ZERO).expect("advance before discard")
            .encode_scene().expect("record frame before discard"));
        assert_eq!(support::capture(&mut host, &view, extent).rgba, expected.rgba);

        let wrong = support::drawable(&gpu.device, initial, SDR);
        let report = host.advance(Duration::ZERO).expect("advance before refusal")
            .encode_scene().expect("record scene before refusal")
            .submit(Some(&SurfaceOutput { view: &wrong, extent: initial }))
            .expect("transient surface refusal");
        assert!(!report.surface_presented && report.surface_fault.is_some());
        if report.submitted {
            drop(support::sample(&mut host));
        }
        assert_eq!(support::capture(&mut host, &view, extent).rgba, expected.rgba);

        assert_eq!(host.capture_pixels().expect("capture current endpoint"), expected.rgba);
        assert_eq!(support::capture(&mut host, &view, extent).rgba, expected.rgba);

        let (replacement, queue, capabilities) = gpu.gpu.device_with_hardware(
            thrum_test_support::Profile::Native,
            &thrum_test_support::Restriction::default(),
            2,
            &[],
        ).expect("open a later Metal device");
        let capabilities = capabilities.with_recipe_override("portable").expect("keep the Metal recipe");
        let mut candidate = pollster::block_on(host.device_prepare_inputs().expect("snapshot retained inputs")
            .prepare(&capabilities, &queue, Some(SurfaceProfile { extent, format: SDR })))
            .expect("prepare replacement device resources");
        host.publish_device(&mut candidate).expect("publish replacement device");
        let view = support::drawable(&replacement, extent, SDR);
        assert_eq!(support::capture(&mut host, &view, extent).rgba, expected.rgba);

        let pipelines = host.creation_report().pipelines;
        let next = host.extent_prepare_inputs_for_format(extent, FLOAT)
            .expect("take inputs for a float generation")
            .build(&AtomicBool::new(false)).expect("build float generation").expect("build admitted");
        assert!(host.publish_extent(next));
        assert_eq!(host.creation_report().pipelines, pipelines, "mode change reuses pipelines");
        host.set_display_headroom(4.0);
        let float = support::drawable(&replacement, extent, FLOAT);
        let mut fresh = support::host(&capabilities, &queue, extent, FLOAT, Some(support::interface(3)), 0.3)
            .expect("prepare fresh float control");
        fresh.set_display_headroom(4.0);
        fresh.source_mut().paint = Some(support::paint(extent, 1));
        assert_eq!(support::capture(&mut host, &float, extent).rgba, support::capture(&mut fresh, &float, extent).rgba);
    }
}
