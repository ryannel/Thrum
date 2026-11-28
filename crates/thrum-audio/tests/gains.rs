use thrum_audio::{Config, Play, VoiceSettings, Volumes};

use crate::support::{audio, close, config, exact, first, mono, pcm, pull, steps};

#[test]
fn effects_and_master_glide_for_both_shares() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![2048; 128]);
    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    handle.play(Play::new(sound, 1));
    let frames = pull(&mut mix, 10);
    exact(frames[0], [pcm(2048); 2]);
    exact(frames[8], [pcm(4096); 2]);
    assert!(handle.emitter_voiced(emitter));

    let mut volumes = Volumes {
        effects: 0.5,
        ..Volumes::default()
    };
    handle.set_volumes(volumes);
    let frames = pull(&mut mix, 10);
    exact(frames[0], [pcm(4096); 2]);
    exact(frames[9], [pcm(2048); 2]);
    steps(&frames, pcm(4096) * config.glide_step);
    volumes.master = 0.5;
    handle.set_volumes(volumes);
    let frames = pull(&mut mix, 10);
    exact(frames[0], [pcm(2048); 2]);
    exact(frames[9], [pcm(1024); 2]);
    steps(&frames, pcm(2048) * config.glide_step);
}

#[test]
fn stopping_holding_releasing_and_ending_use_a_frame_ramp() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![4096; 64]);
    let number = handle.play(Play::new(sound, 1));
    exact(first(&mut mix), [pcm(4096); 2]);
    assert!(handle.stop_play(number));
    let stopped = pull(&mut mix, 9);
    exact(stopped[0], [pcm(4096); 2]);
    exact(stopped[8], [0.0; 2]);
    steps(&stopped, pcm(4096) / config.ramp_frames as f32);
    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    let started = pull(&mut mix, 9);
    exact(started[0], [0.0; 2]);
    exact(started[8], [pcm(4096); 2]);
    steps(&started, pcm(4096) * config.glide_step);
    handle.hold_emitter(emitter, true);
    handle.update();
    assert_eq!(pull(&mut mix, 9), stopped);
    handle.hold_emitter(emitter, false);
    handle.update();
    assert_eq!(pull(&mut mix, 9), started);
    handle.end_emitter(emitter);
    handle.update();
    assert_eq!(pull(&mut mix, 9), stopped);
    assert_eq!(handle.record().voices, 0);
}

#[test]
fn clip_is_exact_below_knee_continuous_monotone_and_bounded() {
    let config = Config {
        one_shots: 8,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let signal = vec![
        -32768, -24000, -10000, -3073, -3072, -3071, -2001, -1, 0, 1, 2001, 3071, 3072, 3073,
        10000, 24000, 32767,
    ];
    let sound = mono(&mut handle, signal.clone());
    for _ in 0..8 {
        handle.play(Play::new(sound, 1));
    }
    let frames = pull(&mut mix, signal.len());
    let mut clipped = 0;
    for (output, input) in frames.iter().zip(signal) {
        assert!(output.iter().all(|sample| (-1.0..1.0).contains(sample)));
        let sum = 8.0 * pcm(input);
        if sum.abs() < config.knee {
            exact(*output, [sum; 2]);
        } else {
            clipped += 1;
        }
    }
    for pair in frames.windows(2) {
        assert!(pair[0][0] < pair[1][0], "{pair:?}");
    }
    close(frames[13][0] - frames[11][0], 0.0, pcm(16));
    close(frames[5][0] - frames[3][0], 0.0, pcm(16));
    assert_eq!(handle.record().clipped_frames, clipped);
    assert_eq!(handle.record().peak_voices, config.one_shots);
}
