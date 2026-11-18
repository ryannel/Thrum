//! Game-owned buoyancy, drag, propulsion and rigid-body integration.
mod draw;
mod scene;

use glam::{DQuat, DVec3};
pub use scene::{FloatingScene, build, build_paired};
use std::time::Duration;
use thrum_sea_model::{LongWaves, Spectrum};
use thrum_water::query::{BodyAnswers, BodyQuery, QueryMailbox, QueryPoint};
use thrum_water_model::{Field, Surface};

const GRAVITY: f64 = 9.80665;
const SAMPLES: usize = 6;
const STATION_WEIGHT: f64 = 1.0 / 6.0;
const MAX_LIFT: f64 = 3.5 * GRAVITY * STATION_WEIGHT;

/// Appearance and waterplane of an example body.
#[derive(Clone, Copy, Debug)]
pub enum Hull {
    /// Small cabin launch.
    Craft,
    /// Painted mooring buoy.
    Float,
}

impl Hull {
    /// Distance below the hydrostatic station that still has wetted hull area.
    pub fn keel_depth(self) -> f64 {
        match self {
            Self::Craft => 0.46,
            Self::Float => 0.1,
        }
    }

    fn dimensions(self) -> (f64, f64, f64) {
        match self {
            Self::Craft => (2.6, 0.87, 0.52),
            Self::Float => (0.40, 0.40, 0.33),
        }
    }

    fn lever(self, index: usize) -> DVec3 {
        let (length, width, _) = self.dimensions();
        let x = [-1.0, 0.0, 1.0][index / 2] * length * 0.78;
        let z = [-1.0, 1.0][index % 2] * width * if index >= 4 { 0.67 } else { 0.86 };
        DVec3::new(x, 0.0, z)
    }
}

/// One rigid body in the example's fixed-step world.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    /// Game identity.
    pub id: u32,
    /// Hull whose waterplane and weight displace water.
    pub hull: Hull,
    /// World-space origin at the bottom of the buoyant columns.
    pub position: DVec3,
    /// Proper rotation from the hull to the world.
    pub rotation: DQuat,
    /// Linear velocity in metres per second.
    pub velocity: DVec3,
    /// World angular velocity in radians per second.
    pub angular: DVec3,
    /// Commanded speed in world x/z. Only a craft has a motor.
    pub propulsion: DVec3,
    last_reply: Option<(u64, u64)>,
}

/// Actual positive work supplied to the normalized rigid-body state by water and motor.
#[derive(Default, Debug)]
pub struct Work {
    /// Positive work in joules per kilogram from the moving surface.
    pub water: f64,
    /// Positive work in joules per kilogram from the motor.
    pub motor: f64,
}

impl Body {
    /// Place a body at its hydrostatic draft with zero velocity.
    pub fn new(id: u32, hull: Hull, position: [f64; 2], level: f64, heading: f64) -> Self {
        let (_, _, draft) = hull.dimensions();
        Self {
            id,
            hull,
            position: DVec3::new(position[0], level - draft, position[1]),
            rotation: DQuat::from_rotation_y(heading),
            velocity: DVec3::ZERO,
            angular: DVec3::ZERO,
            propulsion: DVec3::ZERO,
            last_reply: None,
        }
    }

    /// The displacement implied by the waterplane loading at equilibrium.
    pub fn draft(&self) -> f64 {
        self.hull.dimensions().2
    }

