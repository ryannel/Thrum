use crate::{Cascade, Component, GRAVITY, SeaError};
use std::f64::consts::{PI, TAU};

const SPLIT: f64 = TAU / 20.0;
const MIN_K: f64 = 1e-5;
const LONG_RADIAL: u32 = 24;
const DIRECTIONS: u32 = 8;

/// A narrow Gaussian swell train in log wavenumber (standard deviation 0.08,
/// support truncated at eight standard deviations, below f64 power resolution).
#[derive(Clone, Copy, Debug)]
pub struct Swell {
    /// Significant height in metres.
    pub significant_height: f64,
    /// Peak wavelength in metres, inside the represented spectrum.
    pub wavelength: f64,
    /// Travel direction in radians.
    pub direction: f64,
}

/// Physical driving inputs, independent of any game's named settings.
#[derive(Clone, Debug)]
pub struct Forcing {
    /// Wind at ten metres, metres per second (nonnegative).
    pub wind_speed: f64,
    /// Wind travel direction in radians.
    pub wind_direction: f64,
    /// Fetch in metres (positive).
    pub fetch: f64,
    /// Superposed swell trains.
    pub swells: Vec<Swell>,
}

impl Forcing {
    fn validate(&self) -> Result<(), SeaError> {
        if !self.wind_speed.is_finite()
            || self.wind_speed < 0.0
            || !self.wind_direction.is_finite()
            || !self.fetch.is_finite()
            || self.fetch <= 0.0
            || self.swells.iter().any(|s| {
                !s.significant_height.is_finite()
                    || s.significant_height < 0.0
                    || !s.wavelength.is_finite()
                    || !(0.1..=10000.0).contains(&s.wavelength)
                    || !s.direction.is_finite()
            })
        {
            return Err(SeaError::InvalidInput);
        }
        Ok(())
    }

    /// Omnidirectional variance density per unit log wavenumber.
    /// Fetch-limited JONSWAP, approaching Pierson–Moskowitz at full development.
    /// The frequency-to-log-k Jacobian is omega/2 for deep water.
    pub fn log_density(&self, k: f64) -> f64 {
        let mut density = self.wind_log_density(k);
        for swell in &self.swells {
            let u = (k / (TAU / swell.wavelength)).ln() / 0.08;
            if u.abs() <= 8.0 {
                density += (swell.significant_height / 4.0).powi(2) * (-0.5 * u * u).exp()
                    / (0.08 * TAU.sqrt());
            }
        }
        density
    }

    fn wind_log_density(&self, k: f64) -> f64 {
        let mut density = 0.0;
        if self.wind_speed > 0.0 {
            let dimensionless_fetch = self.fetch * GRAVITY / self.wind_speed.powi(2);
            let omega_pm = 0.877 * GRAVITY / self.wind_speed;
            let omega_peak = (22.0 * GRAVITY / self.wind_speed
                * dimensionless_fetch.powf(-1.0 / 3.0))
            .max(omega_pm);
            let alpha = (0.076 * dimensionless_fetch.powf(-0.22)).max(0.0081);
            let gamma: f64 = if omega_peak > omega_pm { 3.3 } else { 1.0 };
            let omega = (GRAVITY * k).sqrt();
            let sigma: f64 = if omega <= omega_peak { 0.07 } else { 0.09 };
            let peak = (-0.5 * ((omega / omega_peak - 1.0) / sigma).powi(2)).exp();
            density = 0.5 * alpha * GRAVITY.powi(2) / omega.powi(4)
                * (-1.25 * (omega_peak / omega).powi(4)).exp()
                * gamma.powf(peak);
        }
        density
    }

