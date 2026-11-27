//! The submitted foam parcel rates and endpoints on the prepared island valley.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
mod river_foam {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_water::foam::{CHART_LOSS_BIRTH_RATE, CHART_LOSS_BREAKING, Input, Tap, Witness};
    use thrum_water_model::{FlowBands, FlowBandsProfile, Map};
    use water_scene::{channel, island::Wind, river};

    use super::support::{Fixture, foam_buffer, foam_stocks};

    const STEP: Duration = Duration::from_nanos(33_333_333);
    const STATIONS: [(&str, f64); 8] = [
        ("riffle", -84.0),
        ("headwater transition", -75.0),
        ("pool", -65.0),
        ("glide", -60.0),
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

    fn measure(scene: &Fixture, map: &Map, witness: &Witness, enabled: bool) -> [Station; 8] {
        let parcels: Vec<[[f32; 4]; 2]> = foam_buffer(scene, &witness.parcels);
        let stocks = foam_stocks(scene, witness);
        let plane = witness.side * witness.side;
        let first = (witness.banks[1] * 2) * plane as usize;
        let mut stations: [Station; 8] = std::array::from_fn(|_| Station::default());
        for i in 0..plane {
            let p = cell(witness, 0, i);
            let Some((station, _)) = STATIONS
                .iter()
                .enumerate()
                .find(|&(_, &(_, z))| (p[1] - z).abs() <= 2.25)
            else {
                continue;
            };
            let quiet = matches!(station, 2 | 3);
            let Some(row) = map
                .sample(p)
                .filter(|row| (quiet || row.depth > 0.055) && map.river_of(row.body).is_some())
            else {
                continue;
            };
            let (_, power) = map.flow(row.body, p);
            let observed_rate = parcels[i as usize][1][3];
            let [onset, full] = CHART_LOSS_BREAKING.map(f64::from);
            let ramp = ((power - onset) / (full - onset)).clamp(0.0, 1.0);
            let expected = if enabled {
                (f64::from(CHART_LOSS_BIRTH_RATE) * power).min(3.3)
                    * ramp
                    * ramp
                    * (3.0 - 2.0 * ramp)
            } else {
                0.0
            };
            if quiet || !enabled {
                assert_eq!(
                    observed_rate.to_bits(),
                    0,
                    "local birth at {p:?}: {observed_rate}"
                );
            }
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

    struct Trial {
        scene: Fixture,
        map: Arc<Mutex<Map>>,
        control: Input,
        tap: Tap,
    }

    fn trial(gpu: &thrum_test_support::Gpu, stage: f64) -> Trial {
        let forcing = channel::forcing(Wind::Calm);
        let (mut map, files) =
            river::map(&forcing, Some(&super::support::river_store())).expect("prepared island");
        let [lower, upper] = files[0].gauge_range();
        map.set_gauge(0, (upper - lower).mul_add(stage, lower))
            .expect("initial gauge");
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
        let camera = Camera::look_at([176.5, 19.0, -53.0], [180.0, 16.6, -46.0], 70.0);
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
        scene.frame_without_timings(0.0);
        Trial {
            scene,
            map,
            control,
            tap,
        }
    }

    impl Trial {
        fn advance(&mut self, ticks: std::ops::RangeInclusive<u32>) {
            for tick in ticks.step_by(4) {
                self.scene
                    .frame_without_timings(f64::from(tick) * STEP.as_secs_f64());
            }
        }

        fn measure(&self, enabled: bool) -> [Station; 8] {
            measure(
                &self.scene,
                &self.map.lock().expect("valley"),
                &self.tap.latest().expect("foam"),
                enabled,
            )
        }
    }

    #[expect(
        clippy::print_stdout,
        reason = "record measured loss, birth rate and cover at the valley stations"
    )]
    pub(super) fn transport(gpu: &thrum_test_support::Gpu) {
        let mut trial = trial(gpu, 0.5);
        trial.advance(4..=24);
        let current = trial.measure(true);
        trial.control.set_chart_loss_births(false);
        trial.advance(28..=68);
        let fading = trial.measure(false);

        println!(
            "chart loss birth constant {CHART_LOSS_BIRTH_RATE} m²/J, breaking transition {CHART_LOSS_BREAKING:?} W/m²"
        );
        for (((name, _), nominal), fading) in STATIONS.into_iter().zip(&current).zip(&fading) {
            println!(
                "{name}: {} wet cells; power {:.3} W/m²; birth {:.5}/s; cover {:.5}, births off {:.5}",
                nominal.cells, nominal.loss, nominal.births, nominal.cover, fading.cover
            );
        }
        assert!(current[4].births > 0.05 && current[5].births > 0.05 && current[6].births > 0.05);
        assert!(fading.iter().all(|station| station.births == 0.0));
        assert!(current.iter().all(|station| station.cells > 0));
        assert!(
            fading[1].cover >= current[1].cover,
            "the transition receives transported foam even though births are off"
        );
        assert!(fading[4].cover < current[4].cover && fading[5].cover < current[5].cover);
    }

    #[expect(clippy::print_stdout, reason = "record both gauge endpoints")]
    pub(super) fn gauges(gpu: &thrum_test_support::Gpu) {
        let endpoints = [0.0, 1.0].map(|stage| {
            let mut trial = trial(gpu, stage);
            trial.advance(4..=4);
            let first = trial.measure(true);
            assert!(first[4].births > 0.05 && first[5].births > 0.05 && first[6].births > 0.05);
            println!("gauge {stage:.1}, first step: pool {} cells at {:.3} W/m² and {:.6}/s, glide {} cells at {:.3} W/m² and {:.6}/s",
                first[2].cells, first[2].loss, first[2].births, first[3].cells, first[3].loss, first[3].births);
            trial.advance(8..=16);
            trial.measure(true)
        });
        let [low, high] = endpoints;
        for (((name, _), low), high) in STATIONS.into_iter().zip(&low).zip(&high) {
            println!("{name} cover: low {:.5}, high {:.5}", low.cover, high.cover);
        }
        assert!(low[4].cover < high[4].cover);
    }
}

thrum_test_support::macos_gpu_test! {
fn chart_loss_makes_and_carries_whitewater_on_the_valley() {
    let Some(gpu) = thrum_test_support::gpu("river whitewater", Some(wgpu::Backend::Metal)) else { return };
    river_foam::transport(&gpu);
}
}

thrum_test_support::macos_gpu_test! {
fn lowering_the_valley_gauge_reduces_foam_below_the_ledge() {
    let Some(gpu) = thrum_test_support::gpu("river gauge whitewater", Some(wgpu::Backend::Metal)) else { return };
    river_foam::gauges(&gpu);
}
}
