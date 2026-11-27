use crate::{Density, Error, Field, State, smooth, unit};

/// One game-supplied surface node. Water below ground is dry land.
#[derive(Clone, Copy, Debug)]
pub struct SurfacePoint {
    /// Terrain height, metres.
    pub ground: f64,
    /// Optional water level, metres.
    pub water: Option<f64>,
}

/// Validated world-anchored surface grid, with a box-averaged datum.
/// Outside the grid the nearest boundary value is extended. Preparation uses
/// summed areas for the datum and a two-pass chamfer distance for the shore.
pub struct Surface {
    origin: [f64; 2],
    size: [usize; 2],
    spacing: f64,
    heights: Vec<f64>,
    datum: Vec<f64>,
    shore: Vec<f64>,
}

impl Surface {
    /// Prepare a surface; `window` is the full width of the datum box in metres.
    /// Nodes are X-major, then Z. Shore distances use the eight-neighbour metric.
    /// # Errors
    /// Rejects invalid dimensions, count, heights, spacing or window.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "window bounded to grid dimensions before integer conversion"
    )]
    pub fn new(
        origin: [f64; 2],
        size: [usize; 2],
        spacing: f64,
        window: f64,
        nodes: &[SurfacePoint],
    ) -> Result<Self, Error> {
        if size.contains(&0)
            || size[0].checked_mul(size[1]) != Some(nodes.len())
            || !origin.iter().all(|x| x.is_finite())
            || !spacing.is_finite()
            || spacing <= 0.0
            || !(spacing * (size[0] as f64 + size[1] as f64)).is_finite()
            || !window.is_finite()
            || window < 0.0
            || nodes
                .iter()
                .any(|p| !p.ground.is_finite() || p.water.is_some_and(|w| !w.is_finite()))
        {
            return Err(Error::Invalid);
        }
        let heights: Vec<_> = nodes
            .iter()
            .map(|p| p.water.map_or(p.ground, |w| w.max(p.ground)))
            .collect();
        let wet: Vec<_> = nodes
            .iter()
            .map(|p| p.water.is_some_and(|w| w >= p.ground))
            .collect();
        let stride = size[0] + 1;
        let mut prefix = vec![0.0; stride * (size[1] + 1)];
        for z in 0..size[1] {
            let mut row = 0.0;
            for x in 0..size[0] {
                row += heights[z * size[0] + x];
                prefix[(z + 1) * stride + x + 1] = prefix[z * stride + x + 1] + row;
            }
        }
        let radius = (window / (2.0 * spacing))
            .floor()
            .min(size[0].max(size[1]) as f64) as usize;
        let mut datum = vec![0.0; nodes.len()];
        for z in 0..size[1] {
            for x in 0..size[0] {
                let (a, b) = (x.saturating_sub(radius), (x + radius + 1).min(size[0]));
                let (c, d) = (z.saturating_sub(radius), (z + radius + 1).min(size[1]));
                datum[z * size[0] + x] =
                    (prefix[d * stride + b] - prefix[d * stride + a] - prefix[c * stride + b]
                        + prefix[c * stride + a])
                        / ((b - a) * (d - c)) as f64;
            }
        }
        let to_water = distances(&wet, true, size, spacing);
        let to_land = distances(&wet, false, size, spacing);
        let shore = wet
            .iter()
            .enumerate()
            .map(|(i, &w)| if w { to_land[i] } else { -to_water[i] })
            .collect();
        Ok(Self {
            origin,
            size,
            spacing,
            heights,
            datum,
            shore,
        })
    }

    /// Origin, dimensions and node spacing of the world-anchored grid.
    pub fn grid(&self) -> ([f64; 2], [usize; 2], f64) {
        (self.origin, self.size, self.spacing)
    }

    /// Surface height, datum and shore distance at grid nodes, X fastest.
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = [f64; 3]> + '_ {
        self.heights
            .iter()
            .zip(&self.datum)
            .zip(&self.shore)
            .map(|((&height, &datum), &shore)| [height, datum, shore])
    }

    /// Surface height, averaged datum and signed shore distance (positive on water).
    pub fn sample(&self, point: [f64; 2]) -> [f64; 3] {
        [
            self.read(&self.heights, point),
            self.read(&self.datum, point),
            self.read(&self.shore, point),
        ]
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped surface coordinates"
    )]
    fn read(&self, values: &[f64], point: [f64; 2]) -> f64 {
        let p: [f64; 2] = std::array::from_fn(|i| {
            ((point[i] - self.origin[i]) / self.spacing).clamp(0.0, (self.size[i] - 1) as f64)
        });
        let i = p.map(|v| v.floor() as usize);
        let f = [p[0] - i[0] as f64, p[1] - i[1] as f64];
        let mut value = 0.0;
        for z in 0..2 {
            for x in 0..2 {
                let index = (i[1] + z).min(self.size[1] - 1) * self.size[0]
                    + (i[0] + x).min(self.size[0] - 1);
                value += values[index]
                    * if x == 0 { 1.0 - f[0] } else { f[0] }
                    * if z == 0 { 1.0 - f[1] } else { f[1] };
            }
        }
        value
    }
}

