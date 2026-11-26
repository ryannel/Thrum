//! A raindrop's rings, from water's own dispersion: the drop gives the water an impulse,
//! its mass times its fall speed, spread as a Gaussian over its crater, and the water
//! answers each wavenumber `k` as `h'(k, t) = (P k tanh(k d) / (ρ ω)) sin(ω t) e^(-γ t)`,
//! with `ω² = (g k + σ k³ / ρ) tanh(k d)` and viscosity's decay `γ`. The height at a radius
//! is that spectrum's Hankel transform. Nothing is tuned: the crater's width is the one
//! closure. The tables the rain bands read are made here and handed to
//! `thrum-water-model` as data.
use std::f64::consts::PI;
use thrum_water_model::{RingClass, RingTable};

/// Water's constants, SI units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Water {
    /// Gravity, metres a second squared.
    pub gravity: f64,
    /// Surface tension over density, cubic metres a second squared.
    pub tension: f64,
    /// Kinematic viscosity, square metres a second.
    pub viscosity: f64,
    /// Density, kilograms a cubic metre.
    pub density: f64,
}

impl Default for Water {
    fn default() -> Self {
        Self {
            gravity: 9.81,
            tension: 7.28e-5,
            viscosity: 1.0e-6,
            density: 1000.0,
        }
    }
}

fn depth_factor(k: f64, d: f64) -> f64 {
    if d.is_infinite() { 1.0 } else { (k * d).tanh() }
}

impl Water {
    /// Angular frequency of a wave of wavenumber `k` on water `d` deep.
    pub fn omega(&self, k: f64, d: f64) -> f64 {
        ((self.gravity * k + self.tension * k.powi(3)) * depth_factor(k, d)).sqrt()
    }

    /// Group speed, `dω/dk`.
    pub fn group_speed(&self, k: f64, d: f64) -> f64 {
        let t = depth_factor(k, d);
        let sech2 = if d.is_infinite() { 0.0 } else { 1.0 - t * t };
        let f = self.gravity * k + self.tension * k.powi(3);
        let df = self.gravity + 3.0 * self.tension * k * k;
        (df * t + f * d.min(1e300) * sech2) / (2.0 * self.omega(k, d))
    }

    /// Viscous decay rate of a wave's height: the surface's and, on shallow water, the
    /// bed's boundary layer.
    pub fn damping(&self, k: f64, d: f64) -> f64 {
        let x = 2.0 * k * d;
        let bed = if x < 50.0 {
            k * (self.viscosity * self.omega(k, d) / 2.0).sqrt() / x.sinh()
        } else {
            0.0
        };
        2.0 * self.viscosity * k * k + bed
    }

    /// The least group speed on water `d` deep, and its wavenumber.
    pub fn least_group_speed(&self, d: f64) -> (f64, f64) {
        let (mut best, mut at) = (f64::INFINITY, 0.0);
        let mut k = 1.0;
        while k < 4000.0 {
            let c = self.group_speed(k, d);
            if c < best {
                (best, at) = (c, k);
            }
            k *= 1.01;
        }
        // Refine by golden section on the bracket round the coarse minimum.
        let (mut a, mut b) = (at / 1.01, at * 1.01);
        let g = 0.5 * (5.0_f64.sqrt() - 1.0);
        for _ in 0..80 {
            let (c, d2) = (b - g * (b - a), a + g * (b - a));
            if self.group_speed(c, d) < self.group_speed(d2, d) {
                b = d2;
            } else {
                a = c;
            }
        }
        let k = f64::midpoint(a, b);
        (self.group_speed(k, d), k)
    }
}

/// A falling raindrop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Raindrop {
    /// Diameter, metres.
    pub diameter: f64,
}

impl Raindrop {
    /// Mass, kilograms.
    pub fn mass(self, water: &Water) -> f64 {
        water.density * PI / 6.0 * self.diameter.powi(3)
    }

    /// Fall speed, metres a second (Atlas, Srivastava and Sekhon 1973).
    pub fn speed(self) -> f64 {
        (9.65 - 10.3 * (-0.6 * self.diameter * 1000.0).exp()).max(0.0)
    }

    /// Momentum, newton seconds.
    pub fn momentum(self, water: &Water) -> f64 {
        self.mass(water) * self.speed()
    }

    /// Kinetic energy, joules.
    pub fn energy(self, water: &Water) -> f64 {
        0.5 * self.mass(water) * self.speed().powi(2)
    }
}

