//! Metal proof that shadow preparation falls back before publishing a scene.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
#[test]
fn shadow_fallback_keeps_the_map_frame() {
    use ray_support::support;
    use thrum_render::capabilities::RayStructureSelection as Reason;
    use thrum_render::synthetic::{ShadowSelection, SunShadowMethod, SyntheticScene};

    let gpu = support::gate("shadow selection");
    let (device, queue, caps) = support::device(&gpu, "", 1);
    let (ray_device, ray_queue, ray_caps) = support::device(&gpu, "ray-query", 2);
    let portable = ray_caps
        .clone()
        .with_recipe_override("portable")
        .expect("portable");
    let injected = ray_caps.clone().with_traced_pipeline_failure();
    let mut scene = SyntheticScene::grid(9).expect("scene");
    scene.view = ray_shadow_support::view(glam::Vec3::ZERO, 3.2);
    scene.sun_shadows = true;
    for (device, queue, caps, reason) in [
        (&device, &queue, &caps, Reason::MissingRayQuery),
        (
            &ray_device,
            &ray_queue,
            &portable,
            Reason::UnsupportedRecipe,
        ),
        (
            &ray_device,
            &ray_queue,
            &injected,
            Reason::PreparationFailed,
        ),
    ] {
        let (maps, map_target) = support::prepare(device, queue, caps, &scene);
        let baseline = support::submit_bytes(device, queue, &maps, &map_target);
        assert_eq!(maps.shadow_selection(), ShadowSelection::default());
        assert_eq!(maps.ray_mesh_structure_count(), 0);
        let mut request = scene.clone();
        request.sun_shadow_method = SunShadowMethod::Traced;
        request.lamp_shadows = true;
        let (fallback, target) = support::prepare(device, queue, caps, &request);
        assert_eq!(
            fallback.shadow_selection(),
            ShadowSelection {
                requested_sun: SunShadowMethod::Traced,
                sun: SunShadowMethod::Maps,
                requested_lamps: true,
                lamps: false,
                fallback: Some(reason),
            }
        );
        assert_eq!(fallback.ray_mesh_structure_count(), 0);
        assert_eq!(
            fallback.ray_structure_input_usages(),
            maps.ray_structure_input_usages()
        );
        assert_eq!(
            support::submit_bytes(device, queue, &fallback, &target),
            baseline
        );
    }
    scene.sun_shadow_method = SunShadowMethod::Traced;
    scene.lamp_shadows = true;
    let (traced, target) = support::prepare(&ray_device, &ray_queue, &ray_caps, &scene);
    assert_eq!(traced.shadow_selection().sun, SunShadowMethod::Traced);
    assert!(traced.shadow_selection().lamps);
    assert_eq!(traced.shadow_selection().fallback, None);
    assert_eq!(
        support::submit_bytes(&ray_device, &ray_queue, &traced, &target).len(),
        64 * 64 * 4
    );
}