fn distances(wet: &[bool], target: bool, size: [usize; 2], spacing: f64) -> Vec<f64> {
    // No opposite surface means no shore, even for a grid smaller than the
    // requested margin. A finite sentinel keeps interpolation's zero weights safe.
    let cap = f64::MAX / 16.0;
    let mut d: Vec<_> = wet
        .iter()
        .map(|&w| if w == target { 0.0 } else { cap })
        .collect();
    for reverse in [false, true] {
        for n in 0..d.len() {
            let i = if reverse { d.len() - 1 - n } else { n };
            let x = i % size[0];
            let z = i / size[0];
            let mut take = |j: usize, cost: f64| {
                d[i] = d[i].min(d[j] + cost);
            };
            if reverse {
                if x + 1 < size[0] {
                    take(i + 1, spacing);
                }
                if z + 1 < size[1] {
                    take(i + size[0], spacing);
                    if x > 0 {
                        take(i + size[0] - 1, spacing * 2.0_f64.sqrt());
                    }
                    if x + 1 < size[0] {
                        take(i + size[0] + 1, spacing * 2.0_f64.sqrt());
                    }
                }
            } else {
                if x > 0 {
                    take(i - 1, spacing);
                }
                if z > 0 {
                    take(i - size[0], spacing);
                    if x > 0 {
                        take(i - size[0] - 1, spacing * 2.0_f64.sqrt());
                    }
                    if x + 1 < size[0] {
                        take(i - size[0] + 1, spacing * 2.0_f64.sqrt());
                    }
                }
            }
        }
    }
    d
}

/// Which surface supports a row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Affinity {
    /// All surfaces.
    Any,
    /// Water, fading to zero within the shore margin on land.
    Water,
    /// Land only; fades toward the shore on land, zero on water.
    Land,
}

/// One game-authored ground-condensate row, with no engine presets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    /// Physical liquid water kg/m³.
    pub liquid_water: f64,
    /// Droplet radius, metres.
    pub droplet_radius: f64,
    /// Maximum rise of the top with amount, metres.
    pub depth: f64,
    /// Pooled top starts this far below the datum, metres.
    pub depression: f64,
    /// Soft top depth, metres.
    pub softness: f64,
    /// Blend of even depth and datum-level pooling.
    pub pooling: f64,
    /// Whether the bank extent travels in the air frame.
    pub drifting: bool,
    /// Surface affinity.
    pub affinity: Affinity,
    /// Air-frame bank centre X/Z, metres.
    pub centre: [f64; 2],
    /// Bank half extents, metres; outer half fades smoothly.
    pub radius: [f64; 2],
    /// Noise erosion share, 0..=1.
    pub noise: f64,
    /// Noise scale, metres.
    pub noise_scale: f64,
    /// Take amount from cooling and moisture instead of the game.
    pub radiative: bool,
}

impl Fog {
    /// Same physical extinction as a cloud type, m⁻¹.
    pub fn extinction(self) -> f64 {
        3.0 * self.liquid_water / (2.0 * 1000.0 * self.droplet_radius)
    }

    fn valid(self) -> bool {
        [self.liquid_water, self.depth, self.depression]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0)
            && [self.droplet_radius, self.softness, self.noise_scale]
                .iter()
                .chain(self.radius.iter())
                .all(|v| v.is_finite() && *v > 0.0)
            && self.centre.iter().all(|v| v.is_finite())
            && unit(self.pooling)
            && unit(self.noise)
    }
}

/// Numerical constants for ground rows and their radiative day. Angles in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundProfile {
    /// Shore fade distance on each side, metres.
    pub shore_margin: f64,
    /// Wind speeds at start/end of calm suppression, m/s.
    pub calm: [f64; 2],
    /// Night factor transitions across these Sun elevations.
    pub twilight: [f64; 2],
    /// Cooling response time, seconds.
    pub cooling_seconds: f64,
    /// Base warming response time, seconds.
    pub warming_seconds: f64,
}

/// Validated table and profile. Surface data and rows remain game supplied.
#[derive(PartialEq)]
pub struct Ground {
    pub(crate) rows: Vec<Fog>,
    profile: GroundProfile,
}

