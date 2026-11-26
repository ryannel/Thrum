//! Metal proofs of the authored outlet's submitted joins and post foam.
#[cfg(target_os = "macos")]
mod support;
#[cfg(target_os = "macos")]
use water_scene::{channel, island::Wind};

thrum_test_support::macos_gpu_test! {
#[expect(clippy::cast_possible_truncation, reason = "log target measurements in GPU metres")]
fn submitted_channel_depth_follows_the_chart_and_both_joins() {
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_water_model::BodyKind;
    const EPS_METRES: f64 = 0.000_04;
    let Some(gpu) = thrum_test_support::gpu(
        "submitted_channel_depth_follows_the_chart_and_both_joins",
        Some(wgpu::Backend::Metal),
    ) else { return };
    let forcing = channel::forcing(Wind::Trade);
    let mut spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("trade spectrum");
    // Isolate the displaced swell and mean surface from unresolved capillary ripples.
    spectrum.cascades = [160.0, 37.0, 9.0].map(|length|
        thrum_sea_model::Cascade::new(length, &[]).expect("empty calibration cascade"));
    let mut profile = water_scene::profile(&spectrum);
    profile.waves.choppiness = 0.0;
    let field = water_scene::floating::fallback(&spectrum, 0.0).expect("column field");
    let variances = field.variances();
    let extent = [96, 12];
    // The map probe's 40 μm level/bed tolerance is also the seam residual bound.
    for (i, (lake, tide)) in [(0.0,0.0),(0.0,-0.5),(1.0,-0.5),(0.0,0.5),(1.0,0.5)].into_iter().enumerate() {
        let mut cpu = channel::map(&forcing, true).expect("authored outlet");
        cpu.set_stage(1,lake).expect("lake stage");
        cpu.set_stage(0,tide).expect("tide");
        let mouth = cpu.sample([channel::MOUTH,channel::CENTRE]).expect("mouth row");
        let limited = thrum_water_model::limit(mouth.row,mouth.depth,variances,cpu.profile());
        if tide < 0.0 {
            assert!(mouth.depth > 0.0 && mouth.depth < 0.5);
            assert!(limited.gains[0] < 0.85);
        }
        if tide > 0.0 {
            let lower = cpu.sample([342.5,channel::CENTRE]).expect("drowned lower channel");
            assert_eq!(lower.row, cpu.row(0,[342.5,channel::CENTRE]));
        }
        for (x, before, after) in [(channel::SOURCE,1,3),(channel::MOUTH,3,0)]
            .into_iter().chain([(225.0,3,3),(270.0,3,3),(320.0,3,3)].into_iter().filter(|_| i==0))
        {
            let time = 0.0;
            let centre = cpu.surface(&field,[x,channel::CENTRE],time).expect("centre column");
            let y = centre.sample.height as f32;
            let x_gpu = x as f32;
            let z_gpu = channel::CENTRE as f32;
            let camera = Camera::look_at([x_gpu,(y+0.25).max(0.12),z_gpu+0.002], [x_gpu,y,z_gpu], 12.0);
            let matrix = support::depth::matrix(camera, extent);
            let water = thrum_water::Water::mapped(
                std::sync::Arc::new(std::sync::Mutex::new(cpu.clone())), spectrum.clone(), profile,
            ).expect("mapped outlet");
            let report = water.report();
            let tap = support::depth::DepthTap::default();
            let game = SkyGame::new(Controls::new(),camera,|_,_|{}).with_ground(None)
                .with_contribution(Box::new(water),|| ContentStatus::Drawn(1))
                .with_contribution(Box::new(tap.contribution(extent)),|| ContentStatus::Drawn(1));
            let mut scene = support::Fixture::from_game(&gpu, extent, game, report);
            scene.frame_without_timings(0.0);
            let depth = tap.read(&scene);
            let row = extent[1]/2;
            let at = |column| {
                let d = depth[(row*extent[0]+column) as usize];
                assert!(d>0.0, "missing submitted water at {x} column {column} lake={lake} tide={tide}");
                let p = support::world_at(matrix,extent,[column,row],d);
                let reference = cpu.surface(&field,[f64::from(p.x),f64::from(p.z)],time).expect("reference column");
                (reference.body.expect("wet body"),f64::from(p.y),reference.sample.height)
            };
            let left = at(extent[0]/2-1);
            let right = at(extent[0]/2);
            assert_eq!([left.0,right.0],[before,after]);
            let residual = ((right.1-left.1) - (right.2-left.2)).abs();
            println!("Metal x={x} lake={lake} tide={tide} decoded={:.8}/{:.8} model={:.8}/{:.8} seam residual={residual:.8} m mouth depth={:.4} long gain={:.4}",
                left.1,right.1,left.2,right.2,mouth.depth,limited.gains[0]);
            assert!(residual <= EPS_METRES, "surface step at x={x} lake={lake} tide={tide}");
            if x == channel::MOUTH && tide < 0.0 {
                let delta = centre.sample.height - tide;
                assert!(delta.abs() > 0.01, "the low-tide witness needs a nonzero swell displacement");
                assert!((right.1 - tide - delta).abs() < 0.003, "the measured swell is not depth limited");
            }
            if before == 3 && after == 3 {
                assert!((right.1-right.2).abs() < 0.0001, "surface did not follow the chart's fall");
            }
        }
        let BodyKind::Chart(chart) = &cpu.body(3).expect("chart body").kind else {
            panic!("the outlet is an ordinary chart");
        };
        assert_eq!(chart.ends.len(), 2);
    }
}
}

