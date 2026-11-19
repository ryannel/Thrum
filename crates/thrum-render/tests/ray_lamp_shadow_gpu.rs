//! Metal direct-light probes, including a wall outside the camera and Sun passes.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod metal {
    use super::{ray_shadow_support as fixture, ray_support::support};
    use glam::{Mat4, Vec3};
    use thrum_render::{
        contribution::ShadowBounds,
        synthetic::{PreparedSynthetic, SyntheticPointLight, SyntheticView},
    };

    fn lamp(position: Vec3, radius: f32, candela: [f32; 3]) -> SyntheticPointLight {
        SyntheticPointLight {
            position: position.to_array(),
            radius,
            candela,
        }
    }

    fn count(device: &wgpu::Device, queue: &wgpu::Queue, prepared: &PreparedSynthetic) -> u32 {
        let result = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("unshadowed lamp reference readback"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(
            prepared.unshadowed_lamp_counter().expect("counter"),
            0,
            &result,
            0,
            4,
        );
        queue.submit([encoder.finish()]);
        u32::from_le_bytes(
            support::read(device, &result)
                .try_into()
                .expect("one integer"),
        )
    }

    #[test]
    fn direct_lamps_obey_distance_overlap_and_quality() {
        let gpu = support::gate("lamp visibility");
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

    fn outside_camera(view: &SyntheticView, centre: Vec3, half: Vec3) -> bool {
        let matrix = Mat4::from_cols_array_2d(&view.view_proj_columns());
        let corners = (0..8).map(|corner| {
            let point = Vec3::from_array(std::array::from_fn(|axis| {
                centre[axis] + half[axis] * if corner & (1 << axis) == 0 { -1.0 } else { 1.0 }
            }));
            matrix * point.extend(1.0)
        });
        corners.clone().all(|clip| clip.x < -clip.w)
            || corners.into_iter().all(|clip| clip.x > clip.w)
    }

    #[test]
    fn wall_outside_camera_and_sun_volumes_still_casts() {
        let gpu = support::gate("off-screen lamp caster");
        let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
        let wall = Vec3::new(-8.0, 3.7, 0.0);
        let half = Vec3::new(0.1, 1.0, 2.0);
        let bounds = ShadowBounds::new((wall - half).to_array(), (wall + half).to_array()).unwrap();
        let mut scene = fixture::scene(Vec3::ZERO, &[fixture::box_at(1, wall, half)]);
        scene.lamp_shadows = true;
        scene.sun_shadows = true;
        scene.local_lights = vec![lamp(Vec3::new(-16.0, 7.4, 0.0), 40.0, [10_000.0; 3])];
        for side in [-0.2, 0.5] {
            scene.view = fixture::view(Vec3::ZERO, side)
                .with_sun_shadow_profile(2, 4.0)
                .unwrap();
            assert!(outside_camera(&scene.view, wall, half));
            let lighting = fixture::sun(Vec3::Y, 0.0);
            let (prepared, target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
            let volumes = prepared.ray_inspection_shadow_volumes(None);
            assert!(
                volumes.len() >= 2 && volumes[..2].iter().all(|volume| !volume.overlaps(bounds))
            );
            let blocked = support::submit_bytes(&device, &queue, &prepared, &target);
            assert!(fixture::sample(&scene.view, &blocked, Vec3::ZERO)[0] <= 2);
            let mut control = scene.clone();
            control.instances[1].ids[3] = 0;
            let (prepared, target) = fixture::prepare(&device, &caps, &queue, &control, lighting);
            let clear = support::submit_bytes(&device, &queue, &prepared, &target);
            assert!(fixture::sample(&scene.view, &clear, Vec3::ZERO)[0] > 12);
        }
    }
}
