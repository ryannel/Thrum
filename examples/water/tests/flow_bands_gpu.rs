//! Submitted Metal depth and the game's query mailbox, in calm water.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
mod flow {
    use super::support::{Fixture, depth};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_water::query::{BodyQuery, QueryLimits, QueryPoint};
    use thrum_water_model::{
        Bed, Body, BodyKind, Chart, ChartSamples, FlowBands, FlowBandsProfile, FlowSample, Gauge,
        Map, River, Stage,
    };
    use water_scene::{channel, island::Wind};

    const STEP: Duration = Duration::from_nanos(16_666_667);
    const EXTENT: [u32; 2] = [97, 9];
    const WIDTH: usize = 44;
    const HEIGHT: usize = 39;
    const LEVEL: f64 = 1.0;
    const POINTS: usize = 32;

    fn inland(kind: BodyKind, name: &str, level: f64) -> Body {
        Body {
            name: name.into(),
            kind,
            level,
            fetch: 100.0,
            gains: [0.0; 4],
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
        }
    }

    fn bed_index(index: usize) -> (usize, usize) {
        (index % WIDTH, index / WIDTH)
    }

    fn map(isolate_bands: bool) -> Map {
        let cells = WIDTH * HEIGHT;
        let bed = (0..cells)
            .map(|index| {
                let (x, z) = bed_index(index);
                if (3..=40).contains(&x)
                    && [5..=9, 14..=18, 23..=27, 32..=36]
                        .iter()
                        .any(|range| range.contains(&z))
                {
                    0.0
                } else {
                    3.0
                }
            })
            .collect();
        let mut bodies = vec![
            inland(BodyKind::Sea, "Sea", -2.0),
            inland(BodyKind::Basin { seed: [8.0, 17.0] }, "Lake", LEVEL),
        ];
        let mut charts = Vec::new();
        for (j, (speed, gain)) in [(0.5, 0.9), (1.5, 0.9), (0.0, 0.035)]
            .into_iter()
            .enumerate()
        {
            let z = 5 + j * 9;
            let footprint: Vec<_> = (0..cells)
                .map(|index| {
                    let (x, v) = bed_index(index);
                    (3..=40).contains(&x) && (z..=z + 4).contains(&v)
                })
                .collect();
            let levels: Vec<_> = footprint
                .iter()
                .map(|&wet| if wet { LEVEL } else { 2.8 })
                .collect();
            let currents: Vec<_> = footprint
                .iter()
                .map(|&wet| [if wet { speed } else { 0.0 }, 0.0])
                .collect();
            let gains = footprint
                .iter()
                .enumerate()
                .map(|(index, &wet)| {
                    if !wet {
                        return [0.0; 2];
                    }
                    let (x, _) = bed_index(index);
                    if isolate_bands && j == 0 && x < 13 {
                        [gain, 0.0]
                    } else if isolate_bands && j == 0 && x > 29 {
                        [0.0, gain]
                    } else {
                        [gain; 2]
                    }
                })
                .collect();
            charts.push(ChartSamples {
                body: bodies.len(),
                samples: vec![FlowSample {
                    levels: levels.clone(),
                    currents: currents.clone(),
                    loss: vec![0.0; cells],
                    gains,
                }],
                wetting: Vec::new(),
            });
            bodies.push(inland(
                BodyKind::Chart(Chart {
                    levels,
                    currents,
                    footprint,
                    ends: Vec::new(),
                }),
                &format!("Reach {j}"),
                0.0,
            ));
        }
        let mut map = Map::prepare(
            Bed::new([0.0; 2], 0.5, [WIDTH, HEIGHT], bed).expect("test bed"),
            bodies,
            thrum_water_model::Profile {
                margin_height: 0.1,
                swash_ratio: 0.7,
            },
        )
        .expect("test map");
        for (j, chart) in charts.into_iter().enumerate() {
            map.add_river(River {
                name: format!("Reach {j}"),
                gauges: vec![0.0],
                gauge: Gauge::Stage(Stage {
                    min: 0.0,
                    max: 0.0,
                    value: 0.0,
                }),
                charts: vec![chart],
            })
            .expect("flow chart");
        }
        map.set_flow_bands(Some(
            FlowBands::new(FlowBandsProfile::default()).expect("flow bands"),
        ));
        map
    }

    fn camera(z: f32) -> Camera {
        Camera::look_at([11.0, 2.35, z - 0.001], [11.0, 1.0, z], 40.0)
    }

    struct Scene {
        fixture: Fixture,
        map: Map,
        camera: Arc<Mutex<Camera>>,
        query: thrum_water::query::QueryMailbox,
        depth: depth::DepthTap,
    }

