//! Submitted rain film shades the opaque bed without writing depth or admitting a query.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn standing_film_is_an_optical_residual() {
    use thrum_app::sky::ContentStatus;
    use water_scene::island::{Wind, build_rain_ground};

    let name = "standing_film_is_an_optical_residual";
    let Some(gpu) = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal)) else {
        return;
    };
    support::island::with_cooked_island(name, || {
        let extent = [192, 108];
        let mut arms = Vec::new();
        for draw_water in [false, true] {
            let (game, report, map, replies) = build_rain_ground(Wind::Calm, draw_water, true).expect("yard");
            let tap = support::depth::DepthTap::default();
            let game = game.with_contribution(Box::new(tap.contribution(extent)), || ContentStatus::Drawn(1));
            let mut f = support::Fixture::from_game(&gpu, extent, game, report);
            f.parameters(r#"{"camera.island":0,"rain.rate":0,"rain.drying":0,"rain.time_scale":1}"#);
            f.warm_cooked();
            f.frame_without_timings(2.0);
            let dry = f.image();
            let dry_depth = tap.read(&f);
            f.parameters(r#"{"rain.rate":10,"rain.time_scale":600}"#);
            for i in 1..=10 {
                f.frame_without_timings(2.0 + f64::from(i) / 60.0);
            }
            let sample = f.frame(2.0 + 11.0/60.0);
            f.finish_tick_queries();
            let image = f.image();
            let depth = tap.read(&f);
            assert_eq!(f.report.lock().expect("water report").hollow_surface_vertices, 0);
            assert!(!map.lock().expect("map").sample([147.2, 66.2]).is_some_and(|hit| hit.depth > 0.0));
            if draw_water {
                let replies = replies.lock().expect("query replies");
                let answers = replies.as_ref().expect("tick drain");
                assert!(answers.points.iter().all(|point| !point.surface.wet));
            }
            let names = sample.gpu_pass_ns.as_ref().expect("Metal timestamp").passes();
            assert_eq!(names.iter().any(|p| p.name.ends_with("shore film")), draw_water);
            let changed_depth = depth.iter().zip(dry_depth).filter(|(wet,dry)| wet.to_bits()!=dry.to_bits()).count();
            assert_eq!(changed_depth, 0, "standing film writes no depth");
            arms.push((dry, image));
        }
        let changed = |a: &[u8], b: &[u8]| a.chunks_exact(4).zip(b.chunks_exact(4))
            .filter(|(a,b)| a != b).count();
        let wet_delta = changed(&arms[0].1, &arms[1].1);
        let dry_delta = changed(&arms[0].0, &arms[1].0);
        assert!(wet_delta >= dry_delta + 32, "film coverage should change visibly: {wet_delta}, dry {dry_delta}");
    });
}
}
