use serde_json::{Value, json};
use thrum_render::runtime::FrameWorld;
use thrum_water::interaction::Witness;
use water_scene::{island::Wind, river};

use super::support::{Fixture, foam_buffer, river_store};

pub(super) fn pool_witness(gpu: &thrum_test_support::Gpu) -> Value {
    let river::Scene {
        game,
        report,
        map,
        wake,
        ..
    } = river::build_fall(Wind::Calm, true, Some(&river_store())).expect("fall scene");
    let wake = wake.expect("fall interaction witness");
    let mut scene = Fixture::from_game(gpu, [48, 27], game, report);
    scene.parameters(r#"{"camera.river":7,"spring.stage":1,"sun.elevation":32}"#);
    let first = scene.host.world().source().tick;
    let mut observations = Vec::new();
    let mut origin = [0.0; 2];
    for tick in (0..=168).step_by(8) {
        scene.frame_without_timings(f64::from(tick) / 60.0);
        if ![0, 16, 32, 48, 64, 96, 120, 144, 168].contains(&tick) {
            continue;
        }
        let Some(witness) = wake.latest() else {
            continue;
        };
        let states: Vec<[f32; 2]> = foam_buffer(&scene, &witness.buffer);
        let sample = witness.endpoints[1];
        origin = sample.origins[0];
        let plane = &states[sample.offsets[0]..][..Witness::SIDE * Witness::SIDE];
        let mut bands = [0.0_f32; 28];
        for (i, cell) in plane.iter().enumerate() {
            let cell_x = f32::from(u16::try_from(i % Witness::SIDE).expect("window column"));
            let cell_z = f32::from(u16::try_from(i / Witness::SIDE).expect("window row"));
            let x = cell_x * 0.25 + origin[0];
            let z = cell_z * 0.25 + origin[1];
            if (175.0..185.0).contains(&x) {
                for (j, peak) in bands.iter_mut().enumerate() {
                    let floor = f32::from(u16::try_from(j).expect("band index")) - 25.0;
                    if (floor..floor + 1.0).contains(&z) {
                        *peak = peak.max(cell[0].abs());
                    }
                }
            }
        }
        observations.push(json!({"tick": sample.tick, "height_by_world_z_metre": bands}));
    }
    let map = map.lock().expect("pool map");
    let rows = [-24.0, -23.0, -22.0, -21.0, -20.0, -18.0, -16.0, -12.0].map(|z| {
        let row = map.sample([180.0, z]);
        json!({
            "z": z,
            "water": row.and_then(|sample| map.body(sample.body)).map(|body| body.name.as_str()),
            "depth": row.map(|sample| sample.depth),
        })
    });
    json!({"target":"mac-metal","start_tick":first,"region_origin":origin,
        "rows":rows,"samples":observations})
}
