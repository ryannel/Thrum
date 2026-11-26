use crate::{Directed, Error, Parameters, Shape, ShapeSource, State, smooth, unit};
use std::f64::consts::PI;
use std::sync::Arc;

/// Game-supplied row of the type axis. No engine presets exist.
#[derive(Clone, Copy, Debug)]
pub struct Type {
    /// Shared condensation height, metres.
    pub base: f64,
    /// Minimum and maximum cell depth, metres.
    pub depth: [f64; 2],
    /// Liquid water kg/m³.
    pub liquid_water: f64,
    /// Droplet radius, metres.
    pub droplet_radius: f64,
    /// Share of the evolving deck, 0..=1.
    pub deck: f64,
    /// Bottom and top smooth-fade fractions of cloud depth; positive, sum ≤ 1.
    pub profile: [f64; 2],
}

impl Type {
    /// Extinction from liquid water and droplet radius, m⁻¹.
    pub fn extinction(self) -> f64 {
        3.0 * self.liquid_water / (2.0 * 1000.0 * self.droplet_radius)
    }
}

/// Structural constants; a control change never moves the lattice.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Horizontal lattice pitch, metres.
    pub pitch: f64,
    /// Pitch of the finer air-frame turret lattice, metres.
    pub turret_pitch: f64,
    /// A turret's thickness share at which its column starts to appear.
    pub turret_least: f64,
    /// Thickness share over which coverage and thickness fade in together.
    pub turret_fade: f64,
    /// Generation period, weather seconds.
    pub period: f64,
    /// Detail erosion at its mean, 0..=1.
    pub detail_mean: f64,
    /// Shape threshold and edge-detail strength, 0..=1; zero disables erosion.
    pub erosion: f64,
}

/// Stable hash-named cell, available to conformance and birth analysis.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    /// Site and generation identity.
    pub identity: [i64; 3],
    /// Birth weather second.
    pub birth: f64,
    /// Lifetime in seconds.
    pub life: f64,
    /// Centre in the air frame.
    pub centre: [f64; 2],
    /// Coverage threshold, uniform in 0..1.
    pub threshold: f64,
    radius: f64,
    depth: f64,
}

/// Pointwise uneroded column. Clear air retains the weather's type and base,
/// with top equal to base and zero coverage, age and optical depth.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Column {
    /// Dimensional coverage/profile.
    pub coverage: f64,
    /// Interpolated type axis.
    pub kind: f64,
    /// Bottom height, metres.
    pub base: f64,
    /// Top height, metres.
    pub top: f64,
    /// Dominant cell's fractional life.
    pub age: f64,
    /// Bound on vertical optical depth, dimensionless.
    pub optical_depth: f64,
    /// Game's bottom/top vertical fade fractions.
    pub profile: [f64; 2],
}

/// Evaluation fidelity; all variants use the same uneroded column.
#[derive(Clone, Copy, Debug)]
pub enum Density {
    /// No shape or detail erosion, for the query's exact bound.
    Uneroded,
    /// Shape erosion and detail at its mean.
    Coarse,
    /// Shape erosion and spatial detail.
    Full,
}

/// CPU field authority; construction validates all profile/type data once.
pub struct Field {
    pub(super) seed: u32,
    profile: Profile,
    types: Vec<Type>,
}

