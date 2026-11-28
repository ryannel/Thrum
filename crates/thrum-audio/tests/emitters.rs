use std::{cmp::Ordering, sync::Arc};

use thrum_audio::{Config, Listener, Play, VoiceSettings};

use crate::support::{audio, close, config, exact, first, mono, pcm, place, pull, steps, stereo};

#[test]
fn setting_and_clearing_a_stereo_place_glides_both_channel_contributions() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = stereo(&mut handle, vec![8192, -8192]);
    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 9));
    let dry = [pcm(8192), pcm(-8192)];
    exact(first(&mut mix), dry);
    for (location, start, end) in [
        (Some(place(0.0, 0.0, 8.0)), dry, [0.0; 2]),
        (None, [0.0; 2], dry),
    ] {
        handle.move_emitter(emitter, location);
        handle.update();
        let frames = pull(&mut mix, 9);
        exact(frames[0], start);
        exact(frames[8], end);
        steps(&frames, pcm(8192) * config.glide_step);
    }
}

#[test]
fn a_panned_emitter_leaves_its_radius_by_at_most_one_glide_step() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![8192]);
    let emitter = handle
        .start_emitter(
            sound,
            VoiceSettings {
                place: Some(place(1.0, 0.0, 100.0)),
                ..VoiceSettings::default()
            },
        )
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 20));
    let before = first(&mut mix);
    assert!(before[1] > pcm(8192));
    handle.move_emitter(emitter, Some(place(101.0, 0.0, 100.0)));
    handle.update();
    assert!(!handle.emitter_voiced(emitter));
    let mut frames = vec![before];
    frames.extend(pull(&mut mix, 14));
    exact(frames[1], before);
    exact(frames[14], [0.0; 2]);
    steps(&frames, pcm(8192) * config.glide_step);
}

#[test]
fn loops_wrap_exactly_at_small_and_large_lengths() {
    for length in [1_usize, 31, 103] {
        let (mut handle, mut mix) = audio(Config {
            glide_step: 1.0,
            ..config()
        });
        let samples: Vec<_> = (1..=length)
            .map(|value| i16::try_from(value * 32).expect("small input"))
            .collect();
        let sound = mono(&mut handle, samples.clone());
        let emitter = handle
            .start_emitter(sound, VoiceSettings::default())
            .expect("valid emitter");
        handle.update();
        let mut frames = Vec::new();
        for _ in 0..(3 * length + 1).div_ceil(32) {
            frames.extend(pull(&mut mix, 32));
        }
        exact(frames[0], [0.0; 2]);
        for (i, frame) in frames.into_iter().enumerate().skip(1) {
            exact(frame, [pcm(samples[i % length]); 2]);
        }
        assert!(handle.emitter_voiced(emitter));
    }
}

#[test]
fn movement_volume_and_speed_glide_without_resetting_phase() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, (1..=128).map(|n| n * 64).collect());
    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 12));
    handle.set_emitter(
        emitter,
        VoiceSettings {
            volume: 0.5,
            pitch: 1.5,
            place: None,
        },
    );
    handle.update();
    let frames = pull(&mut mix, 8);
    let mut phase = 12.0;
    for (i, frame) in frames.into_iter().enumerate() {
        let pitch = (1.0 + i as f32 * config.glide_step).min(1.5);
        let gain = (1.0 - i as f32 * config.glide_step).max(0.5);
        close(frame[0], (phase + 1.0) * pcm(64) * gain, 1e-8);
        phase += pitch;
    }
    handle.move_emitter(emitter, Some(place(0.0, 2.0, 4.0)));
    handle.update();
    for (i, frame) in pull(&mut mix, 8).into_iter().enumerate() {
        let gain = (0.5 - i as f32 * config.glide_step).max(0.125);
        close(frame[0], (phase + 1.0) * pcm(64) * gain, 1e-8);
        phase += 1.5;
    }

    handle.hold_emitter(emitter, true);
    handle.update();
    let held = pull(&mut mix, 30);
    let level = (phase + 1.0) * pcm(64) * 0.125;
    close(held[0][0], level, 1e-8);
    exact(held[8], [0.0; 2]);
    exact(held[29], [0.0; 2]);
    steps(&held, level / config.ramp_frames as f32);

    handle.hold_emitter(emitter, false);
    handle.update();
    let released = pull(&mut mix, 10);
    exact(released[0], [0.0; 2]);
    close(released[1][0], (phase + 2.5) * pcm(64) * 0.125 / 8.0, 1e-8);

    handle.end_emitter(emitter);
    handle.update();
    let tail = pull(&mut mix, 10);
    assert!(tail[0][0] > 0.0);
    exact(tail[8], [0.0; 2]);
    assert_eq!(handle.record().voices, 0);
}

