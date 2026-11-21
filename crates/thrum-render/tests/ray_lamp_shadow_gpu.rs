//! Metal direct-light probes, including a wall outside the camera and Sun passes.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
mod metal {
    #[cfg(target_os = "macos")]
    use super::{ray_shadow_support as fixture, ray_support::support};
    #[cfg(target_os = "macos")]
    use glam::Vec3;
    #[cfg(target_os = "macos")]
    use thrum_render::{
        contribution::ShadowBounds,
        synthetic::{PreparedSynthetic, SyntheticPointLight, SyntheticView},
    };

    #[cfg(target_os = "macos")]
    fn lamp(position: Vec3, radius: f32, candela: [f32; 3]) -> SyntheticPointLight {
        SyntheticPointLight {
            position: position.to_array(),
            radius,
            candela,
        }
    }

    #[cfg(target_os = "macos")]
    fn count(device: &wgpu::Device, queue: &wgpu::Queue, prepared: &PreparedSynthetic) -> u32 {
        u32::from_le_bytes(
            thrum_test_support::readback::buffer(
                device,
                queue,
                prepared.unshadowed_lamp_counter().expect("counter"),
                0..4,
            )
            .try_into()
            .expect("one integer"),
        )
    }

    thrum_test_support::macos_gpu_test! {
    fn direct_lamps_obey_distance_overlap_and_quality() {
        let Some(gpu) = support::gate("lamp visibility") else {
            return;
        };
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        let source = Vec3::new(0.0, 1.0, 2.0);
        let half = Vec3::new(0.65, 0.65, 0.09);
        let wall = |distance| fixture::box_at(1, Vec3::new(0.0, 0.62, distance), half);
        let lighting = fixture::sun(Vec3::Y, 0.0);
        let red = lamp(source, 2.36, [10_000.0, 0.0, 0.0]);
        let green = lamp(Vec3::new(1.4, 1.0, -0.4), 4.0, [0.0, 500.0, 0.0]);
        let mut scene = fixture::scene(Vec3::ZERO, &[wall(1.0)]);
        scene.lamp_shadows = true;
        for (description, caster, lights, blocked, green_light) in [
            (
                "one lamp, near range edge",
                wall(1.0),
                vec![red],
                true,
                false,
            ),
            (
                "overlapping cluster lights",
                wall(1.0),
                vec![red, green],
                true,
                true,
            ),
            ("wall beyond lamp", wall(3.0), vec![red], false, false),
        ] {
            scene.instances[1] = caster;
            scene.local_lights = lights;
            let (mut prepared, target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
            assert!(prepared.shadow_selection().lamps);
            let pixels = support::submit_bytes(&device, &queue, &prepared, &target);
            let color = fixture::sample(&scene.view, &pixels, Vec3::ZERO);
            assert_eq!(color[0] <= 2, blocked, "{description}: {color:?}");
            assert_eq!(color[1] > 20, green_light, "{description}: {color:?}");
            assert_eq!(
                count(&device, &queue, &prepared),
                0,
                "capacity covers the cluster"
            );
            prepared
                .set_lamp_ray_limit_for_probe(&queue, 0)
                .expect("lower lamp quality");
            let pixels = support::submit_bytes(&device, &queue, &prepared, &target);
            let unshadowed = fixture::sample(&scene.view, &pixels, Vec3::ZERO);
            assert!(unshadowed[0] > 20, "{description}: {unshadowed:?}");
            let skipped = count(&device, &queue, &prepared);
            assert!(skipped > 0);
            println!("{description}: {color:?}, zero-ray {unshadowed:?}, omitted {skipped}");
        }
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn wall_outside_camera_and_sun_volumes_still_casts() {
        let Some(gpu) = support::gate("off-screen lamp caster") else {
            return;
        };
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        // See the receiver at the edge of the camera-centred height map.
        // The wall and the lamp are just past that edge, within a local light's reach.
        let receiver = Vec3::new(16_378.0, 0.0, 0.0);
        let wall = receiver + Vec3::new(16.0, 8.0, 0.0);
        let half = Vec3::new(0.15, 2.0, 2.0);
        let bounds = ShadowBounds::new((wall - half).to_array(), (wall + half).to_array()).unwrap();
        let mut scene = fixture::scene(receiver, &[fixture::box_at(1, wall, half)]);
        scene.lamp_shadows = true;
        scene.sun_shadows = true;
        scene.local_lights = vec![lamp(
            receiver + Vec3::new(26.0, 13.0, 0.0),
            40.0,
            [10_000.0; 3],
        )];
        for side in [-600.0, 800.0] {
            let eye = Vec3::new(0.0, 2300.0, side);
            let forward = (receiver - eye).normalize();
            let far = eye.distance(receiver) + 6.0;
            let nearest_wall = (wall - eye).dot(forward) - half.dot(forward.abs());
            assert!(
                nearest_wall > far,
                "wall near depth {nearest_wall}, camera far {far}"
            );
            scene.view = SyntheticView::perspective(
                eye.to_array(),
                receiver.to_array(),
                Vec3::Y.to_array(),
                0.0006,
                1.0,
                5.0,
                far,
            )
            .expect("camera stopping before the wall")
            .with_sun_shadow_profile(2, 12.0)
            .unwrap();
            let lighting = fixture::sun(Vec3::Y, 0.0);
            let (prepared, target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
            let graph = prepared
                .graph(&target, true)
                .expect("off-screen fixture graph");
            let passes: Vec<_> = graph.pass_names().collect();
            assert_eq!(
                passes
                    .iter()
                    .filter(|name| name.starts_with("synthetic Sun shadow "))
                    .count(),
                2
            );
            assert!(passes.contains(&"synthetic Sun horizon heights"));
            let volumes = prepared.ray_inspection_shadow_volumes(None);
            let overlaps: Vec<_> = volumes
                .iter()
                .map(|volume| volume.overlaps(bounds))
                .collect();
            assert_eq!(overlaps, [false, false, false], "camera at {eye:?}");
            let blocked = support::submit_bytes(&device, &queue, &prepared, &target);
            let blocked = fixture::sample(&scene.view, &blocked, receiver);
            assert!(blocked[0] <= 2, "blocked receiver: {blocked:?}");
            let mut control = scene.clone();
            control.instances[1].deactivate();
            let (prepared, target) = fixture::prepare(&device, &caps, &queue, &control, lighting);
            let clear = support::submit_bytes(&device, &queue, &prepared, &target);
            let clear = fixture::sample(&scene.view, &clear, receiver);
            assert!(clear[0] > 12, "clear receiver: {clear:?}");
            println!(
                "camera at {eye:?}, Sun overlap {overlaps:?}: wall {blocked:?}, removed {clear:?}"
            );
        }
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn mirrored_receiver_keeps_its_exposed_lamp_light() {
        let Some(gpu) = support::gate("mirrored lamp receiver") else {
            return;
        };
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        let mut scene = fixture::scene(Vec3::ZERO, &[]);
        scene.local_lights = vec![lamp(Vec3::new(0.4, 1.0, -0.3), 4.0, [500.0; 3])];
        let mut samples = Vec::new();
        for mirrored in [false, true] {
            if mirrored {
                let row = &mut scene.instances[0];
                row.current[0][0] *= -1.0;
                row.previous = row.current;
                row.normal[0][0] *= -1.0;
            }
            for lamps in [false, true] {
                scene.lamp_shadows = lamps;
                let (prepared, target) =
                    fixture::prepare(&device, &caps, &queue, &scene, fixture::sun(Vec3::Y, 0.0));
                let pixels = support::submit_bytes(&device, &queue, &prepared, &target);
                let rgb = fixture::sample(&scene.view, &pixels, Vec3::ZERO);
                println!("mirrored={mirrored}, traced lamps={lamps}: top {rgb:?}");
                assert!(rgb.into_iter().all(|component| component > 0));
                samples.push(rgb);
            }
        }
        assert!(samples.iter().all(|rgb| *rgb == samples[0]));
    }
    }
}
