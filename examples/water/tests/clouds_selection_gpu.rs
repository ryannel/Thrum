//! The sea's opt-in weather state and contribution on the ordinary Metal route.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn sea_weather_is_opt_in_and_submitted_after_water() {
    use support::Fixture;
    use water_scene::{clouds::Selection, settings::Settings};

    let Some(gpu) = thrum_test_support::gpu("sea weather selection", Some(wgpu::Backend::Metal)) else { return };
    assert_eq!(Settings::from_query(""), Settings::OpenSea);
    assert!(Selection::from_query("").is_none());
    assert!(Selection::from_query("?clouds=off").is_none());
    let Some(selection) = Selection::from_query("?optics=trade&shade=1&clouds=fair") else {
        panic!("the selected weather replaces the stand-in");
    };
    assert_eq!(Selection::from_variables(|key| (key == "THRUM_WATER_CLOUDS").then(|| "fair".into())), Some(selection));
    let off = selection.build(false, false).expect("paired control");
    assert!(off.weather.is_none() && off.ground.is_none());
    let mut off = Fixture::from_game(&gpu, [48, 28], off.game.with_temporal_sampling(true), off.report);
    let first = off.frame_without_timings(0.0);
    assert!(off.host.scene_graph().expect("compiled graph").pass_names().all(|name| !name.contains("cloud ")));
    let enabled = selection.build(false, true).expect("weather scene");
    let tap = enabled.weather.expect("weather publication");
    let mut on = Fixture::from_game(&gpu, [48, 28], enabled.game, enabled.report);
    on.frame_without_timings(0.0);
    let sample = on.frame_without_timings(0.05);
    let passes = on.host.scene_graph().expect("compiled graph");
    let names: Vec<_> = passes.pass_names().collect();
    let position = |pass| names.iter().position(|name| name.ends_with(pass)).unwrap_or_else(|| panic!("{pass}: {names:?}"));
    assert!(position("water surface") < position("cloud trace"));
    assert!(position("cloud trace") < position("cloud compose"));
    assert!(position("cloud sky overlay") < position("water surface"));
    assert!(tap.weather_columns().is_some());
    assert!(tap.sky_products().is_some());
    assert!(sample.tick > first.tick);
}
}