/// Bessel function of the first kind, order 0 (rational approximations).
pub(crate) fn j0(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 8.0 {
        let y = x * x;
        let a = 57_568_490_574.0
            + y * (-13_362_590_354.0
                + y * (651_619_640.7
                    + y * (-11_214_424.18 + y * (77_392.330_17 + y * -184.905_245_6))));
        let b = 57_568_490_411.0
            + y * (1_029_532_985.0
                + y * (9_494_680.718 + y * (59_272.648_53 + y * (267.853_271_2 + y))));
        a / b
    } else {
        let z = 8.0 / ax;
        let y = z * z;
        let xx = ax - 0.785_398_164;
        let a = 1.0
            + y * (-0.109_862_862_7e-2
                + y * (0.273_451_040_7e-4 + y * (-0.207_337_063_9e-5 + y * 0.209_388_721_1e-6)));
        let b = -0.156_249_999_5e-1
            + y * (0.143_048_876_5e-3
                + y * (-0.691_114_765_1e-5 + y * (0.762_109_516_1e-6 - y * 0.934_935_152e-7)));
        (std::f64::consts::FRAC_2_PI / ax).sqrt() * (xx.cos() * a - z * xx.sin() * b)
    }
}

/// Bessel function of the first kind, order 1 (rational approximations).
pub(crate) fn j1(x: f64) -> f64 {
    let ax = x.abs();
    let v = if ax < 8.0 {
        let y = x * x;
        let a = ax
            * (72_362_614_232.0
                + y * (-7_895_059_235.0
                    + y * (242_396_853.1
                        + y * (-2_972_611.439 + y * (15_704.482_60 + y * -30.160_366_06)))));
        let b = 144_725_228_442.0
            + y * (2_300_535_178.0
                + y * (18_583_304.74 + y * (99_447.433_94 + y * (376.999_139_7 + y))));
        a / b
    } else {
        let z = 8.0 / ax;
        let y = z * z;
        let xx = ax - 2.356_194_491;
        let a = 1.0
            + y * (0.183_105e-2
                + y * (-0.351_639_649_6e-4 + y * (0.245_752_017_4e-5 + y * -0.240_337_019e-6)));
        let b = 0.046_874_999_95
            + y * (-0.200_269_087_3e-3
                + y * (0.844_919_909_6e-5 + y * (-0.882_289_87e-6 + y * 0.105_787_412e-6)));
        (std::f64::consts::FRAC_2_PI / ax).sqrt() * (xx.cos() * a - z * xx.sin() * b)
    };
    if x < 0.0 { -v } else { v }
}

/// The wavenumbers a ring is summed over: `step, 2 step, ...` up to `max`, radians a
/// metre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wavenumbers {
    /// Spacing.
    pub step: f64,
    /// Largest.
    pub max: f64,
}

impl Default for Wavenumbers {
    fn default() -> Self {
        Self {
            step: 1.5,
            max: 2400.0,
        }
    }
}

impl Wavenumbers {
    fn iter(self) -> impl Iterator<Item = f64> {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count of positive wavenumbers"
        )]
        let n = (self.max / self.step).floor() as usize;
        (1..=n).map(move |i| i as f64 * self.step)
    }
}

/// One drop's rings on water of one depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    /// The water.
    pub water: Water,
    /// The drop's impulse, newton seconds.
    pub impulse: f64,
    /// The Gaussian radius of the crater the impulse is spread over, metres.
    pub crater: f64,
    /// Still depth, metres; infinite for deep water.
    pub depth: f64,
    /// The wavenumbers summed over.
    pub grid: Wavenumbers,
}

impl Ring {
    /// A drop's rings, its impulse spread over a crater of `crater_radii` drop radii.
    pub fn of(water: Water, drop: Raindrop, crater_radii: f64, depth: f64) -> Self {
        Self {
            water,
            impulse: drop.momentum(&water),
            crater: crater_radii * 0.5 * drop.diameter,
            depth,
            grid: Wavenumbers::default(),
        }
    }

    /// The impulse's spectrum at a wavenumber.
    fn source(&self, k: f64) -> f64 {
        self.impulse * (-0.5 * k * k * self.crater * self.crater).exp()
    }

    /// The height spectrum at a wavenumber and time, before its viscous decay and phase:
    /// `P̂ k tanh(k d) / (ρ ω)`.
    fn amplitude(&self, k: f64) -> f64 {
        let w = self.water.omega(k, self.depth);
        self.source(k) * k * depth_factor(k, self.depth) / (self.water.density * w)
    }