#[test]
fn holding_during_entry_fades_from_the_reached_gain_and_keeps_phase() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![2048, 8192, 4096, 16384]);
    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    exact(first(&mut mix), [0.0; 2]);

    handle.hold_emitter(emitter, true);
    handle.update();
    let held = pull(&mut mix, 9);
    for (frame, gain_eighths) in held.into_iter().zip((0_i16..=8).rev()) {
        exact(frame, [pcm(128 * gain_eighths); 2]);
    }
    assert!(handle.emitter_voiced(emitter));
    assert_eq!(handle.record().voices, 0);
    for frame in pull(&mut mix, 67) {
        exact(frame, [0.0; 2]);
    }

    handle.hold_emitter(emitter, false);
    handle.update();
    let released = pull(&mut mix, 3);
    exact(released[0], [0.0; 2]);
    exact(released[1], [pcm(512); 2]);
    exact(released[2], [pcm(4096); 2]);
    assert_eq!(handle.record().voices, 1);
}

#[test]
fn entering_and_leaving_radius_switches_voices_and_glides() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![4096]);
    let looping = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    let emitter = handle
        .start_emitter(
            sound,
            VoiceSettings {
                place: Some(place(4.0, 0.0, 4.0)),
                ..VoiceSettings::default()
            },
        )
        .expect("valid emitter");
    handle.update();
    assert!(handle.emitter_voiced(looping));
    assert!(!handle.emitter_voiced(emitter));
    drop(pull(&mut mix, 9));
    handle.move_emitter(emitter, Some(place(0.0, 0.0, 4.0)));
    handle.update();
    assert!(handle.emitter_voiced(emitter));
    let mut frames = pull(&mut mix, 9);
    exact(frames[0], [pcm(4096); 2]);
    exact(frames[8], [pcm(8192); 2]);
    handle.move_emitter(emitter, Some(place(0.0, 8.0, 4.0)));
    handle.update();
    assert!(!handle.emitter_voiced(emitter));
    frames.extend(pull(&mut mix, 9));
    exact(frames[17], [pcm(4096); 2]);
    steps(&frames, pcm(4096) * config.glide_step);
}

#[test]
fn audible_and_unplaced_emitters_obey_margin_and_age() {
    for position in [None, Some(place(0.0, 0.0, 4.0))] {
        let config = Config {
            emitters: 1,
            ..config()
        };
        let (mut handle, mut mix) = audio(config);
        let old = mono(&mut handle, vec![4096]);
        let new = mono(&mut handle, vec![-4096]);
        let level = 0.5;
        let margin = config.emitter_margin;
        let settings = VoiceSettings {
            volume: level,
            place: position,
            ..VoiceSettings::default()
        };
        let a = handle.start_emitter(old, settings).expect("valid emitter");
        let b = handle.start_emitter(new, settings).expect("valid emitter");
        for _ in 0..100 {
            handle.update();
            assert!(handle.emitter_voiced(a) && !handle.emitter_voiced(b));
        }
        drop(pull(&mut mix, 9));
        let near = level * (1.0 + margin / 2.0);
        handle.set_emitter(
            b,
            VoiceSettings {
                volume: near,
                ..settings
            },
        );
        for _ in 0..100 {
            handle.update();
            assert!(handle.emitter_voiced(a) && !handle.emitter_voiced(b));
        }
        let far = level * (1.0 + 2.0 * margin);
        handle.set_emitter(
            b,
            VoiceSettings {
                volume: far,
                ..settings
            },
        );
        handle.update();
        assert!(handle.emitter_voiced(b) && !handle.emitter_voiced(a));
        let switched = pull(&mut mix, 10);
        for (i, frame) in switched.into_iter().enumerate() {
            let old_gain = level * (1.0 - i as f32 / 8.0).max(0.0);
            let new_gain = (i as f32 * config.glide_step).min(far);
            exact(frame, [pcm(4096) * (old_gain - new_gain); 2]);
        }
        for volume in [near, level / (1.0 + margin / 2.0)] {
            handle.set_emitter(b, VoiceSettings { volume, ..settings });
            for _ in 0..100 {
                handle.update();
                assert!(handle.emitter_voiced(b) && !handle.emitter_voiced(a));
            }
        }
    }
}

fn rank(a: usize, b: usize, levels: &[Option<f32>]) -> Ordering {
    levels[b]
        .expect("eligible")
        .total_cmp(&levels[a].expect("eligible"))
        .then(a.cmp(&b))
}

fn oracle(held: &mut [bool], levels: &[Option<f32>], capacity: usize, margin: f32) {
    for (held, gain) in held.iter_mut().zip(levels) {
        *held &= gain.is_some();
    }
    let mut occupied = held.iter().filter(|&&value| value).count();
    loop {
        let best = (0..held.len())
            .filter(|&i| levels[i].is_some() && !held[i])
            .min_by(|&a, &b| rank(a, b, levels));
        let Some(best) = best else { break };
        if occupied < capacity {
            held[best] = true;
            occupied += 1;
        } else {
            let worst = (0..held.len())
                .filter(|&i| held[i])
                .max_by(|&a, &b| rank(a, b, levels))
                .expect("full pool");
            if levels[best].expect("eligible") <= levels[worst].expect("eligible") * (1.0 + margin)
            {
                break;
            }
            held[worst] = false;
            held[best] = true;
        }
    }
}

