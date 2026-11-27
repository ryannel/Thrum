//! Recompose an in-flight answer if its predicted water map changed.
use super::super::{BodyAnswers, QueryAnswer, seconds_between};
use bytemuck::{Pod, Zeroable};
use std::time::Duration;
use thrum_sea_model::{LongWaves, Sample};
use thrum_water_model::{Map, Surface};

/// Ungained samples from the actual frame's footprint-filtered cascade textures.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct CascadeSample {
    pub(super) height: f32,
    pub(super) slope: [f32; 2],
    pub(super) differential: [f32; 4],
    pub(super) velocity: [f32; 3],
}

impl CascadeSample {
    fn with_gain(self, gain: f64, dt: f64, advection_rate: f32) -> Sample {
        let [sx, sz] = self.slope.map(|value| f64::from(value) * gain);
        let [xx, xz, zx, zz] = self.differential.map(|value| f64::from(value) * gain);
        let (xx, zz) = (1.0 + xx, 1.0 + zz);
        let det = xx.mul_add(zz, -xz * xz).max(0.01);
        let velocity = self.velocity.map(|value| f64::from(value) * gain);
        let slope = [(zz * sx - zx * sz) / det, (xx * sz - xz * sx) / det];
        let height_dt = f64::from(advection_rate).mul_add(gain, velocity[1]);
        Sample {
            height: height_dt.mul_add(dt, f64::from(self.height) * gain),
            slope,
            velocity,
            height_dt,
        }
    }
}

pub(in crate::query) struct ReadyBody {
    pub(in crate::query) answers: BodyAnswers,
    pub(super) predictions: Vec<MapPrediction>,
    pub(super) frame_offset: f64,
    pub(super) frame_tick: u64,
    pub(super) frame_seconds: f64,
    pub(super) step: Duration,
}

pub(super) type FlowPrediction = ([[f32; 4]; 2], [[f32; 4]; 2]);

pub(super) struct MapPrediction {
    revision: [u64; 2],
    cascades: [CascadeSample; 3],
    advection_rate: [f32; 3],
    wake: [f32; 4],
    flow: Option<FlowPrediction>,
}

impl MapPrediction {
    pub(super) fn new(
        revision: [u64; 2],
        cascades: [CascadeSample; 3],
        advection_rate: [f32; 3],
        wake: [f32; 4],
        flow: Option<FlowPrediction>,
    ) -> Self {
        Self {
            revision,
            cascades,
            advection_rate,
            wake,
            flow,
        }
    }
}

pub(in crate::query) struct Correction {
    long_waves: LongWaves,
    variances: [f64; 4],
}

impl Correction {
    pub(in crate::query) fn new(long_waves: LongWaves, heights: [f32; 4]) -> Self {
        Self {
            long_waves,
            variances: heights.map(|height| (f64::from(height) * 0.25).powi(2)),
        }
    }

