//! Change admitted light budgets while the ordinary Metal scene is running.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn live_light_knobs_use_the_prepared_pipelines() {
    use support::optics::{EXTENT, Rig, attach, camera, sun, water_configured};
    use thrum_render::contribution::QualityInvalidation;
    use thrum_water::{Optics, Profile};

    let Some(gpu) = thrum_test_support::gpu("underwater Sun controls", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = camera([0.0, -1.2, -1.0], [0.0, -2.6, 0.0], 0.66, 1.0);
    let mut absent = Rig::new(&gpu, &view, sun(1.0, 0.0), &[], None);
    let source = absent.host.scene_shader_source().expect("forward shader").to_owned();
    let (_, dry) = absent.scene_colour();
    let mut disabled = water_configured(None, Profile::default());
    let ladder = disabled.tap_caustics();
    let march = disabled.tap_shafts();
    assert!(disabled.sun_path_provider().is_none());
    attach(&mut absent, disabled);
    assert_eq!(absent.host.scene_shader_source(), Some(source.as_str()));
    assert_eq!(absent.scene_colour().1, dry);
    assert!(ladder.texture().is_none() && march.texture().is_none());

    let water = water_configured(None, Profile {
        caustics: true, shafts: true, optics: Some(Optics::default()), underwater: true,
        ..Profile::default()
    });
    let mut rig = Rig::with_scene(
        support::device(&gpu.fresh_adapter()),
        &view,
        sun(1.0, 0.0),
        &[],
        None,
        EXTENT,
        |scene| scene.sun_path = water.sun_path_provider(),
    );
    assert!(rig.host.scene_shader_source().expect("forward shader").contains("sun_path("));
    attach(&mut rig, water);
    rig.frame();
    let pipelines = rig.host.pipeline_creation_count().expect("creation ledger");
    let knobs: Vec<_> = rig.host.quality_knobs().into_iter().filter(|knob| knob.name.starts_with("water.caustic_") || knob.name.starts_with("water.shaft_")).collect();
    assert_eq!(knobs.len(), 6);
    for knob in knobs {
        let expected = if knob.name.starts_with("water.shaft_") {
            QualityInvalidation::None
        } else {
            QualityInvalidation::EffectHistory
        };
        assert_eq!(knob.invalidation, expected);
        for value in [*knob.range.start(), *knob.range.end()] {
            rig.host.set_quality_knob(&knob.name, value).unwrap();
            rig.frame();
            assert_eq!(rig.host.quality_knobs().iter().find(|value| value.name == knob.name).unwrap().value, value);
            assert_eq!(rig.host.pipeline_creation_count(), Some(pipelines));
        }
    }
}
}
