//! The submitted foam parcel rates and endpoints on the prepared island valley.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
mod river_foam {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use thrum_app::sky::{ContentStatus, Controls, SkyGame};
    use thrum_water::foam::{CHART_LOSS_BIRTH_RATE, Witness};
    use thrum_water_model::{FlowBands, FlowBandsProfile, Map};
    use water_scene::{channel, island::Wind, river};

    use super::support::{Fixture, foam_buffer, foam_stocks};

    const STEP: Duration = Duration::from_nanos(16_666_667);
    const STATIONS: [(&str, f64); 7] = [
        ("riffle", -84.0),
        ("glide", -75.0),
        ("pool", -65.0),
        ("ledge", -52.0),
        ("rocks", -41.0),
        ("chute", -32.0),
        ("plunge", -24.0),
    ];

    #[derive(Default)]
    struct Station {
        cells: u32,
        loss: f64,
        births: f32,
        cover: f32,
    }

    fn cell(witness: &Witness, bank: usize, id: u32) -> [f64; 2] {
        let origin = witness.origins[bank][0].map(f64::from);
        let xz = [id % witness.side, id / witness.side];
        std::array::from_fn(|axis| {
            origin[axis] + (0.5 + f64::from(xz[axis])) * f64::from(witness.spacing[0])
        })
    }

    fn measure(scene: &Fixture, map: &Map, witness: &Witness, enabled: bool) -> [Station; 7] {
        let parcels: Vec<[[f32; 4]; 2]> = foam_buffer(scene, &witness.parcels);
        let stocks = foam_stocks(scene, witness);
        let plane = witness.side * witness.side;
        let first = (witness.banks[1] * 2) * plane as usize;
        let mut stations: [Station; 7] = std::array::from_fn(|_| Station::default());
        for i in 0..plane {
            let p = cell(witness, 0, i);
            let Some((station, _)) = STATIONS
                .iter()
                .enumerate()
                .find(|&(_, &(_, z))| (p[1] - z).abs() <= 2.25)
            else {
                continue;
            };
            let Some(row) = map
                .sample(p)
                .filter(|row| row.depth > 0.055 && map.river_of(row.body).is_some())
            else {
                continue;
            };
            let (_, power) = map.flow(row.body, p);
            let observed_rate = parcels[i as usize][1][3];
            let expected = if enabled {
                (f64::from(CHART_LOSS_BIRTH_RATE) * power).min(3.3)
            } else {
                0.0
            };
            assert!(
                (f64::from(observed_rate) - expected).abs() < 0.017,
                "source power at {p:?}: {observed_rate} vs {expected} ({power} W/m²)"
            );
            let stats = &mut stations[station];
            stats.cells += 1;
            stats.loss = stats.loss.max(power);
            stats.births = stats.births.max(observed_rate);
            let visible = stocks[first + i as usize];
            stats.cover = stats.cover.max(visible[1] + visible[2]);
        }
        stations
    }

    #[expect(
        clippy::print_stdout,
        reason = "record measured loss, birth rate and cover at the valley stations"
    )]
    pub(super) fn run(gpu: &thrum_test_support::Gpu) {
        let forcing = channel::forcing(Wind::Calm);
        let (mut map, files) =
            river::map(&forcing, Some(&super::support::river_store())).expect("prepared island");
        let [lower, upper] = files[0].gauge_range();
        map.set_flow_bands(Some(
            FlowBands::new(FlowBandsProfile::default()).expect("flow bands"),
        ));
        let map = Arc::new(Mutex::new(map));
        let spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("calm");
        let profile = water_scene::profile(&spectrum);
        let mut water =
            thrum_water::Water::mapped(Arc::clone(&map), spectrum, profile).expect("mapped water");
        let control = water.enable_foam([0.0; 2]).expect("calm whitewater");
        let tap = water.tap_foam();
        let report = water.report();
        let camera = river::VIEWS[4].1;
        let game = SkyGame::new(Controls::new(), camera, |_, _| {})
            .with_tick_duration(STEP)
            .expect("step")
            .with_ground(None)
            .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
        let mut scene = Fixture::from_game(gpu, [64, 48], game, report);
        let knob = scene
            .host
            .quality_knobs()
            .into_iter()
            .find(|entry| entry.name.ends_with("river.flow_band_cadence"))
            .expect("flow cadence")
            .name;
        scene.host.set_quality_knob(&knob, 4).expect("cadence");
        let mut frame = |seconds: f64| drop(scene.frame_without_timings(seconds));
        frame(0.0);
        for tick in (4..=72).step_by(4) {
            frame(f64::from(tick) * STEP.as_secs_f64());
        }
        let current = measure(
            &scene,
            &map.lock().expect("valley"),
            &tap.latest().expect("foam"),
            true,
        );
        control.set_chart_loss_births(false);
        for tick in (76..=192).step_by(4) {
            scene.frame_without_timings(f64::from(tick) * STEP.as_secs_f64());
        }
        let fading = measure(
            &scene,
            &map.lock().expect("valley"),
            &tap.latest().expect("ageing foam"),
            false,
        );
        control.set_chart_loss_births(true);
        map.lock()
            .expect("valley")
            .set_gauge(0, lower)
            .expect("low gauge");
        for tick in (196..=228).step_by(4) {
            scene.frame_without_timings(f64::from(tick) * STEP.as_secs_f64());
        }
        let low = measure(
            &scene,
            &map.lock().expect("valley"),
            &tap.latest().expect("low foam"),
            true,
        );
        map.lock()
            .expect("valley")
            .set_gauge(0, upper)
            .expect("high gauge");
        for tick in (232..=264).step_by(4) {
            scene.frame_without_timings(f64::from(tick) * STEP.as_secs_f64());
        }
        let high = measure(
            &scene,
            &map.lock().expect("valley"),
            &tap.latest().expect("high foam"),
            true,
        );
        println!("chart loss birth constant {CHART_LOSS_BIRTH_RATE} m²/J");
        for (((((name, _), nominal), fading), low), high) in STATIONS
            .into_iter()
            .zip(&current)
            .zip(&fading)
            .zip(&low)
            .zip(&high)
        {
            println!(
                "{name}: {} wet cells; power {:.3} W/m²; birth {:.5}/s; cover {:.5}, births off {:.5}, low {:.5}, high {:.5}",
                nominal.cells,
                nominal.loss,
                nominal.births,
                nominal.cover,
                fading.cover,
                low.cover,
                high.cover
            );
        }
        assert!(current[3].births > 0.05 && current[4].births > 0.05 && current[5].births > 0.05);
        assert!(fading.iter().all(|station| station.births == 0.0));
        assert!(fading[3].cover < current[3].cover && fading[4].cover < current[4].cover);
        assert!(low[3].cover < high[3].cover);
    }
}

thrum_test_support::macos_gpu_test! {
fn chart_loss_makes_and_carries_whitewater_on_the_valley() {
    let Some(gpu) = thrum_test_support::gpu("river whitewater", Some(wgpu::Backend::Metal)) else { return };
    river_foam::run(&gpu);
}
}
