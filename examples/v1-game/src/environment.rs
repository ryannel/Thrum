//! Game-authored solar time derived from the committed world tick.
//! The renderer receives a Sun sample; it never owns another game clock.

use std::f32::consts::{FRAC_PI_4, TAU};

/// One equinox day at 45° latitude. The active world tick supplies time;
/// display interpolation only samples between its committed endpoints.
pub(crate) struct DayCycle;

#[derive(Clone, Copy)]
pub(crate) struct SunSample {
    pub toward_sun: [f32; 3],
    pub top_illuminance_lux: f32,
    pub colour: [f32; 3],
    pub camera_altitude_m: f32,
    pub profile_epoch: u64,
}

impl DayCycle {
    pub(crate) const TICKS_PER_DAY: u64 = 60 * 60 * 5;

    pub(crate) fn offset_for_time(tick: u64, time_of_day: f32) -> Option<f32> {
        if !time_of_day.is_finite() || !(0.0..=1.0).contains(&time_of_day) {
            return None;
        }
        // User-facing 0/1 is midnight, 0.25 sunrise, 0.5 noon and 0.75
        // sunset. The simulation's zero phase is noon.
        let solar_phase = if time_of_day < 0.5 {
            time_of_day + 0.5
        } else {
            time_of_day - 0.5
        };
        let committed_phase = (tick % Self::TICKS_PER_DAY) as f32 / Self::TICKS_PER_DAY as f32;
        let offset = (solar_phase - committed_phase).rem_euclid(1.0);
        // f32 can round a tiny negative residue plus one to exactly one.
        Some(if offset >= 1.0 { 0.0 } else { offset })
    }

    pub(crate) fn sample(tick: u64, alpha: f32, phase_offset: f32) -> SunSample {
        debug_assert!(alpha.is_finite() && (0.0..=1.0).contains(&alpha));
        debug_assert!(phase_offset.is_finite() && (0.0..1.0).contains(&phase_offset));
        let alpha = if alpha.is_finite() {
            alpha.clamp(0.0, 1.0)
        } else {
            0.0
        };
        // The display interval is [committed tick - 1, committed tick]. The
        // mesh shader uses the same previous-to-current endpoint convention.
        let phase_ticks = if tick == 0 {
            0.0
        } else {
            ((tick - 1) % Self::TICKS_PER_DAY) as f32 + alpha
        };
        let phase = phase_ticks / Self::TICKS_PER_DAY as f32 + phase_offset;
        let phase = if phase >= 1.0 { phase - 1.0 } else { phase };
        let hour_angle = TAU * phase;
        let (sin_hour, cos_hour) = hour_angle.sin_cos();
        let (sin_latitude, cos_latitude) = FRAC_PI_4.sin_cos();
        SunSample {
            toward_sun: [-sin_hour, cos_latitude * cos_hour, -sin_latitude * cos_hour],
            // At the top of the atmosphere; the renderer computes attenuation
            // and apparent disk radiance from this one emitter.
            top_illuminance_lux: 127_000.0,
            colour: [1.0; 3],
            camera_altitude_m: 2.0,
            profile_epoch: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DayCycle;

    #[test]
    fn committed_day_has_continuous_sunrise_noon_sunset_and_night() {
        let day = DayCycle::TICKS_PER_DAY;
        let sunrise = DayCycle::sample(day * 3 / 4, 1.0, 0.0);
        let noon = DayCycle::sample(0, 0.0, 0.0);
        let sunset = DayCycle::sample(day / 4, 1.0, 0.0);
        let midnight = DayCycle::sample(day / 2, 1.0, 0.0);
        assert!(noon.toward_sun[1] > 0.7);
        assert!(midnight.toward_sun[1] < -0.7);
        assert!(sunrise.toward_sun[1].abs() < 1e-5);
        assert!(sunset.toward_sun[1].abs() < 1e-5);
        assert!(sunrise.toward_sun[0] > 0.99);
        assert!(sunset.toward_sun[0] < -0.99);
        assert_eq!(
            DayCycle::sample(0, 0.0, 0.0).toward_sun,
            DayCycle::sample(day + 1, 0.0, 0.0).toward_sun
        );
        assert_eq!(
            DayCycle::sample(0, 0.0, 0.0).toward_sun,
            DayCycle::sample(1, 0.0, 0.0).toward_sun
        );
        for (time, expected) in [
            (0.0, midnight),
            (0.25, sunrise),
            (0.5, noon),
            (0.75, sunset),
        ] {
            let offset = DayCycle::offset_for_time(0, time).expect("valid time of day");
            let selected = DayCycle::sample(0, 0.0, offset);
            assert!(
                selected
                    .toward_sun
                    .into_iter()
                    .zip(expected.toward_sun)
                    .all(|(actual, expected)| (actual - expected).abs() < 1e-5)
            );
        }
        assert!(DayCycle::offset_for_time(0, f32::NAN).is_none());
        assert!(DayCycle::offset_for_time(0, 1.1).is_none());
        let wrapped =
            DayCycle::offset_for_time(1, 0.500_055_55).expect("near-endpoint time is admitted");
        assert!((0.0..1.0).contains(&wrapped));
        let _ = DayCycle::sample(1, 0.5, wrapped);
        for sample in [sunrise, noon, sunset, midnight] {
            let length_squared = sample.toward_sun.into_iter().map(|v| v * v).sum::<f32>();
            assert!((length_squared - 1.0).abs() < 1e-5);
        }
    }
}
