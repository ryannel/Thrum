//! Read the forward-pass receiver's direct Sun under an occluder and beside it.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn sun_blocks_at_every_elevation_and_at_large_coordinates() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support;
    use std::sync::Arc;
    use thrum_render::synthetic::{
        SunShadowMethod, SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes, SyntheticVertex,
    };

    let Some(gpu) = support::gate("direct Sun ray answers") else {
        return;
    };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    for (degrees, x, distance) in [
        (1.0_f32, 0.0, 8.0),
        (10.0, 0.0, 5.0),
        (45.0, 0.0, 3.0),
        (90.0, 0.0, 2.0),
        (45.0, 0.0, 50.0),
        (90.0, 10_000.0, 0.045),
        (90.0, 20_000.0, 0.085),
    ] {
        let centre = Vec3::new(x, 0.0, 0.0);
        let far_wall = x > 0.0;
        let normal = if far_wall { Vec3::Z } else { Vec3::Y };
        let direction = if far_wall {
            Vec3::Z
        } else if degrees >= 90.0 {
            Vec3::Y
        } else {
            let (y, z) = degrees.to_radians().sin_cos();
            Vec3::new(0.0, y, z)
        };
        let half = if far_wall {
            Vec3::new(0.7, 0.6, 0.003)
        } else {
            Vec3::new(0.7, 0.07, 0.6)
        };
        let slab = fixture::box_at(1, centre + direction * distance, half);
        let mut scene = fixture::scene(centre, &[slab]);
        if far_wall {
            scene.instances[0] =
                fixture::box_at(0, centre - Vec3::Z * 0.1, Vec3::new(3.0, 2.0, 0.1));
        }
        scene.view = scene
            .view
            .with_sun_shadow_profile(2, 2.0)
            .expect("short cascade reach");
        scene.sun_shadows = true;
        scene.sun_shadow_method = SunShadowMethod::Traced;

        let lighting = fixture::sun(direction, 1500.0);
        let (mut prepared, mut target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
        assert_eq!(prepared.shadow_selection().sun, SunShadowMethod::Traced);
        let pixels = support::submit_bytes(&device, &queue, &prepared, &target);
        let clear_point = centre + Vec3::NEG_X * 1.6;
        let visibility = fixture::probe_sun(
            &device,
            &queue,
            &prepared,
            direction,
            normal,
            [centre, clear_point],
            support::read,
        );
        assert_eq!(
            visibility.map(f32::to_bits),
            [0.0_f32.to_bits(), 1.0_f32.to_bits()],
            "{degrees} degrees at {x} m"
        );
        if far_wall {
            let beyond = fixture::probe_sun(
                &device,
                &queue,
                &prepared,
                direction,
                normal,
                [centre + direction * (distance + 0.01), clear_point],
                support::read,
            );
            assert_eq!(beyond.map(f32::to_bits), [1.0_f32.to_bits(); 2]);
        } else {
            let dark = fixture::sample(&scene.view, &pixels, centre);
            let lit = fixture::sample(&scene.view, &pixels, clear_point);
            assert!(
                dark[0] <= 2 && lit[0] > 32,
                "{degrees} degrees at {x} m: dark {dark:?}, lit {lit:?}"
            );

            scene.instances[1].deactivate();
            let (clear, clear_target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
            let clear_pixels = support::submit_bytes(&device, &queue, &clear, &clear_target);
            let clear = fixture::sample(&scene.view, &clear_pixels, centre);
            assert!(
                clear[0] > 32,
                "receiver is visible and lit without the slab"
            );

            // Move both the prop and the slab 2 m in the displayed half-frame.
            // The current transform alone is 4 m away and would miss the ray.
            scene.instances[1].set_generation(1);
            for instance in &mut scene.instances {
                instance.current[3][0] += 4.0;
            }
            prepared
                .update_frame(&queue, &mut target, &scene, lighting, 0.5)
                .expect("sample moving frame");
            prepared
                .upload_instances(&queue, &scene.instances, &[0, 1])
                .expect("upload moving rows");
            let moved_pixels = support::submit_bytes(&device, &queue, &prepared, &target);
            let moved_point = centre + Vec3::X * 2.0;
            let moved = fixture::sample(&scene.view, &moved_pixels, moved_point);
            assert!(
                moved[0] <= 2,
                "raster and rays use the mover's presented pose: {moved:?}"
            );
            let moved_answers = fixture::probe_sun(
                &device,
                &queue,
                &prepared,
                direction,
                normal,
                [moved_point, moved_point + Vec3::NEG_X * 1.6],
                support::read,
            );
            assert_eq!(
                moved_answers.map(f32::to_bits),
                [0.0_f32.to_bits(), 1.0_f32.to_bits()]
            );

            // Preparing the receiver as a terrain row exercises per-mesh
            // geometry instead of the stock prop's cube.
            for instance in &mut scene.instances {
                instance.current = instance.previous;
            }
            scene.instances[0] = thrum_render::abi::InstanceRecord::identity(0, 0);
            scene.meshes = Some(Arc::new(SyntheticMeshes {
                meshes: vec![SyntheticMesh {
                    vertices: [
                        [-3.0, 0.0, -2.0],
                        [-3.0, 0.0, 2.0],
                        [3.0, 0.0, -2.0],
                        [3.0, 0.0, 2.0],
                    ]
                    .map(|position| SyntheticVertex {
                        position,
                        normal: [0.0, 1.0, 0.0],
                        uv: [0.0; 2],
                        tangent: [1.0, 0.0, 0.0, 1.0],
                    })
                    .to_vec(),
                    indices: vec![0, 1, 2, 2, 1, 3],
                }],
                slots: vec![SyntheticMeshChoice::Mesh(0), SyntheticMeshChoice::Cube],
                updates: None,
            }));
            let (terrain, target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
            let terrain_pixels = support::submit_bytes(&device, &queue, &terrain, &target);
            let shadow = fixture::sample(&scene.view, &terrain_pixels, centre);
            let light = fixture::sample(&scene.view, &terrain_pixels, clear_point);
            assert!(
                shadow[0] <= 2 && light[0] > 32,
                "terrain {degrees}°: {shadow:?}, {light:?}"
            );
        }
        if far_wall {
            println!("Wall at {x} m, panel {distance} m ahead: visibility {visibility:?}");
        } else {
            println!("Sun {degrees}° on prop, mover and terrain: visibility {visibility:?}");
        }
    }
}
}
