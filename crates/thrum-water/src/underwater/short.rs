use std::{collections::HashMap, f64::consts::TAU};
use thrum_sea_model::{Cascade, GRID_SIZE, LOOP_SECONDS, loop_cycles};
use thrum_water_model::{Bed, Body, BodyKind, Map};

const GRID: i64 = 256;
const MAX_FOOTPRINT_TAPS: usize = 64;
const MAX_RETAINED_TAPS: usize = 128;
const MAX_CYCLE: usize = 8192;
const _: () = assert!(GRID_SIZE == 256);

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
    pub(super) fn gain_ranges(
        self,
        map: &Map,
        centre: [f64; 2],
        radius: [f64; 2],
        heights: [f32; 4],
    ) -> Option<[[f64; 2]; 4]> {
        match self {
            Self::Open(gains) => Some(gains.map(|gain| [f64::from(gain as f32); 2])),
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
                    return mapped_gains(map, centre, radius, heights);
                }
                let mut seas = (0..)
                    .map_while(|index| map.body(index))
                    .filter(|body| matches!(body.kind, BodyKind::Sea));
                let sea = seas.next()?;
                seas.next()
                    .is_none()
                    .then(|| sea.gains.map(|gain| [f64::from(gain as f32); 2]))
            }
        }
    }
}

fn mapped_gains(
    map: &Map,
    centre: [f64; 2],
    radius: [f64; 2],
    heights: [f32; 4],
) -> Option<[[f64; 2]; 4]> {
    let bed = bed_bounds(map.bed(), centre, radius)?;
    let heights = heights.map(f64::from);
    let mut envelope = [[f64::INFINITY, 0.0_f64]; 4];
    // The owner can vary across the footprint. Every still body supplies a
    // constant row, so enclose their gains without predicting ownership.
    for body in (0..).map_while(|index| map.body(index)) {
        if !matches!(body.kind, BodyKind::Sea | BodyKind::Basin { .. }) {
            return None;
        }
        for (range, [lo, hi]) in envelope.iter_mut().zip(row_gains(body, bed, heights, map)?) {
            *range = [range[0].min(lo), range[1].max(hi)];
        }
    }
    Some(envelope)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the map uploads the row, stage and depths in GPU metres"
)]
fn row_gains(
    body: &Body,
    [low_bed, high_bed]: [f64; 2],
    heights: [f64; 4],
    map: &Map,
) -> Option<[[f64; 2]; 4]> {
    let level = f64::from(body.level as f32 + body.stage.value as f32);
    let gains = body.gains.map(|g| f64::from(g as f32));
    let supported = heights
        .iter()
        .zip(gains)
        .map(|(height, gain)| (height * gain).powi(2))
        .sum::<f64>()
        .sqrt();
    let profile = map.profile();
    let swash = (f64::from(profile.swash_ratio as f32) * supported)
        .min(f64::from(profile.margin_height as f32));
    // Enclose the shader's f32 bed interpolation, subtract, length and cap.
    let error = f64::from(f32::EPSILON)
        * 32.0
        * level
            .abs()
            .max(low_bed.abs())
            .max(high_bed.abs())
            .max(supported)
            .max(swash)
            .max(1.0);
    let depths = [
        level - high_bed + swash - error,
        level - low_bed + swash + error,
    ];
    if depths.iter().any(|depth| !depth.is_finite()) {
        return None;
    }
    Some(std::array::from_fn(|i| {
        if heights[i] == 0.0 {
            return [gains[i]; 2];
        }
        let height = heights[i].max(f64::from(1e-30_f32));
        depths.map(|depth| (f64::from(0.6_f32) * depth.max(0.0) / height).min(gains[i]))
    }))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "GPU lattice coordinates are clipped before conversion to indices"
)]
fn bed_bounds(bed: &Bed, centre: [f64; 2], radius: [f64; 2]) -> Option<[f64; 2]> {
    let (origin, spacing, size) = bed.lattice();
    let spacing = f64::from(spacing as f32);
    let axis = |i: usize| {
        let origin = f64::from(origin[i] as f32);
        let last = f64::from(u32::try_from(size[i].checked_sub(1)?).ok()?);
        // Expand in world space for GPU unprojection, then include both
        // texture taps of every bilinear cell touched by the footprint.
        let pad = centre[i]
            .abs()
            .max(origin.abs())
            .max(last * spacing)
            .max(1.0)
            * f64::from(f32::EPSILON)
            * 16.0;
        let low = (centre[i] - radius[i] - pad - origin) / spacing;
        let high = (centre[i] + radius[i] + pad - origin) / spacing;
        (low >= 0.0 && high <= last).then(|| low.floor() as usize..=high.ceil() as usize)
    };
    let x = axis(0)?;
    let z = axis(1)?;
    let width = x.end() - x.start() + 1;
    let height = z.end() - z.start() + 1;
    if width.checked_mul(height)? > MAX_FOOTPRINT_TAPS {
        return None;
    }
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for row in z {
        for column in x.clone() {
            let height = f64::from(bed.heights()[row * size[0] + column] as f32);
            low = low.min(height);
            high = high.max(height);
        }
    }
    Some([low, high])
}

