//! Metal checks for runtime extent, ray reach and explicit fallback.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn ambient_extent_is_live_and_off_restores_the_prior_frame() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support;
    use thrum_render::{capabilities::RayStructureSelection, synthetic::{FrameLighting, SunShadowMethod}, timing::GpuTimings};

    let Some(gpu) = support::gate("ambient extent quality") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let mut scene = fixture::scene(Vec3::ZERO, &[
        fixture::box_at(1, Vec3::new(0.0, 0.5, 0.1), Vec3::new(0.25, 0.5, 0.25)),
    ]);
    scene.sun_shadows = true;
    scene.sun_shadow_method = SunShadowMethod::Traced;
    scene.sun_penumbra = true;
    let direct = fixture::sun(Vec3::new(-0.4, 0.5, 0.7), 8000.0);
    let FrameLighting::Photometric(mut sky) = direct else { unreachable!() };
    sky.ambient_lux = [2000.0; 3];
    let lighting = FrameLighting::Photometric(sky);

    let (mut prior, mut prior_target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
    let prior_scene = scene.clone();
    scene.ambient_occlusion_radius = Some(0.7);
    let (mut renderer, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
    assert_eq!(renderer.ambient_occlusion_selection().radius, Some(0.7));

    let mut timing = GpuTimings::new(&caps, &queue, 64).expect("Metal timings");
    let mut steady_counts = None;
    for extent in [0, 1, 0, 2, 1, 0] {
        prior.update_frame(&queue, &mut prior_target, &prior_scene, lighting, 1.0).unwrap();
        let expected = support::submit_bytes(&device, &queue, &prior, &prior_target);
        prior.commit_frame();
        renderer.set_ambient_extent_for_probe(extent).unwrap();
        renderer.update_frame(&queue, &mut target, &scene, lighting, 1.0).unwrap();
        let mut readback = target.readback(&device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode_with_readback(&mut encoder, 1, &target, Some(&mut timing), &mut readback).unwrap();
        timing.finish_render(1, &encoder).unwrap();
        queue.submit([encoder.finish()]);
        renderer.commit_frame();
        support::wait(&device);
        let bytes = readback.try_collect(1).unwrap().unwrap();
        let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert!(timing.resolve_if_ready(1, &mut resolve).unwrap());
        queue.submit([resolve.finish()]);
        support::wait(&device);
        let passes = timing.try_collect(1).unwrap().unwrap();
        let ambient_passes = passes.iter().filter(|pass| pass.name.starts_with("traced ambient visibility")).count();
        assert_eq!(ambient_passes, if extent == 0 { 0 } else { 2 });
        if extent == 0 {
            assert_eq!(bytes, expected, "off must restore the previous renderer's frame");
        }
        drop(readback);
        let hal = device.get_internal_counters().hal;
        let counts = [hal.textures.read(), hal.texture_views.read(), hal.buffers.read(),
            hal.bind_groups.read(), hal.shader_modules.read(), hal.compute_pipelines.read(), hal.render_pipelines.read()];
        if let Some(baseline) = steady_counts {
            assert_eq!(counts, baseline, "changing extent must reuse the prepared GPU objects");
        } else {
            steady_counts = Some(counts);
        }
    }
    println!("On/half/full/off: recorded visibility passes only while enabled, exact off bytes, stable GPU allocation counts");

    let (device, queue, caps) = support::device(&gpu, "", 2);
    let (fallback, target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
    let selection = fallback.ambient_occlusion_selection();
    assert_eq!(selection.requested_radius, Some(0.7));
    assert_eq!(selection.radius, None);
    assert_eq!(selection.fallback, Some(RayStructureSelection::MissingRayQuery));
    assert_eq!(fallback.ambient_occlusion_quality(), []);
    assert_eq!(support::submit_bytes(&device, &queue, &fallback, &target).len(),
        fixture::EXTENT.pow(2) as usize * 4);
    println!("No ray-query device: {selection:?}, ready Maps frame");
}
}

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn ambient_rays_stop_at_the_declared_radius() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support;
    use thrum_render::synthetic::FrameLighting;

    let Some(gpu) = support::gate("ambient ray distance") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let receiver = Vec3::ZERO;
    // The near face of the wall is 0.7 m to the witness's left. The
    // camera looks from the opposite side so the witness remains visible.
    let wall = fixture::box_at(1, Vec3::new(-0.8, 0.55, 0.5), Vec3::new(0.1, 0.55, 0.8));
    let mut scene = fixture::scene(receiver, &[wall]);
    scene.view = fixture::view(receiver, 3.2);
    let FrameLighting::Photometric(mut sky) = fixture::sun(Vec3::Y, 0.0) else { unreachable!() };
    sky.ambient_lux = [100.0; 3];
    let lighting = FrameLighting::Photometric(sky);
    let clip = glam::Mat4::from_cols_array_2d(&scene.view.view_proj_columns()) * receiver.extend(1.0);
    assert!(clip.x.abs() + clip.y.abs() < 1e-6);
    let mut render = |radius| {
        scene.ambient_occlusion_radius = Some(radius);
        let (mut renderer, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
        assert_eq!(renderer.ambient_occlusion_selection().radius, Some(radius));
        for _ in 0..16 {
            renderer.update_frame(&queue, &mut target, &scene, lighting, 1.0).unwrap();
            drop(support::submit_bytes(&device, &queue, &renderer, &target));
            renderer.commit_frame();
        }
        let samples = renderer.ambient_ray_samples_for_probe(&target).expect("submitted ray history");
        let bytes = thrum_test_support::readback::texture(
            &device, &queue, samples, thrum_test_support::readback::Mip::level(0), ..,
        );
        // The camera aims at the witness at the image centre. Half extent
        // maps this point to (48, 48), using the full texture's row stride.
        let centre = fixture::EXTENT as usize / 4;
        let columns = fixture::EXTENT as usize;
        let votes = |x, y| {
            let start = (y * columns + x) * 16;
            let [bits, count, depth, _] = std::array::from_fn(|channel| {
                let byte = start + 4 * channel;
                u32::from_le_bytes(bytes[byte..byte + 4].try_into().unwrap())
            });
            assert_eq!(count, 16, "settled ray history at {x} {y}");
            assert!((f32::from_bits(depth) - clip.z / clip.w).abs() < 0.001,
                "the samples must be on the receiver");
            count - bits.count_ones()
        };
        (centre-1..=centre+1).flat_map(|y| (centre-1..=centre+1).map(move |x| votes(x, y))).sum::<u32>()
    };
    let short = render(0.35);
    let long = render(2.0);
    assert_eq!(short, 0, "a ray shorter than the gap cannot hit the wall");
    assert!(long > 0, "longer rays must be able to reach the wall");
    println!("Receiver beside wall: 0.35 m {short}/144 blocked, 2.00 m {long}/144 blocked");
}
}