    /// Height at a radius and time after the drop, metres.
    pub fn height(&self, r: f64, t: f64) -> f64 {
        let mut s = 0.0;
        for k in self.grid.iter() {
            let w = self.water.omega(k, self.depth);
            let decay = (-self.water.damping(k, self.depth) * t).exp();
            s += self.amplitude(k) * (w * t).sin() * decay * j0(k * r) * k;
        }
        -s * self.grid.step / (2.0 * PI)
    }

    /// The highest ring over radii from 0 to `reach`, sampled every `step`, at a time.
    pub fn highest(&self, t: f64, reach: f64, step: f64) -> f64 {
        let mut best = 0.0_f64;
        let mut r = 0.0;
        while r <= reach {
            best = best.max(self.height(r, t).abs());
            r += step;
        }
        best
    }

    /// The energy the rings carry on deep water: the impulse's kinetic energy,
    /// `P̂² k² / (4 π ρ)` summed over the wavenumbers.
    pub fn energy(self) -> f64 {
        self.grid
            .iter()
            .map(|k| self.source(k).powi(2) * k * k)
            .sum::<f64>()
            * self.grid.step
            / (4.0 * PI * self.water.density)
    }
}

/// How the ring tables are made: the physical profile of the rain bands' rings.
#[derive(Clone, Debug, PartialEq)]
pub struct RingProfile {
    /// The water.
    pub water: Water,
    /// The crater's Gaussian radius, in drop radii.
    pub crater_radii: f64,
    /// The wavenumbers summed over.
    pub grid: Wavenumbers,
    /// Spacing of the table's radii, metres.
    pub radius_step: f64,
    /// Radii in the table; the last is a ring's reach.
    pub radii: usize,
    /// Spacing of the table's ages, seconds.
    pub age_step: f64,
    /// A ring's life, seconds.
    pub life: f64,
    /// Over this much of its reach, metres, a ring fades to nothing.
    pub edge: f64,
    /// Over this much of its life, seconds, a ring fades to nothing.
    pub fade: f64,
    /// Scheduled drops a square metre a second at full share.
    pub count: f64,
    /// The scheduled drops' size classes: diameter bounds, millimetres.
    pub classes: Vec<[f64; 2]>,
    /// The rain, millimetres an hour, whose drops set the classes' shares and sizes.
    pub reference_rate: f64,
    /// The drop, millimetres, and the age, seconds, whose highest ring sets the depth
    /// response.
    pub response_drop: f64,
    /// See `response_drop`.
    pub response_age: f64,
    /// From this depth, metres, the response is 1.
    pub deep: f64,
}

impl Default for RingProfile {
    fn default() -> Self {
        Self {
            water: Water::default(),
            crater_radii: 3.0,
            grid: Wavenumbers::default(),
            radius_step: 0.002,
            radii: 161,
            age_step: 1.0 / 240.0,
            life: 1.0,
            edge: 0.04,
            fade: 0.1,
            count: 128.0,
            classes: vec![[2.0, 2.5], [2.5, 3.25], [3.25, 6.0]],
            reference_rate: 10.0,
            response_drop: 3.0,
            response_age: 0.4,
            deep: 0.01,
        }
    }
}

/// Marshall and Palmer's drops: those of diameter `d` millimetres that land on a square
/// metre each second in rain of `rate` millimetres an hour, per millimetre of diameter.
pub fn marshall_palmer(rate: f64, d: f64) -> f64 {
    if rate <= 0.0 {
        return 0.0;
    }
    let lambda = 4.1 * rate.powf(-0.21);
    8000.0
        * (-lambda * d).exp()
        * Raindrop {
            diameter: d / 1000.0,
        }
        .speed()
}

