//! Capture-off, same-view measurements of the island with and without surf.
use std::time::Duration;

use serde_json::{Value, json};
use thrum_render::{extract::SourceTick, runtime::FrameWorld};
use thrum_test_support::Gpu;
use thrum_water::query::BodyQuery;
use thrum_water_model::Sheet;
use water_scene::floating::{Body, Hull};
use water_scene::island::{Wind, build_surf_paired, surf_inputs, surf_water_profile};

use super::{sample_json, support::Fixture};

const QUERY_TICK: u32 = 67;
const STEP: Duration = Duration::from_nanos(16_666_667);

pub(super) fn breaker_fleet() -> (Vec<Body>, usize) {
    let (map, spectrum) = surf_inputs(Wind::Gale).expect("gale surf inputs");
    let field =
        water_scene::floating::fallback(&spectrum, surf_water_profile(&spectrum).waves.choppiness)
            .expect("wave field");
    let time = (STEP * (QUERY_TICK + 2)).as_secs_f64();
    let sheet = map.surf_sheet(&field, 0, time).expect("windward sea");
    let mut columns: Vec<[f64; 2]> = Vec::new();
    'beach: for x in -165..-100 {
        for z in -35..55 {
            let p = [f64::from(x), f64::from(z)];
            if !map
                .bed()
                .height(p)
                .is_some_and(|h| (-5.0..-0.4).contains(&h))
            {
                continue;
            }
            let a = sheet.forward(p);
            let b = sheet.forward([p[0] + 0.02, p[1]]);
            let c = sheet.forward([p[0], p[1] + 0.02]);
            let ab = [b.world[0] - a.world[0], b.world[1] - a.world[1]];
            let ac = [c.world[0] - a.world[0], c.world[1] - a.world[1]];
            // A negative Jacobian is the back of a lip. Put station zero
            // beneath it and keep the eight floats distinct along the beach.
            if ab[0] * ac[1] - ab[1] * ac[0] < -1e-5
                && columns
                    .iter()
                    .all(|q| (q[0] - a.world[0]).hypot(q[1] - a.world[1]) > 1.5)
            {
                columns.push(a.world);
                if columns.len() == 8 {
                    break 'beach;
                }
            }
        }
    }
    assert_eq!(
        columns.len(),
        8,
        "gale must place all eight floats in breakers"
    );
    let fleet = (0..8)
        .zip(columns)
        .map(|(id, [x, z])| {
            let mut body = Body::new(id, Hull::Float, [0.0; 2], 0.0, 0.0);
            let station = body.station(0);
            body.position.x = x - station.x;
            body.position.z = z - station.z;
            body
        })
        .collect();
    let eligible = spectrum
        .long
        .iter()
        .filter(|wave| {
            let wavelength = std::f64::consts::TAU / wave.wave_vector[0].hypot(wave.wave_vector[1]);
            (20.0..=640.0).contains(&wavelength)
        })
        .count();
    (fleet, eligible)
}

#[expect(
    clippy::print_stderr,
    reason = "locate a failed GPU wait in the recorded cost arm"
)]
pub(super) fn run(gpu: &Gpu) -> Value {
    let wind = Wind::Gale;
    let (fleet, eligible) = breaker_fleet();
    let fleet: Vec<BodyQuery> = fleet
        .iter()
        .map(|body| body.request(u64::from(QUERY_TICK), STEP))
        .collect();
    let mut arms = Vec::new();
    for (water, surf) in [(true, true), (true, false), (false, false)] {
        let (game, report, _, probes) = build_surf_paired(wind, water, surf).expect("gale arm");
        let query = probes.expect("public island query").query;
        let mut fixture = Fixture::from_game(gpu, [2560, 1440], game, report);
        fixture.parameters(r#"{"camera.island":0}"#);
        fixture.warm_cooked();
        eprintln!("prepared gale cost arm: water={water}, surf={surf}");
        let mut frames = Vec::new();
        for index in 0..32_u32 {
            if water && index == 14 {
                eprintln!("submitting eight floats at tick {QUERY_TICK}, surf={surf}");
                query.post(
                    SourceTick {
                        tick: u64::from(QUERY_TICK),
                        ..fixture.host.world().source()
                    },
                    fleet.clone(),
                );
            }
            let sample = fixture.frame(1.000_001 + f64::from(index) / 120.0);
            let mut frame = sample_json(&sample);
            frame["completion_upper_ms"] = json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
            if water && index == 14 {
                assert_eq!(sample.tick, u64::from(QUERY_TICK));
                let replies = query.drain(
                    SourceTick {
                        tick: u64::from(QUERY_TICK + 2),
                        ..fixture.host.world().source()
                    },
                    STEP,
                );
                assert_eq!(replies.len(), fleet.len());
                assert!(replies.iter().all(|reply| reply.points.len() == 6));
                let stats = query.stats();
                assert_eq!(stats.blocking_reads, 0);
                assert_eq!(stats.dispatches, 1);
                assert_eq!(stats.cut_nodes > 0, surf);
                eprintln!("read forty-eight stations, {} cut nodes", stats.cut_nodes);
                frame["fleet_query"] = json!({
                    "floats": replies.len(), "stations": 6 * replies.len(),
                    "use_tick": QUERY_TICK + 2, "cut_nodes": stats.cut_nodes,
                    "coarse_nodes": stats.cut_coarse_nodes, "fine_nodes": stats.cut_fine_nodes,
                    "columns": fleet.iter().map(|body| body.points[0].position).collect::<Vec<_>>(),
                });
            }
            if index >= 8 {
                frames.push(frame);
            }
        }
        let report = fixture.report.lock().expect("water report");
        arms.push(
            json!({"water":water, "surf":surf, "frames":frames, "resources":report.resources}),
        );
    }
    let camera = water_scene::island::SURF_VIEWS[0];
    json!({
        "target":"mac-metal", "extent":[2560,1440], "capture":false, "wind":"Gale", "arms":arms,
        "camera":{"position":camera.position, "target":camera.target, "fov_degrees":camera.vertical_fov_degrees},
        "shore_reads":{
            "eligible_long_components":eligible,
            "scope":"Logical textureLoad calls, per eligible component and forward evaluation. Invalid shore samples can exit early.",
            "stage_endpoint":{"lag":8,"exposure":4},
            "between_stages":{"lag":16,"exposure":8},
            "additional_lag_probes_per_boundary_axis":2,
            "eligible_swash_node":{
                "contour_reference_reads":{"stage_endpoint":4,"between_stages":8},
                "inner_profile":"One additional lag and exposure sum with no gradient probes.",
                "overlap":"Up to four additional bed reads at the displaced position.",
            },
            "forward_evaluations_per_cached_node":{"interior":2,"ring_edge":6,"distant_shore":1},
            "distant_shore":{
                "scope":"One current sample outside the near window at each eligible prepared bed node. A resolved breaking face adds one displaced-bed cell lookup.",
                "fragment_reads":"One grid-size entry and up to four phase-and-strength samples from the field cache.",
            },
            "sources":["crates/thrum-water/src/surf.wgsl", "crates/thrum-water/src/surf/cache.wgsl"],
        },
    })
}