    pub(in crate::query) fn resolve<'a>(
        &self,
        ready: &mut ReadyBody,
        map_at: impl Fn(u64) -> &'a Map,
    ) -> Option<()> {
        for (answer, prediction) in ready.answers.points.iter_mut().zip(&ready.predictions) {
            let use_tick = answer.point.use_tick;
            let map = map_at(use_tick);
            let [spatial, hollows] = map.query_revision();
            let spatial_changed = spatial != prediction.revision[0];
            let hollow_changed = hollows != prediction.revision[1]
                && map
                    .hollow_bed_at(answer.point.position.map(f64::from))
                    .is_some();
            if spatial_changed || hollow_changed {
                if spatial_changed && map.surf_profile().is_some() {
                    // A stage edit can change the selected fold. Let the
                    // caller use the current map's conformance fallback.
                    return None;
                }
                let dt =
                    ready.frame_offset + seconds_between(use_tick, ready.frame_tick, ready.step);
                let time = use_tick as f64 * ready.step.as_secs_f64();
                let carry_bound = ready.step.as_secs_f64() * super::CARRY_TICKS;
                *answer = self.recompose(
                    *answer,
                    prediction,
                    map,
                    time,
                    dt,
                    carry_bound,
                    ready.frame_seconds,
                )?;
            }
        }
        Some(())
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the public bed and individual wave heights are expressed in f32 metres"
    )]
    fn recompose(
        &self,
        answer: QueryAnswer,
        prediction: &MapPrediction,
        map: &Map,
        time: f64,
        dt: f64,
        carry_bound: f64,
        frame_seconds: f64,
    ) -> Option<QueryAnswer> {
        let point = answer.point.position.map(f64::from);
        let row = map.sample(point);
        let bed = row.map_or_else(
            || map.hollow_bed_at(point).or_else(|| map.bed().height(point)),
            |row| row.bed,
        );
        let mut corrected = QueryAnswer {
            sources: [None; 5],
            supported: [false; 5],
            wake_height: 0.0,
            wave_heights: [0.0; 4],
            bed: bed.map(|bed| bed as f32),
            surface: Surface::default(),
            ..answer
        };
        let Some(row) = row else {
            return Some(corrected);
        };
        let (gains, flow_gains) = map.layer_gains(self.variances, point, row);
        let mut sample = Sample {
            height: row.row.level,
            slope: map.level_slope(row.body, point),
            velocity: [row.row.current[0], 0.0, row.row.current[1]],
            height_dt: 0.0,
        };
        for (layer, gain) in gains.into_iter().enumerate() {
            if gain == 0.0 || self.variances[layer] == 0.0 {
                continue;
            }
            let wave = if layer == 0 {
                corrected.sources[layer] = Some(time);
                self.long_waves
                    .scaled(gain)
                    .ok()?
                    .evaluate(point, time)
                    .ok()?
            } else {
                corrected.sources[layer] = Some(frame_seconds);
                prediction.cascades[layer - 1].with_gain(
                    gain,
                    dt.clamp(-carry_bound, carry_bound),
                    prediction.advection_rate[layer - 1],
                )
            };
            corrected.supported[layer] = layer == 0 || dt.abs() <= carry_bound;
            corrected.wave_heights[layer] = wave.height as f32;
            sample.height += wave.height;
            sample.height_dt += wave.height_dt;
            for (value, delta) in sample.slope.iter_mut().zip(wave.slope) {
                *value += delta;
            }
            for (value, delta) in sample.velocity.iter_mut().zip(wave.velocity) {
                *value += delta;
            }
        }
        if let Some((waves, velocities)) = prediction.flow {
            let dt = dt.clamp(-carry_bound, carry_bound);
            for ((wave, velocity), gain) in waves.into_iter().zip(velocities).zip(flow_gains) {
                let [h, sx, sz, rate] = wave.map(f64::from);
                sample.height += gain * rate.mul_add(dt, h);
                sample.height_dt += gain * rate;
                sample.slope[0] += gain * sx;
                sample.slope[1] += gain * sz;
                for (axis, value) in sample.velocity.iter_mut().enumerate() {
                    *value += gain * f64::from(velocity[axis]);
                }
            }
        }
        if row.depth > 0.0 {
            let wake = prediction.wake;
            corrected.sources[4] = answer.sources[4];
            corrected.supported[4] = answer.supported[4];
            corrected.wake_height = wake[0];
            sample.height += f64::from(wake[0]);
            sample.slope[0] += f64::from(wake[1]);
            sample.slope[1] += f64::from(wake[2]);
            sample.height_dt += f64::from(wake[3]);
            sample.velocity[1] += f64::from(wake[3]);
        }
        corrected.surface = Surface {
            body: Some(row.body),
            sample,
            depth: row.depth,
            current: row.row.current,
            wet: bed.is_none_or(|bed| sample.height > bed),
        };
        Some(corrected)
    }
}