    /// Query columns at the planned point of use, two ticks ahead.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPU positions and footprints use presentation precision"
    )]
    pub fn request(&self, tick: u64, step: Duration) -> BodyQuery {
        let dt = 2.0 * step.as_secs_f64();
        let (_, width, _) = self.hull.dimensions();
        BodyQuery {
            id: self.id,
            generation: 0,
            points: (0..SAMPLES)
                .map(|index| {
                    let offset = self.rotation * self.hull.lever(index);
                    let p =
                        self.position + offset + (self.velocity + self.angular.cross(offset)) * dt;
                    QueryPoint {
                        position: [p.x as f32, p.z as f32],
                        radius: (width * 0.32) as f32,
                        use_tick: tick.saturating_add(2),
                    }
                })
                .collect(),
        }
    }

    fn inertia(&self) -> DVec3 {
        let (length, width, draft) = self.hull.dimensions();
        DVec3::new(
            width * width + draft * draft,
            length * length + width * width,
            length * length + draft * draft,
        ) / 3.0
    }

    /// World position of one of the six hydrostatic stations.
    ///
    /// # Panics
    /// Panics if the station is outside zero through five.
    pub fn station(&self, index: usize) -> DVec3 {
        self.position + self.rotation * self.hull.lever(index)
    }

    /// Specific mechanical energy including gravitational, translational,
    /// angular and hydrostatic elastic energy at the supplied waterline.
    pub fn mechanical_energy(&self, mut height: impl FnMut(DVec3) -> f64) -> f64 {
        let omega = self.rotation.conjugate() * self.angular;
        let stiffness = GRAVITY * STATION_WEIGHT / self.draft();
        let spring_limit = MAX_LIFT / stiffness;
        let hydrostatic: f64 = (0..SAMPLES)
            .map(|i| {
                let p = self.station(i);
                let immersion = (height(p) - p.y).max(0.0);
                let spring = immersion.min(spring_limit);
                0.5 * stiffness * spring.powi(2) + MAX_LIFT * (immersion - spring)
            })
            .sum();
        GRAVITY * self.position.y
            + self
                .velocity
                .length_squared()
                .midpoint(omega.dot(self.inertia() * omega))
            + hydrostatic
    }

    /// Integrate the body's force and torque from the water supplied by the game.
    /// The sampler returns water at the body point for this tick.
    pub fn advance(
        &mut self,
        dt: f64,
        mut water: impl FnMut(usize, DVec3) -> (Surface, Option<f64>),
    ) -> Work {
        let (_, _, draft) = self.hull.dimensions();
        let stiffness = GRAVITY * STATION_WEIGHT / draft;
        let damping = 2.05 * (GRAVITY / draft).sqrt() * STATION_WEIGHT;
        let keel = self.hull.keel_depth();
        let forward = self.rotation * DVec3::X;
        let mut force = -GRAVITY * DVec3::Y;
        let mut torque = DVec3::ZERO;
        let mut work = Work::default();
        for i in 0..SAMPLES {
            let lever = self.rotation * self.hull.lever(i);
            let position = self.position + lever;
            let (surface, bed) = water(i, position);
            let v = self.velocity + self.angular.cross(lever);
            let mut reaction = DVec3::ZERO;
            if surface.wet {
                let immersion = surface.sample.height - position.y;
                if immersion > -keel {
                    let flow = DVec3::from_array(surface.sample.velocity);
                    let slope = DVec3::new(surface.sample.slope[0], 0.0, surface.sample.slope[1]);
                    // Hydrodynamic damping mostly follows the fluid, with part of
                    // the pitch-induced encounter rate applied at the hull stations.
                    let waterline_rate = flow.y + 0.31 * slope.dot(v - flow);
                    let encounter_rate = surface.sample.height_dt + slope.dot(v);
                    // A submerged station has finite displacement and wetted area.
                    // Bound the impulse of a broken-up short crest on a small hull.
                    let lift = (stiffness * immersion).clamp(0.0, MAX_LIFT);
                    // The wetted keel can drag down on a rising hull as long
                    // as the waterline still crosses some of it.
                    let wetted = (1.0 + immersion / keel).clamp(0.0, 1.0);
                    let vertical_drag = damping * wetted * (waterline_rate - v.y);
                    let displacement = (immersion / draft).clamp(0.0, 1.5);
                    let relative = (flow - v).with_y(0.0);
                    let along = relative.dot(forward.with_y(0.0)) * forward.with_y(0.0);
                    let drag = match self.hull {
                        Hull::Craft => 0.55 * along + 1.8 * (relative - along),
                        Hull::Float => 1.4 * relative,
                    };
                    reaction +=
                        (lift + vertical_drag) * DVec3::Y + drag * (displacement * STATION_WEIGHT);
                    work.water += dt
                        * ((lift * encounter_rate).max(0.0)
                            + (damping * wetted * waterline_rate * v.y).max(0.0)
                            + (drag.dot(flow.with_y(0.0)) * displacement * STATION_WEIGHT)
                                .max(0.0));
                }
            }
            if let Some(ground) = bed.filter(|ground| *ground > position.y) {
                let support = ((ground - position.y) * 160.0 - v.y * 10.0).max(0.0) / 6.0;
                reaction += DVec3::Y * support - v.with_y(0.0) * 2.0;
            }
            force += reaction;
            torque += lever.cross(reaction);
        }
        if matches!(self.hull, Hull::Craft) {
            // The speed governor acts along the shaft. Feed forward the
            // resistance at the currently wetted stations, which varies
            // considerably between the crest and trough of a head sea.
            let shaft = self
                .propulsion
                .try_normalize()
                .unwrap_or_else(|| forward.with_y(0.0).normalize_or_zero());
            let error = (self.propulsion - self.velocity).dot(shaft);
            let thrust = shaft * (3.5 * error - force.dot(shaft)).clamp(-16.0, 16.0);
            force += thrust;
            work.motor = (thrust.dot(self.velocity) * dt).max(0.0);
            // Keep the commanded heading while leaving pitch and roll free.
            torque.y -= self.angular.y * 2.2;
        }
        let inverse = self.rotation.conjugate();
        let inertia = self.inertia();
        let omega = inverse * self.angular;
        let body_torque = inverse * torque - omega.cross(inertia * omega);
        self.velocity += force * dt;
        self.angular += self.rotation * (body_torque / inertia) * dt;
        self.position += self.velocity * dt;
        self.rotation = (DQuat::from_scaled_axis(self.angular * dt) * self.rotation).normalize();
        work
    }
}