    fn scene(gpu: &thrum_test_support::Gpu, map: Map, view: Camera) -> Scene {
        let spectrum =
            thrum_sea_model::Spectrum::build(&channel::forcing(Wind::Calm)).expect("calm");
        let profile = thrum_water::Profile {
            query: Some(QueryLimits {
                points: 128,
                max_age: 8,
            }),
            ..water_scene::profile(&spectrum)
        };
        let water =
            thrum_water::Water::mapped(Arc::new(Mutex::new(map.clone())), spectrum, profile)
                .expect("water");
        let query = water.query_mailbox().expect("live query");
        let report = water.report();
        let depth = depth::DepthTap::default();
        let camera = Arc::new(Mutex::new(view));
        let view = Arc::clone(&camera);
        let game = SkyGame::new(Controls::new(), *camera.lock().expect("camera"), |_, _| {})
            .with_presentation_camera(move |_| *view.lock().expect("camera"))
            .with_tick_duration(STEP)
            .expect("step")
            .with_ground(None)
            .with_contribution(Box::new(water), || ContentStatus::Drawn(1))
            .with_contribution(Box::new(depth.contribution(EXTENT)), || {
                ContentStatus::Drawn(1)
            });
        let fixture = Fixture::from_game(gpu, EXTENT, game, report);
        Scene {
            fixture,
            map,
            camera,
            query,
            depth,
        }
    }

    fn lane(band: usize) -> f32 {
        [3.5, 8.0, 12.5, 17.0][band]
    }

    fn interpolated(
        bands: &FlowBands,
        band: usize,
        mut point: [f64; 2],
        time: f64,
        speed: f64,
    ) -> [f64; 4] {
        point[0] -= time * speed;
        let texel = bands.profile().patches[band] / 256.0;
        let p = point.map(|v| v / texel - 0.5);
        let start = p.map(f64::floor);
        let f = std::array::from_fn::<_, 2, _>(|i| p[i] - start[i]);
        let mut out = [0.0; 4];
        for x in [0.0, 1.0] {
            for z in [0.0, 1.0] {
                let at = [(start[0] + x + 0.5) * texel, (start[1] + z + 0.5) * texel];
                let wave = bands.sample(band, at, time);
                let weight = (1.0 - (x - f[0]).abs()) * (1.0 - (z - f[1]).abs());
                for (total, value) in out.iter_mut().zip([
                    wave.height,
                    wave.slope[0],
                    wave.slope[1],
                    wave.height_dt - speed * wave.slope[0],
                ]) {
                    *total += weight * value;
                }
            }
        }
        out
    }