pub(super) struct ShortBand {
    length: f64,
    pub(super) amplitude: f64,
    modes: Vec<Mode>,
    phases: Vec<[f64; 2]>,
    taps: HashMap<[usize; 2], Box<[[f64; 2]]>>,
}

struct Mode {
    lattice: [usize; 2],
    cycle: usize,
    coefficient: [f64; 2],
}

impl ShortBand {
    pub(super) fn new(cascade: &Cascade) -> Self {
        let amplitude = cascade.components().iter().map(|mode| mode.amplitude).sum();
        let modes = if amplitude > 0.0 {
            Self::pack_modes(cascade).unwrap_or_default()
        } else {
            Vec::new()
        };
        let phases = modes
            .iter()
            .map(|mode| mode.cycle)
            .max()
            .map_or_else(Vec::new, |last| vec![[0.0; 2]; last + 1]);
        Self {
            length: cascade.length(),
            amplitude,
            modes,
            phases,
            taps: HashMap::new(),
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the sea upload rounds each supported Fourier lattice index to i32"
    )]
    fn pack_modes(cascade: &Cascade) -> Option<Vec<Mode>> {
        let mut occupied = vec![false; GRID_SIZE * GRID_SIZE];
        let mut modes = Vec::with_capacity(cascade.components().len());
        for mode in cascade.components() {
            let [x, z] = mode
                .wave_vector
                .map(|k| (k * cascade.length() / TAU).round() as i32);
            let [x, z] = [
                usize::try_from(x.checked_add(128)?).ok()?,
                usize::try_from(z.checked_add(128)?).ok()?,
            ];
            let cycle = usize::try_from(loop_cycles(*mode)).ok()?;
            if x >= GRID_SIZE
                || z >= GRID_SIZE
                || cycle > MAX_CYCLE
                || mode.phase.abs() > 2.0 * TAU
                || !mode.amplitude.is_finite()
            {
                return None;
            }
            let slot = &mut occupied[z * GRID_SIZE + x];
            if *slot {
                // Ambient upload overwrites repeated bins. Keep the amplitude
                // envelope whenever the CPU and GPU sums could differ.
                return None;
            }
            *slot = true;
            if mode.amplitude > 0.0 {
                let (sin, cos) = mode.phase.sin_cos();
                modes.push(Mode {
                    lattice: [x, z],
                    cycle,
                    coefficient: [mode.amplitude * cos, mode.amplitude * sin],
                });
            }
        }
        Some(modes)
    }

