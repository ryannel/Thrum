//! Wet ground: the two stores of each ground class, surface water and the pores' fill,
//! stepped with the tick for level uncovered ground under open sky and where a half and a
//! quarter of the rain reaches; and the wetness function, which reads them at a point.
//!
//! Each tick, for each store: rain arrives; the pores take it at the soak rate until they
//! are full, and from then only at the percolation rate, a named sink; what is left stands
//! as surface water up to what the relief holds, and the rest is runoff; then the drying
//! rate takes surface water, and once there is none the pores dry in proportion to their
//! fill, by `exp(-dt * drying / pore capacity)`.
use crate::{Rain, RainError};

/// Most ground classes a game's table holds.
pub const MAX_CLASSES: usize = 8;

/// The shares of the rain the stored reaches take: open sky, a half and a quarter. Ground
/// no rain reaches holds nothing.
pub const REACHES: [f64; 3] = [1.0, 0.5, 0.25];

/// A row of a game's table: what a kind of ground absorbs, holds and looks like wet.
/// Rates are kilograms of water a square metre a second (1 mm/h is 1/3600); depths are
/// metres of water.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundClass {
    /// Rate at which the pores take water until they are full. It may be under the
    /// percolation rate: such ground films while its pores fill and not after.
    pub soak: f64,
    /// Rate at which full pores pass water on down, a named sink.
    pub percolation: f64,
    /// Water the pores hold when full.
    pub pores: f64,
    /// Water the relief of level ground holds standing.
    pub relief: f64,
    /// Slope, rise over run, at which the relief holds nothing.
    pub steepest: f64,
    /// Share of the ground's light the film returns to it, `p`: 0.47 for stone, asphalt
    /// and concrete, up to 0.7 for sand and soil.
    pub share: f64,
    /// How far the ground darkens wet, 0 to 1; a material's own porosity takes its place.
    pub porosity: f64,
    /// Roughness of the water film.
    pub film_roughness: f64,
}

impl GroundClass {
    /// Ground with no pores and no relief, which keeps no state: it passes down up to
    /// `percolation` of the rain it is given and the rest runs off.
    pub fn bare(percolation: f64) -> Self {
        Self {
            soak: percolation,
            percolation,
            pores: 0.0,
            relief: 0.0,
            steepest: 1.0,
            share: 0.47,
            porosity: 0.0,
            film_roughness: 0.05,
        }
    }

    /// Whether the class keeps state: pores or relief to hold water.
    pub fn stores(&self) -> bool {
        self.pores > 0.0 || self.relief > 0.0
    }

    pub(crate) fn validate(&self) -> Result<(), RainError> {
        let all = [
            self.soak,
            self.percolation,
            self.pores,
            self.relief,
            self.steepest,
            self.share,
            self.porosity,
            self.film_roughness,
        ];
        if all.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(RainError::Input("negative or nonfinite ground class"));
        }
        if self.steepest <= 0.0 || self.share >= 1.0 || self.porosity > 1.0 {
            return Err(RainError::Input(
                "ground class slope, share or porosity out of range",
            ));
        }
        Ok(())
    }
}

/// One store: surface water standing in the relief, metres, and the pores' fill, 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Store {
    /// Surface water, metres.
    pub surface: f64,
    /// The pores' fill, 0 to 1.
    pub fill: f64,
}

impl Store {
    /// Metres of water held, for a class's pore capacity.
    pub fn water(&self, pores: f64) -> f64 {
        self.surface + self.fill * pores
    }
}

/// Where one tick's water on a square metre of a store went, metres. The rain equals the
/// change in the store plus the rest.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flow {
    /// Rain that reached the store.
    pub rain: f64,
    /// Passed down out of full pores.
    pub percolated: f64,
    /// Dried from the surface water and the pores.
    pub dried: f64,
    /// Neither held nor taken: runoff.
    pub runoff: f64,
}

impl Flow {
    fn scaled_add(&mut self, o: Self, w: f64) {
        self.rain += o.rain * w;
        self.percolated += o.percolated * w;
        self.dried += o.dried * w;
        self.runoff += o.runoff * w;
    }
}

/// Step one store over a tick. `rain` and `dry` are metres of water over the tick.
pub(crate) fn step_store(class: &GroundClass, s: &mut Store, rain: f64, dry: f64, dt: f64) -> Flow {
    let cap = class.pores;
    let soak = class.soak / 1000.0 * dt;
    let perc = class.percolation / 1000.0 * dt;
    let mut water = s.surface + rain;
    let mut flow = Flow {
        rain,
        ..Flow::default()
    };
    let room = if cap > 0.0 { (1.0 - s.fill) * cap } else { 0.0 };
    if room > 0.0 {
        // Filling: the pores take up to the soak rate; past full, what is left of the take
        // goes down, up to the percolation rate.
        let take = water.min(soak).min(room + perc);
        let into = take.min(room);
        s.fill = if into >= room {
            1.0
        } else {
            s.fill + into / cap
        };
        flow.percolated = take - into;
        water -= take;
    } else {
        let take = water.min(perc);
        flow.percolated = take;
        water -= take;
    }
    s.surface = water.min(class.relief);
    flow.runoff = water - s.surface;
    let d = s.surface.min(dry);
    s.surface -= d;
    flow.dried = d;
    if s.surface <= 0.0 && cap > 0.0 && dry > 0.0 && s.fill > 0.0 {
        let next = s.fill * (-dry / cap).exp();
        flow.dried += (s.fill - next) * cap;
        s.fill = next;
    }
    flow
}

