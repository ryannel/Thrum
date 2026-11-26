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
    interaction::{BodyPressure, Limits},
    query::{QueryLimits, QueryMailbox},
};
use thrum_water_model::{Bed, BodyKind, Impulse, Map, Stage};

pub(super) struct Fleet {
    pub(super) previous: Vec<Body>,
    pub(super) current: Vec<Body>,
    pub(super) craft_present: bool,
    tick: u64,
    observe: Option<Box<dyn FnMut(BodyStep) + Send>>,
    #[cfg(target_arch = "wasm32")]
    reply_ages: std::collections::VecDeque<(u32, u64, u64)>,
    #[cfg(target_arch = "wasm32")]
    wake_sample: Option<[f64; 6]>,
}

const STEP: Duration = Duration::from_nanos(16_666_667);

/// Wake demonstration settings used by the normal water scene route.
#[derive(Clone, Copy, Debug, Default)]
pub struct WakeDemo {
    /// View used for inspection.
    pub view: WakeView,
    /// Drop a stone at one second in the lake.
    pub stone: bool,
    /// Start with the interaction producer disabled for the paired comparison.
    pub disabled: bool,
    /// Override the number of admitted body reactions for a capacity inspection.
    pub body_capacity: Option<u32>,
    /// Whitewater route and its ranked pattern source.
    pub whitewater: WhitewaterDemo,
    /// Use the whitewater course's Sun placement in paired cost arms.
    pub whitewater_lighting: bool,
}

/// Asset route for the example's optional whitewater.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhitewaterDemo {
    /// Leave out foam and shore film.
    #[default]
    Off,
    /// Enable whitewater with the fallback pattern.
    Procedural,
    /// Enable whitewater and lend a ranked texture from the cooked store.
    Cooked,
}

/// Camera for interaction waves in the water example.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WakeView {
    /// Follow the craft from behind.
    #[default]
    Chase,
    /// Follow the craft from 40 metres above the still level.
    Overhead,
    /// Watch the lake from two metres above the still level.
    Shore,
    /// Look out across the sea from two metres.
    Waterline,
    /// Look out across the sea from forty metres.
    Vista,
}

impl WakeView {
    /// Interpret a scene startup option.
    pub fn named(option: &str) -> Self {
        match option {
            "overhead" => Self::Overhead,
            "shore" => Self::Shore,
            "waterline" => Self::Waterline,
            "vista" => Self::Vista,
            _ => Self::Chase,
        }
    }
}

/// Sea or lake body scene, including the public game-facing mailbox.
pub struct FloatingScene {
    /// Normal app-shell scene and fixed-step body simulation.
    pub game: SkyGame,
    /// Query status used by the scene and capture harness.
    pub mailbox: QueryMailbox,
    /// Preparation and resource report.
    pub report: crate::SharedReport,
    /// Optional submitted foam endpoints for numerical inspection.
    pub foam: Option<thrum_water::foam::Tap>,
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
    build_scene(wind, lake, speed, heading, chase, draw_water, None, None)
}

/// Build the craft and float scene with the public interaction wave inputs.
/// # Errors
/// Returns invalid scene, mesh, spectrum, map or controls.
pub fn build_wake(
    wind: Wind,
    lake: bool,
    speed: f32,
    heading: f32,
    demo: WakeDemo,
) -> Result<FloatingScene, Box<dyn Error>> {
    build_wake_paired(wind, lake, speed, heading, demo, true)
}

/// Build the same wake route with or without the water contribution.
/// # Errors
/// Returns invalid scene, mesh, spectrum, map or controls.
pub fn build_wake_paired(
    wind: Wind,
    lake: bool,
    speed: f32,
    heading: f32,
    demo: WakeDemo,
    draw_water: bool,
) -> Result<FloatingScene, Box<dyn Error>> {
    build_scene(
        wind,
        lake,
        speed,
        heading,
        true,
        draw_water,
        Some(demo),
        None,
    )
}

/// Centre of the unit shadow caster kept outside the optics cost view.
pub const OPTICS_COST_CASTER: [f32; 3] = [162.0, 2.0, 29.0];

