//! Interaction carriage with spatially varying currents in one connected chart.
#[cfg(target_os = "macos")]
mod support;
#[cfg(target_os = "macos")]
mod current_ladder {
    use super::support::{Fixture, foam_buffer};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::{
        Water,
        interaction::{BodyPressure, Limits, Report, Witness},
    };
    use thrum_water_model::{
        Bed, Body, BodyKind, Chart, ChartSamples, FlowSample, Gauge, Impulse, Map, River, Stage,
    };

    const SIDE: usize = 67;
    const ORIGIN: f64 = -33.0;
    const STEP: Duration = Duration::from_nanos(16_666_667);
    const POOL: [f64; 2] = [-7.0, -15.0];
    const RIFFLE: [f64; 2] = [-7.0, 15.0];

    fn map(current: impl Fn(f64, f64) -> [f64; 2], ladder: bool) -> Map {
        let cells = SIDE * SIDE;
        let mut footprint = Vec::with_capacity(cells);
        let mut currents = Vec::with_capacity(cells);
        let mut levels = Vec::with_capacity(cells);
        let mut bed = Vec::with_capacity(cells);
        for z in 0..SIDE {
            for x in 0..SIDE {
                let wet = x > 0 && z > 0 && x < SIDE - 1 && z < SIDE - 1;
                footprint.push(wet);
                currents.push(if wet {
                    current(
                        ORIGIN + f64::from(u32::try_from(x).expect("column")),
                        ORIGIN + f64::from(u32::try_from(z).expect("row")),
                    )
                } else {
                    [0.0; 2]
                });
                levels.push(if wet { 0.0 } else { 0.95 });
                bed.push(if wet { -0.25 } else { 1.0 });
            }
        }
        let chart = Chart {
            ends: Vec::new(),
            footprint,
            levels: levels.clone(),
            currents: currents.clone(),
        };
        let stage = Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        };
        let mut map = Map::prepare(
            Bed::new([ORIGIN; 2], 1.0, [SIDE; 2], bed).expect("bed"),
            vec![Body {
                name: "Pool and riffle".into(),
                level: 0.0,
                kind: BodyKind::Chart(chart),
                stage,
                fetch: 100.0,
                gains: [0.0; 4],
            }],
            thrum_water_model::Profile {
                margin_height: 0.05,
                swash_ratio: 0.7,
            },
        )
        .expect("one chart");
        if ladder {
            let sample = FlowSample {
                levels,
                currents,
                loss: vec![0.0; cells],
                gains: vec![[0.0; 2]; cells],
            };
            map.add_river(River {
                name: "Pool and riffle".into(),
                gauge: Gauge::Stage(stage),
                gauges: vec![0.0, 1.0],
                charts: vec![ChartSamples {
                    body: 0,
                    samples: vec![sample.clone(), sample],
                    wetting: vec![vec![None; cells]],
                }],
            })
            .expect("sampled current chart");
        }
        map
    }

    type Plane = Vec<[f32; 2]>;
    type Snapshots = Vec<(Plane, Witness)>;

    fn simulate(
        gpu: &thrum_test_support::Gpu,
        map: Map,
        step: u32,
        end: u32,
        impulses: &[Impulse],
        budget: u32,
        sample_ticks: &[u32],
    ) -> (Snapshots, Report) {
        let spectrum = Spectrum {
            long: Vec::new(),
            tail_slope_variance: 0.0,
            cascades: [160.0, 37.0, 9.0].map(|m| Cascade::new(m, &[]).expect("rest spectrum")),
        };
        let profile = thrum_water::Profile {
            interaction: Some(Limits {
                solves: budget,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut water =
            Water::mapped(Arc::new(Mutex::new(map)), spectrum, profile).expect("carried map");
        let tap = water.tap_interaction();
        let mailbox = water.interaction_mailbox().expect("interaction");
        let report = water.report();
        let game = SkyGame::new(
            Controls::new(),
            Camera::look_at([0.0, 8.0, -0.1], [0.0, 0.0, 0.0], 48.0),
            |_, _| {},
        )
        .with_ground(None)
        .with_tick_duration(STEP)
        .expect("tick interval")
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
        let mut scene = Fixture::from_game(gpu, [12, 8], game, report);
        let follow = |tick| {
            mailbox.post_bodies(
                tick,
                Some(1),
                vec![BodyPressure {
                    id: 1,
                    place: [0.0; 2],
                    heading: 0.0,
                    footprint: [0.25; 2],
                    force: 0.0,
                }],
            );
        };
        follow(0);
        scene.frame_without_timings(0.0);
        for &impulse in impulses {
            mailbox.post_impulse(impulse);
        }
        let mut snapshots = Vec::new();
        let ticks = (step..end)
            .step_by(step as usize)
            .chain(std::iter::once(end));
        for tick in ticks {
            follow(u64::from(tick));
            scene.frame_without_timings((STEP * tick).as_secs_f64());
            if tick == end || sample_ticks.contains(&tick) {
                let witness = tap.latest().expect("submitted carriage endpoint");
                let plane: Plane = foam_buffer(&scene, &witness.buffer);
                snapshots.push((plane, witness));
            }
        }
        let report = scene
            .report
            .lock()
            .expect("water report")
            .interaction
            .expect("wake report");
        (snapshots, report)
    }

    fn impulse(id: u64, tick: u64, place: [f64; 2], vertical: f64) -> Impulse {
        Impulse {
            id,
            tick,
            place,
            vertical,
            radius: 0.50,
        }
    }

    fn moment((planes, witness): &(Plane, Witness), side: Option<bool>) -> (f64, [f64; 2]) {
        let endpoint = witness.endpoints[1];
        let [ox, oz] = endpoint.origins[0].map(f64::from);
        let plane = &planes[endpoint.offsets[0]..][..Witness::SIDE * Witness::SIDE];
        let mut energy = 0.0;
        let mut position = [0.0; 2];
        for (i, state) in plane.iter().enumerate() {
            let x = f64::from(u16::try_from(i % Witness::SIDE).expect("column"));
            let z = f64::from(u16::try_from(i / Witness::SIDE).expect("row"));
            let p = [ox + x * 0.25, oz + z * 0.25];
            if side.is_some_and(|right| right != p[1].is_sign_positive()) {
                continue;
            }
            let power = f64::from(state[0]).powi(2);
            energy += power;
            for (axis, weight) in position.iter_mut().enumerate() {
                *weight += power * p[axis];
            }
        }
        (energy, position.map(|sum| sum / energy))
    }

    #[expect(
        clippy::print_stderr,
        reason = "record each native ring centroid and the mean-current control"
    )]
    pub(super) fn separated(gpu: &thrum_test_support::Gpu) {
        let water = |_: f64, z: f64| [if z < 0.0 { 0.2 } else { 1.5 }, 0.0];
        for step in [1, 8] {
            let tick = u64::from(step);
            let run = |strength, ladder| {
                let mut inputs = vec![
                    impulse(0, tick, POOL, 160.0),
                    impulse(1, tick, RIFFLE, strength),
                ];
                inputs.extend([8_u64, 16, 24].map(|offset| {
                    impulse(
                        offset,
                        tick + offset,
                        [
                            RIFFLE[0]
                                + 1.5 * f64::from(u32::try_from(offset).expect("source tick"))
                                    / 60.0,
                            RIFFLE[1],
                        ],
                        strength * 0.012,
                    )
                }));
                simulate(gpu, map(water, ladder), step, step + 300, &inputs, 16, &[])
                    .0
                    .pop()
                    .expect("final endpoint")
            };
            let observed = run(160.0, true);
            let strong = run(800.0, true);
            let mean = run(160.0, false);
            let pool = moment(&observed, Some(false)).1;
            let riffle = moment(&observed, Some(true)).1;
            let strong_pool = moment(&strong, Some(false)).1;
            let mean = [moment(&mean, Some(false)).1, moment(&mean, Some(true)).1];
            eprintln!(
                "Metal {step}-tick carriage pool={pool:?} riffle={riffle:?} strong={strong_pool:?} window mean={mean:?}"
            );
            for (index, (actual, expected)) in [pool, riffle]
                .into_iter()
                .zip([[-6.0, -15.0], [0.5, 15.0]])
                .enumerate()
            {
                let error = (actual[0] - expected[0]).hypot(actual[1] - expected[1]);
                assert!(error < 0.5, "ring {index} centre error {error}");
                assert!(
                    (mean[index][0] - expected[0]).abs() > 0.5,
                    "one mean did not fail ring {index}"
                );
            }
            assert!((strong_pool[0] - pool[0]).hypot(strong_pool[1] - pool[1]) < 0.1);
        }
    }

    #[expect(
        clippy::print_stderr,
        reason = "record the submitted energy and carriage budget"
    )]
    pub(super) fn blend(gpu: &thrum_test_support::Gpu) {
        let origin = [-8.0, 0.0];
        let source = [impulse(0, 1, origin, 160.0)];
        let compare = |currents: fn(f64, f64) -> [f64; 2]| {
            simulate(
                gpu,
                map(currents, true),
                1,
                301,
                &source,
                16,
                &[61, 121, 181, 241],
            )
            .0
            .iter()
            .map(|frame| moment(frame, None))
            .collect::<Vec<_>>()
        };
        let rest = compare(|_, _| [0.0; 2]);
        for bend in [false, true] {
            let water: fn(f64, f64) -> [f64; 2] = if bend {
                |x, z| {
                    let speed = if z < -13.0 {
                        0.2
                    } else if z > 13.0 {
                        1.5
                    } else {
                        0.85
                    };
                    let angle = ((x + 8.0) * 0.025 + z * 0.015).clamp(-0.78, 0.78);
                    [speed * angle.cos(), speed * angle.sin()]
                }
            } else {
                |_, z| {
                    [
                        if z < -13.0 {
                            0.2
                        } else if z > 13.0 {
                            1.5
                        } else {
                            0.85
                        },
                        0.0,
                    ]
                }
            };
            let observed = compare(water);
            let ratio = observed.last().expect("final").0 / rest.last().expect("rest").0;
            let centre = observed.last().expect("final").1;
            eprintln!(
                "Metal blended bend={bend}, energy share={ratio}, centre={centre:?}, samples={observed:?}"
            );
            assert!((0.5..=1.0).contains(&ratio));
            for ((energy, _), (control, _)) in observed.iter().zip(&rest) {
                assert!(*energy <= *control);
            }
            assert!(centre[0] > origin[0] + 2.0);
            if !bend {
                assert!((centre[0] - (origin[0] + 0.85 * 5.0)).abs() < 0.5);
            }
        }
    }

    pub(super) fn budget(gpu: &thrum_test_support::Gpu) {
        let source = [impulse(0, 1, POOL, 160.0)];
        for speed in [0.0, 0.65] {
            let run = |enabled| {
                simulate(
                    gpu,
                    map(|_, _| [speed, 0.0], enabled),
                    1,
                    3,
                    &source,
                    16,
                    &[],
                )
            };
            let (new, new_report) = run(true);
            let (old, old_report) = run(false);
            assert_eq!(new_report.solves, old_report.solves);
            assert!(
                new[0]
                    .0
                    .iter()
                    .flatten()
                    .zip(old[0].0.iter().flatten())
                    .all(|(a, b)| a.to_bits() == b.to_bits())
            );
        }
        let (_, report) = simulate(
            gpu,
            map(|_, z| [if z < 0.0 { 0.2 } else { 1.5 }, 0.0], true),
            1,
            3,
            &source,
            1,
            &[],
        );
        assert!(report.solves.into_iter().all(|solves| solves <= 1));
        assert_eq!(report.refused_regions, [0; 2]);
        assert!(report.carriage[0].dropped > 0 && report.carriage[1].dropped > 0);
        assert!(report.carriage[1].maximum_error > 0.0);
    }
}

thrum_test_support::macos_gpu_test! {
fn carriage_budget_and_uniform_compatibility() {
    let Some(gpu) = thrum_test_support::gpu("current ladder budget", Some(wgpu::Backend::Metal)) else { return; };
    current_ladder::budget(&gpu);
}

}

thrum_test_support::macos_gpu_test! {
#[ignore = "long: WP6.10 compares whole five-second ring trajectories at both tick cadences"]
fn rings_stay_with_their_water() {
    let Some(gpu) = thrum_test_support::gpu("current ladder lanes", Some(wgpu::Backend::Metal)) else { return; };
    current_ladder::separated(&gpu);
}

}

thrum_test_support::macos_gpu_test! {
#[ignore = "long: WP6.10 checks five seconds of accumulated diffusion against still water"]
fn rings_survive_blended_currents() {
    let Some(gpu) = thrum_test_support::gpu("current ladder blends", Some(wgpu::Backend::Metal)) else { return; };
    current_ladder::blend(&gpu);
}
}
