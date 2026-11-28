use thrum_audio::{Fate, Listener, Play, VoiceSettings};

use crate::support::{
    audio, close, config, exact, fate, first, mono, pcm, place, pull, steps, stereo,
};

#[test]
fn distance_and_constant_pan_power() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![4096]);
    let mut play = Play::new(sound, 1);
    handle.play(play);
    let dry = first(&mut mix);
    for distance in [0.0_f32, 1.0, 2.0, 3.0] {
        play.settings.place = Some(place(0.0, distance, 4.0));
        handle.play(play);
        exact(
            first(&mut mix),
            dry.map(|sample| sample * (1.0 - distance / 4.0).powi(2)),
        );
    }
    let listener = Listener {
        near: f32::MIN_POSITIVE,
        ..Listener::default()
    };
    handle.set_listener(listener);
    let mut powers = Vec::new();
    for [x, y] in [[-2.0, 0.0], [-1.2, 1.6], [0.0, 2.0], [1.2, 1.6], [2.0, 0.0]] {
        play.settings.place = Some(place(x, y, 4.0));
        handle.play(play);
        let [left, right] = first(&mut mix);
        if x < 0.0 {
            assert!(left > right);
        } else if x > 0.0 {
            assert!(right > left);
        }
        if x.abs() >= 2.0 {
            exact([if x < 0.0 { right } else { left }; 2], [0.0; 2]);
        }
        powers.push(left * left + right * right);
    }
    for power in &powers[1..] {
        close(*power, powers[0], 1e-8);
    }
    drop(crate::support::outcomes(&mut handle));
    for distance in [4.0, 8.0] {
        play.settings.place = Some(place(0.0, distance, 4.0));
        let number = handle.play(play);
        exact(first(&mut mix), [0.0; 2]);
        fate(&mut handle, number, Fate::OutOfRange);
    }
    assert_eq!(handle.record().plays(Fate::OutOfRange), 2);
    assert_eq!(handle.record().bad_values, 0);
}

#[test]
fn placed_stereo_combines_channels_and_turning_swaps_them() {
    let (mut handle, mut mix) = audio(config());
    let sound = stereo(&mut handle, vec![2048, 6144]);
    let mut play = Play::new(sound, 1);
    play.settings.place = Some(place(0.0, 0.0, 4.0));
    handle.play(play);
    exact(first(&mut mix), [pcm(4096); 2]);
    play.settings.place = Some(place(1.0, 0.0, 4.0));
    handle.play(play);
    let [left, right] = first(&mut mix);
    assert!(right > left);
    handle.set_listener(Listener {
        right: [-1.0, 0.0, 0.0],
        ..Listener::default()
    });
    handle.play(play);
    exact(first(&mut mix), [right, left]);
}

#[test]
fn spatial_motion_glides_through_centre_and_tracks_listener() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![4096]);
    let settings = VoiceSettings {
        place: Some(place(-1.0, 0.0, 4.0)),
        ..VoiceSettings::default()
    };
    let emitter = handle
        .start_emitter(sound, settings)
        .expect("valid emitter");
    let mut frames = Vec::new();
    let mut endpoints = Vec::new();
    for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        handle.move_emitter(emitter, Some(place(x, 0.0, 4.0)));
        handle.update();
        frames.extend(pull(&mut mix, 16));
        endpoints.push(*frames.last().expect("rendered"));
    }
    assert!(endpoints[0][0] > endpoints[0][1]);
    exact(endpoints[2], [pcm(4096); 2]);
    assert!(endpoints[4][0] < endpoints[4][1]);
    handle.set_listener(Listener {
        position: [1.5, 0.0, 0.0],
        right: [-1.0, 0.0, 0.0],
        ..Listener::default()
    });
    handle.update();
    frames.extend(pull(&mut mix, 16));
    exact(*frames.last().expect("rendered"), endpoints[3]);
    steps(&frames, pcm(4096) * config.glide_step);
}

#[test]
fn nonfinite_place_and_listener_keep_valid_state() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![2048]);
    let valid = Listener {
        position: [1.0, 0.0, 0.0],
        ..Listener::default()
    };
    handle.set_listener(valid);
    handle.set_listener(Listener {
        right: [f32::NAN, 0.0, 0.0],
        ..valid
    });
    assert_eq!(handle.listener(), valid);
    let mut play = Play::new(sound, 1);
    play.settings.place = Some(place(f32::INFINITY, 0.0, 3.0));
    let number = handle.play(play);
    exact(first(&mut mix), [0.0; 2]);
    fate(&mut handle, number, Fate::BadValue);
    play.settings.place = Some(place(1.0, 0.0, 3.0));
    handle.play(play);
    exact(first(&mut mix), [pcm(2048); 2]);
    assert_eq!(handle.record().bad_values, 2);
}
