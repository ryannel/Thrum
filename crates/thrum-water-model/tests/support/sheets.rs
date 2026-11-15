use std::f64::consts::TAU;
use thrum_water_model::{BreakerSection, Sheet, SheetPoint, dispersion, shoaling};

pub(crate) struct Trains {
    section: BreakerSection,
    amplitudes: [f64; 3],
    // A sampled Snell integral for each of the test's three incident trains.
    fields: Vec<Vec<[f64; 3]>>,
    beach: bool,
}

impl Trains {
    pub(crate) fn new(index: usize) -> Self {
        let beach = index == 2;
        let amplitudes = match index {
            0 => [0.7, 0.0, 0.0],
            1 => [0.4667, 0.2333, 0.0],
            _ => [0.84, 0.60, 0.50],
        };
        let mut fields = Vec::new();
        if beach {
            for (length, angle, phase) in [
                (60.0_f64, -15.0_f64, 0.3),
                (45.0, 20.0, 2.1),
                (80.0, 5.0, 4.4),
            ] {
                let k0 = TAU / length;
                let ky = k0 * angle.to_radians().sin();
                let mut integral = phase;
                let mut values = Vec::new();
                let mut prev = None;
                for i in 0..4001 {
                    let x = -100.0 + f64::from(i) * 0.05;
                    let depth = (-x / 20.0).max(0.5);
                    let k = dispersion(k0, depth);
                    let kx = (k * k - ky * ky).sqrt();
                    if let Some(last) = prev {
                        integral += f64::midpoint(last, kx) * 0.05;
                    }
                    prev = Some(kx);
                    values.push([integral, kx, shoaling(k0, depth)]);
                }
                fields.push(values);
            }
        }
        Self {
            section: BreakerSection::first(),
            amplitudes,
            fields,
            beach,
        }
    }

    pub(crate) fn state(&self, p: [f64; 2]) -> (SheetPoint, f64, [f64; 2], [f64; 2]) {
        let mut re = 0.0;
        let mut im = 0.0;
        let mut real_gradient = [0.0; 2];
        let mut imag_gradient = [0.0; 2];
        let mut plain = [0.0; 2];
        for (i, &amplitude) in self.amplitudes.iter().enumerate() {
            let (phase, vector, gain) = if self.beach {
                let (_, angle) = [(60.0_f64, -15.0_f64), (45.0, 20.0), (80.0, 5.0)][i];
                let k0 = TAU / [60.0, 45.0, 80.0][i];
                let ky = k0 * angle.to_radians().sin();
                let s = ((p[0] + 100.0) / 0.05).clamp(0.0, 3999.0);
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "bounded interpolation index"
                )]
                let lo = s.floor() as usize;
                let f = s - lo as f64;
                let field = std::array::from_fn::<_, 3, _>(|a| {
                    self.fields[i][lo][a] * (1.0 - f) + self.fields[i][lo + 1][a] * f
                });
                let exposure = if i == 1 {
                    (0.6 + 0.4 * ((p[1] - 12.0) / 6.0).tanh()).sqrt()
                } else {
                    1.0
                };
                (field[0] + ky * p[1], [field[1], ky], field[2] * exposure)
            } else {
                let k = TAU / 25.0;
                let vector = if i == 0 { [k, 0.0] } else { [0.0, k] };
                (vector[0] * p[0] + vector[1] * p[1], vector, 1.0)
            };
            let amplitude = amplitude * gain;
            let (sin, cos) = phase.sin_cos();
            re += amplitude * cos;
            im += amplitude * sin;
            let norm = vector[0].hypot(vector[1]);
            for a in 0..2 {
                real_gradient[a] += vector[a] * amplitude * cos;
                imag_gradient[a] += vector[a] * amplitude * sin;
                plain[a] += 0.5 * amplitude * sin * vector[a] / norm;
            }
        }
        let env = re.hypot(im);
        let vector =
            std::array::from_fn::<_, 2, _>(|a| real_gradient[a] * re + imag_gradient[a] * im);
        let norm = vector[0].hypot(vector[1]);
        let direction = vector.map(|v| v / norm);
        let index = if self.beach {
            2.0 * env / (0.78 * (-p[0] / 20.0).max(0.5))
        } else {
            1.0
        };
        let s = self.section.sample(im.atan2(re), index);
        let throw = s.throw * env;
        (
            SheetPoint {
                world: std::array::from_fn(|a| p[a] + plain[a] + throw * direction[a]),
                height: env * s.height,
            },
            throw,
            direction,
            plain,
        )
    }
}

impl Sheet for Trains {
    fn forward(&self, p: [f64; 2]) -> SheetPoint {
        self.state(p).0
    }
    fn reach(&self, p: [f64; 2]) -> f64 {
        if !self.beach {
            return self.amplitudes.iter().sum::<f64>() * 6.5;
        }
        // Whole-body bound, then two depth-interval bounds on this analytic bed.
        let ceiling = |lo: f64, hi: f64| {
            let amp: f64 = self
                .amplitudes
                .iter()
                .enumerate()
                .map(|(i, a)| a * shoaling(TAU / [60.0, 45.0, 80.0][i], lo.max(0.5)).max(1.0))
                .sum();
            // Maximum over depth is bounded by the largest envelope at the shallow
            // end and the table's throw at the largest depth in the interval.
            self.section.throw_ceiling(amp, 0.78 * hi.max(0.5)) + 0.5 * amp
        };
        let mut reach = ceiling(0.5, 6.0);
        for _ in 0..2 {
            reach = reach.min(ceiling((-p[0] - reach) / 20.0, (-p[0] + reach) / 20.0));
        }
        reach
    }
}
