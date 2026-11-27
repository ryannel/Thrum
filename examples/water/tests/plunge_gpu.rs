//! Submitted plunge sources, foam and interaction waves.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
#[expect(clippy::print_stderr, reason = "measure birth stocks and ring footprints on Metal")]
fn running_lip_feeds_the_pool_and_the_dry_sample_feeds_nothing() {
    use support::fall;
    use thrum_app::sky::Camera;
    use thrum_water::interaction::Witness;

    let Some(gpu) = thrum_test_support::gpu("plunge stocks and rings", Some(wgpu::Backend::Metal)) else { return; };
    let camera = Camera::look_at([0.0, 7.4, 4.0], [0.0, 0.0, 3.0], 55.0);
    let trial = |gauge| {
        let fall::Scene { game, report, map, foam, wake } = fall::scene(gauge, true, true, camera);
        let step = std::time::Duration::from_millis(50);
        let game = game.with_tick_duration(step).expect("compact transport clock");
        let mut scene = support::Fixture::from_game(&gpu, [20, 12], game, report);
        let foam = foam.expect("stocks");
        let wake = wake.expect("endpoints");
        for tick in (0..=24).step_by(8) {
            scene.frame_without_timings((step * tick).as_secs_f64());
        }
        let sample = foam.latest().expect("foam submitted");
        let stocks = support::foam_stocks(&scene, &sample);
        let plane = sample.side as usize * sample.side as usize;
        let start = sample.banks[1] * 2 * plane;
        let [x0, z0] = sample.origins[1][0];
        let spacing = sample.spacing[0];
        let mut strongest = [0.0_f32; 3];
        for (i, q) in stocks[start..][..plane].iter().enumerate() {
            let x = x0 + (f32::from(u16::try_from(i % sample.side as usize).expect("column")) + 0.5) * spacing;
            let z = z0 + (f32::from(u16::try_from(i / sample.side as usize).expect("row")) + 0.5) * spacing;
            if x.abs() < 1.5 && (1.0..2.2).contains(&z) {
                strongest[0] = strongest[0].max(q[0]);
                strongest[1] = strongest[1].max(q[1]);
            }
            if x.abs() < 1.5 && (2.3..3.7).contains(&z) {
                strongest[2] = strongest[2].max(q[1] + q[2]);
            }
        }
        let wake = wake.latest().expect("interaction submitted");
        let state: Vec<[f32; 2]> = support::foam_buffer(&scene, &wake.buffer);
        let end = wake.endpoints[1];
        let [_, z0] = end.origins[0];
        let live = state[end.offsets[0]..][..Witness::SIDE * Witness::SIDE].iter().enumerate()
            .filter(|(i, value)| {
                let z = z0 + f32::from(u16::try_from(i / Witness::SIDE).expect("row")) * Witness::CELL_METRES[0];
                z > 2.2 && value[0].abs() > 1e-7
            }).count();
        assert_eq!(map.lock().expect("map").gauge(0).to_bits(), f64::to_bits(gauge));
        (strongest, live)
    };
    let (running, rings) = trial(1.0);
    let (dry, dry_rings) = trial(0.0);
    eprintln!("Metal plunge air, fresh film, carried film={running:?}; ring cells={rings}; dry={dry:?}/{dry_rings}");
    assert!(running[0] > 0.02 && running[1] > 0.02 && running[2] > 0.01 && rings > 20);
    assert_eq!(dry.map(f32::to_bits), [0; 3]);
    assert_eq!(dry_rings, 0);
}
}
