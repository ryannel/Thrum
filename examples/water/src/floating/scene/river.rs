//! Float transport in the island's solved rivers.
use std::sync::{Arc, Mutex};
use thrum_app::sky::{ContentStatus, SkyGame};
use thrum_water::Water;

use super::{Body, Draw, Fleet, Hull, advance_fleet};

pub(crate) fn install_river_floats(
    game: SkyGame,
    map: crate::SharedMap,
    water: &Water,
    fallback: thrum_water_model::Field,
    wind: glam::DVec2,
) -> (SkyGame, Box<dyn Fn() -> [Body; 2]>) {
    let initial = [
        (0, [180.0, -92.5], -std::f64::consts::FRAC_PI_2),
        (1, crate::river::OUTLET_RELEASE, 0.0),
    ]
    .map(|(id, point, heading)| {
        let level = map
            .lock()
            .expect("river map")
            .sample(point)
            .expect("float release in water")
            .row
            .level;
        Body::new(id, Hull::Float, point, level, heading)
    });
    let fleet = Arc::new(Mutex::new(Fleet {
        previous: initial.to_vec(),
        current: initial.to_vec(),
        craft_present: false,
        tick: 0,
        observe: None,
        #[cfg(target_arch = "wasm32")]
        reply_ages: std::collections::VecDeque::new(),
        #[cfg(target_arch = "wasm32")]
        wake_sample: None,
    }));
    let mailbox = water.query_mailbox().expect("river float queries");
    let world = Arc::clone(&fleet);
    #[cfg(target_arch = "wasm32")]
    let status_map = Arc::clone(&map);
    let game = game.with_fixed_update_endpoint(move |endpoint, step| {
        let mut fleet = world.lock().expect("river floats");
        let fleet = &mut *fleet;
        fleet.tick = endpoint.tick;
        fleet.previous.clone_from(&fleet.current);
        // The stream valley shelters the float from crosswind. Down-valley
        // air still acts on the float as it crosses the still-water join.
        let valley_wind = glam::DVec2::new(0.0, wind.length() * wind.y.signum());
        for (float, local_wind) in fleet.current.iter_mut().zip([valley_wind, wind]) {
            let slip = local_wind - glam::DVec2::new(float.velocity.x, float.velocity.z);
            let drag = slip * (0.0045 * slip.length() * step.as_secs_f64());
            float.velocity.x += drag.x;
            float.velocity.z += drag.y;
        }
        drop(advance_fleet(
            &mut fleet.current,
            endpoint,
            step,
            &mailbox,
            &map,
            &fallback,
            None,
        ));
        mailbox.post(
            endpoint,
            fleet
                .current
                .iter()
                .map(|body| body.request(endpoint.tick, step))
                .collect(),
        );
    });
    #[cfg(target_arch = "wasm32")]
    let game = {
        let status = Arc::clone(&fleet);
        game.with_status_readouts(move |_| {
            let floats = status.lock().expect("river floats");
            let map = status_map.lock().expect("river map");
            let poses = js_sys::Array::new();
            for body in &floats.current {
                let point = body.position;
                let pose = js_sys::Array::new();
                for value in point.to_array() {
                    pose.push(&value.into());
                }
                if let Some(row) = map.sample([point.x, point.z]) {
                    pose.push(&u32::try_from(row.body).expect("river body").into());
                    pose.push(&row.depth.into());
                }
                poses.push(&pose);
            }
            drop(js_sys::Reflect::set(
                &js_sys::global(),
                &"waterRiverFloats".into(),
                &poses,
            ));
        })
    };
    let game = game.with_contribution(
        Box::new(Draw::new(Arc::clone(&fleet), water.optics_bodies(), 2)),
        || ContentStatus::Drawn(2),
    );
    (
        game,
        Box::new(move || {
            let fleet = fleet.lock().expect("river floats");
            [fleet.current[0], fleet.current[1]]
        }),
    )
}