impl Field {
    /// Create a stateless field with at least two type rows.
    /// # Errors
    /// Rejects invalid dimensions, water content, radii or shares.
    pub fn new(seed: u32, profile: Profile, types: Vec<Type>) -> Result<Self, Error> {
        if !profile.pitch.is_finite()
            || profile.pitch <= 0.0
            || !profile.turret_pitch.is_finite()
            || profile.turret_pitch <= 0.0
            || !unit(profile.turret_least)
            || !unit(profile.turret_fade)
            || profile.turret_fade == 0.0
            || profile.turret_least + profile.turret_fade > 1.0
            || !profile.period.is_finite()
            || profile.period <= 0.0
            || !unit(profile.detail_mean)
            || !unit(profile.erosion)
            || types.len() < 2
            || types.iter().any(|t| {
                !t.base.is_finite()
                    || !t.depth.iter().all(|v| v.is_finite() && *v > 0.0)
                    || t.depth[0] > t.depth[1]
                    || !t.liquid_water.is_finite()
                    || t.liquid_water < 0.0
                    || !t.droplet_radius.is_finite()
                    || t.droplet_radius <= 0.0
                    || !unit(t.deck)
                    || !t.profile.iter().all(|&v| v.is_finite() && v > 0.0)
                    || t.profile.iter().sum::<f64>() > 1.0
            })
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            seed,
            profile,
            types,
        })
    }

    /// Derive one site's generation independently of query order.
    pub fn cell(&self, i: i64, j: i64, g: i64) -> Cell {
        let h = |stream| self.random(i, j, g, stream);
        Cell {
            identity: [i, j, g],
            birth: (g as f64 + self.random(i, j, 0, 1) + h(2)) * self.profile.period,
            life: self.profile.period * (0.4 + 0.6 * h(3)),
            centre: [
                (i as f64 + h(5)) * self.profile.pitch,
                (j as f64 + h(6)) * self.profile.pitch,
            ],
            threshold: h(4),
            radius: self.profile.pitch * (0.3 + 0.5 * h(7).powi(2)),
            // The life envelope already biases young and old cells toward
            // shallow profiles. Sample the authored mature depth range evenly.
            depth: h(8),
        }
    }

    /// Query world X/Y/Z at a committed state. Height does not alter a column.
    pub fn query(&self, point: [f64; 3], state: &State) -> Column {
        let air = state.to_air([point[0], point[2]]);
        let mut column = self.procedural_column(
            air,
            state.time(),
            state.parameters(),
            state.procedural_share(),
        );
        let hashed = self.hashed_at(air, state);
        for (shape, selection) in state
            .shapes()
            .iter()
            .map(|shape| (shape, state.shape_source(shape.id)))
            .chain(hashed.iter().map(|(shape, source)| (shape, Some(*source))))
        {
            let c = self.placed_column(air, shape, selection, state);
            if c.optical_depth > 0.0 {
                if column.optical_depth == 0.0 {
                    column = c;
                } else {
                    column.optical_depth += c.optical_depth;
                    column.base = column.base.min(c.base);
                    column.top = column.top.max(c.top);
                    column.coverage = column.coverage.max(c.coverage);
                }
            }
        }
        column
    }

    /// Evaluate procedural columns at an air-frame point and weather time.
    pub fn column(&self, air: [f64; 2], time: f64, parameters: Parameters) -> Column {
        self.procedural_column(air, time, parameters, 1.0)
    }

    /// Evaluate the procedural column for a weather-map row with its committed heaped-cloud share.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "lattice integers are floor coordinates"
    )]
    pub fn procedural_column(
        &self,
        air: [f64; 2],
        time: f64,
        parameters: Parameters,
        share: f64,
    ) -> Column {
        let t = self.kind(parameters.kind);
        let clear = Column {
            kind: parameters.kind,
            base: t.base,
            top: t.base,
            profile: t.profile,
            ..Column::default()
        };
        // Zero coverage multiplies every cell and the deck down to zero.
        // Placed shapes are evaluated separately from this procedural column.
        if parameters.coverage == 0.0 {
            return clear;
        }
        let site = air.map(|p| (p / self.profile.pitch).floor() as i64);
        let turret = self.turret(air);
        let mut coverage: f64 = 0.0;
        let mut depth: f64 = 0.0;
        let mut age = 0.0;
        for di in -1..=1 {
            for dj in -1..=1 {
                let i = site[0].wrapping_add(di);
                let j = site[1].wrapping_add(dj);
                let generation =
                    (time / self.profile.period - self.random(i, j, 0, 1)).floor() as i64;
                for g in [generation.wrapping_sub(1), generation] {
                    let cell = self.cell(i, j, g);
                    let phase = (time - cell.birth) / cell.life;
                    let envelope = if (0.0..1.0).contains(&phase) {
                        (PI * phase).sin().powi(2)
                    } else {
                        0.0
                    };
                    let envelope = envelope
                        * smooth(
                            (0.95 * smooth(parameters.coverage / 0.6) - cell.threshold) / 0.15,
                        )
                        * share;
                    let radius = cell.radius * (0.5 + 0.5 * envelope);
                    let r2 = ((air[0] - cell.centre[0]).powi(2)
                        + (air[1] - cell.centre[1]).powi(2))
                        / radius.powi(2);
                    let dome = envelope * (1.0 - r2).max(0.0) * (1.0 - t.deck);
                    let share = dome.powf(0.7).min(envelope * turret);
                    let clearing =
                        smooth((share - self.profile.turret_least) / self.profile.turret_fade);
                    let d = dome * clearing;
                    if d > coverage {
                        age = phase;
                    }
                    coverage = coverage.max(d);
                    depth = depth.max(
                        (t.depth[0] + (t.depth[1] - t.depth[0]) * cell.depth) * share * clearing,
                    );
                }
            }
        }
        let n = self.noise(
            [
                air[0] / (4.0 * self.profile.pitch),
                air[1] / (4.0 * self.profile.pitch),
                time / (2.0 * self.profile.period),
            ],
            20,
        );
        let filling = smooth((parameters.coverage - 0.45) / 0.55);
        let gaps = 6.2 * t.deck.powi(3) * filling * (1.0 - filling);
        let opening = if gaps > 0.0 {
            // Separate neighbourhoods within a broken deck open and close as
            // their own part of the field evolves. The closed sheet keeps the
            // broad thickness gradient and its full optical depth.
            n + gaps
                * (self.noise(
                    [
                        air[0] / self.profile.pitch,
                        air[1] / self.profile.pitch,
                        time / self.profile.period,
                    ],
                    21,
                ) - 0.5)
        } else {
            n
        };
        let deck = t.deck * smooth((opening - (1.0 - 1.3 * filling)) / 0.2);
        coverage = coverage.max(deck);
        depth = depth.max(t.depth[0] * deck.powf(0.7) * (0.6 + 0.4 * n));
        if coverage == 0.0 || depth == 0.0 {
            return clear;
        }
        Column {
            coverage,
            kind: parameters.kind,
            base: t.base,
            top: t.base + depth,
            age,
            optical_depth: t.extinction()
                * depth
                * coverage
                * (1.0 - t.profile[0].midpoint(t.profile[1])),
            profile: t.profile,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "lattice indices are floor coordinates"
    )]
    fn turret(&self, air: [f64; 2]) -> f64 {
        let position = air.map(|x| x / self.profile.turret_pitch);
        let site = position.map(|x| x.floor() as i64);
        let local = position.map(|x| x - x.floor());
        let mut cap: f64 = 0.0;
        for x in -1..=1 {
            for z in -1..=1 {
                let i = site[0].wrapping_add(x);
                let j = site[1].wrapping_add(z);
                let random = |stream| self.random(i, j, 7, stream);
                let rank = random(73).powf(1.6);
                // A cap fits within its site's one-ring neighbourhood.
                let radius = ((0.45 + 0.45 * rank) * (0.85 + 0.3 * random(72))).min(1.0);
                let dx = local[0] - x as f64 - random(70);
                let dz = local[1] - z as f64 - random(71);
                let r2 = (dx * dx + dz * dz) / (radius * radius);
                cap = cap.max((0.3 + 0.7 * rank) * (1.0 - r2).max(0.0).sqrt());
            }
        }
        cap
    }

    /// Width of one placement block in the air frame, where the hash places the library.
    pub fn placed_block_width(&self, state: &State) -> Option<f64> {
        Some(f64::from(state.hashed_shapes()?.block_sites) * self.profile.pitch)
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "shape blocks use integer floor coordinates"
    )]
    fn hashed_at(&self, air: [f64; 2], state: &State) -> Option<(Directed, ShapeSource)> {
        let width = self.placed_block_width(state)?;
        self.hashed_shape(air.map(|axis| (axis / width).floor() as i64), state)
    }

    /// Resolve a hash-selected placed cell by its air-frame block, independent of presentation.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded hash choices and floor coordinates"
    )]
    pub fn hashed_shape(&self, block: [i64; 2], state: &State) -> Option<(Directed, ShapeSource)> {
        let placement = state.hashed_shapes()?;
        let library = state.library()?;
        let p = state.parameters();
        let period = self.profile.period;
        let [i, j] = block;
        // Stagger the lives of neighbouring blocks. A newly opened sky already
        // has mature cells, and a generation boundary cannot clear the horizon.
        let phase_offset = self.random(i, j, 0, 88);
        let cycle = state.time() / period - phase_offset;
        let generation = cycle.floor() as i64;
        let random = |stream| self.random(i, j, generation, stream);
        let share = placement.share(p.kind);
        let selected = smooth((share - random(80)) / 0.1);
        let covered = smooth((0.95 * smooth(p.coverage / 0.6) - random(81)) / 0.15);
        let birth = 0.12 * random(82) * period;
        let life = (0.55 + 0.25 * random(83)) * period;
        let age = ((cycle - generation as f64) * period - birth) / life;
        if selected == 0.0 || covered == 0.0 || !(0.0..1.0).contains(&age) {
            return None;
        }
        let envelope = (PI * age).sin().powi(2) * selected * covered;
        let count = library.shapes().len();
        let shape_count = count + usize::from(share < 1.0 && !library.growth().is_empty());
        let index = (random(84) * shape_count as f64) as usize;
        let source = if index == count {
            ShapeSource::Growth
        } else {
            ShapeSource::Library(u32::try_from(index).ok()?)
        };
        let slab = library.geometry(source)?;
        let width = self.placed_block_width(state)?;
        let centre = [
            (i as f64 + 0.3 + 0.4 * random(85)) * width,
            (j as f64 + 0.3 + 0.4 * random(86)) * width,
        ];
        let scale = (width * 0.29 / slab.footprint.radius()).min(0.88 + 0.28 * random(89));
        let wind_heading = p.wind[1].atan2(p.wind[0]);
        let shape = Directed {
            id: random(87).to_bits(),
            centre,
            yaw: wind_heading - slab.heading,
            scale,
            footprint: Arc::clone(&slab.footprint),
            envelope,
            target: envelope,
            growth: age,
            growth_target: age,
        };
        Some((shape, source))
    }

    /// One density function shared by all consumers. `volume` resolves the voxel
    /// slab for a directed shape; absent slabs use the bounded column form.
    /// `weight` blends volume and column forms of the same condensate once.
    pub fn density<'a>(
        &self,
        point: [f64; 3],
        state: &State,
        fidelity: Density,
        volume: impl Fn(u64) -> Option<&'a Shape>,
        weight: f64,
    ) -> f64 {
        self.density_with_footprint(point, state, fidelity, volume, weight, false)
            .0
    }

    // Visibility knows a missing volume only by its integral and outline.
    // Preserve that column sum without changing the older Field density API.
    pub(crate) fn density_with_footprint<'a>(
        &self,
        point: [f64; 3],
        state: &State,
        fidelity: Density,
        volume: impl Fn(u64) -> Option<&'a Shape>,
        weight: f64,
        preserve_footprint: bool,
    ) -> (f64, bool) {
        let air = state.to_air([point[0], point[2]]);
        let p = state.parameters();
        let kind = self.kind(p.kind);
        let extinction = kind.extinction();
        let point_air = [air[0], point[1], air[1]];
        let noise = std::cell::LazyCell::new(|| {
            let shape = self.shape_noise(point_air);
            let detail = match fidelity {
                Density::Full => self.detail_noise(point_air),
                _ => self.profile.detail_mean,
            };
            (shape, detail)
        });
        let erode = |sigma, extinction, placed| {
            if sigma <= 0.0
                || extinction <= 0.0
                || self.profile.erosion == 0.0
                || matches!(fidelity, Density::Uneroded)
            {
                sigma
            } else {
                self.erode(sigma, extinction, noise.0, noise.1, placed)
            }
        };
        let column = self.procedural_column(air, state.time(), p, state.procedural_share());
        let mut sigma = erode(column_density(column, point[1]), extinction, false);
        if sigma > 0.0 && self.profile.erosion > 0.0 && !matches!(fidelity, Density::Uneroded) {
            let height = (point[1] - column.base) / (column.top - column.base);
            sigma *= self.bank_shape(point_air, height, kind.deck);
        }
        let mut used_footprint = false;
        let hashed = self.hashed_at(air, state);
        for (shape, selection) in state
            .shapes()
            .iter()
            .map(|shape| (shape, state.shape_source(shape.id)))
            .chain(hashed.iter().map(|(shape, source)| (shape, Some(*source))))
        {
            let column = self.placed_column(air, shape, selection, state);
            let column_sigma = column_density(column, point[1]);
            let voxels = volume(shape.id);
            used_footprint |= voxels.is_none()
                && column.optical_depth > 0.0
                && point[1] >= column.base
                && point[1] <= column.top;
            if preserve_footprint && voxels.is_none() {
                sigma += column_sigma;
                continue;
            }
            let volume_sigma = voxels.map_or(column_sigma, |v| {
                let local = local(air, shape);
                let floor = selection
                    .and_then(|source| state.library()?.geometry(source))
                    .map_or(0.0, |slab| slab.floor);
                v.density([
                    local[0],
                    (point[1] - kind.base) / shape.scale - floor,
                    local[1],
                ]) * shape.envelope
                    * extinction
            });
            let reference_extinction = extinction * selection.map_or(1.0, |_| shape.envelope);
            if selection.is_some() {
                let sigma_blended = volume_sigma * weight.clamp(0.0, 1.0)
                    + column_sigma * (1.0 - weight.clamp(0.0, 1.0));
                sigma += erode(sigma_blended, reference_extinction, true);
            } else {
                sigma += erode(volume_sigma, reference_extinction, false) * weight.clamp(0.0, 1.0)
                    + erode(column_sigma, reference_extinction, false)
                        * (1.0 - weight.clamp(0.0, 1.0));
            }
        }
        (sigma, used_footprint)
    }

    fn erode(&self, sigma: f64, extinction: f64, shape: f64, detail: f64, placed: bool) -> f64 {
        let signal = (sigma / extinction).min(1.0);
        let threshold = self.profile.erosion * (1.0 - shape);
        if placed {
            let edge = smooth((signal - 0.45 * threshold) / 0.28);
            return sigma * edge * (1.0 - 0.20 * self.profile.erosion * detail * (1.0 - edge));
        }
        let edge = (1.0 - threshold).clamp(0.0001, 0.12);
        let shaped = ((signal - threshold) / edge).clamp(0.0, 1.0);
        let eroded = (shaped - self.profile.erosion * detail * (1.0 - shaped)).max(0.0);
        sigma * eroded
    }

    /// Midpoint f64 path reference, for any direction. The caller states a finite
    /// segment and sample count; direction must be a unit vector.
    pub fn optical_depth<'a>(
        &self,
        sample: &Sample,
        state: &State,
        fidelity: Density,
        volume: impl Fn(u64) -> Option<&'a Shape>,
        weight: f64,
    ) -> f64 {
        let step = sample.length / f64::from(sample.steps);
        (0..sample.steps)
            .map(|i| {
                let distance = (f64::from(i) + 0.5) * step;
                self.density(
                    std::array::from_fn(|j| sample.origin[j] + sample.direction[j] * distance),
                    state,
                    fidelity,
                    &volume,
                    weight,
                ) * step
            })
            .sum()
    }

    fn placed_column(
        &self,
        air: [f64; 2],
        shape: &Directed,
        source: Option<ShapeSource>,
        state: &State,
    ) -> Column {
        let p = state.parameters();
        let point = local(air, shape);
        let c = match (state.library(), source) {
            (Some(library), Some(source)) => library.column(source, shape.growth, point),
            _ => shape.footprint.column(point),
        };
        if c[0] == 0.0 || shape.envelope == 0.0 {
            return Column::default();
        }
        let t = self.kind(p.kind);
        Column {
            coverage: shape.envelope,
            kind: p.kind,
            base: t.base + c[1] * shape.scale,
            top: t.base + c[2] * shape.scale,
            age: shape.growth,
            optical_depth: c[0] * shape.scale * shape.envelope * t.extinction(),
            profile: t.profile,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped type table coordinate"
    )]
    pub(crate) fn kind(&self, kind: f64) -> Type {
        let position = kind.clamp(0.0, 1.0) * (self.types.len() - 1) as f64;
        let i = (position.floor() as usize).min(self.types.len() - 2);
        let a = self.types[i];
        let b = self.types[i + 1];
        let f = position - i as f64;
        let mix = |a, b| a + (b - a) * f;
        Type {
            base: mix(a.base, b.base),
            depth: std::array::from_fn(|j| mix(a.depth[j], b.depth[j])),
            liquid_water: mix(a.liquid_water, b.liquid_water),
            droplet_radius: mix(a.droplet_radius, b.droplet_radius),
            deck: mix(a.deck, b.deck),
            profile: std::array::from_fn(|j| mix(a.profile[j], b.profile[j])),
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "hash deliberately wraps integer coordinates to 32 bits"
    )]
    pub(crate) fn random(&self, i: i64, j: i64, g: i64, stream: u32) -> f64 {
        let mut x = (i as u32)
            .wrapping_mul(0x9e37_79b1)
            .wrapping_add((j as u32).wrapping_mul(0x85eb_ca77))
            .wrapping_add((g as u32).wrapping_mul(0xc2b2_ae3d))
            .wrapping_add(
                stream
                    .wrapping_add(131_u32.wrapping_mul(self.seed))
                    .wrapping_mul(0x27d4_eb2f),
            );
        x ^= x >> 15;
        x = x.wrapping_mul(0x2c1b_3c6d);
        x ^= x >> 12;
        x = x.wrapping_mul(0x297a_2d39);
        x ^= x >> 15;
        f64::from(x) / 4_294_967_296.0
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "noise lattice uses integer floor coordinates"
    )]
    pub(crate) fn noise(&self, point: [f64; 3], stream: u32) -> f64 {
        let index = point.map(|p| p.floor() as i64);
        let f = point.map(|p| smooth(p - p.floor()));
        let mut sum = 0.0;
        for x in 0..=1 {
            for y in 0..=1 {
                for z in 0..=1 {
                    let w = |axis: usize, offset| if offset == 0 { 1.0 - f[axis] } else { f[axis] };
                    sum += w(0, x)
                        * w(1, y)
                        * w(2, z)
                        * self.random(
                            index[0].wrapping_add(x),
                            index[1].wrapping_add(y),
                            index[2].wrapping_add(z),
                            stream,
                        );
                }
            }
        }
        sum
    }
}

