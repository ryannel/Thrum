use crate::{Error, Footprint, HashedShapes, PlacedLibrary, ShapeSource, unit};
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
    /// Whether the model accepts these controls as a target.
    #[must_use]
    pub fn valid(self) -> bool {
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
    placed: Vec<(u64, ShapeSource)>,
    library: Option<Arc<PlacedLibrary>>,
    hashed: Option<HashedShapes>,
    limit: usize,
    refusals: u64,
    ground: crate::GroundState,
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
            placed: Vec::new(),
            library: None,
            hashed: None,
            limit,
            refusals: 0,
            ground: crate::GroundState::default(),
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
        let parameters = self.parameters_toward_target(self.parameters, dt);
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
        self.ground.ramp(rate);
        for shape in &mut self.shapes {
            shape.envelope = approach(shape.envelope, shape.target, rate);
            shape.growth = approach(shape.growth, shape.growth_target, rate);
        }
        self.shapes.retain(|s| s.envelope != 0.0 || s.target != 0.0);
        self.placed
            .retain(|(id, _)| self.shapes.iter().any(|shape| shape.id == *id));
        Ok(())
    }

    /// Replace ground controls; a scene supplies one amount per fog-table row.
    pub fn set_ground(&mut self, ground: crate::GroundState) {
        self.ground = ground;
    }

    /// Committed ground and haze controls.
    pub fn ground(&self) -> &crate::GroundState {
        &self.ground
    }

    /// Mutable access for setting ground-control targets.
    pub fn ground_mut(&mut self) -> &mut crate::GroundState {
        &mut self.ground
    }

    /// Commit a weather tick and radiative cooling using the supplied true Sun
    /// elevation in degrees. No environment, renderer or wall clock is read.
    /// # Errors
    /// Rejects invalid Sun or the same invalid operations as `step`, atomically.
    pub fn step_day(
        &mut self,
        tick: u64,
        dt: f64,
        sun: f64,
        ground: &crate::Ground,
    ) -> Result<(), Error> {
        if !sun.is_finite() || !(-90.0..=90.0).contains(&sun) {
            return Err(Error::Invalid);
        }
        self.step(tick, dt)?;
        self.ground.day(
            ground,
            dt * self.parameters.time_scale,
            sun,
            self.parameters,
        );
        Ok(())
    }

    /// Integrate ground cooling over the solar time that elapsed on this tick.
    /// Use this after `step` when the game's solar clock advances independently of advection.
    /// # Errors
    /// Rejects invalid elapsed solar time or elevation without changing the state.
    pub fn advance_ground_cooling(
        &mut self,
        ground: &crate::Ground,
        solar_seconds: f64,
        elevation_degrees: f64,
    ) -> Result<(), Error> {
        if !solar_seconds.is_finite()
            || solar_seconds < 0.0
            || !(-90.0..=90.0).contains(&elevation_degrees)
        {
            return Err(Error::Invalid);
        }
        self.ground
            .day(ground, solar_seconds, elevation_degrees, self.parameters);
        Ok(())
    }

    /// Last committed weather tick.
    pub fn tick(&self) -> u64 {
        self.tick
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
    /// Presentation travel at the consumed-target phase, with at most 50 ms
    /// of prediction. Each predicted tick uses the committed control targets
    /// and the same arithmetic as `step`, without copying shapes or committing.
    /// A cut or an invalid duration uses the committed tick.
    pub fn display_travel(&self, phase: f64, dt: f64, cut: bool) -> [f64; 2] {
        if cut || !dt.is_finite() || dt <= 0.0 || !phase.is_finite() {
            return self.travel;
        }
        let phase = phase.clamp(0.0, 1.0 + 0.05 / dt);
        if phase <= 1.0 {
            return std::array::from_fn(|i| {
                self.travel[i] - (1.0 - phase) * (self.travel[i] - self.previous[i])
            });
        }
        let mut remaining = phase - 1.0;
        let mut travel = self.travel;
        let mut parameters = self.parameters;
        while remaining > 0.0 {
            parameters = self.parameters_toward_target(parameters, dt);
            let elapsed = dt * parameters.time_scale;
            let next = std::array::from_fn::<_, 2, _>(|i| travel[i] + parameters.wind[i] * elapsed);
            let fraction = remaining.min(1.0);
            travel = if fraction >= 1.0 {
                next
            } else {
                std::array::from_fn(|i| travel[i] + fraction * (next[i] - travel[i]))
            };
            remaining -= fraction;
        }
        travel
    }

    fn parameters_toward_target(&self, p: Parameters, dt: f64) -> Parameters {
        let rate = dt * p.time_scale / self.response;
        let t = self.target;
        Parameters {
            coverage: approach(p.coverage, t.coverage, rate),
            kind: approach(p.kind, t.kind, rate),
            wind: std::array::from_fn(|i| approach(p.wind[i], t.wind[i], 40.0 * rate)),
            time_scale: approach(p.time_scale, t.time_scale, 59.0 * rate),
        }
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

    /// Supply immutable library footprints before admitting shapes through the library.
    /// # Errors
    /// Rejects changing the library with directed library instances still active.
    pub fn set_library(&mut self, library: Arc<PlacedLibrary>) -> Result<(), Error> {
        if !self.placed.is_empty() {
            return Err(Error::Invalid);
        }
        self.library = Some(library);
        self.hashed = None;
        Ok(())
    }

    /// Select hashed placement independently of the camera, instead of admitting directed instances.
    /// # Errors
    /// Rejects invalid block sizes, empty libraries, shares outside 0..=1 or existing directed shapes.
    pub fn set_hashed_library(
        &mut self,
        library: Arc<PlacedLibrary>,
        placement: HashedShapes,
    ) -> Result<(), Error> {
        if !self.shapes.is_empty()
            || !(3..=64).contains(&placement.block_sites)
            || placement.shares.len() < 2
            || !placement.shares.iter().copied().all(unit)
        {
            return Err(Error::Invalid);
        }
        self.set_library(library)?;
        self.hashed = Some(placement);
        Ok(())
    }

    /// Library selection in a profile that places shapes through the field.
    pub fn hashed_shapes(&self) -> Option<&HashedShapes> {
        self.hashed.as_ref()
    }

    /// Fraction of the heaped-cloud envelope kept for procedural cells.
    /// Decks and directed shapes do not participate in hashed placement.
    pub fn procedural_share(&self) -> f64 {
        self.hashed_shapes()
            .map_or(1.0, |placement| 1.0 - placement.share(self.parameters.kind))
    }

    /// Library owned by the game's weather state.
    pub fn library(&self) -> Option<&PlacedLibrary> {
        self.library.as_deref()
    }

    /// Source a resident stack can render for this directed instance, if bound.
    pub fn shape_source(&self, id: u64) -> Option<ShapeSource> {
        self.placed
            .iter()
            .find_map(|&(key, source)| (key == id).then_some(source))
    }

    /// Admit a library instance at envelope zero, using its registered footprint.
    /// # Errors
    /// Rejects absent sources and the same invalid placements as [Self::add].
    pub fn add_from_library(
        &mut self,
        mut shape: Directed,
        source: ShapeSource,
    ) -> Result<(), Error> {
        let Some(slab) = self.library().and_then(|library| library.geometry(source)) else {
            self.refusals += 1;
            return Err(Error::Missing);
        };
        shape.footprint = Arc::clone(&slab.footprint);
        let id = shape.id;
        self.add(shape)?;
        self.placed.push((id, source));
        Ok(())
    }

    /// Admit a shape at envelope zero. Conservative rotated bounding circles
    /// reject all possible overlapping footprints, including hollow regions.
    /// # Errors
    /// Rejects invalid transforms, duplicate ids, overlaps and exhausted capacity.
    pub fn add(&mut self, shape: Directed) -> Result<(), Error> {
        let error = if self.hashed.is_some()
            || !shape.centre.iter().all(|x| x.is_finite())
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
