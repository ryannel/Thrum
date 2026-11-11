use super::{AtmosphereFrame, EnvironmentState};

fn frame(elevation_degrees: f32, profile_epoch: u64, seconds: f64) -> AtmosphereFrame {
    let (y, x) = elevation_degrees.to_radians().sin_cos();
    AtmosphereFrame::new([x, y, 0.0], 127_000.0, [1.0; 3], 0.0, profile_epoch)
        .unwrap()
        .with_simulation_time(seconds)
        .unwrap()
}

#[test]
fn epoch_tracks_profile_model_and_resource_replacement_without_tracking_time() {
    let initial = frame(30.0, 1, 0.0);
    let mut state = EnvironmentState::new(initial);
    let epoch = state.sample().epoch;
    state.update(frame(-20.0, 1, 45.0));
    assert_eq!(state.sample().epoch, epoch);
    assert_eq!(state.sample().simulation_time_seconds, 45.0);
    state.update(frame(-20.0, 2, 45.0));
    assert_ne!(state.sample().epoch, epoch);
    let profile_epoch = state.sample().epoch;
    state.update(frame(-20.0, 2, 45.0).with_nightglow());
    assert_ne!(state.sample().epoch, profile_epoch);
    let replacement = EnvironmentState::new(frame(-20.0, 2, 45.0).with_nightglow());
    assert_ne!(state.sample().epoch, replacement.sample().epoch);
}

#[test]
fn raw_source_crosses_sunset_continuously_while_surface_light_extinguishes() {
    let mut state = EnvironmentState::new(frame(0.01, 1, 10.0));
    let before = state.sample();
    state.update(frame(-0.01, 1, 10.01));
    let after = state.sample();
    assert_eq!(before.epoch, after.epoch);
    assert_eq!(
        before.true_sun.illuminance_lux,
        after.true_sun.illuminance_lux
    );
    assert_eq!(before.true_sun.colour, after.true_sun.colour);
    assert_eq!(
        before.true_sun.angular_radius_radians,
        after.true_sun.angular_radius_radians
    );
    for (a, b) in before
        .true_sun
        .toward_sun
        .into_iter()
        .zip(after.true_sun.toward_sun)
    {
        assert!((a - b).abs() < 0.0004);
    }
    assert!(before.surface_sun.irradiance_lux.iter().all(|x| *x > 0.0));
    assert_eq!(after.surface_sun.irradiance_lux, [0.0; 3]);
}

#[test]
fn simulation_time_rejects_nonfinite_samples() {
    for seconds in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(frame(30.0, 1, 0.0).with_simulation_time(seconds).is_err());
    }
}
