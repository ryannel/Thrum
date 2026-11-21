//! Pixel equality against fresh preparation after each real host transition.

mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use std::time::Duration;
use thrum_render::{
    runtime::SurfaceOutput,
    sprite::{SpriteRect, SpriteRow},
    synthetic::CaptureSource,
};

#[test]
fn redraws_at_the_surface_grid_after_host_transitions_and_device_replacement() {
    for recipe in fixture::RECIPES {
        let Some(thrum_test_support::Opened {
            gpu,
            device,
            queue,
            capabilities: caps,
        }) = fixture::open("sprite lifecycle", recipe)
        else {
            return;
        };
        let art = fixture::sheet(
            [2, 2],
            &[
                240, 11, 22, 255, 20, 211, 81, 255, 40, 92, 213, 255, 251, 207, 2, 255,
            ],
            &[SpriteRect {
                origin: [0; 2],
                size: [2, 2],
            }],
        );
        let scene = fixture::Scene::new(vec![SpriteRow::new(0, [-1.0, 1.0])], 4);
        let mut extent = [48, 32];
        let mut host = fixture::host(&caps, &queue, extent, scene.clone(), art.clone());
        let compare = |label, live: &mut fixture::Host, device, queue, caps, extent, generation| {
            let mut fresh = fixture::host(caps, queue, extent, scene.clone(), art.clone());
            let actual = fixture::capture(live, device, extent, CaptureSource::Sdr);
            let expected = fixture::capture(&mut fresh, device, extent, CaptureSource::Sdr);
            assert_eq!(actual.device_generation, generation, "{label}");
            assert_eq!(actual.rgba, expected.rgba, "{recipe} {label}");
            println!("{recipe}: {label} matched fresh at {extent:?}, generation {generation}");
        };
        extent = [71, 45];
        assert!(
            host.resize_scene_surface(extent)
                .expect("resize should be admitted")
        );
        compare("resize", &mut host, &device, &queue, &caps, extent, 1);
        // Same logical size at 2× density. The platform supplies physical pixels.
        extent = [142, 90];
        host.resize_surface(extent)
            .expect("resize should be admitted");
        compare(
            "density change",
            &mut host,
            &device,
            &queue,
            &caps,
            extent,
            1,
        );

        let last = host.last_submission();
        host.advance(Duration::ZERO)
            .expect("advance the fixture world")
            .encode_scene()
            .expect("record the admitted scene")
            .skip();
        assert_eq!(host.last_submission(), last);
        compare(
            "discarded encoding",
            &mut host,
            &device,
            &queue,
            &caps,
            extent,
            1,
        );

        let refused = fixture::drawable(&device, [142, 89]);
        let report = host
            .drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &refused,
                    extent: [142, 89],
                }),
            )
            .expect("drive the real frame executor");
        assert!(!report.surface_presented);
        compare(
            "refused surface",
            &mut host,
            &device,
            &queue,
            &caps,
            extent,
            1,
        );

        let last = host.last_submission();
        let pixels = host
            .capture_pixels()
            .expect("capture the committed endpoint");
        assert_eq!(
            host.last_submission(),
            last,
            "endpoint capture does not publish a view"
        );
        let mut fresh = fixture::host(&caps, &queue, extent, scene.clone(), art.clone());
        assert_eq!(
            pixels,
            fresh
                .capture_pixels()
                .expect("capture the committed endpoint")
        );
        compare(
            "capture frame",
            &mut host,
            &device,
            &queue,
            &caps,
            extent,
            1,
        );

        let mut old_capture = host.begin_capture().expect("begin the endpoint capture");
        let inputs = host
            .device_prepare_inputs()
            .expect("retain scene inputs for recovery");
        let (replacement, replacement_queue, replacement_caps) =
            fixture::metal(&gpu.fresh_adapter(), recipe, 2);
        let mut candidate = pollster::block_on(inputs.prepare(
            &replacement_caps,
            &replacement_queue,
            Some(fixture::profile(extent)),
        ))
        .expect("prepare a replacement GPU scene");
        device.destroy();
        host.publish_device(&mut candidate)
            .expect("publish the replacement generation");
        assert!(
            host.try_finish_capture(&mut old_capture).is_err(),
            "old generation must be refused"
        );
        compare(
            "device replacement",
            &mut host,
            &replacement,
            &replacement_queue,
            &replacement_caps,
            extent,
            2,
        );
    }
}