/// Weights of the stored reaches for an exposure: the stores are read linearly between
/// open sky, a half, a quarter and nothing.
pub fn reach_weights(exposure: f64) -> [f64; 3] {
    let e = exposure.clamp(0.0, 1.0);
    if e >= 0.5 {
        let w = (e - 0.5) / 0.5;
        [w, 1.0 - w, 0.0]
    } else if e >= 0.25 {
        let w = (e - 0.25) / 0.25;
        [0.0, w, 1.0 - w]
    } else {
        [0.0, 0.0, e / 0.25]
    }
}

/// The ground's stores for a game's table of classes. Plain data the game keeps in its
/// world and steps once a tick, after the hollows bodies' fill steps of the tick.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundStores {
    pub(crate) classes: Vec<GroundClass>,
    pub(crate) stores: Vec<[Store; 3]>,
    pub(crate) tick: u64,
    /// Running totals for every class's stores, per square metre of each reach.
    pub(crate) totals: Vec<[Flow; 3]>,
}

impl GroundStores {
    /// Dry stores for a table of classes.
    /// # Errors
    /// Refuses more than [`MAX_CLASSES`] classes, none, or an invalid class.
    pub fn new(classes: &[GroundClass]) -> Result<Self, RainError> {
        if classes.is_empty() || classes.len() > MAX_CLASSES {
            return Err(RainError::Input(
                "a ground table holds one to eight classes",
            ));
        }
        for c in classes {
            c.validate()?;
        }
        Ok(Self {
            classes: classes.to_vec(),
            stores: vec![[Store::default(); 3]; classes.len()],
            tick: 0,
            totals: vec![[Flow::default(); 3]; classes.len()],
        })
    }

    /// The table's classes.
    pub fn classes(&self) -> &[GroundClass] {
        &self.classes
    }

    /// Ticks stepped.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// One class's store for a stored reach, an index into [`REACHES`].
    pub fn store(&self, class: usize, reach: usize) -> Store {
        self.stores[class][reach]
    }

    /// One class's flows since the stores began, per square metre of a stored reach.
    pub fn totals(&self, class: usize, reach: usize) -> Flow {
        self.totals[class][reach]
    }

    /// A class's stores read at an exposure, between the stored reaches and nothing.
    pub fn read(&self, class: usize, exposure: f64) -> Store {
        let w = reach_weights(exposure);
        let s = &self.stores[class];
        let mut out = Store::default();
        for r in 0..3 {
            out = Store {
                surface: out.surface + w[r] * s[r].surface,
                fill: out.fill + w[r] * s[r].fill,
            };
        }
        out
    }

    /// What one tick of `dt` seconds of this rain would do to each class's stores, per
    /// square metre of each stored reach, without changing them.
    /// # Errors
    /// Refuses a negative or nonfinite rain, drying or step.
    pub fn flows(&self, rain: Rain, dt: f64) -> Result<Vec<[Flow; 3]>, RainError> {
        check(rain, dt)?;
        Ok(self
            .classes
            .iter()
            .zip(&self.stores)
            .map(|(c, s)| {
                std::array::from_fn(|r| {
                    let mut s = s[r];
                    step_store(
                        c,
                        &mut s,
                        REACHES[r] * rain.flux / 1000.0 * dt,
                        rain.drying / 1000.0 * dt,
                        dt,
                    )
                })
            })
            .collect())
    }

    /// Step every class's stores one tick of `dt` seconds.
    /// # Errors
    /// Refuses a negative or nonfinite rain, drying or step, without changing state.
    pub fn step(&mut self, rain: Rain, dt: f64) -> Result<(), RainError> {
        check(rain, dt)?;
        for ((c, s), t) in self
            .classes
            .iter()
            .zip(&mut self.stores)
            .zip(&mut self.totals)
        {
            for r in 0..3 {
                let f = step_store(
                    c,
                    &mut s[r],
                    REACHES[r] * rain.flux / 1000.0 * dt,
                    rain.drying / 1000.0 * dt,
                    dt,
                );
                t[r].scaled_add(f, 1.0);
            }
        }
        self.tick += 1;
        Ok(())
    }
}

fn check(rain: Rain, dt: f64) -> Result<(), RainError> {
    if [rain.flux, rain.drying, dt]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err(RainError::Input(
            "negative or nonfinite rain, drying or step",
        ));
    }
    Ok(())
}

