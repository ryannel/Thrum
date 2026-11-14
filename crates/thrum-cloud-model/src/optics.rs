use crate::{Error, unit};
use std::f64::consts::PI;

/// Ordered radiative transport through one disjoint interval.
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    /// Transmittance.
    pub transmittance: f64,
    /// Integrated radiance.
    pub scattering: f64,
}

impl Segment {
    /// Analytic Beer segment, using expm1 for the zero-extinction limit.
    /// # Errors
    /// Rejects negative/nonfinite coefficients or length.
    pub fn beer(extinction: f64, source: f64, length: f64) -> Result<Self, Error> {
        if [extinction, source, length]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(Error::Invalid);
        }
        let tau = extinction * length;
        Ok(Self {
            transmittance: (-tau).exp(),
            scattering: source
                * if extinction == 0.0 {
                    length
                } else {
                    -(-tau).exp_m1() / extinction
                },
        })
    }

    /// Compose this near segment with a far segment; order matters.
    #[must_use]
    pub fn then(self, far: Self) -> Self {
        Self {
            transmittance: self.transmittance * far.transmittance,
            scattering: self.scattering + self.transmittance * far.scattering,
        }
    }
}

/// Four optical orders, as the retained optical reference defines them.
pub struct Orders;
impl Orders {
    /// Scattering weights.
    pub const WEIGHTS: [f64; 4] = [0.7, 0.2, 0.07, 0.03];
    /// Extinction multipliers.
    pub const EXTINCTION: [f64; 4] = [1.0, 0.5, 0.25, 0.125];
    /// Later-order asymmetries; first order uses the droplet instead.
    pub const LATER_G: [f64; 3] = [0.43, 0.215, 0.1075];
}

/// Normalised HG plus Draine phase, with game-supplied or fitted parameters.
#[derive(Clone, Copy, Debug)]
pub struct Phase {
    hg: f64,
    draine: f64,
    alpha: f64,
    weight: f64,
}

impl Phase {
    /// Construct a normalised phase.
    /// # Errors
    /// Rejects singular lobes, negative alpha and invalid mixing weights.
    pub fn new(hg: f64, draine: f64, alpha: f64, weight: f64) -> Result<Self, Error> {
        if !hg.is_finite()
            || hg.abs() >= 1.0
            || !draine.is_finite()
            || draine.abs() >= 1.0
            || !alpha.is_finite()
            || alpha < 0.0
            || !unit(weight)
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            hg,
            draine,
            alpha,
            weight,
        })
    }

    /// Jendersie and d'Eon 2023, supplemental equations 7–10, diameter in µm.
    /// # Errors
    /// Rejects diameters outside the fit's published 5..=50 µm range.
    pub fn fit(diameter: f64) -> Result<Self, Error> {
        if !diameter.is_finite() || !(5.0..=50.0).contains(&diameter) {
            return Err(Error::Invalid);
        }
        Self::new(
            (-0.099_056_7 / (diameter - 1.671_54)).exp(),
            (-2.206_79 / (diameter + 3.910_29) - 0.428_934).exp(),
            (3.624_89 - 8.292_88 / (diameter + 5.528_25)).exp(),
            (-0.599_085 / (diameter - 0.641_583) - 0.665_888).exp(),
        )
    }

    /// HG g, Draine g, alpha, Draine mixing weight.
    pub fn parameters(self) -> [f64; 4] {
        [self.hg, self.draine, self.alpha, self.weight]
    }

    /// Point-source phase in sr⁻¹, evaluated stably from sin²(theta/2).
    pub fn point(self, sin_half_squared: f64) -> f64 {
        let s2 = sin_half_squared.clamp(0.0, 1.0);
        let c = 1.0 - 2.0 * s2;
        (1.0 - self.weight) * hg(s2, self.hg)
            + self.weight * hg(s2, self.draine) * (1.0 + self.alpha * c * c)
                / (1.0 + self.alpha * (1.0 + 2.0 * self.draine.powi(2)) / 3.0)
    }

    /// Closed-form mean cosine of the point-source phase.
    pub fn mean_cosine(self) -> f64 {
        let g = self.draine;
        let d = g * (1.0 + self.alpha * (3.0 + 2.0 * g * g) / 5.0)
            / (1.0 + self.alpha * (1.0 + 2.0 * g * g) / 3.0);
        (1.0 - self.weight) * self.hg + self.weight * d
    }

    /// Disc-averaged phase by polar quadrature, with solid-angle weights.
    /// Radius zero is the point-source limit. Preparation only, never per sample.
    pub fn disc(self, theta: f64, radius: f64) -> f64 {
        if radius == 0.0 {
            return self.point((theta * 0.5).sin().powi(2));
        }
        let rule = gauss(24);
        let mut sum = 0.0;
        // sin(radius/2)^2 avoids cancellation in the disc solid angle.
        let solid_angle = 4.0 * PI * (radius * 0.5).sin().powi(2);
        for (node, weight) in rule {
            let r = 0.5 * radius * (node + 1.0);
            let radial_weight = radius * 0.5 * weight * r.sin() * 2.0 * PI / solid_angle;
            let mut ring = 0.0;
            for j in 0..48 {
                let phi = (f64::from(j) + 0.5) * PI / 48.0;
                let s2 = ((theta - r) * 0.5).sin().powi(2)
                    + theta.sin() * r.sin() * (phi * 0.5).sin().powi(2);
                ring += self.point(s2) / 48.0;
            }
            sum += radial_weight * ring;
        }
        sum
    }
}

