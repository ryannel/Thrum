//! The flow bands (`RIVERS` ruling 3): two wave layers of the cascades' kind that the wind does
//! not fill. Each is a band of an isotropic spectrum on a periodic 256-square patch, near 3 m and
//! 13 m across, whose modes turn a whole number of times in the cascades' loop. Their power never
//! changes; a river chart's row holds a gain for each, and every other body's is zero. The map adds
//! them at those gains through the one sum, the depth limit and the moments a cascade has.
use crate::waves::Limited;
use crate::{Field, Map, MapSample, Profile, WaterError, limit};
use std::f64::consts::TAU;
use thrum_sea_model::{GRAVITY, GRID_SIZE, LOOP_SECONDS, Sample};

/// Surface tension over density, m³/s², in the bands' own dispersion.
const TENSION: f64 = 7.28e-5;

/// The flow bands' physical profile. Never a quality setting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowBandsProfile {
    /// Patch side of the short and the long band, metres.
    pub patches: [f64; 2],
    /// Each band's own rms height, metres: a gain of one gives it.
    pub rms: [f64; 2],
    /// Least and greatest lattice radius of each band's modes, in lattice steps.
    pub radii: [[f64; 2]; 2],
    /// Seed of the modes' phases.
    pub seed: u64,
}

impl Default for FlowBandsProfile {
    /// The first profile: a short band of 0.75 m to 0.125 m waves on a 3 m patch and a long
    /// band of 3.25 m to 0.76 m waves on a 13 m patch, with no gap between them.
    fn default() -> Self {
        Self {
            patches: [3.0, 13.0],
            rms: [0.01, 0.03],
            radii: [[4.0, 24.0], [4.0, 17.0]],
            seed: 0x7468_7275_6d20_666c,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Mode {
    wave_vector: [f64; 2],
    amplitude: f64,
    phase: f64,
    omega: f64,
}

/// The two flow bands: a function of time on the cascades' loop, with no state.
#[derive(Clone, Debug)]
pub struct FlowBands {
    profile: FlowBandsProfile,
    modes: [Vec<Mode>; 2],
    slope_variance: [f64; 2],
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The bands' own dispersion, deep water with surface tension, on the cascades' loop: the
/// nearest whole number of turns in `LOOP_SECONDS`.
pub fn flow_frequency(k: f64) -> f64 {
    let omega = (GRAVITY * k + TENSION * k * k * k).sqrt();
    (omega * LOOP_SECONDS / TAU).round() * TAU / LOOP_SECONDS
}

impl FlowBands {
    /// Build both bands from the profile. Each band's modes are every lattice vector whose
    /// length lies in its radii, with power falling as the wavenumber to the fourth, a phase
    /// from the seed, and amplitudes scaled so the band's rms height is the profile's.
    /// # Errors
    /// Refuses nonfinite or nonpositive patches and heights, and radii outside the lattice or
    /// with no mode between them.
    pub fn new(profile: FlowBandsProfile) -> Result<Self, WaterError> {
        let bad = |r| Err(WaterError::new("flow bands", r));
        let half = (GRID_SIZE / 2) as f64;
        for b in 0..2 {
            let [lo, hi] = profile.radii[b];
            let positive = |v: f64| v.is_finite() && v > 0.0;
            let radii = lo.is_finite() && hi.is_finite() && lo >= 1.0 && hi > lo && hi < half;
            if !(positive(profile.patches[b]) && positive(profile.rms[b]) && radii) {
                return bad("invalid patch, height or radii");
            }
        }
        let mut modes: [Vec<Mode>; 2] = [Vec::new(), Vec::new()];
        let mut slope_variance = [0.0; 2];
        for (b, out) in modes.iter_mut().enumerate() {
            let [lo, hi] = profile.radii[b];
            let length = profile.patches[b];
            #[expect(
                clippy::cast_possible_truncation,
                reason = "radii are checked under half the lattice"
            )]
            let reach = hi.ceil() as i32;
            for z in -reach..=reach {
                for x in -reach..=reach {
                    let r = f64::from(x).hypot(f64::from(z));
                    if r < lo || r > hi {
                        continue;
                    }
                    let k = [f64::from(x) * TAU / length, f64::from(z) * TAU / length];
                    let kk = k[0].hypot(k[1]);
                    #[expect(
                        clippy::cast_sign_loss,
                        reason = "lattice indices are offset to be non-negative"
                    )]
                    let id = ((b as u64) << 40) | (((x + 128) as u64) << 20) | (z + 128) as u64;
                    let u = (mix(profile.seed ^ mix(id)) >> 11) as f64 / (1u64 << 53) as f64;
                    out.push(Mode {
                        wave_vector: k,
                        // Power per mode: the density, k^-4, times the lattice cell's area.
                        amplitude: kk.powi(-2),
                        phase: TAU * u,
                        omega: flow_frequency(kk),
                    });
                }
            }
            if out.is_empty() {
                return bad("a band has no mode");
            }
            let variance: f64 = out.iter().map(|m| 0.5 * m.amplitude * m.amplitude).sum();
            let scale = profile.rms[b] / variance.sqrt();
            for m in out.iter_mut() {
                m.amplitude *= scale;
            }
            slope_variance[b] = out
                .iter()
                .map(|m| 0.5 * (m.amplitude * m.wave_vector[0].hypot(m.wave_vector[1])).powi(2))
                .sum();
        }
        Ok(Self {
            profile,
            modes,
            slope_variance,
        })
    }