pub(crate) fn local(air: [f64; 2], shape: &Directed) -> [f64; 2] {
    let x = (air[0] - shape.centre[0]) / shape.scale;
    let z = (air[1] - shape.centre[1]) / shape.scale;
    let (s, c) = shape.yaw.sin_cos();
    [c * x + s * z, -s * x + c * z]
}

fn column_density(column: Column, y: f64) -> f64 {
    if column.optical_depth == 0.0 || y <= column.base || y >= column.top {
        return 0.0;
    }
    let h = (y - column.base) / (column.top - column.base);
    let [rise, fall] = column.profile;
    let profile = smooth(h / rise) * (1.0 - smooth((h - (1.0 - fall)) / fall));
    column.optical_depth / (column.top - column.base) * profile / (1.0 - rise.midpoint(fall))
}

/// A stated path and tick for conformance comparison.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    /// Committed tick identity supplied to the provider.
    pub tick: u64,
    /// World origin, metres.
    pub origin: [f64; 3],
    /// Unit direction.
    pub direction: [f64; 3],
    /// Path extent, metres.
    pub length: f64,
    /// Midpoint samples.
    pub steps: u32,
}

/// Maximum absolute provider error, with the sample index that exhibits it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Conformance {
    /// Largest absolute difference.
    pub maximum: f64,
    /// Index of its witness.
    pub index: usize,
}

impl Conformance {
    /// Compare any column or Sun-depth provider with stated reference samples.
    /// # Errors
    /// Rejects nonfinite reference/provider results rather than hiding them.
    pub fn compare(
        samples: &[Sample],
        reference: impl Fn(&Sample) -> f64,
        provider: impl Fn(&Sample) -> f64,
    ) -> Result<Self, Error> {
        let mut result = Self::default();
        for (index, sample) in samples.iter().enumerate() {
            let a = reference(sample);
            let b = provider(sample);
            if !a.is_finite() || !b.is_finite() {
                return Err(Error::Invalid);
            }
            let difference = (a - b).abs();
            if difference > result.maximum {
                result = Self {
                    maximum: difference,
                    index,
                };
            }
        }
        Ok(result)
    }
}
