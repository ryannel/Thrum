//! Sea reflection and the visible cloud layer on the same submitted frame.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn sea_reads_the_overlaid_reflection_and_draws_before_cloud_transport() {
    use support::Fixture;
    use water_scene::clouds::Selection;

    let Some(gpu) = thrum_test_support::gpu("clouds over sea", Some(wgpu::Backend::Metal)) else { return };
    let Some(selection) = Selection::from_query("?clouds=broken&cloud-sun=low&cloud-height=40") else {
        panic!("broken sky selection");
    };
    let [clear, cloudy] = [false, true].map(|enabled| {
        let scene = selection.build(false, enabled).expect("sea scene");
        let mut scene = Fixture::from_game(&gpu, [64, 36], scene.game.with_temporal_sampling(true), scene.report);
        scene.frame_without_timings(0.0);
        scene.frame_without_timings(0.033);
        let passes = scene.host.scene_graph().expect("compiled graph");
        let names: Vec<_> = passes.pass_names().collect();
        let position = |pass| names.iter().position(|name| name.ends_with(pass)).unwrap_or_else(|| panic!("{pass}: {names:?}"));
        if enabled {
            let overlay = position("cloud sky overlay");
            let reflection = names.iter().enumerate().find(|(_, name)| name.starts_with("atmosphere reflection")).expect("reflection filter").0;
            assert!(overlay < reflection && reflection < position("water surface"));
            assert!(position("water surface") < position("cloud trace"));
        }
        scene.image()
    });
    let changed_water = clear.as_chunks::<4>().0.iter().zip(cloudy.as_chunks::<4>().0)
        .skip(64 * 27).filter(|(clear, cloudy)| clear != cloudy).count();
    assert!(changed_water > 64, "weather must reach visible water");
}
}

#[cfg(target_os = "macos")]
mod clouds_capture;

thrum_test_support::macos_gpu_test! {
fn capture_selected_water_weather() {
    if let Some(mode) = std::env::var_os("THRUM_WATER_CLOUD_CAPTURE") {
        let gpu = thrum_test_support::gpu("water weather capture", Some(wgpu::Backend::Metal)).expect("Metal");
        clouds_capture::run(&gpu, &mode);
    }
}
}