    /// The profile the bands were built from.
    pub fn profile(&self) -> FlowBandsProfile {
        self.profile
    }

    /// Each band's height variance at a gain of one, square metres.
    pub fn variances(&self) -> [f64; 2] {
        self.profile.rms.map(|r| r * r)
    }

    /// Each band's slope variance at a gain of one: the moment the surface's roughness reads.
    pub fn slope_variances(&self) -> [f64; 2] {
        self.slope_variance
    }

    /// Number of modes in each band.
    pub fn mode_counts(&self) -> [usize; 2] {
        [self.modes[0].len(), self.modes[1].len()]
    }

    /// One band at a gain of one, at a world point and time: an f64 sum of its modes, each
    /// turning on the cascades' loop. The bands have no horizontal displacement, so a point is
    /// its own label and the Jacobian is one.
    /// # Panics
    /// If `band` is not 0 or 1.
    pub fn sample(&self, band: usize, point: [f64; 2], time: f64) -> Sample {
        let t = time.rem_euclid(LOOP_SECONDS);
        let mut s = Sample::default();
        for m in &self.modes[band] {
            let theta = m.wave_vector[0].mul_add(point[0], m.wave_vector[1] * point[1]) + m.phase
                - m.omega * t;
            let (sin, cos) = theta.sin_cos();
            let k = m.wave_vector[0].hypot(m.wave_vector[1]);
            s.height += m.amplitude * cos;
            s.slope[0] -= m.amplitude * m.wave_vector[0] * sin;
            s.slope[1] -= m.amplitude * m.wave_vector[1] * sin;
            s.height_dt += m.amplitude * m.omega * sin;
            s.velocity[0] += m.amplitude * m.omega * m.wave_vector[0] / k * cos;
            s.velocity[2] += m.amplitude * m.omega * m.wave_vector[1] / k * cos;
        }
        s.velocity[1] = s.height_dt;
        s
    }
}

/// The depth limit of the wind's layers and the flow bands together: the swash is the whole
/// sum's, and each layer, a flow band as a cascade, is held under the cap on its own.
pub(crate) fn limit_with_flow(
    map: MapSample,
    variances: [f64; 4],
    flow: [f64; 2],
    flow_variances: [f64; 2],
    profile: Profile,
) -> (Limited, [f64; 2]) {
    let wind: f64 = (0..4)
        .map(|i| variances[i] * map.row.gains[i].powi(2))
        .sum();
    let variance = wind + flow_variances[0] * flow[0].powi(2) + flow_variances[1] * flow[1].powi(2);
    let swash_depth = (profile.swash_ratio * 4.0 * variance.sqrt()).min(profile.margin_height);
    let cap = 0.6 * (map.depth + swash_depth).max(0.0);
    let held = |gain: f64, v: f64| {
        let hs = 4.0 * v.sqrt();
        if hs > 0.0 { gain.min(cap / hs) } else { gain }
    };
    let gains = std::array::from_fn(|i| held(map.row.gains[i], variances[i]));
    let flow_gains = std::array::from_fn(|b| held(flow[b], flow_variances[b]));
    (
        Limited {
            gains,
            lost_power: std::array::from_fn(|i| {
                variances[i] * (map.row.gains[i].powi(2) - gains[i].powi(2))
            }),
            swash_depth,
        },
        flow_gains,
    )
}

