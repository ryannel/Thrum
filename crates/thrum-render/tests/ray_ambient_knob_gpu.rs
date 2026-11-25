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
        let named = |name: &str| passes.iter().filter(|pass| pass.name.as_ref() == name).count();
        assert_eq!(named("traced Sun and ambient visibility trace"), 1);
        assert_eq!(named("traced Sun visibility trace"), 0);
        assert_eq!(named("traced ambient visibility"), 0);
        assert_eq!(named("traced ambient visibility filter"), usize::from(extent > 0));
        if extent == 0 {
            assert_eq!(bytes, expected, "off must restore the previous renderer's frame");
        } else {
            let read_sun = |target: &thrum_render::synthetic::OffscreenTarget| {
                thrum_test_support::readback::texture(&device, &queue, target.sun_disc_witness().unwrap(),
                    thrum_test_support::readback::Mip::level(0), ..)
            };
            assert_eq!(read_sun(&target), read_sun(&prior_target), "shared trace must preserve the Sun samples");
            let samples = renderer.ambient_ray_samples_for_probe(&target).expect("submitted ray history");
            let raw = thrum_test_support::readback::texture(
                &device, &queue, samples, thrum_test_support::readback::Mip::level(0), ..,
            );
            let columns = fixture::EXTENT as usize;
            let width = columns / if extent == 1 { 2 } else { 1 };
            let count = |x: usize, y: usize| raw[(y * columns + x) * 16 + 4];
            assert_eq!(count(width / 2, width / 2), 1, "shared trace must write ambient history");
            if width < columns {
                assert_eq!(count(width, width / 2), 0, "half extent must not trace past its last column");
            }
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
    println!("Soft Sun and ambient use one recorded trace, with ambient samples at the selected extent, exact off bytes and stable allocations");

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

    let (device, queue, caps) = support::device(&gpu, "ray-query", 3);
    let portable = caps.with_recipe_override("portable").unwrap();
    let (control, _) = fixture::prepare(&device, &portable, &queue, &prior_scene, lighting);
    let (declined, target) = fixture::prepare(&device, &portable, &queue, &scene, lighting);
    assert_eq!(declined.ambient_occlusion_selection().fallback, Some(RayStructureSelection::UnsupportedRecipe));
    assert_eq!(control.forward_source_for_probe(), declined.forward_source_for_probe());
    assert!(!declined.forward_source_for_probe().contains("ambient_contact"));
    assert_eq!(support::submit_bytes(&device, &queue, &declined, &target).len(),
        fixture::EXTENT.pow(2) as usize * 4);
    println!("Portable requested and ordinary Maps forward sources match byte for byte");
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

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn common_origin_sees_a_thin_sheet_at_one_hundred_metres() {
    use glam::{Mat4, Vec3, Vec4};
    use ray_shadow_support as fixture;
    use ray_support::support;
    use thrum_render::synthetic::SyntheticView;

    let Some(gpu) = support::gate("ambient origin at the supported caster separation") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let view = SyntheticView::perspective(
        [0.0, 100.0, 0.0], [0.0; 3], Vec3::Z.to_array(), 0.8, 1.0, 0.05, 400.0,
    ).unwrap();
    let raw_projection = Mat4::from_cols_array_2d(&view.view_proj_columns());
    let projected = raw_projection * Vec4::W;
    let z = projected.z / projected.w;
    let width = fixture::EXTENT as f32;
    let pixel = 0.5 + width / 2.0 + 1.0;
    let point = raw_projection.inverse() * Vec4::new(2.0 * pixel / width - 1.0, 0.0, z, 1.0);
    let x = point.x / point.w;
    // Keep the selected receiver column visible just outside the sheet edge.
    // The sheet stands 5 mm above the flat receiver and is 0.2 mm thick.
    let sheet = |gap: f32| {
        fixture::box_at(1, Vec3::new(x + 1.0005, gap + 0.0001, 0.0), Vec3::new(1.0, 0.0001, 1.8))
    };
    let lighting = fixture::sun(Vec3::Y, 0.0);
    let votes = |radius, casters: &[thrum_render::abi::InstanceRecord]| {
        let mut scene = fixture::scene(Vec3::ZERO, casters);
        scene.view = view;
        scene.ambient_occlusion_radius = Some(radius);
        let (mut renderer, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
        for _ in 0..16 {
            renderer.update_frame(&queue, &mut target, &scene, lighting, 1.0).unwrap();
            drop(support::submit_bytes(&device, &queue, &renderer, &target));
            renderer.commit_frame();
        }
        let samples = renderer.ambient_ray_samples_for_probe(&target).unwrap();
        let raw = thrum_test_support::readback::texture(
            &device, &queue, samples, thrum_test_support::readback::Mip::level(0), ..,
        );
        let columns = fixture::EXTENT as usize;
        let centre = columns / 4;
        (centre - 1..=centre + 1).map(|row| {
            let start = (row * columns + centre) * 16;
            let words: [u32; 4] = std::array::from_fn(|word| {
                u32::from_le_bytes(raw[start + word * 4..start + word * 4 + 4].try_into().unwrap())
            });
            assert_eq!(words[1], 16);
            assert!((f32::from_bits(words[2]) - z).abs() < z * 1e-5, "sample must see the receiver");
            words[1] - words[0].count_ones()
        }).sum::<u32>()
    };
    let clear = votes(2.0, &[]);
    let blocked = votes(2.0, &[sheet(0.005)]);
    assert_eq!(clear, 0, "the ray must leave the flat receiver without self-hits");
    assert!(blocked > 0, "the ray must reach the sheet above the supported separation");
    let inside = votes(0.006, &[sheet(0.0035)]);
    let outside = votes(0.006, &[sheet(0.00602)]);
    assert!(inside > 0, "a sheet in the supported opening must block within 6 mm");
    assert_eq!(outside, 0, "a lifted start must not extend beyond 6 mm from the visible face");
    println!("At a 6 mm radius: {inside}/48 hits at 3.5 mm, {outside}/48 at 6.02 mm");
    println!("At 100 m: {clear}/48 hits on the open plane, {blocked}/48 hits with the 5 mm sheet");
}
}

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn distant_thin_receiver_keeps_its_front_face_and_rejects_rear_sheets() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support;
    use thrum_render::synthetic::{FrameLighting, SyntheticView};

    let Some(gpu) = support::gate("ambient reach at the visible face of a thin plate") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let view = SyntheticView::perspective(
        [0.0, 100.0, 0.0], [0.0; 3], Vec3::Z.to_array(), 0.8, 1.0, 0.05, 400.0,
    ).unwrap();
    let FrameLighting::Photometric(mut sky) = fixture::sun(Vec3::Y, 0.0) else { unreachable!() };
    sky.ambient_lux = [2000.0; 3];
    let lighting = FrameLighting::Photometric(sky);
    let plate = |slot, top: f32, thickness: f32| {
        fixture::box_at(slot, Vec3::new(0.0, top - thickness / 2.0, 0.0),
            Vec3::new(8.0, thickness / 2.0, 8.0))
    };
    let render = |thickness, radius, rear: &[thrum_render::abi::InstanceRecord]| {
        let mut scene = fixture::scene(Vec3::ZERO, rear);
        scene.instances[0] = plate(0, 0.0, thickness);
        scene.view = view;
        scene.ambient_occlusion_radius = radius;
        let (mut renderer, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
        let mut colour = [0; 3];
        for _ in 0..18 {
            renderer.update_frame(&queue, &mut target, &scene, lighting, 1.0).unwrap();
            let pixels = support::submit_bytes(&device, &queue, &renderer, &target);
            colour = fixture::sample(&view, &pixels, Vec3::ZERO);
            renderer.commit_frame();
        }
        let votes = renderer.ambient_ray_samples_for_probe(&target).map(|samples| {
            let raw = thrum_test_support::readback::texture(
                &device, &queue, samples, thrum_test_support::readback::Mip::level(0), ..,
            );
            let columns = fixture::EXTENT as usize;
            let index = (columns / 4 * columns + columns / 4) * 16;
            let [bits, count, depth, _] = std::array::from_fn(|channel| {
                let start = index + 4 * channel;
                u32::from_le_bytes(raw[start..start + 4].try_into().unwrap())
            });
            assert_eq!(count, 16);
            (count - bits.count_ones(), depth)
        });
        (colour, votes)
    };

    let (off, _) = render(0.005, None, &[]);
    let (thin, thin_votes) = render(0.005, Some(2.0), &[]);
    let (thick, thick_votes) = render(0.2, Some(2.0), &[]);
    assert!(off.iter().any(|channel| *channel > 0), "the sky must light the control");
    assert_eq!(thin, off, "open sky on a thin visible upper face");
    assert_eq!(thick, off, "the same upper face on a thick plate");
    assert_eq!(thin_votes, thick_votes);
    let (blocked, depth) = thin_votes.unwrap();
    assert_eq!(blocked, 0);
    println!("At 100 m, 5 mm and 200 mm plates under open sky have {blocked}/16 blocked and match the off RGB {off:?}");

    let rear = [plate(1, -0.014, 0.0002), plate(2, -0.010, 0.0002)];
    for extra in [&rear[..0], &rear[..1], &rear] {
        let (colour, votes) = render(0.0002, Some(0.006), extra);
        assert_eq!(colour, off);
        assert_eq!(votes, Some((0, depth)), "rear geometry must leave the visible receiver's votes and depth alone");
        println!("At a 6 mm radius, rear sheets present = {}: zero blocked votes at the same depth", extra.len());
    }
}
}
