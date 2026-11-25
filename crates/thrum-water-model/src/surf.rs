use crate::{
    BedPyramid, BodyKind, BreakerSection, CutConfig, Field, Map, MapSample, Sheet, SheetPoint,
    ShoreConfig, ShoreTransform, Source, Surface, WaterError, cut, limit, shoaling,
};
use std::f64::consts::{FRAC_PI_4, TAU};
use thrum_sea_model::Sample;

const SWASH_C: f64 = 0.6 / (2.0 * std::f64::consts::SQRT_2);

/// Nearest shore-transform sector for a finite nonzero wave vector.
/// A vector on a boundary belongs to the sector in the increasing angle direction.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "wrapped angle yields an index in the eight prepared sectors"
)]
pub fn shore_sector([x, z]: [f64; 2]) -> u32 {
    let sectors = z.atan2(x).rem_euclid(TAU) / FRAC_PI_4;
    // The sea spectrum lies on sector boundaries. The addition covers
    // rounding in vector construction and atan2 on either target.
    ((sectors + 0.5 + 1e-12).floor() as u32) % 8
}

/// Authored surf capability. Omitting it leaves the original model untouched.
pub struct SurfProfile {
    /// Minimum dispersion depth, metres.
    pub lag_floor: f64,
    /// Breaking height/depth ratio.
    pub breaker_ratio: f64,
    /// Periodic phase/index section table.
    pub section: BreakerSection,
    /// Highest-column controls.
    pub cut: CutConfig,
}

impl Default for SurfProfile {
    fn default() -> Self {
        Self {
            lag_floor: 0.5,
            breaker_ratio: 0.78,
            section: BreakerSection::first(),
            cut: CutConfig::default(),
        }
    }
}

pub(crate) struct Surf {
    profile: SurfProfile,
    transforms: Vec<Option<ShoreTransform>>,
    pyramid: BedPyramid,
}

impl Map {
    /// The profile used to prepare the current surf capability.
    pub fn surf_profile(&self) -> Option<&SurfProfile> {
        self.surf.as_ref().map(|surf| &surf.profile)
    }

    /// The immutable shore transform prepared for a still body.
    pub fn shore_transform(&self, body: usize) -> Option<&ShoreTransform> {
        self.surf.as_ref()?.transforms.get(body)?.as_ref()
    }

    /// The immutable depth-bound hierarchy prepared for surf queries.
    pub fn surf_bed_pyramid(&self) -> Option<&BedPyramid> {
        self.surf.as_ref().map(|surf| &surf.pyramid)
    }