    fn directional_density(&self, k: f64, angle: f64) -> f64 {
        // Normalized finite Fourier spreading: (1+b*cos(theta))/(2*pi).
        // b decreases at high frequency. Swell uses the narrower (1+cos)^2.
        let b = 0.95 / (1.0 + k / SPLIT);
        let mut value =
            self.wind_log_density(k) * (1.0 + b * (angle - self.wind_direction).cos()) / TAU;
        for swell in &self.swells {
            let u = (k / (TAU / swell.wavelength)).ln() / 0.08;
            if u.abs() > 8.0 {
                continue;
            }
            let radial = (swell.significant_height / 4.0).powi(2) * (-0.5 * u * u).exp()
                / (0.08 * TAU.sqrt());
            value += radial * (1.0 + (angle - swell.direction).cos()).powi(2) / (3.0 * PI);
        }
        value
    }
}

/// One finite physical profile, with immutable sample identities across forcing changes.
/// Heights represent k in [1e-5, finest Nyquist); the shorter tail is slope variance.
#[derive(Clone, Debug)]
pub struct Spectrum {
    /// At most 192 nonperiodic components, all wavelengths at least 20 m.
    pub long: Vec<Component>,
    /// Three disjoint bands on their own Fourier lattices.
    pub cascades: [Cascade; 3],
    /// Unresolved short-wave slope variance (not height displacement).
    pub tail_slope_variance: f64,
}

impl Spectrum {
    /// Construct the fixed 192-component / three 256-square cascade profile.
    /// Quadrature: composite midpoint in log k, 512 subintervals per long cell;
    /// lattice weights are conservatively normalized to that band's integral.
    ///
    /// # Errors
    /// Rejects nonfinite or out-of-range physical inputs.
    pub fn build(forcing: &Forcing) -> Result<Self, SeaError> {
        forcing.validate()?;
        let lengths = [160.0, 37.0 * 1.001_f64.sqrt(), 9.0 * 1.003_f64.sqrt()];
        let edges = [
            SPLIT,
            PI * 256.0 / lengths[0],
            PI * 256.0 / lengths[1],
            PI * 256.0 / lengths[2],
        ];
        let mut long = Vec::with_capacity(192);
        let width = (SPLIT / MIN_K).ln() / f64::from(LONG_RADIAL);
        for radial in 0..LONG_RADIAL {
            let low = MIN_K * (f64::from(radial) * width).exp();
            let high = low * width.exp();
            for direction in 0..DIRECTIONS {
                // Irrational offsets avoid harmonics without moving cell ownership.
                let id = radial * DIRECTIONS + direction;
                let angle = TAU * (f64::from(direction) + 0.5) / f64::from(DIRECTIONS);
                let offset = 0.25 + 0.5 * (f64::from(id + 1) * 2.0_f64.sqrt()).fract();
                let k = low * (width * offset).exp();
                let power = integrate(low, high, 512, |v| forcing.directional_density(v, angle))
                    * TAU
                    / f64::from(DIRECTIONS);
                long.push(Component {
                    wave_vector: [k * angle.cos(), k * angle.sin()],
                    amplitude: (2.0 * power).sqrt(),
                    phase: phase(id),
                });
            }
        }
        let cascades = std::array::from_fn(|band| {
            let length = lengths[band];
            let step = TAU / length;
            let mut modes = Vec::new();
            for zi in 0..255_u32 {
                for xi in 0..255_u32 {
                    let kx = (f64::from(xi) - 127.0) * step;
                    let kz = (f64::from(zi) - 127.0) * step;
                    let k = kx.hypot(kz);
                    if k >= edges[band] && k < edges[band + 1] {
                        let power =
                            forcing.directional_density(k, kz.atan2(kx)) / k.powi(2) * step.powi(2);
                        modes.push(Component {
                            wave_vector: [kx, kz],
                            amplitude: (2.0 * power).sqrt(),
                            phase: phase(zi * 255 + xi),
                        });
                    }
                }
            }
            let target = integrate(edges[band], edges[band + 1], 8192, |k| {
                forcing.log_density(k)
            });
            let actual: f64 = modes.iter().map(|c| c.amplitude.powi(2) * 0.5).sum();
            let scale = if actual > 0.0 {
                (target / actual).sqrt()
            } else {
                0.0
            };
            for mode in &mut modes {
                mode.amplitude *= scale;
            }
            Cascade::from_components(length, modes)
        });
        // The gravity-only spectrum's slope tail diverges logarithmically. A
        // declared 1 cm cutoff bounds this model; capillarity is outside it.
        let tail_slope_variance = integrate(edges[3], TAU / 0.01, 4096, |k| {
            forcing.log_density(k) * k * k
        });
        Ok(Self {
            long,
            cascades,
            tail_slope_variance,
        })
    }

