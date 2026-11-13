use crate::{Component, SeaError};
use std::f64::consts::TAU;

/// Transform width and height of the physical profile.
pub const GRID_SIZE: usize = 256;

/// Parametric displacement and its spatial differential at a surface label.
#[derive(Clone, Copy, Debug, Default)]
pub struct Differential {
    /// Horizontal x, vertical y, horizontal z displacement in metres.
    pub displacement: [f64; 3],
    /// Parametric height gradient (divide by the horizontal Jacobian for world slopes).
    pub slope: [f64; 2],
    /// Derivatives of horizontal displacement: dX/dx, dX/dz, dZ/dz.
    pub horizontal_derivative: [f64; 3],
    /// Determinant of identity plus the horizontal displacement differential.
    pub jacobian: f64,
}

impl Differential {
    fn from_channels(v: [f64; 8]) -> Self {
        Self {
            displacement: [v[0], v[1], v[2]],
            slope: [v[3], v[4]],
            horizontal_derivative: [v[5], v[6], v[7]],
            jacobian: (1.0 + v[5]) * (1.0 + v[7]) - v[6] * v[6],
        }
    }
}

/// A band of travelling modes on a periodic 256-square lattice.
#[derive(Clone, Debug)]
pub struct Cascade {
    length: f64,
    modes: Vec<Component>,
}

impl Cascade {
    /// Construct a reference band from signed lattice indices, variance and phase.
    /// # Errors
    /// Rejects nonfinite inputs, zero vectors, negative power and Nyquist modes.
    pub fn new(length: f64, modes: &[(i32, i32, f64, f64)]) -> Result<Self, SeaError> {
        if !length.is_finite()
            || length <= 0.0
            || modes.iter().any(|&(x, z, p, phase)| {
                (x == 0 && z == 0)
                    || x.unsigned_abs() >= 128
                    || z.unsigned_abs() >= 128
                    || !p.is_finite()
                    || p < 0.0
                    || !phase.is_finite()
            })
        {
            return Err(SeaError::InvalidInput);
        }
        Ok(Self::from_components(
            length,
            modes
                .iter()
                .map(|&(x, z, p, phase)| Component {
                    wave_vector: [f64::from(x) * TAU / length, f64::from(z) * TAU / length],
                    amplitude: (2.0 * p).sqrt(),
                    phase,
                })
                .collect(),
        ))
    }

    pub(crate) fn from_components(length: f64, modes: Vec<Component>) -> Self {
        Self { length, modes }
    }
    /// Patch side in metres.
    pub fn length(&self) -> f64 {
        self.length
    }
    /// Fixed travelling-mode identities and their amplitudes.
    pub fn components(&self) -> &[Component] {
        &self.modes
    }
    pub(crate) fn components_mut(&mut self) -> &mut [Component] {
        &mut self.modes
    }
    /// Height variance in square metres.
    pub fn variance(&self) -> f64 {
        self.modes.iter().map(|c| c.amplitude.powi(2) * 0.5).sum()
    }

    /// Direct f64 sum at a parametric point. Choppiness is one.
    pub fn direct(&self, point: [f64; 2], time: f64) -> Differential {
        let mut values = [0.0; 8];
        for c in &self.modes {
            let (sin, cos) = c.anchored_phase(point, time).sin_cos();
            let [kx, kz] = c.wave_vector;
            let k = kx.hypot(kz);
            let a = c.amplitude;
            let terms = [
                -a * kx / k * sin,
                a * cos,
                -a * kz / k * sin,
                -a * kx * sin,
                -a * kz * sin,
                -a * kx * kx / k * cos,
                -a * kx * kz / k * cos,
                -a * kz * kz / k * cos,
            ];
            for (value, term) in values.iter_mut().zip(terms) {
                *value += term;
            }
        }
        Differential::from_channels(values)
    }

    /// Inverse radix-two transform at all texel centres, row-major x then z.
    /// Positive transform sign; conjugate pairs and the half-cell phase are
    /// assembled explicitly. Reference-only: allocates O(8*N*N) working memory.
    pub fn inverse_transform(&self, time: f64) -> Vec<Differential> {
        let mut channels = vec![[Complex::default(); 8]; GRID_SIZE * GRID_SIZE];
        for c in &self.modes {
            let [kx, kz] = c.wave_vector;
            let k = kx.hypot(kz);
            let x = lattice_index(kx, self.length);
            let z = lattice_index(kz, self.length);
            let phase =
                c.phase - c.frequency() * time + (kx + kz) * self.length / (2.0 * GRID_SIZE as f64);
            let (sin, cos) = phase.sin_cos();
            let h = Complex {
                re: 0.5 * c.amplitude * cos,
                im: 0.5 * c.amplitude * sin,
            };
            let multipliers = [
                Complex::imag(kx / k),
                Complex::real(1.0),
                Complex::imag(kz / k),
                Complex::imag(kx),
                Complex::imag(kz),
                Complex::real(-kx * kx / k),
                Complex::real(-kx * kz / k),
                Complex::real(-kz * kz / k),
            ];
            for (channel, multiplier) in multipliers.into_iter().enumerate() {
                let v = h.mul(multiplier);
                channels[z * GRID_SIZE + x][channel].add(v);
                channels[((GRID_SIZE - z) % GRID_SIZE) * GRID_SIZE + (GRID_SIZE - x) % GRID_SIZE]
                    [channel]
                    .add(v.conjugate());
            }
        }
        for row in channels.as_chunks_mut::<GRID_SIZE>().0 {
            fft(row);
        }
        let mut column = vec![[Complex::default(); 8]; GRID_SIZE];
        for x in 0..GRID_SIZE {
            for z in 0..GRID_SIZE {
                column[z] = channels[z * GRID_SIZE + x];
            }
            fft(&mut column);
            for z in 0..GRID_SIZE {
                channels[z * GRID_SIZE + x] = column[z];
            }
        }
        channels
            .into_iter()
            .map(|v| Differential::from_channels(v.map(|v| v.re)))
            .collect()
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "validated lattice modes are integral indices in -127..128"
)]
fn lattice_index(k: f64, length: f64) -> usize {
    ((k * length / TAU).round() as i32).rem_euclid(256) as usize
}

#[derive(Clone, Copy, Debug, Default)]
struct Complex {
    re: f64,
    im: f64,
}
impl Complex {
    fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }
    fn imag(im: f64) -> Self {
        Self { re: 0.0, im }
    }
    fn conjugate(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }
    fn add(&mut self, rhs: Self) {
        self.re += rhs.re;
        self.im += rhs.im;
    }
    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

// Unnormalized positive-sign Cooley–Tukey. Inputs already contain physical
// Fourier-series amplitudes, so no N-squared normalization follows the inverse.
fn fft(data: &mut [[Complex; 8]]) {
    let n = data.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            data.swap(i, j);
        }
    }
    let mut width = 2;
    while width <= n {
        for start in (0..n).step_by(width) {
            for offset in 0..width / 2 {
                let (sin, cos) = (TAU * offset as f64 / width as f64).sin_cos();
                let twiddle = Complex { re: cos, im: sin };
                let a = start + offset;
                let b = a + width / 2;
                let (left, right) = data.split_at_mut(b);
                for (u, v) in left[a].iter_mut().zip(&mut right[0]) {
                    let product = v.mul(twiddle);
                    *v = Complex {
                        re: u.re - product.re,
                        im: u.im - product.im,
                    };
                    *u = Complex {
                        re: u.re + product.re,
                        im: u.im + product.im,
                    };
                }
            }
        }
        width *= 2;
    }
}
