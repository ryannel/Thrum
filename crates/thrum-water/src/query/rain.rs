//! The tick query reads the rain schedule at the use time, independently of display cadence.
use super::BodyAnswers;
use crate::map_gpu::WaveTracks;
use glam::{DMat2, DVec2};
use std::time::Duration;
use thrum_water_model::Map;

#[derive(Clone, Copy)]
pub(super) struct QueryRain {
    pub(super) variances: [f64; 4],
    pub(super) period: f64,
}

impl QueryRain {
    pub(super) fn add(
        self,
        answers: &mut [BodyAnswers],
        map: &Map,
        tracks: Option<&WaveTracks>,
        tick: u64,
        step: Duration,
    ) {
        let time = tick as f64 * step.as_secs_f64();
        let Some(rain) = map.rain().filter(|rain| rain.active(time)) else {
            return;
        };
        let query = rain.query(time);
        for point in answers.iter_mut().flat_map(|answer| &mut answer.points) {
            let world = point.point.position.map(f64::from);
            let Some(column) = map
                .sample(world)
                .filter(|column| Some(column.body) == point.surface.body)
            else {
                continue;
            };
            let gain = map.rain_gain_with_variances(self.variances, world, time, column);
            if gain <= 0.0 {
                continue;
            }
            let current = DVec2::from_array(column.row.current);
            let (track, gradients) = tracks.map_or(([0.0; 4], [[0.0; 4]; 2]), |tracks| {
                tracks.query_track(column.body, world, column.row.current, map)
            });
            let radius = f64::from(point.point.radius);
            let motion = carriage(world.into(), current, track, gradients, time, self.period);
            let sample = |at: DVec2, drift: DVec2, derivative: DMat2| {
                let wave = query.sample(at.into(), radius);
                let slope = DVec2::from_array(wave.slope);
                let rate = wave.height_dt - slope.dot(drift);
                (
                    wave.height,
                    derivative.transpose() * slope,
                    rate,
                    wave.height_dt,
                )
            };
            let (mut h, mut slope, mut rate, mut vertical) =
                sample(motion.first, motion.drift, motion.first_gradient);
            if motion.weight < 1.0 || motion.weight_gradient != DVec2::ZERO {
                let (other_h, other_slope, other_rate, other_vertical) =
                    sample(motion.second, motion.second_drift, motion.second_gradient);
                rate = (rate - other_rate).mul_add(motion.weight, other_rate)
                    + motion.weight_rate * (h - other_h);
                slope =
                    other_slope.lerp(slope, motion.weight) + motion.weight_gradient * (h - other_h);
                h = (h - other_h).mul_add(motion.weight, other_h);
                vertical = (vertical - other_vertical).mul_add(motion.weight, other_vertical);
            }
            let surface = &mut point.surface.sample;
            surface.height += gain * h;
            surface.height_dt += gain * rate;
            surface.velocity[1] += gain * vertical;
            surface.slope[0] += gain * slope.x;
            surface.slope[1] += gain * slope.y;
            point.surface.wet = point.bed.is_none_or(|bed| surface.height > f64::from(bed));
        }
    }
}

struct Motion {
    first: DVec2,
    second: DVec2,
    drift: DVec2,
    second_drift: DVec2,
    weight: f64,
    weight_rate: f64,
    first_gradient: DMat2,
    second_gradient: DMat2,
    weight_gradient: DVec2,
}

// Same departure, renewal and blend as carriage.wgsl.
fn carriage(
    point: DVec2,
    current: DVec2,
    track: [f64; 4],
    gradients: [[f64; 4]; 2],
    time: f64,
    period: f64,
) -> Motion {
    let [x, y, blend, axis] = track;
    let mut motion = Motion {
        first: point - current * time,
        second: point,
        drift: current,
        second_drift: DVec2::ZERO,
        weight: 1.0,
        weight_rate: 0.0,
        first_gradient: DMat2::IDENTITY,
        second_gradient: DMat2::IDENTITY,
        weight_gradient: DVec2::ZERO,
    };
    if axis < 0.0 {
        let reference = DVec2::new(x, y);
        let residual = current - reference;
        let base = point - reference * time;
        let [current_gradient, reference_gradient] =
            gradients.map(|values| DMat2::from_cols_array(&values));
        let residual_gradient = current_gradient - reference_gradient;
        let base_gradient = DMat2::IDENTITY - reference_gradient * time;
        let span = period.min(blend);
        let a = (time / span).rem_euclid(1.0);
        let b = (a + 0.5).rem_euclid(1.0);
        let u = 2.0 * a.min(1.0 - a);
        let weight = u * u * (3.0 - 2.0 * u);
        let da = (a - 0.5) * span;
        let db = (b - 0.5) * span;
        if residual.abs().max_element() < 1e-6 {
            motion.first = base;
            motion.drift = reference;
            motion.first_gradient =
                base_gradient - residual_gradient * (da - db).mul_add(weight, db);
        } else {
            let direction = if a < 0.5 { 1.0 } else { -1.0 };
            motion.first = base - residual * da;
            motion.second = base - residual * db;
            motion.second_drift = current;
            motion.weight = weight;
            motion.weight_rate = 12.0 * u * (1.0 - u) * direction / span;
            motion.first_gradient = base_gradient - residual_gradient * da;
            motion.second_gradient = base_gradient - residual_gradient * db;
        }
    } else if axis > 0.0 {
        let i = usize::from(axis > 1.0);
        motion.first = point;
        motion.first[i] = y.mul_add(-time, x);
        motion.drift = DVec2::ZERO;
        motion.drift[i] = y;
        motion.weight = blend;
        let [dx, dwx, dz, dwz] = gradients[1];
        motion.first_gradient.x_axis[i] = dx;
        motion.first_gradient.y_axis[i] = dz;
        motion.weight_gradient = DVec2::new(dwx, dwz);
    }
    motion
}
