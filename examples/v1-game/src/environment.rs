//! Game-authored solar time derived from the committed world tick.
//! The renderer receives a Sun sample; it never owns another game clock.

use std::f32::consts::{FRAC_PI_4, TAU};

/// One equinox day at 45° latitude. The active world tick supplies time;
/// display interpolation only samples between its committed endpoints.
pub(crate) struct DayCycle;

#[derive(Clone, Copy)]
pub(crate) struct SunSample {
    pub toward_sun: [f32; 3],
    pub equatorial_to_world: [[f32; 3]; 3],
    pub top_illuminance_lux: f32,
    pub colour: [f32; 3],
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
        let committed_phase = Self::committed_phase(tick);
        let offset = (solar_phase - committed_phase).rem_euclid(1.0);
        // f32 can round a tiny negative residue plus one to exactly one.
        Some(if offset >= 1.0 { 0.0 } else { offset })
    }

    pub(crate) fn time_of_day(tick: u64, phase_offset: f32) -> f32 {
        (Self::committed_phase(tick) + phase_offset + 0.5).rem_euclid(1.0)
    }

    fn committed_phase(tick: u64) -> f32 {
        (tick % Self::TICKS_PER_DAY) as f32 / Self::TICKS_PER_DAY as f32
    }

    fn rendered_phase(tick: u64, alpha: f32, phase_offset: f32) -> f32 {
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
        if phase >= 1.0 { phase - 1.0 } else { phase }
    }

    pub(crate) fn rendered_time_of_day(tick: u64, alpha: f32, phase_offset: f32) -> f32 {
        (Self::rendered_phase(tick, alpha, phase_offset) + 0.5).rem_euclid(1.0)
    }

    pub(crate) fn sample(tick: u64, alpha: f32, phase_offset: f32) -> SunSample {
        let phase = Self::rendered_phase(tick, alpha, phase_offset);
        let hour_angle = TAU * phase;
        let (sin_hour, cos_hour) = hour_angle.sin_cos();
        let (sin_latitude, cos_latitude) = FRAC_PI_4.sin_cos();
        SunSample {
            toward_sun: [-sin_hour, cos_latitude * cos_hour, -sin_latitude * cos_hour],
            equatorial_to_world: [
                [-sin_hour, cos_latitude * cos_hour, -sin_latitude * cos_hour],
                [cos_hour, cos_latitude * sin_hour, -sin_latitude * sin_hour],
                [0.0, sin_latitude, cos_latitude],
            ],
            // At the top of the atmosphere; the renderer computes attenuation
            // and apparent disk radiance from this one emitter.
            top_illuminance_lux: 127_000.0,
            colour: [1.0; 3],
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
            let displayed = DayCycle::time_of_day(0, offset);
            assert!((displayed - time).abs() < 1e-6);
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

    #[test]
    fn rendered_solar_time_uses_the_same_previous_to_current_interval_as_lighting() {
        let tick = 1;
        let offset = DayCycle::offset_for_time(tick, 0.0).expect("midnight");
        let alpha = 0.25;
        let rendered = DayCycle::rendered_time_of_day(tick, alpha, offset);
        let ticks_per_day = DayCycle::TICKS_PER_DAY as f32;
        let expected = 1.0 - (1.0 - alpha) / ticks_per_day;
        assert!((rendered - expected).abs() < 1e-6);
        assert_eq!(
            DayCycle::rendered_time_of_day(tick, 1.0, offset),
            DayCycle::time_of_day(tick, offset)
        );
        assert_eq!(
            DayCycle::rendered_time_of_day(0, alpha, 0.0),
            DayCycle::time_of_day(0, 0.0)
        );
        let direction = DayCycle::sample(tick, alpha, offset).toward_sun;
        let offset_at_rendered = DayCycle::offset_for_time(0, rendered).expect("rendered time");
        let resampled = DayCycle::sample(0, 0.0, offset_at_rendered).toward_sun;
        assert!(
            direction
                .into_iter()
                .zip(resampled)
                .all(|(a, b)| (a - b).abs() < 1e-5)
        );
    }

    #[test]
    fn equatorial_basis_follows_sun_and_preserves_the_pole_across_wrap() {
        let day = DayCycle::TICKS_PER_DAY;
        let noon = DayCycle::sample(0, 0.0, 0.0);
        let sunset = DayCycle::sample(day / 4, 1.0, 0.0);
        assert_eq!(noon.equatorial_to_world[0], noon.toward_sun);
        assert_eq!(sunset.equatorial_to_world[0], sunset.toward_sun);
        assert_eq!(noon.equatorial_to_world[1], [1.0, 0.0, 0.0]);
        assert!(sunset.equatorial_to_world[0][0] < -0.99);
        assert_eq!(noon.equatorial_to_world[2], sunset.equatorial_to_world[2]);
        assert_eq!(
            DayCycle::sample(day, 1.0, 0.0).equatorial_to_world,
            DayCycle::sample(day + 1, 0.0, 0.0).equatorial_to_world,
        );
        let start = DayCycle::offset_for_time(0, 0.0).expect("midnight");
        let end = DayCycle::offset_for_time(0, 1.0).expect("midnight at slider wrap");
        assert_eq!(
            DayCycle::sample(0, 0.0, start).equatorial_to_world,
            DayCycle::sample(0, 0.0, end).equatorial_to_world,
        );
    }
}