/// Same camera, wave forcing and poses in both arms of the optics record.
/// # Errors
/// Propagates scene construction errors.
pub fn build_optics_arm(
    wind: Wind,
    bodies: usize,
    optics: bool,
) -> Result<FloatingScene, Box<dyn Error>> {
    build_scene(
        wind,
        false,
        0.0,
        std::f32::consts::PI,
        false,
        true,
        None,
        Some((optics, bodies)),
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "the game route owns the connected camera, controls, tick writers and status callbacks"
)]
fn build_scene(
    wind: Wind,
    lake: bool,
    speed: f32,
    heading: f32,
    chase: bool,
    draw_water: bool,
    demo: Option<WakeDemo>,
    optics_arm: Option<(bool, usize)>,
) -> Result<FloatingScene, Box<dyn Error>> {
    let demo = demo.map(|mut demo| {
        demo.stone &= lake;
        demo
    });
    let map = Arc::new(Mutex::new(if lake {
        crate::island::map(wind)?
    } else {
        sea(wind)?
    }));
    let spectrum = Spectrum::build(&wind.forcing())?;
    let mut profile = crate::profile(&spectrum);
    profile.optics = optics_arm
        .is_none_or(|(enabled, _)| enabled)
        .then(thrum_water::Optics::default);
    profile.query = Some(QueryLimits {
        points: 2048,
        max_age: 3,
    });
    if let Some(demo) = demo.filter(|demo| !demo.disabled) {
        let mut limits = Limits::default();
        limits.bodies = demo.body_capacity.unwrap_or(limits.bodies);
        profile.interaction = Some(limits);
    }
    let fallback = fallback(&spectrum, profile.waves.choppiness)?;
    let whitewater = demo.is_some_and(|demo| demo.whitewater != WhitewaterDemo::Off);
    let mut water = if lake || whitewater {
        Water::mapped(Arc::clone(&map), spectrum, profile)?
    } else {
        Water::new(Arc::clone(&map), 0, spectrum, profile)?
    };
    let foam_input = whitewater
        .then(|| water.enable_foam(wind.velocity()))
        .transpose()?;
    let foam_tap = whitewater.then(|| water.tap_foam());
    let mailbox = water
        .query_mailbox()
        .ok_or("floating scene requires a body query")?;
    let report = water.report();
    let status_report = Arc::clone(&report);
    let interaction = water.interaction_mailbox();
    let (start, level) = if lake && demo.is_some() {
        ([204.0, 0.0], 12.0)
    } else if lake {
        ([182.0, 10.0], 12.0)
    } else {
        ([150.0, 32.0], 0.0)
    };
    let mut bodies = vec![Body::new(0, Hull::Craft, start, level, f64::from(heading))];
    let forward = (bodies[0].rotation * DVec3::X).with_y(0.0);
    bodies[0].propulsion = forward * f64::from(speed);
    bodies[0].velocity = bodies[0].propulsion;
    // The example's prevailing wind runs along X. Leave more clearance
    // when its drift carries the floats across the course.
    let moving_clearance = if forward.z.abs() > forward.x.abs() {
        6.6
    } else {
        5.0
    };
    for id in 1..=8 {
        let (x, z) = if speed > 0.0 && !lake {
            if demo.is_some() && id <= 2 {
                (17.0, if id == 1 { 20.0 } else { -20.0 })
            } else {
                (
                    f64::from(id - 1) * 5.0 + 2.0,
                    moving_clearance + f64::from(id % 2),
                )
            }
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
        craft_present: true,
        tick: 0,
        observe: None,
        #[cfg(target_arch = "wasm32")]
        reply_ages: std::collections::VecDeque::new(),
        #[cfg(target_arch = "wasm32")]
        wake_sample: None,
    }));
    let mut controls = Controls::new();
    let query_status = controls.table("water.query", "Water query")?;
    let speed_control = controls
        .slider("craft.speed", "Craft speed", 0.0..=15.0)
        .default(speed)
        .add()?;
    let heading_control = demo
        .map(|_| {
            controls
                .slider("craft.turn", "Course change", -90.0..=90.0)
                .default(0.0)
                .add()
        })
        .transpose()?;
    let (elevation, azimuth) = match demo.map(|demo| demo.view) {
        Some(WakeView::Chase | WakeView::Overhead)
            if whitewater || demo.is_some_and(|demo| demo.whitewater_lighting) =>
        {
            (49.0, 76.0)
        }
        Some(WakeView::Overhead) => (79.0, 269.0),
        Some(WakeView::Shore) if demo.is_some_and(|demo| demo.stone) => (15.0, 23.0),
        Some(WakeView::Shore) => (14.0, 43.0),
        Some(WakeView::Chase) => (23.0, 251.0),
        Some(WakeView::Waterline | WakeView::Vista) => (56.0, 235.0),
        None => (35.0, 128.0),
    };
    let craft_control = whitewater
        .then(|| {
            controls
                .slider("craft.present", "Craft present", 0.0..=1.0)
                .default(1.0)
                .step(1.0)
                .add()
        })
        .transpose()?;
    let sun = controls
        .slider("sun.elevation", "Sun elevation", -25.0..=80.0)
        .default(elevation)
        .add()?;
    let chase_control = controls
        .slider("camera.chase", "Side / Chase", 0.0..=1.0)
        .default(f32::from(chase))
        .step(1.0)
        .add()?;
    let optics_height = controls
        .slider("camera.optics_height", "Optics eye height", 0.0..=10.0)
        .default(0.0)
        .add()?;
    let optics_pan = controls
        .slider("camera.optics_pan", "Optics pan", -85.0..=85.0)
        .default(0.0)
        .add()?;
    let camera_angles = Arc::new(Mutex::new([0.0_f32; 2]));
    let presented_angles = Arc::clone(&camera_angles);
    let speed_value = Arc::new(Mutex::new((speed, 0.0)));
    let current_speed = Arc::clone(&speed_value);
    let view_choice = Arc::new(AtomicBool::new(chase));
    let view_control = Arc::clone(&view_choice);
    let visible = Arc::new(AtomicBool::new(true));
    let craft_present = Arc::clone(&visible);
    let camera_fleet = Arc::clone(&fleet);
    let initial = camera(
        &fleet.lock().map_err(|_| "fleet")?.current[0],
        chase,
        level,
        demo,
    );
    let world_fleet = Arc::clone(&fleet);
    let camera_swell = demo
        .filter(|demo| demo.view == WakeView::Waterline)
        .map(|_| fallback.layers[0].clone());
    let world_mailbox = mailbox.clone();
    let game = SkyGame::new(controls, initial, move |controls, sky| {
        *current_speed.lock().expect("speed control") = (
            controls.get(speed_control),
            heading_control.map_or(0.0, |turn| controls.get(turn)),
        );
        view_control.store(controls.get(chase_control) > 0.5, Ordering::Relaxed);
        visible.store(
            craft_control.is_none_or(|control| controls.get(control) > 0.5),
            Ordering::Relaxed,
        );
        *camera_angles.lock().expect("optics angles") =
            [controls.get(optics_height), controls.get(optics_pan)];
        sky.sun_elevation_degrees = controls.get(sun);
        sky.sun_azimuth_degrees = azimuth;
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
        let mut camera = camera(&body, view_choice.load(Ordering::Relaxed), level, demo);
        let [height, pan] = *presented_angles.lock().expect("optics angles");
        if height > 0.0 {
            let target = glam::Vec3::from_array(camera.target);
            let eye = glam::Vec3::from_array(camera.position);
            let distance = (height - 4.0).max(0.0) * 3.5;
            let offset = (eye - target).with_y(0.0).normalize_or_zero() * distance;
            camera.position = (eye + offset).to_array();
            camera.position[1] = (body.position + DVec3::Y * body.draft()).as_vec3().y + height;
            camera.vertical_fov_degrees /= 1.0 + distance / 24.0;
        }
        crate::island::pan_camera(&mut camera, pan, 0.0);
        if let Some(swell) = &camera_swell {
            keep_camera_above_swell(&mut camera, swell, fleet.tick, alpha);
        }
        camera
    })
    .with_fixed_update_endpoint(move |endpoint, step| {
        let number = endpoint.tick;
        let mut fleet = world_fleet.lock().expect("floating fleet");
        let fleet = &mut *fleet;
        fleet.craft_present = craft_present.load(Ordering::Relaxed);
        fleet.tick = number;
        fleet.previous.clone_from(&fleet.current);
        let craft = &mut fleet.current[0];
        let (speed, turn) = *speed_value.lock().expect("speed control");
        let steering = glam::DQuat::from_rotation_y(f64::from(turn).to_radians());
        craft.propulsion = steering * forward * f64::from(speed);
        if turn != 0.0 {
            let target = steering * forward;
            let axis = craft.rotation * DVec3::X;
            craft.angular.y += 2.5 * axis.cross(target).y * step.as_secs_f64();
        }
        let first = usize::from(!fleet.craft_present);
        let active = &mut fleet.current[first..];
        let answers = advance_fleet(
            active,
            endpoint,
            step,
            &world_mailbox,
            &map,
            &fallback,
            fleet.observe.as_deref_mut(),
        );
        if let Some(foam) = &foam_input {
            foam.post(number, active.iter().map(foam_hull));
        }
        if let Some(interaction) = &interaction {
            let reactions = active
                .iter()
                .map(|body| {
                    let axis = body.rotation * DVec3::X;
                    let (mass, footprint) = match body.hull {
                        Hull::Craft => (3600.0, [1.45, 0.58]),
                        Hull::Float => (75.0, [0.26, 0.26]),
                    };
                    BodyPressure {
                        id: body.id,
                        place: [body.position.x, body.position.z],
                        heading: axis.z.atan2(axis.x),
                        footprint,
                        force: mass * body.water_reaction,
                    }
                })
                .collect();
            let follow = (fleet.craft_present && !demo.is_some_and(|demo| demo.stone)).then_some(0);
            interaction.post_bodies(number, follow, reactions);
            if number == 60 && lake && demo.is_some_and(|demo| demo.stone) {
                interaction.post_impulse(Impulse {
                    id: 1,
                    place: [184.0, 28.0],
                    vertical: 1250.0,
                    radius: 0.58,
                    tick: number,
                });
            }
        }
        world_mailbox.post(
            endpoint,
            active
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
            for answer in answers.into_iter().filter(|answer| !answer.held) {
                if interaction.is_some()
                    && answer.id == 1
                    && let Some(point) = answer.points.first()
                    && let Some(time) = point.sources[4]
                {
                    fleet.wake_sample = Some([
                        (time / STEP.as_secs_f64()).round(),
                        number as f64,
                        point.point.use_tick as f64,
                        f64::from(point.point.position[0]),
                        f64::from(point.point.position[1]),
                        point.surface.sample.height,
                    ]);
                }
                if fleet.reply_ages.len() == REPLY_HISTORY {
                    fleet.reply_ages.pop_front();
                }
                fleet
                    .reply_ages
                    .push_back((answer.id, answer.posted, number));
            }
        }
    });
    let game = if lake || demo.is_some_and(|demo| demo.whitewater == WhitewaterDemo::Cooked) {
        let meshes = if lake {
            vec![MeshPlacement::new("meshes/island", "textures/white")]
        } else {
            Vec::new()
        };
        let material = lake.then_some(("water island", include_str!("../../island.wgsl")));
        let (game, lease) = game.with_meshes_and_texture_lease(
            meshes,
            material,
            if whitewater {
                &["textures/foam-rank"]
            } else {
                &[]
            },
        )?;
        if whitewater {
            water.lend_foam_pattern(lease);
        }
        game
    } else {
        game
    };
    let status_mailbox = mailbox.clone();
    #[cfg(target_arch = "wasm32")]
    let status_fleet = Arc::clone(&fleet);
    let game = game
        .with_status_readouts(move |controls| {
            let stats = status_mailbox.stats();
            let wake = status_report.lock().expect("water report").interaction;
            let mut rows = vec![
                ("Refused bodies".into(), stats.refused.to_string()),
                ("Late points".into(), stats.late.to_string()),
                ("Unavailable".into(), stats.unavailable.to_string()),
                ("Blocking reads".into(), stats.blocking_reads.to_string()),
            ];
            if let Some(wake) = wake {
                rows.extend([
                    ("Wake endpoints".into(), format!("{:?}", wake.endpoints)),
                    (
                        "Refused body reactions".into(),
                        wake.refused_bodies.to_string(),
                    ),
                    ("Refused impulses".into(), wake.refused_impulses.to_string()),
                    (
                        "Sources in refused regions".into(),
                        format!("{:?}", wake.refused_regions),
                    ),
                    ("Wake resets".into(), wake.resets.to_string()),
                    ("Dropped wake ticks".into(), wake.dropped_ticks.to_string()),
                ]);
            }
            controls.show_rows(query_status, rows);
            #[cfg(target_arch = "wasm32")]
            browser_status(stats, wake, &status_fleet.lock().expect("floating fleet"));
        })
        .with_contribution(
            Box::new(Draw::new(
                Arc::clone(&fleet),
                water.optics_bodies(),
                optics_arm.map_or(9, |(_, count)| count),
            )),
            || ContentStatus::Drawn(1),
        );
    let game = if optics_arm.is_some() {
        game.with_meshes(vec![
            MeshPlacement::new("meshes/float", "textures/white").at(OPTICS_COST_CASTER),
        ])?
    } else {
        game
    };
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok(FloatingScene {
        game,
        mailbox,
        report,
        foam: foam_tap,
        fleet,
    })
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "camera inspection uses the bounded game clock and metre-scale swell"
)]
fn keep_camera_above_swell(
    camera: &mut Camera,
    swell: &thrum_sea_model::LongWaves,
    tick: u64,
    alpha: f32,
) {
    let time = (tick as f64 - 1.0 + f64::from(alpha)).max(0.0) * STEP.as_secs_f64();
    let point = [camera.position[0], camera.position[2]].map(f64::from);
    // The dry-side inspection rides above the local swell. Shorter waves
    // still pass below the eye and the still-water height sets the floor.
    let rise = swell
        .evaluate(point, time)
        .map_or(0.0, |wave| wave.height.max(0.0) as f32);
    camera.position[1] += rise;
    camera.target[1] += rise;
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the bounded example course fits GPU metres and watts"
)]
fn foam_hull(body: &Body) -> thrum_water::foam::Hull {
    let forward = (body.rotation * DVec3::X).with_y(0.0).normalize_or_zero();
    let (length, width, _) = body.hull.dimensions();
    let mass = match body.hull {
        Hull::Craft => 3600.0,
        Hull::Float => 75.0,
    };
    thrum_water::foam::Hull {
        place: [body.position.x as f32, body.position.z as f32],
        forward: [forward.x as f32, forward.z as f32],
        extent: [length as f32, width as f32],
        velocity: [body.velocity.x as f32, body.velocity.z as f32],
        drag_power: (mass * body.drag_power) as f32,
    }
}

