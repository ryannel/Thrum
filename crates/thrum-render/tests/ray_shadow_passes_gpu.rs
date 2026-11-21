//! Compare the submitted height field and the graph on Apple tile Metal.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn traced_skips_surface_cascades_but_keeps_air_and_sun_off() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support;
    use thrum_render::{
        atmosphere::AtmosphereFrame,
        synthetic::{FrameLighting, SunShadowMethod},
    };

    let Some(gpu) = support::gate("traced Sun pass selection") else {
        return;
    };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let caps = caps.with_ray_preparation_observation();
    let mut scene = fixture::scene(
        Vec3::ZERO,
        &[fixture::box_at(
            1,
            Vec3::new(128.0, 60.0, 340.0),
            Vec3::new(128.0, 70.0, 128.0),
        )],
    );
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.24, 0.055, 0.89], 100_000.0, [1.0; 3], 2.0, 1).expect("atmosphere"),
    );
    let mut heights = None;
    let mut unshadowed = None;
    for sun in [true, false] {
        scene.sun_shadows = sun;
        for method in [SunShadowMethod::Maps, SunShadowMethod::Traced] {
            scene.sun_shadow_method = method;
            let (prepared, target) = fixture::prepare(&device, &caps, &queue, &scene, lighting);
            assert_eq!(prepared.shadow_selection().sun, method);
            let graph = prepared.graph(&target, true).expect("compiled graph");
            let passes = graph.pass_names().collect::<Vec<_>>();
            let cascades = passes
                .iter()
                .filter(|name| name.starts_with("synthetic Sun shadow "))
                .count();
            assert_eq!(
                cascades,
                if sun && method == SunShadowMethod::Maps {
                    2
                } else {
                    0
                }
            );
            assert_eq!(passes.contains(&"synthetic Sun horizon heights"), sun);
            assert_eq!(passes.contains(&"synthetic Sun air shadow heights"), sun);
            assert_eq!(
                passes.contains(&"ray structure build"),
                sun && method == SunShadowMethod::Traced
            );
            let pixels = support::submit_bytes(&device, &queue, &prepared, &target);
            if !sun {
                if let Some(maps) = &unshadowed {
                    assert_eq!(&pixels, maps);
                } else {
                    unshadowed = Some(pixels);
                }
                continue;
            }
            let texture = target.air_shadow_witness().expect("air shadow heights");
            let bytes = thrum_test_support::readback::texture(
                &device,
                &queue,
                texture,
                thrum_test_support::readback::Mip::level(0),
                ..,
            );
            let byte_count = bytes.len();
            if let Some(maps) = &heights {
                assert!(maps == &bytes, "air shadow height textures differ");
            } else {
                assert!(
                    bytes
                        .as_chunks::<8>()
                        .0
                        .windows(2)
                        .any(|pair| pair[0] != pair[1]),
                    "height field must contain a shadow"
                );
                heights = Some(bytes);
            }
            println!("{method:?}: {cascades} cascades, {byte_count} air-shadow bytes");
        }
    }
}
}
