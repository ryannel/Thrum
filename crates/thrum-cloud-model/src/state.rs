use crate::{Error, Footprint, unit};
use std::sync::Arc;

/// Game-supplied weather controls. Wind is horizontal X/Z metres per weather second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// Fractional weather coverage.
    pub coverage: f64,
    /// Position along the game's type table.
    pub kind: f64,
    /// Horizontal wind.
    pub wind: [f64; 2],
    /// Weather seconds per simulation second, in 1..=60.
    pub time_scale: f64,
}

impl Parameters {
    pub(crate) fn valid(self) -> bool {
        unit(self.coverage)
            && unit(self.kind)
            && self.wind.iter().all(|x| x.is_finite())
            && self.time_scale.is_finite()
            && (1.0..=60.0).contains(&self.time_scale)
    }
}

/// One game-directed shape, fixed in the air frame, with continuously varying controls.
#[derive(Clone, Debug, PartialEq)]
pub struct Directed {
    /// Stable game identity.
    pub id: u64,
    /// Horizontal centre in the air frame.
    pub centre: [f64; 2],
    /// Rotation about world up, radians.
    pub yaw: f64,
    /// Uniform physical scale.
    pub scale: f64,
    /// Shape footprint; the headless state does not retain voxels.
    pub footprint: Arc<Footprint>,
    /// Current amplitude.
    pub envelope: f64,
    /// Target amplitude.
    pub target: f64,
    /// Current growth coordinate.
    pub growth: f64,
    /// Target growth coordinate.
    pub growth_target: f64,
}

/// Committed tick state; stepping never reads a wall clock.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    tick: u64,
    parameters: Parameters,
    target: Parameters,
    response: f64,
    travel: [f64; 2],
    previous: [f64; 2],
    time: f64,
    shapes: Vec<Directed>,
    limit: usize,
    refusals: u64,
}

