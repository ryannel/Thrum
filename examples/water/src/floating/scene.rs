use super::{Body, BodyStep, Hull, advance_fleet, draw::Draw, fallback};
use crate::island::Wind;
use glam::DVec3;
use std::{
    error::Error,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_sea_model::Spectrum;
use thrum_water::{
    Water,
    query::{QueryLimits, QueryMailbox},
};
use thrum_water_model::{Bed, BodyKind, Map, Stage};

pub(super) struct Fleet {
    pub(super) previous: Vec<Body>,
    pub(super) current: Vec<Body>,
    observe: Option<Box<dyn FnMut(BodyStep) + Send>>,
    #[cfg(target_arch = "wasm32")]
    reply_ages: std::collections::VecDeque<(u32, u64, u64)>,
}

const STEP: Duration = Duration::from_nanos(16_666_667);

/// Sea or lake body scene, including the public game-facing mailbox.
pub struct FloatingScene {
    /// Normal app-shell scene and fixed-step body simulation.
    pub game: SkyGame,
    /// Query status used by the scene and capture harness.
    pub mailbox: QueryMailbox,
    /// Preparation and resource report.
    pub report: crate::SharedReport,
    fleet: Arc<Mutex<Fleet>>,
}

impl FloatingScene {
    /// Observe station contact and work on each game tick.
    /// # Panics
    /// Panics if a thread poisoned the fleet lock.
    pub fn observe_steps(&self, observe: impl FnMut(BodyStep) + Send + 'static) {
        self.fleet.lock().expect("floating fleet").observe = Some(Box::new(observe));
    }

    /// Copy the game's most recent complete body poses.
    /// # Panics
    /// Panics if the game poisoned its fleet lock.
    pub fn bodies(&self) -> Vec<Body> {
        self.fleet.lock().expect("floating fleet").current.clone()
    }

    /// Obtain a body-state observer that can be kept after moving the game into a host.
    /// # Panics
    /// The returned function panics if a thread poisons the fleet lock.
    pub fn observer(&self) -> impl Fn() -> Vec<Body> + use<> {
        let fleet = Arc::clone(&self.fleet);
        move || fleet.lock().expect("floating fleet").current.clone()
    }
}

fn sea(wind: Wind) -> Result<Map, Box<dyn Error>> {
    Map::prepare(
        Bed::new([-1.0; 2], 2.0, [2, 2], vec![-1000.0; 4]).map_err(|e| format!("{e:?}"))?,
        vec![thrum_water_model::Body {
            name: "Sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: wind.forcing().fetch,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 2.0,
            swash_ratio: 0.7,
        },
    )
    .map_err(|e| format!("{e:?}").into())
}

/// Build the floating-body route. The side camera sees the length of the hull;
/// the chase camera sees the beam. Bearing is the craft's world heading.
/// # Errors
/// Returns invalid scene, mesh, spectrum, map or controls.
/// # Panics
/// The running scene panics if a thread poisons its game state locks.
pub fn build(
    wind: Wind,
    lake: bool,
    speed: f32,
    heading: f32,
    chase: bool,
) -> Result<FloatingScene, Box<dyn Error>> {
    build_paired(wind, lake, speed, heading, chase, true)
}

/// Build either arm of the example's capture-off cost comparison.
/// # Errors
/// Returns invalid scene, mesh, spectrum, map or controls.
/// # Panics
/// The running scene panics if a thread poisons its game state locks.
pub fn build_paired(
    wind: Wind,
    lake: bool,
    speed: f32,
    heading: f32,
    chase: bool,
    draw_water: bool,
) -> Result<FloatingScene, Box<dyn Error>> {
    let map = Arc::new(Mutex::new(if lake {
        crate::island::map(wind)?
    } else {
        sea(wind)?
    }));
    let spectrum = Spectrum::build(&wind.forcing())?;
    let mut profile = crate::profile(&spectrum);
    profile.query = Some(QueryLimits {
        points: 2048,
        max_age: 3,
    });
    let fallback = fallback(&spectrum, profile.waves.choppiness)?;
    let water = if lake {
        Water::mapped(Arc::clone(&map), spectrum, profile)?
    } else {
        Water::new(Arc::clone(&map), 0, spectrum, profile)?
    };
    let mailbox = water
        .query_mailbox()
        .ok_or("floating scene requires a body query")?;
    let report = water.report();
    let (start, level) = if lake {
        ([182.0, 10.0], 12.0)
    } else {
        ([150.0, 32.0], 0.0)
    };
    let mut bodies = vec![Body::new(0, Hull::Craft, start, level, f64::from(heading))];
    let forward = (bodies[0].rotation * DVec3::X).with_y(0.0);
    bodies[0].propulsion = forward * f64::from(speed);
    bodies[0].velocity = bodies[0].propulsion;
    for id in 1..=8 {
        let (x, z) = if speed > 0.0 && !lake {
            // Keep the row alongside the straight course, with room for
            // the floats' crosswind drift as the craft overtakes them.
            (f64::from(id - 1) * 5.0 + 2.0, 5.0 + f64::from(id % 2))
        } else {
            (
                f64::from((id - 1) % 4) * 2.4 - 3.6,
                f64::from((id - 1) / 4) * 2.8 + 2.2,
            )
        };
        let hull = &bodies[0];
        let p = hull.position + hull.rotation * DVec3::new(x, 0.0, z);
        bodies.push(Body::new(id, Hull::Float, [p.x, p.z], level, 0.0));
    }
    {
        let authored = map.lock().map_err(|_| "floating scene map")?;
        for body in &mut bodies {
            let p = [body.position.x, body.position.z];
            let sample = authored
                .surface(&fallback, p, 0.0)
                .map_err(|error| format!("{error:?}"))?;
            if sample.wet {
                body.position.y = sample.sample.height - body.draft();
                let [sx, sz] = sample.sample.slope;
                let normal = DVec3::new(-sx, 1.0, -sz).normalize();
                body.rotation = glam::DQuat::from_rotation_arc(DVec3::Y, normal) * body.rotation;
                body.velocity.y =
                    sample.sample.height_dt + sx * body.velocity.x + sz * body.velocity.z;
            }
        }
    }
    let fleet = Arc::new(Mutex::new(Fleet {
        previous: bodies.clone(),
        current: bodies,
        observe: None,
        #[cfg(target_arch = "wasm32")]
        reply_ages: std::collections::VecDeque::new(),
    }));
    let mut controls = Controls::new();
    let query_status = controls.table("water.query", "Water query")?;
    let speed_control = controls
        .slider("craft.speed", "Craft speed", 0.0..=15.0)
        .default(speed)
        .add()?;
    let sun = controls
        .slider("sun.elevation", "Sun elevation", 5.0..=80.0)
        .default(35.0)
        .add()?;
    let chase_control = controls
        .slider("camera.chase", "Side / Chase", 0.0..=1.0)
        .default(f32::from(chase))
        .step(1.0)
        .add()?;
    let speed_value = Arc::new(Mutex::new(speed));
    let current_speed = Arc::clone(&speed_value);
    let view_choice = Arc::new(AtomicBool::new(chase));
    let view_control = Arc::clone(&view_choice);
    let camera_fleet = Arc::clone(&fleet);
    let initial = camera(&fleet.lock().map_err(|_| "fleet")?.current[0], chase, level);
    let world_fleet = Arc::clone(&fleet);
    let world_mailbox = mailbox.clone();
    let game = SkyGame::new(controls, initial, move |controls, sky| {
        *current_speed.lock().expect("speed control") = controls.get(speed_control);
        view_control.store(controls.get(chase_control) > 0.5, Ordering::Relaxed);
        sky.sun_elevation_degrees = controls.get(sun);
        sky.sun_azimuth_degrees = 128.0;
        sky.exposure_compensation_ev = 0.3;
        sky.temporal_inputs = true;
    })
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(STEP)?
    .with_presentation_camera(move |alpha| {
        let fleet = camera_fleet.lock().expect("floating fleet");
        let mut body = fleet.current[0];
        body.position = fleet.previous[0]
            .position
            .lerp(body.position, f64::from(alpha));
        body.rotation = fleet.previous[0]
            .rotation
            .slerp(body.rotation, f64::from(alpha));
        camera(&body, view_choice.load(Ordering::Relaxed), level)
    })
    .with_fixed_update(move |number, step| {
        let mut fleet = world_fleet.lock().expect("floating fleet");
        let fleet = &mut *fleet;
        fleet.previous.clone_from(&fleet.current);
        let craft = &mut fleet.current[0];
        let speed = *speed_value.lock().expect("speed control");
        craft.propulsion = forward * f64::from(speed);
        let answers = advance_fleet(
            &mut fleet.current,
            number,
            step,
            &world_mailbox,
            &map.lock().expect("water map"),
            &fallback,
            fleet.observe.as_deref_mut(),
        );
        world_mailbox.post(
            number,
            fleet
                .current
                .iter()
                .map(|body| body.request(number, step))
                .collect(),
        );
        #[cfg(not(target_arch = "wasm32"))]
        drop(answers);
        #[cfg(target_arch = "wasm32")]
        {
            // Retain a sliding window for status readers, including deliveries
            // on ticks followed by ticks with no reply in the same frame.
            const REPLY_HISTORY: usize = 64;
            for answer in answers {
                if fleet.reply_ages.len() == REPLY_HISTORY {
                    fleet.reply_ages.pop_front();
                }
                fleet
                    .reply_ages
                    .push_back((answer.id, answer.posted, number));
            }
        }
    });
    let game = if lake {
        game.with_meshes_and_material(
            vec![MeshPlacement::new("meshes/island", "textures/white")],
            Some(("water island", include_str!("../../island.wgsl"))),
        )?
    } else {
        game
    };
    let status_mailbox = mailbox.clone();
    #[cfg(target_arch = "wasm32")]
    let status_fleet = Arc::clone(&fleet);
    let game = game
        .with_status_readouts(move |controls| {
            let stats = status_mailbox.stats();
            controls.show_rows(
                query_status,
                vec![
                    ("Refused bodies".into(), stats.refused.to_string()),
                    ("Late points".into(), stats.late.to_string()),
                    ("Unavailable".into(), stats.unavailable.to_string()),
                    ("Blocking reads".into(), stats.blocking_reads.to_string()),
                ],
            );
            #[cfg(target_arch = "wasm32")]
            browser_status(stats, &status_fleet.lock().expect("floating fleet"));
        })
        .with_contribution(Box::new(Draw::new(Arc::clone(&fleet))), || {
            ContentStatus::Drawn(1)
        });
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok(FloatingScene {
        game,
        mailbox,
        report,
        fleet,
    })
}

fn camera(body: &Body, chase: bool, level: f64) -> Camera {
    let heading = (body.rotation * DVec3::X).with_y(0.0).normalize_or_zero();
    let across = heading.cross(DVec3::Y);
    let centre = body.position.with_y(level);
    let position = if chase {
        centre - 8.3 * heading + 2.6 * across + 5.2 * DVec3::Y
    } else {
        centre + 10.2 * across + 3.7 * DVec3::Y
    };
    let target = centre + 0.7 * across;
    Camera::look_at(
        position.as_vec3().to_array(),
        target.as_vec3().to_array(),
        52.0,
    )
}

#[cfg(target_arch = "wasm32")]
fn browser_status(stats: thrum_water::query::QueryStats, fleet: &Fleet) {
    let report = js_sys::Object::new();
    let fields = [
        ("dispatches", stats.dispatches),
        ("refused", stats.refused),
        ("busy", stats.busy),
        ("late", stats.late),
        ("unavailable", stats.unavailable),
        ("blockingReads", stats.blocking_reads),
        ("ageOne", stats.age_one),
        ("ageTwo", stats.age_two),
        ("ageOlder", stats.age_older),
    ];
    for (name, value) in fields {
        drop(js_sys::Reflect::set(
            &report,
            &name.into(),
            &(value as f64).into(),
        ));
    }
    let poses = js_sys::Array::new();
    let replies = js_sys::Array::new();
    for body in &fleet.current {
        let pose = js_sys::Array::new();
        for value in body
            .position
            .to_array()
            .into_iter()
            .chain(body.rotation.to_array())
        {
            pose.push(&value.into());
        }
        poses.push(&pose);
    }
    for &(id, posted, used) in &fleet.reply_ages {
        let reply = js_sys::Array::new();
        reply.push(&f64::from(id).into());
        reply.push(&(posted as f64).into());
        reply.push(&(used as f64).into());
        replies.push(&reply);
    }
    drop(js_sys::Reflect::set(&report, &"poses".into(), &poses));
    drop(js_sys::Reflect::set(&report, &"replies".into(), &replies));
    drop(js_sys::Reflect::set(
        &js_sys::global(),
        &"waterBodyStatus".into(),
        &report,
    ));
}