thrum_test_support::macos_gpu_test! {
fn post_foams_downstream_and_a_float_reaches_the_sea() {
    let Some(gpu) = thrum_test_support::gpu(
        "post_foams_downstream_and_a_float_reaches_the_sea",
        Some(wgpu::Backend::Metal),
    ) else { return };
    support::island::with_cooked_island("post_foams_downstream_and_a_float_reaches_the_sea", || {
        check_post_and_float(&gpu);
    });
}
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stdout,
    reason = "retain the actual Metal target readings"
)]
fn check_post_and_float(gpu: &thrum_test_support::Gpu) {
    let extent = [112, 64];
    check_query_join(gpu, extent);
    for running in [false, true] {
        let route = channel::build(Wind::Trade, true, running).expect("channel game");
        let mut scene = support::Fixture::from_game(gpu, extent, route.game, route.report);
        scene.frame_without_timings(0.0);
        for tick in 1..=12 {
            scene.frame_without_timings(f64::from(tick) / 30.0);
        }
        let foam = route.foam.latest().expect("submitted whitewater");
        let cells = support::foam_stocks(&scene, &foam);
        let spacing = foam.spacing[0];
        let [ox, oz] = foam.origins[1][0];
        let side = foam.side;
        let count = (side * side) as usize;
        let origin = foam.banks[1] * 2 * count;
        let mut upstream = 0.0_f32;
        let mut downstream = 0.0_f32;
        for (i, stock) in (0..side * side).zip(&cells[origin..][..count]) {
            #[expect(clippy::cast_precision_loss, reason = "the foam window has 256 cells")]
            let x = ox + (i % side) as f32 * spacing + spacing * 0.5;
            #[expect(clippy::cast_precision_loss, reason = "the foam window has 256 cells")]
            let z = oz + (i / side) as f32 * spacing + spacing * 0.5;
            if (z - channel::POST[1]).abs() < 0.45 {
                let dx = x - channel::POST[0];
                let film = stock[1] + stock[2];
                if (-2.0..-0.1).contains(&dx) {
                    upstream = upstream.max(film);
                }
                if (0.3..2.0).contains(&dx) {
                    downstream = downstream.max(film);
                }
            }
        }
        println!("Metal current={running} post upstream={upstream:.6} downstream={downstream:.6}");
        assert!(upstream < 0.002);
        if !running {
            assert!(downstream < 0.002);
            continue;
        }
        assert!(downstream > 0.03);

        // Drive the same game world at fixed ticks, without encoding unobserved frames.
        // The game falls back to the same map between the sampled GPU frames.
        let mut before = 3;
        for second in [12, 50, 90, 135, 175, 220, 270, 310, 350, 390] {
            scene
                .host
                .advance_without_rendering(std::time::Duration::from_secs(second))
                .expect("fixed world advance");
            let body = (route.float)();
            let place = [body.position.x, body.position.z];
            let map = route.map.lock().expect("outlet");
            let answer = map.sample(place).expect("wet course");
            println!(
                "Metal float t={second}s position={:?} body={} current={:?}",
                body.position, answer.body, answer.row.current
            );
            assert!(body.position.y.is_finite() && answer.depth > 0.0);
            if before == 3 && answer.body == 0 {
                let ends = [
                    [channel::MOUTH - 0.001, channel::CENTRE],
                    [channel::MOUTH + 0.001, channel::CENTRE],
                ]
                .map(|point| map.sample(point).expect("mouth"));
                assert_eq!(ends.map(|sample| sample.body), [3, 0]);
                assert!((ends[0].row.level - ends[1].row.level).abs() < 0.00004);
                assert!((ends[0].row.current[0] - ends[1].row.current[0]).abs() < 0.000_004);
            }
            before = answer.body;
            if before == 0 && body.position.x > channel::MOUTH + 0.5 {
                break;
            }
        }
        assert_eq!(before, 0, "float did not transfer to sea ownership");
        assert!((route.float)().position.x > channel::MOUTH + 0.5);
    }
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stdout,
    reason = "retain the actual Metal target readings"
)]
fn check_query_join(gpu: &thrum_test_support::Gpu, extent: [u32; 2]) {
    use std::time::Duration;
    use thrum_water::query::{BodyQuery, QueryPoint};
    const STEP: Duration = Duration::from_nanos(16_666_667);
    const PROBE: u32 = 40;
    let route = channel::build(Wind::Trade, true, true).expect("query fixture");
    let mailbox = route.query;
    let game = route.game.with_fixed_update_endpoint(|_, _| {});
    let mut scene = support::Fixture::from_game(gpu, extent, game, route.report);
    mailbox.post(
        scene.endpoint(0),
        vec![BodyQuery {
            id: PROBE,
            generation: 1,
            points: [349.999, 350.001]
                .into_iter()
                .map(|x| QueryPoint {
                    position: [x, 5.0],
                    radius: 0.2,
                    use_tick: 2,
                })
                .collect(),
        }],
    );
    scene.frame_without_timings(0.0);
    scene.frame_without_timings(STEP.as_secs_f64());
    let replies = mailbox.drain(scene.endpoint(2), STEP);
    let reply = replies
        .iter()
        .find(|row| row.id == PROBE)
        .expect("submitted end queries");
    let [upstream, downstream] = reply.points.as_slice() else {
        panic!("two query answers")
    };
    assert_eq!(
        [upstream.surface.body, downstream.surface.body],
        [Some(3), Some(0)]
    );
    assert!(upstream.surface.wet && downstream.surface.wet);
    let level = |answer: &thrum_water::query::QueryAnswer| {
        answer.surface.sample.height - answer.wave_heights.into_iter().map(f64::from).sum::<f64>()
    };
    assert!((level(upstream) - level(downstream)).abs() < 0.000_04);
    assert!((upstream.surface.current[0] - downstream.surface.current[0]).abs() < 0.000_004);
    println!(
        "Metal mouth queries: chart and sea levels {:.8}/{:.8}; currents {:?}/{:?}",
        level(upstream),
        level(downstream),
        upstream.surface.current,
        downstream.surface.current
    );
}