    /// Forward long-wave sheet for content tools and conformance references.
    /// # Errors
    /// Refuses a disabled capability, unknown body or nonfinite time.
    pub fn surf_sheet<'a>(
        &'a self,
        field: &'a Field,
        body: usize,
        time: f64,
    ) -> Result<SurfSheet<'a>, WaterError> {
        if body >= self.bodies.len() || !time.is_finite() {
            return Err(WaterError::new("surf sheet", "invalid body or time"));
        }
        Ok(SurfSheet {
            surf: self
                .surf
                .as_ref()
                .ok_or_else(|| WaterError::new("surf sheet", "surf is disabled"))?,
            map: self,
            field,
            body,
            time,
        })
    }

    /// Prepare surf for every declared still body and stage range.
    /// Stage and forcing updates keep the prepared content.
    /// # Errors
    /// Rejects invalid controls, preserving the previous capability on failure.
    pub fn enable_surf(&mut self, profile: SurfProfile) -> Result<(), WaterError> {
        if !profile.lag_floor.is_finite()
            || profile.lag_floor <= 0.0
            || !profile.breaker_ratio.is_finite()
            || profile.breaker_ratio <= 0.0
            || !profile.cut.valid()
        {
            return Err(WaterError::new("surf", "invalid surf profile"));
        }
        let joins = self.swash_joins();
        let transforms = self
            .bodies
            .iter()
            .enumerate()
            .map(|(id, b)| {
                if matches!(b.kind, BodyKind::Chart(_) | BodyKind::Hollows(_)) {
                    return Ok(None);
                }
                ShoreTransform::prepare_with_joins(
                    self.shore_bed(id),
                    ShoreConfig {
                        level: b.level,
                        stage: b.stage,
                        margin: self.profile.margin_height,
                        floor: profile.lag_floor,
                        fetch: b.fetch,
                        seed: if let BodyKind::Basin { seed } = b.kind {
                            Some(seed)
                        } else {
                            None
                        },
                        source: if matches!(b.kind, BodyKind::Basin { .. }) {
                            Source::Enclosed
                        } else {
                            Source::Water
                        },
                    },
                    &joins,
                )
                .map(Some)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.surf = Some(std::sync::Arc::new(Surf {
            profile,
            transforms,
            pyramid: BedPyramid::new(&self.bed),
        }));
        Ok(())
    }

    fn swash_joins(&self) -> Vec<bool> {
        let mut joins = Vec::new();
        for body in &self.bodies {
            if let BodyKind::Chart(chart) = &body.kind {
                joins.resize(self.bed.heights.len(), false);
                for (i, &inside) in chart.footprint.iter().enumerate() {
                    if inside {
                        joins[i] = true;
                        for j in self.bed.neighbours(i) {
                            joins[j] = true;
                        }
                    }
                }
            }
        }
        joins
    }

    fn shore_bed(&self, id: usize) -> crate::Bed {
        let mut bed = self.bed.as_ref().clone();
        let body = &self.bodies[id];
        if matches!(body.kind, BodyKind::Basin { .. }) {
            // Reuse the map's strict wet topology, including its chart barriers.
            // A long join must not make unrelated sea coast into basin sources.
            let wet = self.still_fill(id, body.stage.max);
            for (i, &owned) in wet.iter().enumerate() {
                let chart = self
                    .bodies
                    .iter()
                    .any(|b| matches!(&b.kind, BodyKind::Chart(c) if c.footprint[i]));
                if !owned && !chart && bed.heights[i] < body.level + body.stage.max {
                    bed.heights[i] = body.level + body.stage.max + self.profile.margin_height + 1.0;
                }
            }
        }
        // A still body's water stops at a chart. Continue its phase only through
        // the part of the chart that can blend toward it, over all declared stages.
        // In particular a basin must not become edge-fed through its outlet.
        for chart_body in &self.bodies {
            let BodyKind::Chart(chart) = &chart_body.kind else {
                continue;
            };
            let joined: Vec<_> = chart
                .footprint
                .iter()
                .enumerate()
                .map(|(i, &inside)| {
                    if !inside {
                        return false;
                    }
                    let point = bed.point(i);
                    chart.ends.iter().any(|end| {
                        end.body == id
                            && (end.distance(point) < end.length
                                || end.mouth
                                    && chart.levels[i] + chart_body.stage.min
                                        < body.level
                                            + body.stage.max
                                            + 0.175 * self.profile.margin_height
                                            + 0.25)
                    })
                })
                .collect();
            for (i, &inside) in chart.footprint.iter().enumerate() {
                if !inside {
                    continue;
                }
                // Bilinear reads and their gradient probes need finite corner
                // lags for the whole interval in which the join has weight.
                let padding = bed.connections(i).any(|(j, _)| joined[j]);
                if !joined[i] && !padding {
                    bed.heights[i] = body.level + body.stage.max + self.profile.margin_height + 1.0;
                }
            }
        }
        bed
    }
}

impl Surf {
    pub(crate) fn surface(
        &self,
        map: &Map,
        field: &Field,
        point: [f64; 2],
        time: f64,
        row: MapSample,
    ) -> Result<Surface, WaterError> {
        if !time.is_finite() || point.iter().any(|v| !v.is_finite()) {
            return Err(WaterError::new("surf", "nonfinite point or time"));
        }
        let sheet = SurfSheet {
            surf: self,
            map,
            field,
            time,
            body: row.body,
        };
        let (rest, height) = if sheet.can_throw(point) {
            let column = cut(&sheet, point, self.profile.cut)?;
            (column.rest, column.height)
        } else {
            let mut rest = point;
            let mut converged = false;
            for _ in 0..4096 {
                let forward = sheet.forward(rest);
                let next = std::array::from_fn(|a| point[a] - (forward.world[a] - rest[a]));
                let error = (next[0] - rest[0]).hypot(next[1] - rest[1]);
                rest = next;
                if error < 1e-11 {
                    converged = true;
                    break;
                }
            }
            if !converged {
                return Err(WaterError::new(
                    "surf",
                    "plain displacement did not converge",
                ));
            }
            (rest, sheet.forward(rest).height)
        };
        let mut sample = sheet.derivatives(rest);
        sample.height = height + row.row.level;
        for (a, b) in sample
            .slope
            .iter_mut()
            .zip(map.level_slope(row.body, point))
        {
            *a += b;
        }
        sample.velocity[0] += row.row.current[0];
        sample.velocity[2] += row.row.current[1];
        let (limited, flow) = map.limited(field, point, row);
        let mut gains = limited.gains;
        if row.row.gains[0] > 0.0 && field.variances()[0] > 0.0 {
            let weight = self
                .contour(map, row.body, point, row.depth, limited.swash_depth)
                .map_or(1.0, |(_, coverage)| {
                    1.0 - smooth(coverage)
                        * (1.0
                            - short_geometry_weight(
                                row.depth,
                                limited.swash_depth,
                                row.depth + height,
                            ))
                });
            for gain in &mut gains[1..] {
                *gain *= weight;
            }
        }
        for (i, layer) in field.layers.iter().enumerate().skip(1) {
            let wave = layer
                .scaled(gains[i])
                .and_then(|l| l.evaluate(point, time))
                .map_err(|_| WaterError::new("surf", "invalid short waves"))?;
            sample.height += wave.height;
            sample.height_dt += wave.height_dt;
            for (a, b) in sample.slope.iter_mut().zip(wave.slope) {
                *a += b;
            }
            for (a, b) in sample.velocity.iter_mut().zip(wave.velocity) {
                *a += b;
            }
        }
        map.add_flow(flow, point, time, &mut sample);
        Ok(Surface {
            body: Some(row.body),
            sample,
            depth: row.depth,
            current: row.row.current,
            wet: row.bed.is_none_or(|bed| sample.height > bed),
        })
    }

    fn contour(
        &self,
        map: &Map,
        body: usize,
        point: [f64; 2],
        depth: f64,
        swash: f64,
    ) -> Option<([f64; 2], f64)> {
        self.transforms[body]
            .as_ref()
            .filter(|_| swash > 0.0 && depth < 4.0 * map.profile.margin_height)
            .and_then(|transform| transform.contour(map.bodies[body].stage.value, point))
    }

    fn transform(
        &self,
        map: &Map,
        body: usize,
        point: [f64; 2],
        direction: usize,
        wavelength: f64,
    ) -> (f64, [f64; 2], f64) {
        if let Some(transform) = &self.transforms[body] {
            let stage = map.bodies[body].stage.value;
            let (lag, gradient) = transform.sample(direction, wavelength, stage, point);
            return (lag, gradient, transform.exposure(direction, stage, point));
        }
        let mut answer = (0.0, [0.0; 2], 1.0);
        if let BodyKind::Chart(chart) = &map.bodies[body].kind {
            let mut total = 1.0;
            let own_level = map.own_level(body, point);
            for end in chart.ends.iter().filter(|e| !e.is_brink()) {
                let other_row = map.row(end.body, point);
                let w = map.join_weight(end, point, own_level, other_row.level);
                if w <= 0.0 {
                    continue;
                }
                let other = self.transform(map, end.body, point, direction, wavelength);
                if w >= 1.0 {
                    return other;
                }
                let relative = w / (1.0 - w);
                total += relative;
                let f = relative / total;
                answer = (
                    answer.0 * (1.0 - f) + other.0 * f,
                    [0.0; 2],
                    answer.2 * (1.0 - f) + other.2 * f,
                );
            }
        }
        answer
    }
}

fn smooth(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn short_geometry_weight(depth: f64, swash: f64, thickness: f64) -> f64 {
    if swash > 0.0 {
        smooth((depth / swash - 0.85) / (1.3 - 0.85))
            * smooth((thickness / swash - 0.12) / (0.4 - 0.12))
    } else {
        1.0
    }
}

fn wet_floor(thickness: f64, swash: f64) -> f64 {
    let floor = 0.08 * swash;
    let knee = 0.15 * swash;
    if thickness >= floor + knee {
        return thickness;
    }
    let t = ((thickness - floor) / knee).max(0.0);
    floor + knee * t * t * (2.0 - t)
}

/// Borrowed assembled long-wave sheet, including transformation and shaping.
pub struct SurfSheet<'a> {
    surf: &'a Surf,
    map: &'a Map,
    field: &'a Field,
    time: f64,
    body: usize,
}

impl SurfSheet<'_> {
    fn evaluate(&self, rest: [f64; 2], time: f64) -> SheetPoint {
        // Horizontal displacement may carry a rest place across a chart seam.
        // Both sides must evaluate that place from the map's one owning row,
        // rather than continuing two different bodies through the same column.
        let owner = if self.map.bodies.len() > 1 {
            self.map.sample(rest)
        } else {
            None
        };
        let body = owner.map_or(self.body, |s| s.body);
        let row = owner.map_or_else(|| self.map.row(body, rest), |s| s.row);
        let depth = self
            .map
            .bed
            .height(rest)
            .map_or(f64::INFINITY, |h| row.level - h);
        let limited = limit(row, depth, self.field.variances(), self.map.profile);
        let swash = limited.swash_depth;
        let cap = 0.6 * (depth + swash).max(0.0);
        if cap == 0.0 || row.gains[0] == 0.0 {
            return SheetPoint {
                world: rest,
                height: 0.0,
            };
        }
        let contour = self.surf.contour(self.map, body, rest, depth, swash);
        let draw = SWASH_C * swash / (1.0 - SWASH_C);
        let fade = if swash > 0.0 {
            smooth((depth - draw) / (0.5 * swash - draw))
        } else {
            f64::from(depth > 0.0)
        };
        let mut re = 0.0;
        let mut im = 0.0;
        let mut gx = [0.0; 2];
        let mut gy = [0.0; 2];
        let mut displacement = [0.0; 2];
        let mut variance = 0.0;
        let spacing = self.map.bed.spacing;
        let step = spacing * 0.01;
        // Inside one bilinear cell the central difference is its exact
        // derivative. Keep the probes at cell edges and at body joins.
        let probes: [[([f64; 2], usize); 2]; 2] = std::array::from_fn(|axis| {
            [-step, step].map(|offset| {
                let mut p = rest;
                p[axis] += offset;
                let owner = if self.map.bodies.len() > 1 {
                    self.map.sample(p).map_or(body, |s| s.body)
                } else {
                    body
                };
                (p, owner)
            })
        });
        let use_gradient: [bool; 2] = std::array::from_fn(|axis| {
            depth.is_finite()
                && self.surf.transforms[body].is_some()
                && probes[axis].iter().all(|&(_, id)| id == body)
                && (0.01..=0.99)
                    .contains(&((rest[axis] - self.map.bed.origin[axis]) / spacing).fract())
        });
        let chop = self.field.layers[0].choppiness();
        for component in self.field.layers[0].components() {
            if component.amplitude == 0.0 {
                continue;
            }
            let k0 = component.wave_vector[0].hypot(component.wave_vector[1]);
            let wavelength = TAU / k0;
            let direction = shore_sector(component.wave_vector) as usize;
            let transform = (20.0..=640.0).contains(&wavelength) && depth.is_finite();
            let (lag, gradient, exposure) = if transform {
                self.surf
                    .transform(self.map, body, rest, direction, wavelength)
            } else {
                (0.0, [0.0; 2], 1.0)
            };
            // The section and shoaling fade in the swash. Keep the accumulated
            // travel time so the uprush stays on the incoming crest.
            let eligible = transform && lag.is_finite();
            let lag = if eligible { lag } else { 0.0 };
            let gain = if eligible {
                1.0 + (shoaling(k0, depth.max(self.surf.profile.lag_floor)) - 1.0) * fade
            } else {
                1.0
            };
            let amplitude = component.amplitude * row.gains[0] * exposure.sqrt() * gain;
            let phase = component.anchored_phase(rest, time) + k0 * lag;
            let (sin, cos) = phase.sin_cos();
            re += amplitude * cos;
            im += amplitude * sin;
            variance += 0.5 * amplitude.powi(2);
            let vector: [f64; 2] = std::array::from_fn(|axis| {
                if !eligible {
                    return component.wave_vector[axis];
                }
                if use_gradient[axis] {
                    return component.wave_vector[axis] + k0 * gradient[axis];
                }
                let [lo, hi] = probes[axis].map(|(p, owner)| {
                    self.surf
                        .transform(self.map, owner, p, direction, wavelength)
                        .0
                });
                let delta = if lo.is_finite() && hi.is_finite() {
                    (hi - lo) / (2.0 * step)
                } else {
                    0.0
                };
                component.wave_vector[axis] + k0 * delta
            });
            let norm = vector[0].hypot(vector[1]);
            for axis in 0..2 {
                gx[axis] += amplitude * cos * vector[axis];
                gy[axis] += amplitude * sin * vector[axis];
                if norm > 0.0 {
                    displacement[axis] -= chop * amplitude * sin * vector[axis] / norm;
                }
            }
        }
        let envelope = re.hypot(im);
        let index = 2.0 * envelope
            / (self.surf.profile.breaker_ratio * depth.max(self.surf.profile.lag_floor));
        let scale = if variance > 0.0 {
            (cap / (4.0 * variance.sqrt())).min(1.0)
        } else {
            1.0
        };
        let mut displacement = displacement.map(|value| value * scale);
        let phase = im.atan2(re);
        let section = self.surf.profile.section.sample(phase, index);
        let mut height = re * scale + (section.height * envelope - re) * scale * fade;
        let vector = std::array::from_fn::<_, 2, _>(|axis| gx[axis] * re + gy[axis] * im);
        let norm = vector[0].hypot(vector[1]);
        if norm > 1e-20 {
            for axis in 0..2 {
                displacement[axis] += section.throw * envelope * scale * fade * vector[axis] / norm;
            }
        }
        if let Some((reference, coverage)) = contour {
            let support = smooth(coverage);
            let ordinary = smooth((depth / swash - 0.35) / (0.85 - 0.35));
            let inner = support * (1.0 - ordinary);
            for distance in &mut displacement {
                *distance *= 1.0 - inner;
            }
            if inner > 0.0 {
                let q = self.contour_phase(body, reference, row.gains[0], time, swash);
                height = height * (1.0 - inner) + inner * SWASH_C * (depth + swash) * q;
            }
            if depth > 0.35 * swash
                && let Some(bed) = self
                    .map
                    .bed
                    .height(std::array::from_fn(|axis| rest[axis] + displacement[axis]))
            {
                let thickness = row.level - bed + height;
                let merge = support * smooth((depth / swash - 0.35) / 0.1);
                height += merge * (wet_floor(thickness, swash) - thickness);
            }
        }
        SheetPoint {
            world: std::array::from_fn(|a| rest[a] + displacement[a]),
            height,
        }
    }

    fn contour_phase(&self, body: usize, point: [f64; 2], gain: f64, time: f64, swash: f64) -> f64 {
        let mut re = 0.0;
        let mut im = 0.0;
        let mut variance = 0.0;
        for component in self.field.layers[0].components() {
            if component.amplitude == 0.0 {
                continue;
            }
            let k = component.wave_vector[0].hypot(component.wave_vector[1]);
            let wavelength = TAU / k;
            let (lag, _, exposure) = if (20.0..=640.0).contains(&wavelength) {
                self.surf.transform(
                    self.map,
                    body,
                    point,
                    shore_sector(component.wave_vector) as usize,
                    wavelength,
                )
            } else {
                (0.0, [0.0; 2], 1.0)
            };
            let amplitude = component.amplitude * gain * exposure.sqrt();
            let phase =
                component.anchored_phase(point, time) + k * if lag.is_finite() { lag } else { 0.0 };
            let (sin, cos) = phase.sin_cos();
            re += amplitude * cos;
            im += amplitude * sin;
            variance += 0.5 * amplitude * amplitude;
        }
        let envelope = re.hypot(im);
        if envelope == 0.0 {
            return 0.0;
        }
        let scale = (0.6 * swash / (4.0 * variance.sqrt())).min(1.0);
        re / envelope * smooth(scale * envelope / (SWASH_C * swash))
    }

    fn derivatives(&self, rest: [f64; 2]) -> Sample {
        let h = 1e-4;
        let jac: [[f64; 3]; 2] = std::array::from_fn(|axis| {
            let mut a = rest;
            a[axis] -= h;
            let mut b = rest;
            b[axis] += h;
            let a = self.evaluate(a, self.time);
            let b = self.evaluate(b, self.time);
            [
                (b.world[0] - a.world[0]) / (2.0 * h),
                (b.world[1] - a.world[1]) / (2.0 * h),
                (b.height - a.height) / (2.0 * h),
            ]
        });
        let a = self.evaluate(rest, self.time - h);
        let b = self.evaluate(rest, self.time + h);
        let velocity = [
            (b.world[0] - a.world[0]) / (2.0 * h),
            (b.height - a.height) / (2.0 * h),
            (b.world[1] - a.world[1]) / (2.0 * h),
        ];
        let det = jac[0][0] * jac[1][1] - jac[0][1] * jac[1][0];
        let slope = [
            (jac[0][2] * jac[1][1] - jac[1][2] * jac[0][1]) / det,
            (jac[1][2] * jac[0][0] - jac[0][2] * jac[1][0]) / det,
        ];
        Sample {
            height: self.evaluate(rest, self.time).height,
            slope,
            velocity,
            height_dt: velocity[1] - slope[0] * velocity[0] - slope[1] * velocity[2],
        }
    }
}