fn camera(body: &Body, chase: bool, level: f64, demo: Option<WakeDemo>) -> Camera {
    let heading = (body.rotation * DVec3::X).with_y(0.0).normalize_or_zero();
    let across = heading.cross(DVec3::Y);
    let centre = body.position.with_y(level);
    if let Some(demo) = demo.filter(|demo| demo.stone) {
        match demo.view {
            WakeView::Shore => {
                return Camera::look_at([181.0, 14.0, 35.0], [184.0, 12.0, 26.2], 62.0);
            }
            WakeView::Overhead => {
                return Camera::look_at([190.0, 52.0, 27.5], [184.0, 12.0, 27.0], 65.0);
            }
            WakeView::Chase | WakeView::Waterline | WakeView::Vista => {}
        }
    }
    if demo.is_some_and(|demo| demo.view == WakeView::Shore) {
        return crate::island::CAMERAS[1];
    }
    if demo.is_some_and(|demo| demo.view == WakeView::Overhead) {
        let target = centre - 13.0 * heading;
        return Camera::look_at(
            (target + DVec3::Y * 40.0 - 4.0 * heading)
                .as_vec3()
                .to_array(),
            target.as_vec3().to_array(),
            72.0,
        );
    }
    if let Some(demo) =
        demo.filter(|demo| matches!(demo.view, WakeView::Waterline | WakeView::Vista))
    {
        let height = if demo.view == WakeView::Vista {
            40.0
        } else {
            2.0
        };
        let eye = centre - 8.0 * DVec3::X + height * DVec3::Y;
        let look = eye - 90.0 * DVec3::X - (height * 0.7 + 4.0) * DVec3::Y;
        return Camera::look_at(eye.as_vec3().to_array(), look.as_vec3().to_array(), 65.0);
    }
    let position = if demo.is_some() && chase {
        centre - 16.0 * heading + 3.6 * across + 6.4 * DVec3::Y
    } else if chase {
        centre - 8.3 * heading + 2.6 * across + 5.2 * DVec3::Y
    } else {
        centre + 10.2 * across + 3.7 * DVec3::Y
    };
    let mut target = centre + 0.7 * across;
    if !chase {
        target.y += 0.4;
    }
    Camera::look_at(
        position.as_vec3().to_array(),
        target.as_vec3().to_array(),
        52.0,
    )
}