impl Map {
    /// Name the flow bands in the map's profile, or take them away. With none, every answer of
    /// the map is what it was, to the bit.
    pub fn set_flow_bands(&mut self, bands: Option<FlowBands>) {
        self.flow = bands;
    }

    /// The map's flow bands.
    pub fn flow_bands(&self) -> Option<&FlowBands> {
        self.flow.as_ref()
    }

    /// The wind's layers' and the flow bands' depth-limited gains at a map sample. Without flow
    /// bands this is [`limit`] and no flow gain.
    pub(crate) fn limited(
        &self,
        field: &Field,
        point: [f64; 2],
        map: MapSample,
    ) -> (Limited, [f64; 2]) {
        self.limited_with_variances(field.variances(), point, map)
    }

    pub(crate) fn limited_with_variances(
        &self,
        variances: [f64; 4],
        point: [f64; 2],
        map: MapSample,
    ) -> (Limited, [f64; 2]) {
        let Some(bands) = &self.flow else {
            return (limit(map.row, map.depth, variances, self.profile), [0.0; 2]);
        };
        limit_with_flow(
            map,
            variances,
            self.flow_gains(map.body, point),
            bands.variances(),
            self.profile,
        )
    }

    /// The flow bands' depth-limited gains at a point on a map sample; none without the bands.
    pub fn flow_band_gains(&self, field: &Field, point: [f64; 2], map: MapSample) -> [f64; 2] {
        self.limited(field, point, map).1
    }

    /// The squared slope the flow bands add to the surface's moments at a point: each band's
    /// slope variance at its depth-limited gain, as a cascade's.
    pub fn flow_moments(&self, field: &Field, point: [f64; 2]) -> f64 {
        let (Some(bands), Some(map)) = (&self.flow, self.sample(point)) else {
            return 0.0;
        };
        let g = self.flow_band_gains(field, point, map);
        let s = bands.slope_variances();
        g[0] * g[0] * s[0] + g[1] * g[1] * s[1]
    }

    /// Add the flow bands at their gains to a sample through the one sum.
    pub(crate) fn add_flow(
        &self,
        gains: [f64; 2],
        point: [f64; 2],
        time: f64,
        sample: &mut Sample,
    ) {
        let Some(bands) = &self.flow else {
            return;
        };
        for (b, &gain) in gains.iter().enumerate() {
            if gain <= 0.0 {
                continue;
            }
            let s = bands.sample(b, point, time);
            sample.height += gain * s.height;
            sample.height_dt += gain * s.height_dt;
            for (a, v) in sample.slope.iter_mut().zip(s.slope) {
                *a += gain * v;
            }
            for (a, v) in sample.velocity.iter_mut().zip(s.velocity) {
                *a += gain * v;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_band_holds_its_rms_and_turns_on_the_loop() {
        let bands = FlowBands::new(FlowBandsProfile::default()).unwrap();
        for b in 0..2 {
            let v: f64 = bands.modes[b]
                .iter()
                .map(|m| 0.5 * m.amplitude.powi(2))
                .sum();
            assert!((v.sqrt() / bands.profile.rms[b] - 1.0).abs() < 1e-12);
            let p = [1.3, -2.7];
            let a = bands.sample(b, p, 5.0);
            let c = bands.sample(b, p, 5.0 + LOOP_SECONDS);
            assert!((a.height - c.height).abs() < 1e-9);
        }
    }
}
