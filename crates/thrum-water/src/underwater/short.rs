use std::{collections::HashSet, f64::consts::TAU};
use thrum_sea_model::{Cascade, GRID_SIZE, LOOP_SECONDS, cascade_frequency};
use thrum_water_model::{BodyKind, Map};

const PHASE_BOUND_MODES: usize = 64;

pub(crate) enum ShortWaveDomain {
    Open([f64; 4]),
    Mapped,
}

impl ShortWaveDomain {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "the mapped shader reads the bed lattice and gains as f32"
    )]
    pub(super) fn gains(self, map: &Map, centre: [f64; 2], radius: [f64; 2]) -> Option<[f64; 4]> {
        match self {
            Self::Open(gains) => Some(gains),
            Self::Mapped => {
                let (origin, spacing, size) = map.bed().lattice();
                // Outside the entire bed, the shader takes its uniform sea
                // row without transport or the shallow-water gain limiter.
                let offshore = (0..2).any(|i| {
                    let start = f64::from(origin[i] as f32);
                    let end = start + f64::from(spacing as f32) * (size[i] - 1) as f64;
                    let margin = f64::from(f32::EPSILON)
                        * 16.0
                        * start.abs().max(end.abs()).max(centre[i].abs()).max(1.0);
                    centre[i] + radius[i] + margin < start.min(origin[i])
                        || centre[i] - radius[i] - margin
                            > end.max(origin[i] + spacing * (size[i] - 1) as f64)
                });
                if !offshore {
                    return None;
                }
                let mut seas = (0..)
                    .map_while(|index| map.body(index))
                    .filter(|body| matches!(body.kind, BodyKind::Sea));
                let sea = seas.next()?;
                seas.next().is_none().then_some(sea.gains)
            }
        }
    }
}

pub(super) struct ShortBand {
    length: f64,
    pub(super) amplitude: f64,
    remainder: f64,
    modes: Vec<PhaseMode>,
}

struct PhaseMode {
    amplitude: f64,
    wave_vector: [f64; 2],
    phase: f64,
    frequency: f64,
}

impl ShortBand {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the sea upload rounds each supported Fourier lattice index to i32"
    )]
    pub(super) fn new(cascade: &Cascade) -> Self {
        let amplitude = cascade.components().iter().map(|mode| mode.amplitude).sum();
        let length = cascade.length();
        let mut bins = HashSet::with_capacity(cascade.components().len());
        let mut strongest = Vec::new();
        for mode in cascade.components() {
            let bin = mode.wave_vector.map(|k| (k * length / TAU).round() as i32);
            if !bins.insert(bin) || mode.phase.abs() > 2.0 * TAU {
                // The upload can overwrite a repeated bin. Keep the whole
                // amplitude envelope unless CPU and GPU realize the same sum.
                return Self {
                    length,
                    amplitude,
                    remainder: amplitude,
                    modes: Vec::new(),
                };
            }
            if mode.amplitude > 0.0 {
                strongest.push(mode);
            }
        }
        if strongest.len() > PHASE_BOUND_MODES {
            strongest.select_nth_unstable_by(PHASE_BOUND_MODES, |a, b| {
                b.amplitude.total_cmp(&a.amplitude)
            });
            strongest.truncate(PHASE_BOUND_MODES);
        }
        let kept: f64 = strongest.iter().map(|mode| mode.amplitude).sum();
        Self {
            length,
            amplitude,
            remainder: (amplitude - kept).max(0.0),
            modes: strongest
                .into_iter()
                .map(|mode| PhaseMode {
                    amplitude: mode.amplitude,
                    wave_vector: mode.wave_vector,
                    phase: mode.phase,
                    frequency: cascade_frequency(*mode),
                })
                .collect(),
        }
    }

    pub(super) fn height_bound(&self, centre: [f64; 2], radius: [f64; 2], seconds: f64) -> f64 {
        // A bilinear level-zero read is a convex combination of texels at
        // most one texel away. Expand the box for both the filter and f32 UVs.
        let padding = self.length / f64::from(u16::try_from(GRID_SIZE).expect("fixed sea lattice"));
        let radius: [f64; 2] = std::array::from_fn(|i| {
            (centre[i].abs().max(self.length) * 2e-6 + padding) + radius[i]
        });
        let time = seconds.rem_euclid(LOOP_SECONDS);
        let upper = self.modes.iter().map(|mode| {
            let [kx, kz] = mode.wave_vector;
            let phase = (kx * centre[0] + kz * centre[1] + mode.phase - mode.frequency * time)
                .rem_euclid(TAU);
            let nearest_peak = phase.min(TAU - phase);
            let spread = kx.abs() * radius[0] + kz.abs() * radius[1];
            mode.amplitude * (nearest_peak - spread).max(0.0).cos()
        });
        // Allow for the packed FFT coefficients, phase word and reconstruction.
        self.remainder + upper.sum::<f64>() + self.amplitude * 0.001
    }
}
