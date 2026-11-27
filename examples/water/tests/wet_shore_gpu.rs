//! Submitted island sand retains a damp band after the tide falls.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn wet_shore_follows_the_tide() {
    use water_scene::island::{Wind, build_wet_shore};
    let name = "wet_shore_follows_the_tide";
    let Some(gpu) = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal)) else {
        return;
    };
    support::island::with_cooked_island(name, || {
        let build = || {
            let (game, report, _, _) = build_wet_shore(Wind::Calm, false).expect("wet shore");
            let mut f = support::Fixture::from_game(&gpu, [256, 144], game, report);
            f.parameters(r#"{"camera.island":0,"sea.stage":0}"#);
            f.warm_cooked();
            f
        };
        let mut control = build();
        let mut ebb = build();
        control.parameters(r#"{"sea.stage":-0.5}"#);
        ebb.parameters(r#"{"sea.stage":0.5}"#);
        for i in 1..=4 {
            let t = 2.0 + f64::from(i) / 60.0;
            control.frame_without_timings(t);
            ebb.frame_without_timings(t);
        }
        ebb.parameters(r#"{"sea.stage":-0.5}"#);
        for i in 5..=12 {
            let t = 2.0 + f64::from(i) / 60.0;
            ebb.frame_without_timings(t);
            control.frame_without_timings(t);
        }
        let dry = control.image();
        let damp = ebb.image();
        let darker = damp.as_chunks::<4>().0.iter().zip(dry.as_chunks::<4>().0.iter())
            .filter(|(a,b)| a[0] < b[0] && a[1] < b[1] && a[2] < b[2]).count();
        assert!(darker >= 100, "falling tide left {darker} darker sand pixels");
        assert!(ebb.host.scene_shader_source().expect("forward source").contains("fn wet_shore("));
    });
}
}

thrum_test_support::macos_gpu_test! {
fn a_dry_hollow_keeps_the_authored_shore_film_edge() {
    use std::sync::{Arc, Mutex};
    use thrum_app::sky::ContentStatus;
    use thrum_sea_model::Spectrum;
    use thrum_water::Water;
    use thrum_water_model::Map;
    use water_scene::island::{Wind, build_rain_ground, build_wet_shore};

    let name = "a_dry_hollow_keeps_the_authored_shore_film_edge";
    let Some(gpu) = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal)) else {
        return;
    };
    support::island::with_cooked_island(name, || {
        let wind = Wind::Trade;
        let (_, _, yard, _) = build_rain_ground(wind, false, true).expect("yard");
        let hollows = yard.lock().unwrap().body(3).expect("yard hollows").clone();
        let width = 384;
        let height = 216;
        let record = |mixed, draw_water| {
            let (mut game, mut report, coast, _) = build_wet_shore(wind, false).expect("coast");
            if draw_water {
                let coast = coast.lock().expect("coastal map");
                let mut bodies: Vec<_> = (0..).map_while(|i| coast.body(i).cloned()).collect();
                if mixed { bodies.push(hollows.clone()); }
                let map = Map::prepare(coast.bed().clone(), bodies, coast.profile()).expect("shared coast and bed");
                let spectrum = Spectrum::build(&wind.forcing()).expect("trade spectrum");
                let profile = water_scene::profile(&spectrum);
                let mut water = Water::mapped(Arc::new(Mutex::new(map)), spectrum, profile).expect("water");
                water.enable_foam([5.0, 1.0]).expect("shore foam");
                report = water.report();
                game = game.with_contribution(Box::new(water), || ContentStatus::Drawn(1));
            }
            let mut f = support::Fixture::from_game(&gpu, [width, height], game, report);
            f.parameters(r#"{"camera.island":0,"sun.elevation":37}"#);
            f.warm_cooked();
            let frame = f.frame(2.0);
            let names = frame.gpu_pass_ns.as_ref().expect("Metal timestamps").passes();
            assert_eq!(draw_water, names.iter().any(|p| p.name.ends_with("shore film")));
            assert_eq!(f.report.lock().unwrap().hollow_surface_vertices, 0);
            f.image()
        };
        let ground = record(false, false);
        let coast = record(false, true);
        let mixed = record(true, true);
        let mut witness = 0;
        let mut difference = 0;
        for y in (height / 2 - 13)..(height / 2 + 10) {
            for x in (width * 17 / 20)..(width - 4) {
                let at = ((y * width + x) * 4) as usize;
                let visible = ground[at..at+3].iter().zip(&coast[at..at+3]).any(|(a,b)| a.abs_diff(*b) > 4);
                if visible {
                    witness += 1;
                    difference += usize::from(coast[at..at+3].iter().zip(&mixed[at..at+3]).any(|(a,b)| a.abs_diff(*b) > 2));
                }
            }
        }
        assert!(witness >= 40, "authored beach film visible at {witness} pixels");
        assert!(difference < witness / 10, "mixed shore differs at {difference} of {witness} film pixels");
    });
}
}