    pub(super) fn height_bound(&mut self, centre: [f64; 2], radius: [f64; 2], seconds: f64) -> f64 {
        if self.modes.is_empty() {
            return self.amplitude;
        }
        let Some(keys) = self.footprint(centre, radius) else {
            return self.amplitude;
        };
        if self.taps.len() + keys.len() > MAX_RETAINED_TAPS {
            self.taps.retain(|key, _| keys.contains(key));
        }
        let time = seconds.rem_euclid(LOOP_SECONDS);
        for (cycle, phase) in (0_u32..).zip(&mut self.phases) {
            let (sin, cos) = (TAU * time / LOOP_SECONDS * f64::from(cycle)).sin_cos();
            *phase = [cos, sin];
        }
        let count = self.phases.len();
        let modes = &self.modes;
        let mut peak = f64::NEG_INFINITY;
        for key in keys {
            let coefficients = self
                .taps
                .entry(key)
                .or_insert_with(|| tap_coefficients(modes, count, key));
            let height = coefficients
                .iter()
                .zip(&self.phases)
                .map(|(&[re, im], &[cos, sin])| re * cos + im * sin)
                .sum::<f64>();
            peak = peak.max(height);
        }
        // The level-zero bilinear lookup cannot exceed any upper bound on
        // its contributing taps. Leave room for GPU phase and FFT packing.
        peak + self.amplitude * 0.001
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "use the shader's f32 patch length and checked, floored lattice coordinates"
    )]
    fn footprint(&self, centre: [f64; 2], radius: [f64; 2]) -> Option<Vec<[usize; 2]>> {
        let texels = f64::from(u16::try_from(GRID_SIZE).expect("fixed sea lattice"));
        let length = f64::from(self.length as f32);
        let axis = |i: usize| {
            // Allow for f32 unprojection and normalized UV division before
            // enumerating the centre-based texels that can reach the lens.
            let padding = centre[i].abs().max(length) * 2e-6;
            let coordinate =
                |side: f64| (centre[i] + side * (radius[i] + padding)) / length * texels - 0.5;
            let low = coordinate(-1.0).floor();
            let high = coordinate(1.0).floor() + 1.0;
            (low.abs() <= 1e12 && high.abs() <= 1e12).then_some(low as i64..=high as i64)
        };
        let x = axis(0)?;
        let z = axis(1)?;
        let width = x.end() - x.start() + 1;
        let height = z.end() - z.start() + 1;
        let count = usize::try_from(width.checked_mul(height)?).ok()?;
        if count > MAX_FOOTPRINT_TAPS {
            return None;
        }
        let mut keys = Vec::with_capacity(count);
        let wrap = |i: i64| usize::try_from(i.rem_euclid(GRID)).expect("wrapped sea texel");
        for row in z {
            keys.extend(x.clone().map(|column| [wrap(column), wrap(row)]));
        }
        Some(keys)
    }
}

fn tap_coefficients(modes: &[Mode], count: usize, key: [usize; 2]) -> Box<[[f64; 2]]> {
    let tables = key.map(|index| {
        let position = f64::from(u16::try_from(index).expect("sea texel")) + 0.5;
        let mut lane = -128_i32;
        std::array::from_fn::<_, GRID_SIZE, _>(|_| {
            let phase = TAU * f64::from(lane) * position / f64::from(256);
            lane += 1;
            phase.sin_cos()
        })
    });
    let mut coefficients = vec![[0.0; 2]; count];
    for mode in modes {
        let [x, z] = mode.lattice;
        let [re, im] = mode.coefficient;
        let (sx, cx) = tables[0][x];
        let (sz, cz) = tables[1][z];
        let sin = sx * cz + cx * sz;
        let cos = cx * cz - sx * sz;
        let sum = &mut coefficients[mode.cycle];
        sum[0] += re * cos - im * sin;
        sum[1] += im * cos + re * sin;
    }
    coefficients.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_taps_match_the_full_field_across_spatial_and_time_wraps() {
        let cascade = Cascade::new(
            9.0,
            &[
                (21, -14, 0.08, 0.4),
                (-21, 14, 0.01, 1.9),
                (127, 1, 0.02, 3.0),
            ],
        )
        .expect("physical spectrum");
        let mut band = ShortBand::new(&cascade);
        let texels = [0.5 * 9.0 / 256.0, 255.5 * 9.0 / 256.0];
        for time in [-0.01, 0.0, 3.7, 86_400.03] {
            let exact = texels
                .into_iter()
                .flat_map(|x| texels.map(|z| [x, z]))
                .map(|point| cascade.direct(point, time).displacement[1])
                .fold(f64::NEG_INFINITY, f64::max);
            let upper = band.height_bound([0.0; 2], [0.001; 2], time);
            assert!((upper - exact - band.amplitude * 0.001).abs() < 1e-12);
        }
        assert_eq!(band.taps.len(), 4);
    }
}