impl State {
    /// Construct tick zero with no travel or shapes.
    /// # Errors
    /// Rejects invalid controls, response time or capacity.
    pub fn new(parameters: Parameters, response: f64, limit: usize) -> Result<Self, Error> {
        if !parameters.valid() || !response.is_finite() || response <= 0.0 || limit > 16 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            tick: 0,
            parameters,
            target: parameters,
            response,
            travel: [0.0; 2],
            previous: [0.0; 2],
            time: 0.0,
            shapes: Vec::with_capacity(limit),
            limit,
            refusals: 0,
        })
    }

    /// Set control targets without changing committed values.
    /// # Errors
    /// Rejects nonfinite or out-of-domain controls.
    pub fn set_target(&mut self, target: Parameters) -> Result<(), Error> {
        if !target.valid() {
            return Err(Error::Invalid);
        }
        self.target = target;
        Ok(())
    }

    /// Commit exactly the next tick, using its simulation duration.
    /// Parameters ramp at one full range per response time; wind ramps by 40 m/s
    /// per response time. Travel uses the resulting tick's wind and time scale.
    /// # Errors
    /// Rejects nonconsecutive ticks, invalid duration and arithmetic overflow.
    pub fn step(&mut self, tick: u64, dt: f64) -> Result<(), Error> {
        if self.tick.checked_add(1) != Some(tick) {
            return Err(Error::Tick);
        }
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::Invalid);
        }
        let rate = dt * self.parameters.time_scale / self.response;
        let p = self.parameters;
        let t = self.target;
        let parameters = Parameters {
            coverage: approach(p.coverage, t.coverage, rate),
            kind: approach(p.kind, t.kind, rate),
            wind: std::array::from_fn(|i| approach(p.wind[i], t.wind[i], 40.0 * rate)),
            time_scale: approach(p.time_scale, t.time_scale, 59.0 * rate),
        };
        let elapsed = dt * parameters.time_scale;
        let travel = std::array::from_fn(|i| self.travel[i] + parameters.wind[i] * elapsed);
        if !travel.iter().all(|x| x.is_finite()) || !(self.time + elapsed).is_finite() {
            return Err(Error::Invalid);
        }
        self.previous = self.travel;
        self.travel = travel;
        self.time += elapsed;
        self.parameters = parameters;
        self.tick = tick;
        for shape in &mut self.shapes {
            shape.envelope = approach(shape.envelope, shape.target, rate);
            shape.growth = approach(shape.growth, shape.growth_target, rate);
        }
        self.shapes.retain(|s| s.envelope != 0.0 || s.target != 0.0);
        Ok(())
    }

    /// Current controls.
    pub fn parameters(&self) -> Parameters {
        self.parameters
    }
    /// Weather time integrated on ticks.
    pub fn time(&self) -> f64 {
        self.time
    }
    /// Current integrated travel.
    pub fn travel(&self) -> [f64; 2] {
        self.travel
    }
    /// Previous tick's integrated travel.
    pub fn previous_travel(&self) -> [f64; 2] {
        self.previous
    }
    /// Presentation travel; a cut uses the committed tick.
    pub fn display_travel(&self, alpha: f64, cut: bool) -> [f64; 2] {
        if cut {
            return self.travel;
        }
        std::array::from_fn(|i| {
            self.travel[i] - (1.0 - alpha.clamp(0.0, 1.0)) * (self.travel[i] - self.previous[i])
        })
    }
    /// World horizontal point to the current air frame.
    pub fn to_air(&self, world: [f64; 2]) -> [f64; 2] {
        std::array::from_fn(|i| world[i] - self.travel[i])
    }
    /// Air horizontal point to world.
    pub fn to_world(&self, air: [f64; 2]) -> [f64; 2] {
        std::array::from_fn(|i| air[i] + self.travel[i])
    }
    /// Directed shapes in stable insertion order.
    pub fn shapes(&self) -> &[Directed] {
        &self.shapes
    }
    /// Number of rejected directed operations.
    pub fn refusals(&self) -> u64 {
        self.refusals
    }

    /// Admit a shape at envelope zero. Conservative rotated bounding circles
    /// reject all possible overlapping footprints, including hollow regions.
    /// # Errors
    /// Rejects invalid transforms, duplicate ids, overlaps and exhausted capacity.
    pub fn add(&mut self, shape: Directed) -> Result<(), Error> {
        let error = if !shape.centre.iter().all(|x| x.is_finite())
            || !shape.yaw.is_finite()
            || !shape.scale.is_finite()
            || shape.scale <= 0.0
            || shape.envelope != 0.0
            || !unit(shape.target)
            || !unit(shape.growth)
            || !unit(shape.growth_target)
        {
            Some(Error::Invalid)
        } else if self.shapes.len() == self.limit {
            Some(Error::Capacity)
        } else if self.shapes.iter().any(|s| {
            s.id == shape.id
                || (s.centre[0] - shape.centre[0]).hypot(s.centre[1] - shape.centre[1])
                    < s.footprint.radius() * s.scale + shape.footprint.radius() * shape.scale
        }) {
            Some(Error::Overlap)
        } else {
            None
        };
        if let Some(error) = error {
            self.refusals += 1;
            return Err(error);
        }
        self.shapes.push(shape);
        Ok(())
    }

    /// Change a directed shape's targets.
    /// # Errors
    /// Rejects absent ids and targets outside 0..=1.
    pub fn direct(&mut self, id: u64, envelope: f64, growth: f64) -> Result<(), Error> {
        if !unit(envelope) || !unit(growth) {
            self.refusals += 1;
            return Err(Error::Invalid);
        }
        let Some(s) = self.shapes.iter_mut().find(|s| s.id == id) else {
            self.refusals += 1;
            return Err(Error::Missing);
        };
        s.target = envelope;
        s.growth_target = growth;
        Ok(())
    }
}

fn approach(value: f64, target: f64, delta: f64) -> f64 {
    value + (target - value).clamp(-delta, delta)
}