fn hg(s2: f64, g: f64) -> f64 {
    (1.0 - g * g) / (4.0 * PI * ((1.0 - g).powi(2) + 4.0 * g * s2).powf(1.5))
}

/// Renderer-ready f32 phase table, uniform in u = sin(theta/2)^(1/2).
/// The renderer computes u from its stable sin²(theta/2), then linearly samples.
pub struct PhaseTable {
    values: Vec<f32>,
}

impl PhaseTable {
    /// Prepare a disc-averaged table; 1,025 entries are the design proof's size.
    /// # Errors
    /// Rejects invalid radius or fewer than two entries.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "renderer table deliberately stores f32"
    )]
    pub fn new(phase: Phase, radius: f64, entries: u32) -> Result<Self, Error> {
        if !radius.is_finite() || !(0.0..=0.1).contains(&radius) || !(2..=65_537).contains(&entries)
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            values: (0..entries)
                .map(|i| {
                    let u = f64::from(i) / f64::from(entries - 1);
                    phase.disc(2.0 * (u * u).asin(), radius) as f32
                })
                .collect(),
        })
    }

    /// Values to upload unchanged to a renderer.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// The f32 linear interpolation the renderer uses, from sin²(theta/2).
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded table coordinate and f32 renderer arithmetic"
    )]
    pub fn sample(&self, sin_half_squared: f32) -> f32 {
        let x = sin_half_squared.clamp(0.0, 1.0).sqrt().sqrt() * (self.values.len() - 1) as f32;
        let i = (x as usize).min(self.values.len() - 2);
        let f = x - i as f32;
        self.values[i] + f * (self.values[i + 1] - self.values[i])
    }
}

fn gauss(order: u32) -> Vec<(f64, f64)> {
    (0..order)
        .map(|i| {
            let mut z = (PI * (f64::from(i) + 0.75) / (f64::from(order) + 0.5)).cos();
            let mut derivative = 0.0;
            for _ in 0..16 {
                let mut p = 1.0;
                let mut previous = 0.0;
                for n in 1..=order {
                    let next = ((2.0 * f64::from(n) - 1.0) * z * p
                        - (f64::from(n) - 1.0) * previous)
                        / f64::from(n);
                    previous = p;
                    p = next;
                }
                derivative = f64::from(order) * (z * p - previous) / (z * z - 1.0);
                let delta = p / derivative;
                z -= delta;
                if delta.abs() < 1e-15 {
                    break;
                }
            }
            (z, 2.0 / ((1.0 - z * z) * derivative * derivative))
        })
        .collect()
}
