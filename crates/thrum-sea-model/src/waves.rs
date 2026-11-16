use crate::GRAVITY;
use std::f64::consts::TAU;

/// A rejected input or an unsupported overturned surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeaError {
    /// All parameters must be finite and inside their documented ranges.
    InvalidInput,
    /// Horizontal displacement is not contractive, or iteration did not converge.
    NonConvergent,
}
impl std::fmt::Display for SeaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidInput => "sea input is nonfinite or outside its supported range",
            Self::NonConvergent => {
                "sea displacement is not contractive or its inverse did not converge"
            }
        })
    }
}
impl std::error::Error for SeaError {}

/// One travelling deep-water Gerstner component, with fixed vector and phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Component {
    /// Horizontal wave vector, radians per metre.
    pub wave_vector: [f64; 2],
    /// Vertical amplitude in metres; nonnegative.
    pub amplitude: f64,
    /// Initial phase in radians.
    pub phase: f64,
}

impl Component {
    /// Deep-water angular frequency, radians per second.
    pub fn frequency(self) -> f64 {
        (GRAVITY * self.wavenumber()).sqrt()
    }

    pub(crate) fn wavenumber(self) -> f64 {
        self.wave_vector[0].hypot(self.wave_vector[1])
    }

    /// Phase at an anchor, reduced in f64 before a GPU adds local coordinates.
    pub fn anchored_phase(self, anchor: [f64; 2], time: f64) -> f64 {
        (self.wave_vector[0].mul_add(anchor[0], self.wave_vector[1] * anchor[1])
            - self.frequency() * time
            + self.phase)
            .rem_euclid(TAU)
    }
}

/// Surface quantities evaluated above a fixed world x/z point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    /// Height in metres.
    pub height: f64,
    /// World-space height gradient, dh/dx and dh/dz.
    pub slope: [f64; 2],
    /// Material surface velocity in x/y/z order, metres per second.
    pub velocity: [f64; 3],
    /// Eulerian height derivative at fixed world x/z, metres per second.
    pub height_dt: f64,
}

/// A contractive Gerstner sum. Its component identities never depend on forcing.
#[derive(Clone, Debug)]
pub struct LongWaves {
    components: Vec<Component>,
    choppiness: f64,
}

/// Uniform spectral-norm bound on the horizontal displacement's differential.
/// For M = sum(q*a/k * k*k^T), every phase has -M <= dD <= M. The largest
/// eigenvalue of this positive-semidefinite matrix is therefore a Lipschitz
/// bound. Unlike its trace, it does not add orthogonal directions as if aligned.
/// # Errors
/// Rejects nonfinite or negative amplitude/choppiness and zero/nonfinite vectors.
pub fn contraction_bound(components: &[Component], choppiness: f64) -> Result<f64, SeaError> {
    if !choppiness.is_finite() || choppiness < 0.0 {
        return Err(SeaError::InvalidInput);
    }
    let mut matrix = [0.0; 3];
    for c in components {
        let [x, z] = c.wave_vector;
        let k = x.hypot(z);
        if !c.amplitude.is_finite() || c.amplitude < 0.0 || !k.is_finite() || k == 0.0 {
            return Err(SeaError::InvalidInput);
        }
        let scale = choppiness * c.amplitude / k;
        matrix[0] += scale * x * x;
        matrix[1] += scale * x * z;
        matrix[2] += scale * z * z;
    }
    let bound = matrix[0].midpoint(matrix[2]) + matrix[0].midpoint(-matrix[2]).hypot(matrix[1]);
    if !bound.is_finite() {
        return Err(SeaError::InvalidInput);
    }
    Ok(bound)
}

impl LongWaves {
    /// Construct a sum. Choppiness is nonnegative; [`contraction_bound`] must
    /// be below one so inversion above a world point has a unique solution.
    ///
    /// # Errors
    /// Rejects nonfinite/negative input, zero vectors, or a noncontractive sum.
    pub fn new(components: Vec<Component>, choppiness: f64) -> Result<Self, SeaError> {
        if !choppiness.is_finite()
            || choppiness < 0.0
            || components.iter().any(|c| {
                !c.amplitude.is_finite()
                    || c.amplitude < 0.0
                    || !c.phase.is_finite()
                    || c.wave_vector.iter().any(|v| !v.is_finite())
                    || c.wavenumber() == 0.0
            })
        {
            return Err(SeaError::InvalidInput);
        }
        let steepness = contraction_bound(&components, choppiness)?;
        if steepness >= 1.0 {
            return Err(SeaError::NonConvergent);
        }
        Ok(Self {
            components,
            choppiness,
        })
    }

    /// Fixed component identities, amplitudes and phases.
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// Horizontal orbital displacement as a share of each component's amplitude.
    pub fn choppiness(&self) -> f64 {
        self.choppiness
    }