    #[expect(
        clippy::print_stdout,
        reason = "Metal evidence reports numerical errors and cadence counts"
    )]
    pub(super) fn mailbox(gpu: &thrum_test_support::Gpu) {
        let map = map(true);
        let reference = run_mailbox(gpu, map.clone(), 1, camera(lane(0)));
        let slow = run_mailbox(gpu, map.clone(), 4, camera(lane(0)));
        let remote = Camera::look_at([1000.0, 12.0, 1000.0], [1000.0, 0.0, 1020.0], 50.0);
        let offscreen = run_mailbox(gpu, map, 4, remote);
        assert_eq!(reference.0, slow.0);
        assert_eq!(reference.0, offscreen.0);
        assert!(slow.1 > 0 && offscreen.1 > slow.1);
        assert!(slow.2 > 0 && offscreen.2 > 0);
        println!(
            "Metal mailbox matched to the bit ({} answers), skipped frames {}/{}/{}, query refreshes {}/{}/{}",
            reference.0.len(),
            reference.1,
            slow.1,
            offscreen.1,
            reference.2,
            slow.2,
            offscreen.2
        );
    }

    fn reference(
        bands: &FlowBands,
        point: [f64; 2],
        time: f64,
        speed: f64,
        dt: f64,
        gains: [f64; 2],
    ) -> ([f64; 4], [f64; 4]) {
        let mut sample = [0.0; 4];
        let mut scale = [0.0; 4];
        let profile = bands.profile();
        for (band, gain) in gains.into_iter().enumerate() {
            if gain == 0.0 {
                continue;
            }
            let [h, sx, sz, rate] = interpolated(bands, band, point, time, speed);
            for (total, value) in sample.iter_mut().zip([rate.mul_add(dt, h), sx, sz, rate]) {
                *total += gain * value;
            }
            // Express slopes and rates in equivalent height at the band's
            // highest wavenumber and fixed-column frequency.
            let k = std::f64::consts::TAU * profile.radii[band][1] / profile.patches[band];
            for (total, units) in
                scale
                    .iter_mut()
                    .zip([1.0, k, k, thrum_water_model::flow_frequency(k) + speed * k])
            {
                *total += gain * profile.rms[band] * units;
            }
        }
        (sample, scale)
    }

    #[expect(
        clippy::print_stdout,
        reason = "Metal evidence reports normalized query error"
    )]
    fn run_mailbox(
        gpu: &thrum_test_support::Gpu,
        map: Map,
        cadence: u32,
        view: Camera,
    ) -> (Vec<Vec<u64>>, usize, u64) {
        let mut scene = scene(gpu, map, view);
        let knob = scene
            .fixture
            .host
            .quality_knobs()
            .into_iter()
            .find(|entry| entry.name.ends_with("river.flow_band_cadence"))
            .expect("flow cadence")
            .name;
        scene
            .fixture
            .host
            .set_quality_knob(&knob, cadence)
            .expect("cadence knob");
        let bands = scene.map.flow_bands().expect("model bands");
        let mut bits = Vec::new();
        let mut skips = 0;
        let mut worst: [[f64; 4]; 3] = [[0.0; 4]; 3];
        let mut counts = [0; 3];
        let mut rms = [[0.0; 2]; 2];
        drop(scene.fixture.frame_without_timings(0.0));
        for tick in 1..=5 {
            let mut time = 0.0;
            for at in [STEP * tick, STEP * tick + STEP / 2] {
                let frame = scene.fixture.frame_without_timings(at.as_secs_f64());
                let source_tick = u32::try_from(frame.tick).expect("fixture tick");
                time = (f64::from(source_tick) - 1.0 + f64::from(frame.presentation_alpha))
                    * STEP.as_secs_f64();
                skips += usize::from(
                    scene.fixture.report.lock().expect("report").flow_bands_ran == Some(false),
                );
            }
            let use_tick = tick + 2;
            let points: Vec<_> = [0, 2]
                .into_iter()
                .flat_map(|arm| {
                    (0..POINTS).map(move |index| QueryPoint {
                        position: [
                            4.0 + 0.41 * f32::from(u8::try_from(index).expect("station count")),
                            lane(arm),
                        ],
                        radius: 0.001,
                        use_tick: u64::from(use_tick),
                    })
                })
                .collect();
            scene.query.post(
                scene.fixture.endpoint(u64::from(tick)),
                vec![BodyQuery {
                    id: 17,
                    generation: 1,
                    points,
                }],
            );
            scene.fixture.finish_tick_queries();
            let response = scene
                .query
                .drain(scene.fixture.endpoint(u64::from(use_tick)), STEP);
            let answers = &response
                .iter()
                .find(|answer| answer.id == 17)
                .expect("mailbox result")
                .points;
            for (i, answer) in answers.iter().enumerate() {
                assert!(answer.surface.wet);
                let arm = i / POINTS;
                let sample = answer.surface.sample;
                let point = answer.point.position.map(f64::from);
                let speed = [0.5, 0.0][arm];
                let row = scene.map.sample(point).expect("wet station");
                let gains = scene.map.flow_gains(row.body, point);
                let dt = (STEP * use_tick).as_secs_f64() - time;
                let (expected, scale) = reference(bands, point, time, speed, dt, gains);
                let actual = [
                    sample.height - LEVEL,
                    sample.slope[0],
                    sample.slope[1],
                    sample.height_dt,
                ];
                let group = if gains[1] == 0.0 {
                    0
                } else if gains[0] == 0.0 {
                    1
                } else {
                    2
                };
                counts[group] += 1;
                for axis in 0..4 {
                    worst[group][axis] =
                        worst[group][axis].max((actual[axis] - expected[axis]).abs() / scale[axis]);
                }
                rms[arm][0] += actual[0].powi(2);
                rms[arm][1] += expected[0].powi(2);
                let mut fields: Vec<_> = [sample.height, sample.height_dt, answer.surface.depth]
                    .into_iter()
                    .chain(sample.slope)
                    .chain(sample.velocity)
                    .chain(answer.surface.current)
                    .map(f64::to_bits)
                    .collect();
                fields.extend([
                    u64::from(answer.surface.wet),
                    answer.surface.body.map_or(u64::MAX, |body| body as u64),
                    answer.point.use_tick,
                    answer.index as u64,
                ]);
                fields.extend(
                    answer
                        .sources
                        .map(|time| time.map_or(u64::MAX, f64::to_bits)),
                );
                fields.extend(answer.supported.map(u64::from));
                fields.extend(
                    answer
                        .wave_heights
                        .map(|height| u64::from(height.to_bits())),
                );
                fields.push(u64::from(answer.wake_height.to_bits()));
                fields.push(answer.bed.map_or(u64::MAX, |bed| u64::from(bed.to_bits())));
                bits.push(fields);
            }
        }
        let rms = rms.map(|arm| arm.map(|sum| (sum / 160.0).sqrt()));
        println!(
            "cadence {cadence}: short/long/mixed normalized errors {worst:?}, samples {counts:?}, GPU/model RMS {rms:?}"
        );
        assert!(counts.iter().all(|count| *count > 20));
        assert!(
            worst.into_iter().flatten().all(|value| value < 0.0004),
            "model cascade tolerance"
        );
        assert!((rms[0][0] / rms[0][1] - 1.0).abs() < 0.1);
        assert!(
            rms[1][0] < 0.1 * rms[0][0],
            "pool is at most one tenth of riffle rms"
        );
        let refreshes = scene
            .fixture
            .report
            .lock()
            .expect("report")
            .flow_query_evaluations;
        (bits, skips, refreshes)
    }

    #[expect(
        clippy::print_stdout,
        reason = "Metal evidence reports submitted depth and fitted speed"
    )]
    pub(super) fn drawn(gpu: &thrum_test_support::Gpu) {
        let mut scene = scene(gpu, map(false), camera(lane(0)));
        let bands = scene.map.flow_bands().expect("model bands");
        let mut squares = [0.0_f64; 5];
        drop(scene.fixture.frame_without_timings(0.0));
        for (arm, square) in squares.iter_mut().enumerate() {
            let (view, level) = if arm == 4 {
                (
                    Camera::look_at([-7.5, 0.15, 17.0], [-7.5, -2.0, 17.001], 40.0),
                    -2.0,
                )
            } else {
                (camera(lane(arm)), LEVEL)
            };
            *scene.camera.lock().expect("camera") = view;
            let matrix = depth::matrix(view, EXTENT);
            let mut observed = Vec::new();
            for sample in 1..=3 {
                let ticks = 8 + 8 * u32::try_from(arm).expect("lane") + 2 * sample;
                let frame = scene
                    .fixture
                    .frame_without_timings((STEP * (ticks + 1)).as_secs_f64());
                let tick = u32::try_from(frame.tick).expect("fixture tick");
                let time = (f64::from(tick) - 1.0 + f64::from(frame.presentation_alpha))
                    * STEP.as_secs_f64();
                let depths = scene.depth.read(&scene.fixture);
                for column in 8..EXTENT[0] - 8 {
                    let at = [column, EXTENT[1] / 2];
                    let pixel = (at[1] * EXTENT[0] + at[0]) as usize;
                    assert!(depths[pixel] > 0.0);
                    let hit = super::support::world_at(matrix, EXTENT, at, depths[pixel]);
                    let h = f64::from(hit.y) - level;
                    *square += h * h;
                    if column % 4 == 0 {
                        observed.push(([f64::from(hit.x), f64::from(hit.z)], time, h));
                    }
                }
            }
            if arm < 3 {
                let gain = [0.9, 0.9, 0.035][arm];
                let error = |speed| {
                    observed
                        .iter()
                        .map(|&(point, time, height)| {
                            let x = [point[0] - speed * time, point[1]];
                            let estimate = (0..2)
                                .map(|band| bands.sample(band, x, time).height * gain)
                                .sum::<f64>();
                            (height - estimate).powi(2)
                        })
                        .sum::<f64>()
                };
                let nominal = [0.5, 1.5, 0.0][arm];
                let best = (-4..=16)
                    .map(|n| f64::from(n) * 0.125)
                    .min_by(|&a, &b| error(a).total_cmp(&error(b)))
                    .expect("speed fit");
                println!("lane {arm}: fitted current {best:.3} m/s (declared {nominal:.3})");
                assert!((best - nominal).abs() <= nominal.mul_add(0.2, 0.002));
            }
        }
        println!(
            "drawn rms heights on Metal: {:?}",
            squares.map(|v| (v / 243.0).sqrt())
        );
        assert!(squares[0] > 0.0005 && squares[2] < squares[0] * 0.01);
        assert!(
            squares[3..].iter().all(|&squared| squared < 1.0e-8),
            "lake and sea are flat"
        );
    }
}

thrum_test_support::macos_gpu_test! {
fn submitted_calm_surface_and_mailbox_follow_the_current() {
    let Some(gpu) = thrum_test_support::gpu("river flow bands", Some(wgpu::Backend::Metal)) else { return };
    flow::mailbox(&gpu);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_depth_tracks_running_water_in_calm() {
    let Some(gpu) = thrum_test_support::gpu("river flow depth", Some(wgpu::Backend::Metal)) else { return };
    flow::drawn(&gpu);
}
}
