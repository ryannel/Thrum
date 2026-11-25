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

#[test]
#[cfg(target_os = "macos")]
fn float_mode_survives_generation_and_capture_transitions() {
    use thrum_render::{present::SurfaceProfile, sprite::SpriteScene};

    let art = fixture::sheet(
        [1, 1],
        &[255, 255, 255, 255],
        &[SpriteRect {
            origin: [0; 2],
            size: [1, 1],
        }],
    );
    let mut row = SpriteRow::new(0, [-1.0, 1.0]);
    row.tint = [4.0, 0.5, 0.06, 1.0];
    let policy = fixture::Scene::new(vec![row], 4);
    let scene = SpriteScene::new(art, 32);
    for recipe in fixture::RECIPES {
        let Some(thrum_test_support::Opened {
            gpu,
            device,
            queue,
            capabilities,
            ..
        }) = fixture::open("float sprite lifecycle", recipe)
        else {
            return;
        };
        let fresh = |caps, queue, extent, format| {
            fixture::host_for_format(caps, queue, extent, policy.clone(), scene.clone(), format)
        };
        let float = fixture::FLOAT;
        let mut extent = [48, 32];
        let mut host = fresh(&capabilities, &queue, extent, fixture::FORMAT);
        #[cfg(feature = "creation-counters")]
        let pipelines = host.pipeline_creation_count();
        publish(&mut host, extent, float);
        assert!(host.is_edr());
        #[cfg(feature = "creation-counters")]
        assert_eq!(host.pipeline_creation_count(), pipelines);
        host.set_display_headroom(2.0);
        let mut comparison = fresh(&capabilities, &queue, extent, float);
        comparison.set_display_headroom(2.0);
        let expected = fixture::capture(&mut comparison, &device, extent, CaptureSource::Edr).rgba;
        for label in [
            "SDR to float",
            "discarded frame",
            "refused surface",
            "endpoint capture",
        ] {
            match label {
                "discarded frame" => {
                    let last = host.last_submission();
                    host.advance(Duration::ZERO)
                        .expect("advance before discarding")
                        .encode_scene()
                        .expect("encode to discard")
                        .skip();
                    assert_eq!(host.last_submission(), last);
                }
                "refused surface" => {
                    let wrong_extent = [48, 31];
                    let drawable = fixture::drawable_for_format(&device, wrong_extent, float);
                    let report = host
                        .drive(
                            Duration::ZERO,
                            Some(&SurfaceOutput {
                                view: &drawable,
                                extent: wrong_extent,
                            }),
                        )
                        .expect("drop the invalid surface without losing the host");
                    assert!(!report.surface_presented);
                    fixture::drain(&mut host);
                }
                "endpoint capture" => {
                    let last = host.last_submission();
                    assert_eq!(
                        host.capture_pixels().expect("capture endpoint"),
                        comparison.capture_pixels().expect("capture fresh endpoint"),
                    );
                    assert_eq!(host.last_submission(), last);
                }
                _ => {}
            }
            let actual = fixture::capture(&mut host, &device, extent, CaptureSource::Edr);
            assert_eq!(actual.rgba, expected, "{recipe}: {label}");
            println!("{recipe}: {label} matches a fresh float host");
        }
        // Admit the tap before the change so the next submission itself proves the new ceiling.
        host.enable_submitted_capture(40)
            .expect("default float tap");
        #[cfg(feature = "creation-counters")]
        let before = host.creation_report();
        host.set_display_headroom(1.7);
        #[cfg(feature = "creation-counters")]
        {
            let after = host.creation_report();
            assert_eq!(format!("{:?}", before.rows), format!("{:?}", after.rows));
            assert_eq!(
                format!("{:?}", before.pipelines),
                format!("{:?}", after.pipelines)
            );
        }
        let surface = fixture::drawable_for_format(&device, extent, float);
        fixture::submit(&mut host, &surface, extent);
        let first = fixture::submitted(&mut host, extent, CaptureSource::Edr);
        assert_eq!(
            first.edr_max_rgb().map(f32::to_bits),
            Some(half::f16::from_bits(0x3ecc).to_f32().to_bits())
        );
        host.disable_submitted_capture();
        host.set_display_headroom(2.0);

        extent = [71, 45];
        assert!(host.resize_scene_surface(extent).expect("resize targets"));
        let mut comparison = fresh(&capabilities, &queue, extent, float);
        comparison.set_display_headroom(2.0);
        let expected = fixture::capture(&mut comparison, &device, extent, CaptureSource::Edr).rgba;
        assert_eq!(
            fixture::capture(&mut host, &device, extent, CaptureSource::Edr).rgba,
            expected
        );
        let mut sdr = fresh(
            &capabilities,
            &queue,
            extent,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let sdr_surface =
            fixture::drawable_for_format(&device, extent, wgpu::TextureFormat::Rgba8Unorm);
        fixture::submit(&mut sdr, &sdr_surface, extent);
        let sdr_pixels = read_drawable(&device, &queue, &sdr_surface);
        assert!(sdr_pixels.as_chunks::<4>().0.contains(&[255, 188, 69, 255]));

        enable_sdr(&mut host, 41);
        let surface = fixture::drawable_for_format(&device, extent, float);
        fixture::submit(&mut host, &surface, extent);
        assert!(host.pending_frames() > 0);
        host.disable_submitted_capture();
        enable_sdr(&mut host, 42);
        verify_named_sdr(
            &mut host,
            &device,
            &queue,
            extent,
            42,
            (&sdr_pixels, &expected),
        );
        fixture::drain(&mut host);

        // Publish a new surface generation with an active capture, then restart the observer tap.
        publish(&mut host, extent, float);
        host.disable_submitted_capture();
        enable_sdr(&mut host, 43);
        verify_named_sdr(
            &mut host,
            &device,
            &queue,
            extent,
            43,
            (&sdr_pixels, &expected),
        );
        fixture::drain(&mut host);

        let inputs = host.device_prepare_inputs().expect("retain device inputs");
        let (new_device, new_queue, new_caps) = fixture::metal(&gpu.fresh_adapter(), recipe, 2);
        let mut next = pollster::block_on(inputs.prepare(
            &new_caps,
            &new_queue,
            Some(SurfaceProfile {
                extent,
                format: float,
            }),
        ))
        .expect("prepare new device");
        device.destroy();
        host.publish_device(&mut next).expect("publish new device");
        host.disable_submitted_capture();
        let mut comparison = fresh(&new_caps, &new_queue, extent, float);
        comparison.set_display_headroom(2.0);
        let fresh = fixture::capture(&mut comparison, &new_device, extent, CaptureSource::Edr);
        let actual = fixture::capture(&mut host, &new_device, extent, CaptureSource::Edr);
        assert_eq!(actual.device_generation, 2);
        assert_eq!(actual.rgba, fresh.rgba);
        enable_sdr(&mut host, 44);
        verify_named_sdr(
            &mut host,
            &new_device,
            &new_queue,
            extent,
            44,
            (&sdr_pixels, &expected),
        );
        println!(
            "{recipe}: resize, H=1.7, in-flight restart, surface and device replacements verified"
        );
    }
}

#[cfg(target_os = "macos")]
fn publish(host: &mut fixture::Host, extent: [u32; 2], format: wgpu::TextureFormat) {
    let candidate = host
        .extent_prepare_inputs_for_format(extent, format)
        .expect("retain generation inputs")
        .build(&std::sync::atomic::AtomicBool::new(false))
        .expect("build generation")
        .expect("the build was not cancelled");
    assert!(host.publish_extent(candidate));
}

#[cfg(target_os = "macos")]
fn read_drawable(device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView) -> Vec<u8> {
    thrum_test_support::readback::texture(
        device,
        queue,
        view.texture(),
        thrum_test_support::readback::Mip::level(0),
        ..,
    )
}

#[cfg(target_os = "macos")]
fn enable_sdr(host: &mut fixture::Host, session: u64) {
    host.enable_submitted_capture_from(
        session,
        thrum_render::runtime::CaptureLimits::default(),
        CaptureSource::Sdr,
    )
    .expect("enable named 8-bit stage");
}

#[cfg(target_os = "macos")]
fn verify_named_sdr(
    host: &mut fixture::Host,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    session: u64,
    (sdr, float): (&[u8], &[u8]),
) {
    let surface = fixture::drawable_for_format(device, extent, fixture::FLOAT);
    fixture::submit(host, &surface, extent);
    let image = fixture::submitted(host, extent, CaptureSource::Sdr);
    assert_eq!(image.session, session);
    assert_eq!(image.rgba.len(), sdr.len());
    assert!(image.rgba.iter().zip(sdr).all(|(a, b)| a.abs_diff(*b) <= 1));
    assert_eq!(read_drawable(device, queue, &surface), float);
}