    /// Total represented height variance, metres squared.
    pub fn variance(&self) -> f64 {
        self.long
            .iter()
            .map(|c| 0.5 * c.amplitude.powi(2))
            .sum::<f64>()
            + self.cascades.iter().map(Cascade::variance).sum::<f64>()
    }

    /// Significant height, four times the height standard deviation.
    pub fn significant_height(&self) -> f64 {
        4.0 * self.variance().sqrt()
    }
}

fn phase(id: u32) -> f64 {
    TAU * (f64::from(id + 1) * 0.618_033_988_749_894_9).fract()
}

fn integrate(low: f64, high: f64, count: u32, f: impl Fn(f64) -> f64) -> f64 {
    let step = (high / low).ln() / f64::from(count);
    (0..count)
        .map(|i| f(low * ((f64::from(i) + 0.5) * step).exp()) * step)
        .sum()
}

/// Phase-preserving power response `P(t)=target+(start-target)*exp(-t/tau)`.
/// Retargeting snapshots the current powers, so repeated changes are continuous.
pub struct Transition {
    start: Spectrum,
    target: Spectrum,
    start_time: f64,
    response_seconds: f64,
}

impl Transition {
    /// Begin a response between two spectra with identical vector/phase identities.
    /// # Errors
    /// Rejects incompatible profiles, nonfinite time, or nonpositive response time.
    pub fn new(
        start: Spectrum,
        target: Spectrum,
        time: f64,
        response_seconds: f64,
    ) -> Result<Self, SeaError> {
        let same = |a: &[Component], b: &[Component]| {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.wave_vector.map(f64::to_bits) == b.wave_vector.map(f64::to_bits)
                        && a.phase.to_bits() == b.phase.to_bits()
                })
        };
        if !time.is_finite()
            || !response_seconds.is_finite()
            || response_seconds <= 0.0
            || !same(&start.long, &target.long)
            || !start.cascades.iter().zip(&target.cascades).all(|(a, b)| {
                a.length().to_bits() == b.length().to_bits() && same(a.components(), b.components())
            })
        {
            return Err(SeaError::InvalidInput);
        }
        Ok(Self {
            start,
            target,
            start_time: time,
            response_seconds,
        })
    }

    /// Snapshot powers at an absolute time. Before the start, returns the start.
    /// # Errors
    /// Rejects a nonfinite time.
    pub fn at(&self, time: f64) -> Result<Spectrum, SeaError> {
        if !time.is_finite() {
            return Err(SeaError::InvalidInput);
        }
        let weight = -(-((time - self.start_time).max(0.0)) / self.response_seconds).exp_m1();
        let mut value = self.start.clone();
        let blend = |a: &mut Component, b: &Component| {
            a.amplitude =
                (a.amplitude.powi(2) * (1.0 - weight) + b.amplitude.powi(2) * weight).sqrt();
        };
        for (a, b) in value.long.iter_mut().zip(&self.target.long) {
            blend(a, b);
        }
        for (a, b) in value.cascades.iter_mut().zip(&self.target.cascades) {
            for (a, b) in a.components_mut().iter_mut().zip(b.components()) {
                blend(a, b);
            }
        }
        value.tail_slope_variance = self.start.tail_slope_variance * (1.0 - weight)
            + self.target.tail_slope_variance * weight;
        Ok(value)
    }
}