/// Drops of `from` millimetres and over a square metre a second, by the midpoint rule in
/// steps of 0.05 mm up to 8 mm.
pub fn drops_over(rate: f64, from: f64) -> f64 {
    let step = 0.05;
    let mut d = from + 0.5 * step;
    let mut n = 0.0;
    while d < 8.0 {
        n += marshall_palmer(rate, d) * step;
        d += step;
    }
    n
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl RingProfile {
    /// The depth response at a depth: the highest ring of the response drop at the
    /// response age on that depth over its highest on deep water; none on dry ground and
    /// 1 from the profile's deep.
    pub fn response(&self, depth: f64) -> f64 {
        if depth <= 0.0 {
            return 0.0;
        }
        if depth >= self.deep {
            return 1.0;
        }
        let drop = Raindrop {
            diameter: self.response_drop / 1000.0,
        };
        let at = |d: f64| {
            let mut ring = Ring::of(self.water, drop, self.crater_radii, d);
            ring.grid = self.grid;
            ring.highest(self.response_age, 0.36, 0.004)
        };
        (at(depth) / at(f64::INFINITY)).min(1.0)
    }

    /// The share of the schedule that falls at a rain rate in millimetres an hour.
    pub fn scheduled_share(&self, rate: f64) -> f64 {
        (drops_over(rate, 2.0) / self.count).min(1.0)
    }

    /// The tables the rain bands read.
    #[expect(
        clippy::too_many_lines,
        reason = "one pass over classes, ages and radii, then the curves"
    )]
    pub fn table(&self) -> RingTable {
        let w = self.water;
        let ks: Vec<f64> = self.grid.iter().collect();
        let dk = self.grid.step;
        let radii: Vec<f64> = (0..self.radii)
            .map(|i| i as f64 * self.radius_step)
            .collect();
        let reach = radii[radii.len() - 1];
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count of ages in a ring's life"
        )]
        let ages = (self.life / self.age_step).round() as usize + 1;
        // Bessel tables, by wavenumber then radius.
        let nr = radii.len();
        let mut b0 = vec![0.0; ks.len() * nr];
        let mut b1 = vec![0.0; ks.len() * nr];
        for (i, &k) in ks.iter().enumerate() {
            for (j, &r) in radii.iter().enumerate() {
                b0[i * nr + j] = j0(k * r) * k;
                b1[i * nr + j] = -j1(k * r) * k * k;
            }
        }
        // The classes' shares and sizes at the reference rate.
        let mut classes = Vec::new();
        let mut total = 0.0;
        for &[lo, hi] in &self.classes {
            let (mut n, mut sum) = (0.0, 0.0);
            let mut d = lo + 0.025;
            while d < hi {
                let m = marshall_palmer(self.reference_rate, d) * 0.05;
                n += m;
                sum += m * d;
                d += 0.05;
            }
            total += n;
            classes.push((n, sum / n));
        }
        let taper = |r: f64| smooth((reach - r) / self.edge);
        let taper_dr = |r: f64| {
            let x = ((reach - r) / self.edge).clamp(0.0, 1.0);
            if x <= 0.0 || x >= 1.0 {
                0.0
            } else {
                -6.0 * x * (1.0 - x) / self.edge
            }
        };
        let fade = |t: f64| smooth((self.life - t) / self.fade);
        let fade_dt = |t: f64| {
            let x = ((self.life - t) / self.fade).clamp(0.0, 1.0);
            if x <= 0.0 || x >= 1.0 {
                0.0
            } else {
                -6.0 * x * (1.0 - x) / self.fade
            }
        };
        let rings: Vec<RingClass> = classes
            .iter()
            .map(|&(n, d)| {
                let ring = Ring {
                    grid: self.grid,
                    ..Ring::of(
                        w,
                        Raindrop {
                            diameter: d / 1000.0,
                        },
                        self.crater_radii,
                        f64::INFINITY,
                    )
                };
                let amp: Vec<f64> = ks.iter().map(|&k| ring.amplitude(k)).collect();
                let (mut height, mut slope, mut rate) = (
                    vec![0.0; ages * nr],
                    vec![0.0; ages * nr],
                    vec![0.0; ages * nr],
                );
                let mut variance = vec![0.0; ages];
                let mut sin = vec![0.0; ks.len()];
                let mut dsin = vec![0.0; ks.len()];
                for a in 0..ages {
                    let t = a as f64 * self.age_step;
                    let mut var = 0.0;
                    for (i, &k) in ks.iter().enumerate() {
                        let om = w.omega(k, f64::INFINITY);
                        let g = w.damping(k, f64::INFINITY);
                        let e = (-g * t).exp();
                        let c = -amp[i] * dk / (2.0 * PI);
                        sin[i] = c * (om * t).sin() * e;
                        dsin[i] = c * (om * (om * t).cos() - g * (om * t).sin()) * e;
                        var += k * (amp[i] * e).powi(2) * 0.5 * dk / (2.0 * PI);
                    }
                    let (f, fd) = (fade(t), fade_dt(t));
                    for (j, &r) in radii.iter().enumerate() {
                        let (mut h, mut s, mut ht) = (0.0, 0.0, 0.0);
                        for i in 0..ks.len() {
                            h += sin[i] * b0[i * nr + j];
                            s += sin[i] * b1[i * nr + j];
                            ht += dsin[i] * b0[i * nr + j];
                        }
                        let (tp, tpd) = (taper(r), taper_dr(r));
                        height[a * nr + j] = h * tp * f;
                        slope[a * nr + j] = (s * tp + h * tpd) * f;
                        rate[a * nr + j] = (ht * f + h * fd) * tp;
                    }
                    variance[a] = var * f * f;
                }
                RingClass {
                    share: n / total,
                    height,
                    slope,
                    rate,
                    variance,
                }
            })
            .collect();
        // How a drop's squared slope spreads over its age, on deep water, for the
        // response drop; and each drop size's whole squared slope over its life.
        let slope_at = |drop: Raindrop, t: f64| {
            let ring = Ring {
                grid: self.grid,
                ..Ring::of(w, drop, self.crater_radii, f64::INFINITY)
            };
            ks.iter()
                .map(|&k| {
                    let e = (-w.damping(k, f64::INFINITY) * t).exp();
                    k.powi(3) * (ring.amplitude(k) * e).powi(2) * 0.5
                })
                .sum::<f64>()
                * dk
                / (2.0 * PI)
        };
        let reference = Raindrop {
            diameter: self.response_drop / 1000.0,
        };
        let mut moment_age: Vec<f64> = (0..ages)
            .map(|a| {
                let age = a as f64 * self.age_step;
                // Spreading small waves gradually occupy more of the pixel.
                // Fade their diffuse energy ahead of the one-second cutoff, so
                // the last sliver cannot hold the direct reflection open.
                // Normalization below preserves the steady slope variance.
                let growth = smooth(age / (0.20 * self.life)).powi(2);
                let decay = smooth((self.life - age) / (0.55 * self.life)).powi(4);
                slope_at(reference, age) * growth * decay
            })
            .collect();
        let norm: f64 = moment_age.iter().sum::<f64>() * self.age_step;
        for v in &mut moment_age {
            *v /= norm;
        }
        let life_slope = |drop: Raindrop| {
            let ring = Ring {
                grid: self.grid,
                ..Ring::of(w, drop, self.crater_radii, f64::INFINITY)
            };
            ks.iter()
                .map(|&k| {
                    let g = w.damping(k, f64::INFINITY);
                    let whole = (1.0 - (-2.0 * g * self.life).exp()) / (2.0 * g);
                    k.powi(3) * ring.amplitude(k).powi(2) * 0.5 * whole
                })
                .sum::<f64>()
                * dk
                / (2.0 * PI)
        };
        let sizes: Vec<(f64, f64)> = (0..120)
            .map(|i| {
                let d = 0.025 + 0.05 * f64::from(i);
                (
                    d,
                    life_slope(Raindrop {
                        diameter: d / 1000.0,
                    }),
                )
            })
            .collect();
        let rates = [
            0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0, 50.0, 75.0, 100.0, 200.0,
        ];
        let flux = |r: f64| r / 3600.0;
        let moments = std::iter::once([0.0, 0.0])
            .chain(rates.iter().map(|&r| {
                // The cut: the largest drops up to the count are scheduled, none under
                // 2 mm.
                let mut cut = 2.0;
                while drops_over(r, cut) > self.count {
                    cut += 0.05;
                }
                let m: f64 = sizes
                    .iter()
                    .filter(|(d, _)| *d < cut)
                    .map(|&(d, s)| marshall_palmer(r, d) * 0.05 * s)
                    .sum();
                [flux(r), m]
            }))
            .collect();
        let scheduled = std::iter::once([0.0, 0.0])
            .chain(rates.iter().map(|&r| [flux(r), self.scheduled_share(r)]))
            .collect();
        let depths = [
            0.0, 0.0005, 0.001, 0.0015, 0.002, 0.0025, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008,
            0.009,
        ];
        let depth_response = depths
            .iter()
            .map(|&d| [d, self.response(d)])
            .chain(std::iter::once([self.deep, 1.0]))
            .collect();
        RingTable {
            radius_step: self.radius_step,
            radii: self.radii,
            age_step: self.age_step,
            ages,
            classes: rings,
            depth_response,
            scheduled,
            moments,
            moment_age,
        }
    }
}