impl Ground {
    /// Validate a fog table and its profile.
    /// # Errors
    /// Rejects invalid coefficients, extents and response times.
    pub fn new(profile: GroundProfile, rows: Vec<Fog>) -> Result<Self, Error> {
        if rows.iter().any(|r| !r.valid())
            || [
                profile.shore_margin,
                profile.cooling_seconds,
                profile.warming_seconds,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
            || [profile.calm, profile.twilight]
                .iter()
                .any(|a| !a.iter().all(|v| v.is_finite()) || a[0] >= a[1])
            || profile.calm[0] < 0.0
        {
            return Err(Error::Invalid);
        }
        Ok(Self { rows, profile })
    }

    /// The authored rows in the same order as the state's amounts.
    pub fn rows(&self) -> &[Fog] {
        &self.rows
    }

    /// Distance over which affinity fades at the shore, in metres.
    pub fn shore_margin(&self) -> f64 {
        self.profile.shore_margin
    }

    /// Uneroded bottom and top of one row. Empty amount has zero depth.
    /// # Panics
    /// Panics if `row` is outside this table, as with indexing any authored row.
    pub fn column(&self, row: usize, surface: &Surface, point: [f64; 2], amount: f64) -> [f64; 2] {
        let [height, datum, _] = surface.sample(point);
        let r = self.rows[row];
        let top = height + amount * r.depth + r.pooling * (datum - height - r.depression);
        [
            height,
            if amount <= 0.0 {
                height
            } else {
                top.max(height)
            },
        ]
    }

    /// Row extinction; terrain, datum and shore are always evaluated in the world.
    pub fn density(
        &self,
        surface: &Surface,
        field: &Field,
        point: [f64; 3],
        state: &State,
        fidelity: Density,
    ) -> f64 {
        self.rows
            .iter()
            .enumerate()
            .map(|(i, _)| self.row_density(i, surface, field, point, state, fidelity))
            .sum()
    }

    pub(crate) fn row_density(
        &self,
        i: usize,
        surface: &Surface,
        field: &Field,
        point: [f64; 3],
        state: &State,
        fidelity: Density,
    ) -> f64 {
        let world = [point[0], point[2]];
        let [height, _, shore] = surface.sample(world);
        if point[1] < height {
            return 0.0;
        }
        let air = state.to_air(world);
        let r = &self.rows[i];
        let amount = state.ground().amount(i, r.radiative);
        if amount == 0.0 {
            return 0.0;
        }
        let [_, top] = self.column(i, surface, world, amount);
        if top <= height || point[1] >= top {
            return 0.0;
        }
        let affinity = match r.affinity {
            Affinity::Any => 1.0,
            Affinity::Water => {
                smooth((shore + self.profile.shore_margin) / self.profile.shore_margin)
            }
            Affinity::Land => smooth(-shore / self.profile.shore_margin),
        };
        let extent = if r.drifting {
            (1.0 - smooth(2.0 * ((air[0] - r.centre[0]).abs() / r.radius[0]) - 1.0))
                * (1.0 - smooth(2.0 * ((air[1] - r.centre[1]).abs() / r.radius[1]) - 1.0))
        } else {
            1.0
        };
        let noise_value = match fidelity {
            Density::Uneroded => 1.0,
            Density::Coarse => 0.5,
            Density::Full => field.noise(
                [
                    air[0] / r.noise_scale,
                    point[1] / r.noise_scale,
                    air[1] / r.noise_scale + state.time() / 1000.0,
                ],
                70,
            ),
        };
        let noise = if matches!(fidelity, Density::Uneroded) {
            1.0
        } else {
            1.0 - r.noise + r.noise * smooth((noise_value - (1.0 - amount)) / 0.2)
        };
        // Multiplying by amount makes zero absent even in deep hollows and
        // keeps the uneroded row a bound on both eroded fidelities.
        r.extinction() * amount * affinity * extent * noise * smooth((top - point[1]) / r.softness)
    }
}

/// Extra committed weather controls. Stored inside `State`, with no clock read.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundState {
    amounts: Vec<f64>,
    targets: Vec<f64>,
    cooling: f64,
    moisture: f64,
    moisture_target: f64,
    visual_range: f64,
    range_target: f64,
    angstrom_exponent: f64,
    angstrom_target: f64,
    radiative_day: bool,
}

impl Default for GroundState {
    fn default() -> Self {
        Self {
            amounts: Vec::new(),
            targets: Vec::new(),
            cooling: 0.0,
            moisture: 0.0,
            moisture_target: 0.0,
            visual_range: 100_000.0,
            range_target: 100_000.0,
            angstrom_exponent: 0.0,
            angstrom_target: 0.0,
            radiative_day: true,
        }
    }
}

