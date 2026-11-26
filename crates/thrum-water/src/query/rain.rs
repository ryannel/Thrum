//! The tick query reads the rain schedule at the use time, independently of display cadence.
use super::BodyAnswers;
use crate::map_gpu::WaveTracks;
use glam::DVec2;
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
        answer: &mut BodyAnswers,
        map: &Map,
        tracks: Option<&WaveTracks>,
        tick: u64,
        step: Duration,
    ) {
        let time = tick as f64 * step.as_secs_f64();
        let Some(rain) = map.rain().filter(|rain| rain.active(time)) else {
            return;
        };
        for point in &mut answer.points {
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
            let track = tracks.map_or([0.0; 4], |tracks| {
                tracks.at(column.body, world, column.row.current, map.bed())
            });
            let radius = f64::from(point.point.radius);
            let motion = carriage(
                world.into(),
                column.row.current.into(),
                track,
                time,
                self.period,
            );
            let sample = |at: DVec2, drift: DVec2| {
                let wave = rain.sample_footprint(at.into(), time, radius);
                let slope = DVec2::from_array(wave.slope);
                let rate = wave.height_dt - slope.dot(drift);
                (wave.height, slope, rate, wave.height_dt)
            };
            let (mut h, mut slope, mut rate, mut vertical) = sample(motion.first, motion.drift);
            if motion.weight < 1.0 {
                let (other_h, other_slope, other_rate, other_vertical) =
                    sample(motion.second, motion.second_drift);
                rate = (rate - other_rate).mul_add(motion.weight, other_rate)
                    + motion.weight_rate * (h - other_h);
                h = (h - other_h).mul_add(motion.weight, other_h);
                slope = other_slope.lerp(slope, motion.weight);
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
}

// Same departure, renewal and blend as carriage.wgsl.
fn carriage(point: DVec2, current: DVec2, track: [f64; 4], time: f64, period: f64) -> Motion {
    let [x, y, blend, axis] = track;
    let mut motion = Motion {
        first: point - current * time,
        second: point,
        drift: current,
        second_drift: DVec2::ZERO,
        weight: 1.0,
        weight_rate: 0.0,
    };
    if axis < 0.0 {
        let reference = DVec2::new(x, y);
        let residual = current - reference;
        let base = point - reference * time;
        if residual.abs().max_element() < 1e-6 {
            motion.first = base;
            motion.drift = reference;
        } else {
            let span = period.min(blend);
            let a = (time / span).rem_euclid(1.0);
            let b = (a + 0.5).rem_euclid(1.0);
            let u = 2.0 * a.min(1.0 - a);
            let direction = if a < 0.5 { 1.0 } else { -1.0 };
            motion.first = base - residual * ((a - 0.5) * span);
            motion.second = base - residual * ((b - 0.5) * span);
            motion.second_drift = current;
            motion.weight = u * u * (3.0 - 2.0 * u);
            motion.weight_rate = 12.0 * u * (1.0 - u) * direction / span;
        }
    } else if axis > 0.0 {
        let i = usize::from(axis > 1.0);
        motion.first = point;
        motion.first[i] = y.mul_add(-time, x);
        motion.drift = DVec2::ZERO;
        motion.drift[i] = y;
        motion.weight = blend;
    }
    motion
}
