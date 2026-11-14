use crate::{Directed, Error, Parameters, Shape, State, smooth, unit};
use std::f64::consts::PI;

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
    /// Generation period, weather seconds.
    pub period: f64,
    /// Detail erosion at its mean, 0..=1.
    pub detail_mean: f64,
    /// Maximum fractional shape/detail erosion, 0..=1; zero disables erosion.
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

/// Pointwise uneroded column; all fields are exactly zero in clear air.
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
    seed: u32,
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
            depth: h(8).powi(2),
        }
    }

    /// Query world X/Y/Z at a committed state. Height does not alter a column.
    pub fn query(&self, point: [f64; 3], state: &State) -> Column {
        let air = state.to_air([point[0], point[2]]);
        let mut column = self.column(air, state.time(), state.parameters());
        for shape in state.shapes() {
            let c = self.placed_column(air, shape, state.parameters());
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
    #[expect(
        clippy::cast_possible_truncation,
        reason = "lattice integers are floor coordinates"
    )]
    pub fn column(&self, air: [f64; 2], time: f64, parameters: Parameters) -> Column {
        let t = self.kind(parameters.kind);
        let site = air.map(|p| (p / self.profile.pitch).floor() as i64);
        let mut coverage: f64 = 0.0;
        let mut depth: f64 = 0.0;
        let mut age = 0.0;
        for di in -1..=1 {
            for dj in -1..=1 {
                let i = site[0] + di;
                let j = site[1] + dj;
                let generation =
                    (time / self.profile.period - self.random(i, j, 0, 1)).floor() as i64;
                for g in [generation - 1, generation] {
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
                        );
                    let radius = cell.radius * (0.5 + 0.5 * envelope);
                    let r2 = ((air[0] - cell.centre[0]).powi(2)
                        + (air[1] - cell.centre[1]).powi(2))
                        / radius.powi(2);
                    let d = envelope * (1.0 - r2).max(0.0) * (1.0 - t.deck);
                    if d > coverage {
                        age = phase;
                    }
                    coverage = coverage.max(d);
                    depth = depth
                        .max((t.depth[0] + (t.depth[1] - t.depth[0]) * cell.depth) * d.powf(0.7));
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
        let deck =
            t.deck * smooth((n - (1.0 - 1.3 * smooth((parameters.coverage - 0.45) / 0.55))) / 0.2);
        coverage = coverage.max(deck);
        depth = depth.max(t.depth[0] * deck.powf(0.7) * (0.6 + 0.4 * n));
        if coverage == 0.0 || depth == 0.0 {
            return Column::default();
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
        let air = state.to_air([point[0], point[2]]);
        let p = state.parameters();
        let column = self.column(air, state.time(), p);
        let mut sigma = column_density(column, point[1]);
        for shape in state.shapes() {
            let column = self.placed_column(air, shape, p);
            let column_sigma = column_density(column, point[1]);
            let local = local(air, shape);
            let volume_sigma = volume(shape.id).map_or(column_sigma, |v| {
                v.density([
                    local[0],
                    (point[1] - self.kind(p.kind).base) / shape.scale,
                    local[1],
                ]) * shape.envelope
                    * self.kind(p.kind).extinction()
            });
            sigma += volume_sigma * weight.clamp(0.0, 1.0)
                + column_sigma * (1.0 - weight.clamp(0.0, 1.0));
        }
        let shape_noise = self.noise([air[0] / 200.0, point[1] / 200.0, air[1] / 200.0], 40);
        let detail = match fidelity {
            Density::Uneroded => return sigma,
            Density::Coarse => self.profile.detail_mean,
            Density::Full => self.noise([air[0] / 25.0, point[1] / 25.0, air[1] / 25.0], 41),
        };
        sigma * (1.0 - self.profile.erosion * shape_noise) * (1.0 - self.profile.erosion * detail)
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

    fn placed_column(&self, air: [f64; 2], shape: &Directed, p: Parameters) -> Column {
        let c = shape.footprint.column(local(air, shape));
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
    fn kind(&self, kind: f64) -> Type {
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
    fn random(&self, i: i64, j: i64, g: i64, stream: u32) -> f64 {
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
    fn noise(&self, point: [f64; 3], stream: u32) -> f64 {
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
                        * self.random(index[0] + x, index[1] + y, index[2] + z, stream);
                }
            }
        }
        sum
    }
}

fn local(air: [f64; 2], shape: &Directed) -> [f64; 2] {
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