#[cfg(target_arch = "wasm32")]
fn browser_status(
    stats: thrum_water::query::QueryStats,
    wake: Option<thrum_water::interaction::Report>,
    fleet: &Fleet,
) {
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
    if let Some(wake) = wake {
        for (name, value) in [
            (
                "wakeTick",
                wake.endpoints.map_or(0, |endpoints| endpoints[1]),
            ),
            ("wakeResets", wake.resets),
            ("wakeDropped", wake.dropped_ticks),
        ] {
            drop(js_sys::Reflect::set(
                &report,
                &name.into(),
                &(value as f64).into(),
            ));
        }
        if let Some(sample) = fleet.wake_sample {
            let values = js_sys::Array::new();
            for value in sample {
                values.push(&value.into());
            }
            drop(js_sys::Reflect::set(&report, &"wakeSample".into(), &values));
        }
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

pub(crate) fn install_channel_float(
    game: SkyGame,
    map: crate::SharedMap,
    water: &Water,
    fallback: thrum_water_model::Field,
    foam: thrum_water::foam::Input,
    wind: glam::DVec2,
) -> (SkyGame, Box<dyn Fn() -> Body>) {
    use crate::channel::{POST, RELEASE, WIDE_POST};
    let initial = Body::new(
        0,
        Hull::Float,
        RELEASE,
        map.lock().expect("channel map").row(3, RELEASE).level,
        0.0,
    );
    let fleet = Arc::new(Mutex::new(Fleet {
        previous: vec![initial],
        current: vec![initial],
        craft_present: false,
        tick: 0,
        observe: None,
        #[cfg(target_arch = "wasm32")]
        reply_ages: std::collections::VecDeque::new(),
        #[cfg(target_arch = "wasm32")]
        wake_sample: None,
    }));
    let mailbox = water.query_mailbox().expect("channel float query");
    let world = Arc::clone(&fleet);
    #[cfg(target_arch = "wasm32")]
    let status_map = Arc::clone(&map);
    let game = game.with_fixed_update_endpoint(move |endpoint, step| {
        let mut fleet = world.lock().expect("channel float");
        let fleet = &mut *fleet;
        fleet.tick = endpoint.tick;
        fleet.previous.clone_from(&fleet.current);
        // Air drag acts on the exposed float in the scene's wind. Water
        // drag at the six stations supplies the much faster river drift.
        let float = &mut fleet.current[0];
        let slip = wind - glam::DVec2::new(float.velocity.x, float.velocity.z);
        let air_drag = slip * (0.0045 * slip.length() * step.as_secs_f64());
        float.velocity.x += air_drag.x;
        float.velocity.z += air_drag.y;
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
            vec![fleet.current[0].request(endpoint.tick, step)],
        );
        foam.post(
            endpoint.tick,
            [POST, WIDE_POST].map(|place| thrum_water::foam::Hull {
                place,
                forward: [1.0, 0.0],
                extent: [0.32, 0.32],
                velocity: [0.0; 2],
                drag_power: 0.0,
            }),
        );
    });
    #[cfg(target_arch = "wasm32")]
    let game = {
        let status = Arc::clone(&fleet);
        game.with_status_readouts(move |_| {
            let fleet = status.lock().expect("channel float");
            let values = js_sys::Array::new();
            let position = fleet.current[0].position;
            for value in position.to_array() {
                values.push(&value.into());
            }
            if let Some(row) = status_map
                .lock()
                .expect("channel map")
                .sample([position.x, position.z])
            {
                let owner = u32::try_from(row.body).expect("island body index");
                values.push(&owner.into());
                values.push(&row.depth.into());
            }
            drop(js_sys::Reflect::set(
                &js_sys::global(),
                &"waterChannelFloat".into(),
                &values,
            ));
        })
    };
    let game = game.with_contribution(
        Box::new(Draw::new(Arc::clone(&fleet), water.optics_bodies(), 1)),
        || ContentStatus::Drawn(1),
    );
    (
        game,
        Box::new(move || fleet.lock().expect("channel float").current[0]),
    )
}