/// What a surface is at a point, as the wetness function reads it: prepared fields only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WetPoint {
    /// Exposure at the point: the share of the rain that reaches it.
    pub exposure: f64,
    /// The surface's unit normal, x/y/z with y up.
    pub normal: [f64; 3],
    /// Where the class has a tiling pattern of relief: its height here, 0 at its lows and
    /// 1 at its highs. None where it has none.
    pub pattern: Option<f64>,
}

/// The ground's wetness at a point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wetness {
    /// The pores' fill, 0 to 1.
    pub pores: f64,
    /// Surface water standing at the point, metres.
    pub surface: f64,
    /// Share of the point the surface water covers, 0 to 1.
    pub cover: f64,
}

/// The share of the sky a surface sees, from its normal: all for level ground, a quarter
/// for a wall, none for an underside. A declared closure.
pub fn sky_share(normal: [f64; 3]) -> f64 {
    let up = normal[1].clamp(-1.0, 1.0);
    if up >= 0.0 {
        0.25 + 0.75 * up
    } else {
        0.25 * (1.0 + up)
    }
}

/// The share of a class's relief that holds water on a surface of this normal: all on
/// level ground, falling with the slope to none at the class's steepest.
pub fn relief_share(class: &GroundClass, normal: [f64; 3]) -> f64 {
    let up = normal[1];
    if up <= 0.0 {
        return 0.0;
    }
    let across = normal[0].hypot(normal[2]);
    (1.0 - across / up / class.steepest).max(0.0)
}

/// The wetness function: the pores' fill and the surface water with its cover at a
/// point, from the class's stores read at the rain that reaches the surface. The same in
/// f64 here and in WGSL; it reads no camera and keeps nothing.
pub fn wetness(stores: &GroundStores, class: usize, at: WetPoint) -> Wetness {
    let c = &stores.classes[class];
    let reach = at.exposure.clamp(0.0, 1.0) * sky_share(at.normal);
    let s = stores.read(class, reach);
    let hold = c.relief * relief_share(c, at.normal);
    // The exposed, lower-capacity slope runs out before the level store.
    let surface = (s.surface - (c.relief - hold)).max(0.0);
    let share = if c.relief > 0.0 {
        (surface / c.relief).min(1.0)
    } else {
        0.0
    };
    let cover = if hold <= 0.0 {
        0.0
    } else {
        match at.pattern {
            Some(h) => f64::from(u8::from(share > h)),
            None => share,
        }
    };
    Wetness {
        pores: s.fill,
        surface,
        cover,
    }
}

/// The wetness function at a world point (x, y, z): exposure read from the canopy grid,
/// the other fields as [`wetness`] takes them.
pub fn wetness_at(
    stores: &GroundStores,
    class: usize,
    canopy: &crate::Canopy,
    point: [f64; 3],
    normal: [f64; 3],
    pattern: Option<f64>,
) -> Wetness {
    wetness(
        stores,
        class,
        WetPoint {
            exposure: canopy.exposure(point),
            normal,
            pattern,
        },
    )
}

/// The pores' fill a damp band gives a point `height` above the water it bounds: full at
/// the waterline, falling to nothing over the band's height `band`.
pub fn band_fill(height: f64, band: f64) -> f64 {
    if height <= 0.0 {
        1.0
    } else if band <= 0.0 {
        0.0
    } else {
        (1.0 - height / band).max(0.0)
    }
}

/// One tick of a damp band's height above a waterline: a fall of the level adds to it, a
/// rise takes from it down to nothing, and it then shrinks by
/// `exp(-dt * drying / pore capacity)` and is held under `rim`, the height from the
/// waterline to the top of what bounds the water. Levels in metres, drying in kilograms
/// a square metre a second.
pub fn band_step(
    band: f64,
    level_before: f64,
    level_after: f64,
    dt: f64,
    drying: f64,
    pores: f64,
    rim: f64,
) -> f64 {
    let m = (band + level_before - level_after).max(0.0);
    let rate = drying / 1000.0 * dt;
    let factor = if rate <= 0.0 {
        1.0
    } else if pores <= 0.0 {
        0.0
    } else {
        (-rate / pores).exp()
    };
    (m * factor).min(rim.max(0.0))
}

impl Wetness {
    /// The wetness with a damp band's fill: the pores take the wetter of the two.
    #[must_use]
    pub fn with_band(self, fill: f64) -> Self {
        Self {
            pores: self.pores.max(fill.clamp(0.0, 1.0)),
            ..self
        }
    }
}

/// The pores' fill a shore gives a point: full inside a body's wave support, the support
/// `thrum-water-model` gives and `WATERS`' film reads, and falling off above it over that
/// body's damp band, `bands[body]`; none off every body's margin.
pub fn shore_fill(
    map: &thrum_water_model::Map,
    field: &thrum_water_model::Field,
    point: [f64; 2],
    bands: &[f64],
) -> f64 {
    match map.wave_support(field, point) {
        None => 0.0,
        Some(s) if s.inside() => 1.0,
        Some(s) => band_fill(s.above(), bands.get(s.body).copied().unwrap_or(0.0)),
    }
}