/// Long-wave fallback through the public CPU map model.
/// # Errors
/// Rejects an invalid wave spectrum or choppiness.
pub fn fallback(spectrum: &Spectrum, choppiness: f64) -> Result<Field, thrum_sea_model::SeaError> {
    let calm = LongWaves::new(Vec::new(), 0.0)?;
    Ok(Field {
        layers: [
            LongWaves::new(spectrum.long.clone(), choppiness)?,
            calm.clone(),
            calm.clone(),
            calm,
        ],
    })
}

/// Use a completed query at the predicted column and account for position error to first order.
pub fn sample_answer(
    answer: Option<&BodyAnswers>,
    index: usize,
    position: DVec3,
) -> Option<(Surface, Option<f64>)> {
    let answer = answer?.points.get(index)?;
    let mut surface = answer.surface;
    let offset = position
        - DVec3::new(
            f64::from(answer.point.position[0]),
            position.y,
            f64::from(answer.point.position[1]),
        );
    if offset.length_squared() > 4.0 {
        return None;
    }
    let [sx, sz] = surface.sample.slope;
    surface.sample.height += sx * offset.x + sz * offset.z;
    let bed = answer.bed.map(f64::from);
    surface.wet = surface.body.is_some() && bed.is_none_or(|bed| surface.sample.height > bed);
    Some((surface, bed))
}

/// Advance every body at the world tick, then post the next query page.
pub fn tick(
    bodies: &mut [Body],
    tick: u64,
    step: Duration,
    mailbox: &QueryMailbox,
    map: &thrum_water_model::Map,
    field: &Field,
) {
    let dt = step.as_secs_f64();
    if tick != 0 {
        let answers = mailbox.drain(tick, step);
        for body in bodies.iter_mut() {
            let answer = answers.iter().find(|answer| answer.id == body.id);
            body.last_reply = answer.map(|answer| (answer.posted, tick));
            body.advance(dt, |index, position| {
                sample_answer(answer, index, position).unwrap_or_else(|| {
                    mailbox.report_unavailable();
                    let p = [position.x, position.z];
                    let sample = map.surface(field, p, tick as f64 * dt).unwrap_or_default();
                    (sample, map.bed().height(p))
                })
            });
        }
    }
    mailbox.post(
        tick,
        bodies.iter().map(|body| body.request(tick, step)).collect(),
    );
}