fn valid_ground_controls(amounts: &[f64], moisture: f64, range: f64) -> bool {
    amounts.iter().all(|&amount| unit(amount))
        && unit(moisture)
        && (3000.0..=300_000.0).contains(&range)
}

impl GroundState {
    /// Create ground controls for exactly the given table's rows.
    /// # Errors
    /// Rejects amounts/moisture outside 0..=1 or range outside 3..=300 km.
    pub fn new(amounts: Vec<f64>, moisture: f64, visual_range: f64) -> Result<Self, Error> {
        if !valid_ground_controls(&amounts, moisture, visual_range) {
            return Err(Error::Invalid);
        }
        Ok(Self {
            targets: amounts.clone(),
            amounts,
            cooling: 0.0,
            moisture,
            moisture_target: moisture,
            visual_range,
            range_target: visual_range,
            ..Self::default()
        })
    }

    /// Set targets atomically without changing committed controls.
    /// # Errors
    /// Rejects a changed row count or invalid controls.
    pub fn set_targets(
        &mut self,
        amounts: &[f64],
        moisture: f64,
        visual_range: f64,
    ) -> Result<(), Error> {
        if amounts.len() != self.amounts.len()
            || !valid_ground_controls(amounts, moisture, visual_range)
        {
            return Err(Error::Invalid);
        }
        self.targets.copy_from_slice(amounts);
        self.moisture_target = moisture;
        self.range_target = visual_range;
        Ok(())
    }

    /// Current row amount; missing controls are absent.
    pub fn amount(&self, row: usize, radiative: bool) -> f64 {
        if radiative && self.radiative_day {
            // Even saturated air needs finite cooling to condense. Warming
            // approaches zero asymptotically, so a zero threshold would leave
            // a positive amount throughout every clear saturated afternoon.
            smooth((self.cooling + 0.5 * self.moisture - 1.0) / 0.5)
        } else {
            self.amounts.get(row).copied().unwrap_or(0.0)
        }
    }
    /// Whether the radiative response is enabled for rows marked radiative.
    pub fn radiative_day(&self) -> bool {
        self.radiative_day
    }

    /// Enable the radiative response of rows marked radiative.
    pub fn set_radiative_day(&mut self, enabled: bool) {
        self.radiative_day = enabled;
    }

    /// Integrated ground cooling in 0..=1.
    pub fn cooling(&self) -> f64 {
        self.cooling
    }
    /// Clear-air visual range, metres.
    pub fn visual_range(&self) -> f64 {
        self.visual_range
    }

    /// The committed Ångström exponent of clear-air aerosol.
    pub fn angstrom_exponent(&self) -> f64 {
        self.angstrom_exponent
    }

    /// Set the clear-air aerosol's spectral target.
    /// # Errors
    /// Rejects values outside 0..=2, including nonfinite values.
    pub fn set_angstrom_target(&mut self, exponent: f64) -> Result<(), Error> {
        if !(0.0..=2.0).contains(&exponent) {
            return Err(Error::Invalid);
        }
        self.angstrom_target = exponent;
        Ok(())
    }

    pub(crate) fn ramp(&mut self, rate: f64) {
        for (v, t) in self.amounts.iter_mut().zip(&self.targets) {
            *v += (*t - *v).clamp(-rate, rate);
        }
        self.angstrom_exponent +=
            (self.angstrom_target - self.angstrom_exponent).clamp(-2.0 * rate, 2.0 * rate);
        self.moisture += (self.moisture_target - self.moisture).clamp(-rate, rate);
        self.visual_range +=
            (self.range_target - self.visual_range).clamp(-297_000.0 * rate, 297_000.0 * rate);
    }

    pub(crate) fn day(&mut self, ground: &Ground, dt: f64, sun: f64, state: crate::Parameters) {
        let p = ground.profile;
        let night = 1.0 - smooth((sun - p.twilight[0]) / (p.twilight[1] - p.twilight[0]));
        let calm = 1.0
            - smooth((state.wind[0].hypot(state.wind[1]) - p.calm[0]) / (p.calm[1] - p.calm[0]));
        // A closed sky removes this radiative source even at saturated
        // moisture; residual cooling would otherwise form fog under overcast.
        let target = night * (1.0 - state.coverage) * calm;
        let response = if target > self.cooling {
            p.cooling_seconds
        } else {
            p.warming_seconds / (0.25 + smooth(sun / 30.0))
        };
        self.cooling += (target - self.cooling) * (-(-dt / response).exp_m1());
    }
}