fn random(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed
}

#[test]
fn ten_updates_match_plain_policy_for_ten_thousand_emitters() {
    const COUNT: usize = 10_000;
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![512]);
    let mut seed = 7;
    let mut scene = Vec::new();
    for _ in 0..COUNT {
        let x = (random(&mut seed) % 512) as f32 - 256.0;
        let volume = (random(&mut seed) % 1024) as f32 / 1024.0;
        let settings = VoiceSettings {
            volume,
            place: Some(place(x, 0.0, 128.0)),
            ..VoiceSettings::default()
        };
        scene.push((
            handle
                .start_emitter(sound, settings)
                .expect("valid emitter"),
            x,
            volume,
        ));
    }
    let mut held = vec![false; COUNT];
    for update in 0..10 {
        for (id, x, _) in &mut scene[update * 1000..(update * 1000 + 2000).min(COUNT)] {
            *x = (random(&mut seed) % 512) as f32 - 256.0;
            handle.move_emitter(*id, Some(place(*x, 0.0, 128.0)));
        }
        if update == 9 {
            for (id, x, _) in &mut scene[..1000] {
                *x = (random(&mut seed) % 512) as f32 - 256.0;
                handle.move_emitter(*id, Some(place(*x, 0.0, 128.0)));
            }
        }
        let listener_x = (random(&mut seed) % 256) as f32 - 128.0;
        handle.set_listener(Listener {
            position: [listener_x, 0.0, 0.0],
            ..Listener::default()
        });
        let levels: Vec<_> = scene
            .iter()
            .map(|&(_, x, volume)| {
                let distance = (x - listener_x).abs();
                let linear = 1.0 - distance / 128.0;
                (distance < 128.0).then_some(volume * (linear * linear))
            })
            .collect();
        oracle(&mut held, &levels, config.emitters, config.emitter_margin);
        handle.update();
        drop(pull(&mut mix, 9));
        for (index, (id, ..)) in scene.iter().enumerate() {
            assert_eq!(
                handle.emitter_voiced(*id),
                held[index],
                "update {update}, emitter {index}"
            );
            if held[index] {
                assert!(levels[index].is_some());
            }
        }
        assert_eq!(handle.record().voiced_emitters, config.emitters);
        assert_eq!(
            handle.record().eligible_emitters,
            levels.iter().flatten().count()
        );
        let max_unvoiced = levels
            .iter()
            .zip(&held)
            .filter_map(|(level, &held)| if held { None } else { *level })
            .max_by(f32::total_cmp)
            .expect("unvoiced candidate");
        let min_voiced = levels
            .iter()
            .zip(&held)
            .filter_map(|(level, &held)| if held { *level } else { None })
            .min_by(f32::total_cmp)
            .expect("voiced candidate");
        assert!(max_unvoiced <= min_voiced * (1.0 + config.emitter_margin));
    }
}

#[test]
fn replacing_samples_keeps_active_voices_and_retires_storage_on_handle() {
    let (mut handle, mut mix) = audio(config());
    let original: Arc<[i16]> = Arc::from([1024; 9]);
    let weak = Arc::downgrade(&original);
    let sound = handle.add_sound(original, 1).expect("valid sound");
    handle.play(Play::new(sound, 1));
    exact(first(&mut mix), [pcm(1024); 2]);
    handle
        .replace_sound(sound, Arc::from([2048; 10]), 1)
        .expect("registered sound");
    assert!(weak.upgrade().is_some());
    handle.play(Play::new(sound, 2));
    for _ in 0..8 {
        exact(first(&mut mix), [pcm(3072); 2]);
        assert!(weak.upgrade().is_some());
    }
    exact(first(&mut mix), [pcm(2048); 2]);
    exact(first(&mut mix), [pcm(2048); 2]);
    for _ in 0..1000 {
        exact(first(&mut mix), [0.0; 2]);
        assert!(weak.upgrade().is_some());
    }
    handle.update();
    assert!(weak.upgrade().is_none());

    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    assert!(handle.emitter_voiced(emitter));
    drop(pull(&mut mix, 9));
    handle.remove_sound(sound).expect("registered sound");
    handle.update();
    assert!(!handle.emitter_voiced(emitter));
    let tail = pull(&mut mix, 9);
    exact(tail[0], [pcm(2048); 2]);
    exact(tail[8], [0.0; 2]);
    assert_eq!(handle.record().emitters, 0);
}