impl Sheet for SurfSheet<'_> {
    fn forward(&self, rest: [f64; 2]) -> SheetPoint {
        self.evaluate(rest, self.time)
    }

    fn reach(&self, point: [f64; 2]) -> f64 {
        self.throw_reach().at(point).0
    }
}

impl SurfSheet<'_> {
    fn throw_reach(&self) -> ThrowReach<'_> {
        let waves = &self.field.layers[0];
        self.map
            .surf_throw_reach(waves.components(), waves.choppiness())
            .expect("prepared surf sheet")
    }

    fn can_throw(&self, point: [f64; 2]) -> bool {
        self.throw_reach().at(point).1
    }
}

/// Bounds the horizontal displacement of a surf sheet for the current wave field.
pub struct ThrowReach<'a> {
    surf: &'a Surf,
    components: &'a [thrum_sea_model::Component],
    chop: f64,
    minimum_level: f64,
    first: f64,
}

impl Map {
    /// Prepare the reach calculation shared by the model and body-query dispatch.
    pub fn surf_throw_reach<'a>(
        &'a self,
        components: &'a [thrum_sea_model::Component],
        chop: f64,
    ) -> Option<ThrowReach<'a>> {
        let surf = self.surf.as_deref()?;
        let mut reach = ThrowReach {
            surf,
            components,
            chop,
            minimum_level: self.minimum_level(),
            first: 0.0,
        };
        reach.first = reach.ceiling(surf.profile.lag_floor).0;
        Some(reach)
    }

    fn minimum_level(&self) -> f64 {
        self.bodies
            .iter()
            .map(|b| {
                let base = if let BodyKind::Chart(c) = &b.kind {
                    c.levels.iter().copied().fold(b.level, f64::min)
                } else {
                    b.level
                };
                base + b.stage.min
            })
            .fold(f64::INFINITY, f64::min)
    }
}

impl ThrowReach<'_> {
    fn ceiling(&self, depth: f64) -> (f64, bool) {
        let amplitude: f64 = self
            .components
            .iter()
            .map(|c| {
                let k0 = c.wave_vector[0].hypot(c.wave_vector[1]);
                c.amplitude * shoaling(k0, depth).max(1.0)
            })
            .sum();
        let throws = self
            .surf
            .profile
            .section
            .throws_below(2.0 * amplitude / (self.surf.profile.breaker_ratio * depth));
        (
            amplitude
                * (if throws {
                    self.surf.profile.section.maximum_throw()
                } else {
                    0.0
                } + self.chop),
            throws,
        )
    }

    /// Return the reach in metres and whether any crest can be thrown into the column.
    pub fn at(&self, point: [f64; 2]) -> (f64, bool) {
        let floor = self.surf.profile.lag_floor;
        let depth = |reach| {
            self.surf
                .pyramid
                .range(point, reach)
                .map_or(floor, |range| (self.minimum_level - range[1]).max(floor))
        };
        let mut reach = self.first;
        for _ in 0..2 {
            reach = self.ceiling(depth(reach)).0.min(reach);
        }
        (reach, self.ceiling(depth(reach)).1)
    }
}