    /// Amplitude-scaled copy preserving component identities and choppiness.
    /// # Errors
    /// Rejects a nonfinite gain, a gain outside zero to one, or invalid waves.
    pub fn scaled(&self, gain: f64) -> Result<Self, SeaError> {
        if !gain.is_finite() || !(0.0..=1.0).contains(&gain) {
            return Err(SeaError::InvalidInput);
        }
        let mut components = self.components.clone();
        for c in &mut components {
            c.amplitude *= gain;
        }
        Self::new(components, self.choppiness)
    }

    /// Evaluate the parametric sheet once at a rest place, without inversion.
    /// Derivatives are with respect to that rest place, before the horizontal map.
    /// # Errors
    /// Rejects nonfinite coordinates or time.
    pub fn forward(&self, rest: [f64; 2], time: f64) -> Result<crate::Differential, SeaError> {
        if !time.is_finite() || rest.iter().any(|v| !v.is_finite()) {
            return Err(SeaError::InvalidInput);
        }
        let mut result = crate::Differential::default();
        for c in &self.components {
            let (s, co) = c.anchored_phase(rest, time).sin_cos();
            let k = c.wavenumber();
            let q = self.choppiness;
            let a = c.amplitude;
            result.displacement[1] += a * co;
            result.velocity[1] += a * c.frequency() * s;
            for axis in 0..2 {
                result.displacement[axis * 2] -= q * a * c.wave_vector[axis] / k * s;
                result.slope[axis] -= a * c.wave_vector[axis] * s;
                result.velocity[axis * 2] += q * a * c.frequency() * c.wave_vector[axis] / k * co;
            }
            result.horizontal_derivative[0] -= q * a * c.wave_vector[0] * c.wave_vector[0] / k * co;
            result.horizontal_derivative[1] -= q * a * c.wave_vector[0] * c.wave_vector[1] / k * co;
            result.horizontal_derivative[2] -= q * a * c.wave_vector[1] * c.wave_vector[1] / k * co;
        }
        let [x, c, z] = result.horizontal_derivative;
        result.jacobian = (1.0 + x) * (1.0 + z) - c * c;
        Ok(result)
    }

    /// Evaluate at a world point, inverting horizontal displacement to 1e-11 m.
    /// `velocity` follows the material label; `height_dt` holds world x/z fixed.
    ///
    /// # Errors
    /// Rejects nonfinite points/time or failure to converge within 4096 iterations.
    pub fn evaluate(&self, point: [f64; 2], time: f64) -> Result<Sample, SeaError> {
        if !time.is_finite() || point.iter().any(|v| !v.is_finite()) {
            return Err(SeaError::InvalidInput);
        }
        // Anchor at the query: even far from the origin the iteration handles
        // local offsets, rather than losing convergence to world-coordinate ulps.
        let phases: Vec<_> = self
            .components
            .iter()
            .map(|c| c.anchored_phase(point, time))
            .collect();
        let mut offset = [0.0; 2];
        for _ in 0..4096 {
            let mut next = [0.0; 2];
            for (c, phase) in self.components.iter().zip(&phases) {
                let theta =
                    c.wave_vector[0].mul_add(offset[0], c.wave_vector[1] * offset[1]) + phase;
                for (axis, value) in next.iter_mut().enumerate() {
                    *value += self.choppiness * c.amplitude * c.wave_vector[axis] / c.wavenumber()
                        * theta.sin();
                }
            }
            let error = (next[0] - offset[0]).abs().max((next[1] - offset[1]).abs());
            offset = next;
            if error < 1e-11 {
                return Ok(self.sample(offset, &phases));
            }
        }
        Err(SeaError::NonConvergent)
    }

    fn sample(&self, offset: [f64; 2], phases: &[f64]) -> Sample {
        let mut sample = Sample::default();
        let mut gradient = [0.0; 2];
        let mut jac = [[1.0, 0.0], [0.0, 1.0]];
        for (c, phase) in self.components.iter().zip(phases) {
            let (sin, cos) = (c.wave_vector[0].mul_add(offset[0], c.wave_vector[1] * offset[1])
                + phase)
                .sin_cos();
            let k = c.wavenumber();
            let omega = c.frequency();
            sample.height += c.amplitude * cos;
            sample.velocity[1] += c.amplitude * omega * sin;
            for axis in 0..2 {
                gradient[axis] -= c.amplitude * c.wave_vector[axis] * sin;
                sample.velocity[axis * 2] +=
                    self.choppiness * c.amplitude * c.wave_vector[axis] / k * omega * cos;
                for (other, value) in jac[axis].iter_mut().enumerate() {
                    *value -=
                        self.choppiness * c.amplitude * c.wave_vector[axis] * c.wave_vector[other]
                            / k
                            * cos;
                }
            }
        }
        let det = jac[0][0] * jac[1][1] - jac[0][1] * jac[1][0];
        sample.slope = [
            (gradient[0] * jac[1][1] - gradient[1] * jac[1][0]) / det,
            (gradient[1] * jac[0][0] - gradient[0] * jac[0][1]) / det,
        ];
        sample.height_dt = sample.velocity[1]
            - sample.slope[0] * sample.velocity[0]
            - sample.slope[1] * sample.velocity[2];
        sample
    }
}
