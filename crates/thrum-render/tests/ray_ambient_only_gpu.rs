//! A prepared ambient effect leaves direct Sun and lamp lighting byte-identical.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(feature = "creation-counters")]
thrum_test_support::macos_gpu_test! {
fn ambient_occlusion_never_changes_direct_light() {
    use glam::Vec3;
    use ray_shadow_support as fixture;
    use ray_support::support;
    use thrum_render::synthetic::{FrameLighting, SyntheticPointLight, SunShadowMethod};

    let Some(gpu) = support::gate("ambient occlusion direct-light isolation") else { return };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let mut scene = fixture::scene(Vec3::ZERO, &[
        fixture::box_at(1, Vec3::new(0.0, 0.5, 0.1), Vec3::new(0.25, 0.5, 0.25)),
    ]);
    scene.sun_shadows = true;
    scene.sun_shadow_method = SunShadowMethod::Traced;
    scene.lamp_shadows = true;
    scene.local_lights = vec![
        SyntheticPointLight { position: [0.8, 1.2, -1.0], candela: [600.0, 420.0, 240.0], radius: 4.0 },
    ];
    let direct = fixture::sun(Vec3::new(-0.4, 0.5, 0.7), 8_000.0);
    let (mut reference, mut target) = fixture::prepare(&device, &caps, &queue, &scene, direct);
    reference.update_frame(&queue, &mut target, &scene, direct, 1.0).unwrap();
    let expected = support::submit_bytes(&device, &queue, &reference, &target);

    scene.ambient_occlusion_radius = Some(1.5);
    let (mut traced, mut target) = fixture::prepare(&device, &caps, &queue, &scene, direct);
    assert_eq!(traced.ambient_occlusion_selection().radius, Some(1.5));
    let render = |traced: &mut thrum_render::synthetic::PreparedSynthetic,
                  target: &mut thrum_render::synthetic::OffscreenTarget, lighting| {
        traced.update_frame(&queue, target, &scene, lighting, 1.0).unwrap();
        let bytes = support::submit_bytes(&device, &queue, traced, target);
        traced.commit_frame();
        bytes
    };
    for _ in 0..18 {
        assert_eq!(render(&mut traced, &mut target, direct), expected);
    }

    // Positive control: add ambient only, leaving the same rays and geometry.
    let FrameLighting::Photometric(mut sky) = direct else { unreachable!() };
    sky.ambient_lux = [2_000.0; 3];
    let lighting = FrameLighting::Photometric(sky);
    let with = render(&mut traced, &mut target, lighting);
    traced.set_ambient_extent_for_probe(0).unwrap();
    let without = render(&mut traced, &mut target, lighting);
    assert_ne!(with, without, "the traced history must affect indirect lighting");
}
}
