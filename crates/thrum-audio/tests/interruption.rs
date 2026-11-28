use thrum_audio::{Config, Fate, Handle, Listener, Outcome, Play, VoiceSettings, Volumes};

use crate::support::{
    audio, config, exact, first, mono, outcomes, pcm, place, pull, steps, stereo,
};

#[test]
fn two_stops_expire_pending_plays_and_cut_started_voices_by_epoch() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![2048; 64]);
    let mut play = Play::new(sound, 1);
    play.tick = Some(12);
    let mut expected = Vec::new();
    for epoch in 0..2 {
        let started = handle.play(play);
        let started_frame = pull(&mut mix, 9);
        exact(started_frame[8], [pcm(2048); 2]);
        expected.push((started, epoch, Fate::Started));
        let expired = handle.play(play);
        mix.stop_pulling();
        expected.push((expired, epoch, Fate::Interrupted));
        assert_eq!(handle.record().epoch, epoch + 1);
        let absent = handle.play(play);
        expected.push((absent, epoch + 1, Fate::NothingPulling));
        exact(first(&mut mix), [0.0; 2]);
        mix.start_pulling();
        for frame in pull(&mut mix, 12) {
            exact(frame, [0.0; 2]);
        }
    }
    let final_play = handle.play(play);
    exact(first(&mut mix), [pcm(2048); 2]);
    expected.push((final_play, 2, Fate::Started));
    let results = outcomes(&mut handle);
    assert_eq!(results.len(), expected.len());
    for (outcome, (number, epoch, fate)) in results.into_iter().zip(expected) {
        assert_eq!(
            (
                outcome.number,
                outcome.event,
                outcome.tick,
                outcome.epoch,
                outcome.fate
            ),
            (number, 1, Some(12), epoch, fate)
        );
    }
    assert_eq!(handle.record().interrupted_voices, 2);
}

#[test]
fn continuous_loops_resume_at_their_phase_with_latest_state() {
    let config = Config {
        commands: 2,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let signal = mono(&mut handle, (0..64).map(|i| (i + 1) * 128).collect());
    let steady = stereo(&mut handle, vec![4096, 0]);
    let removed = mono(&mut handle, vec![16384]);
    let retained = handle
        .start_emitter(signal, VoiceSettings::default())
        .expect("valid emitter");
    let ended = handle
        .start_emitter(removed, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 10));
    mix.stop_pulling();
    handle.end_emitter(ended);
    let added = handle
        .start_emitter(steady, VoiceSettings::default())
        .expect("valid emitter");
    handle.set_emitter(
        retained,
        VoiceSettings {
            volume: 0.5,
            place: Some(place(4.0, 0.0, 8.0)),
            ..VoiceSettings::default()
        },
    );
    handle.set_listener(Listener {
        position: [4.0, 0.0, 0.0],
        ..Listener::default()
    });
    let volumes = Volumes {
        master: 0.5,
        effects: 0.5,
        music: 0.25,
    };
    handle.set_volumes(volumes);
    handle.update();
    assert_eq!(handle.volumes(), volumes);
    mix.start_pulling();
    let frames = pull(&mut mix, 18);
    exact(frames[0], [0.0; 2]);
    // The retained loop's first resumed frame is its source frame 10. The end of
    // the ramp makes its position and both persistent volume changes exact.
    let base = pcm(19 * 128) * 0.125;
    exact(frames[8], [base + pcm(1024), base]);
    let base = pcm(28 * 128) * 0.125;
    exact(frames[17], [base + pcm(1024), base]);
    assert!(handle.emitter_voiced(added));
    assert!(!handle.emitter_voiced(ended));
    assert_eq!(handle.record().voices, 2);
}

#[test]
fn resume_ramps_and_a_new_mix_rebuilds_desired_state_without_old_plays() {
    let config = config();
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![2048; 64]);
    let looping = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 9));
    mix.stop_pulling();
    mix.start_pulling();
    let resume = pull(&mut mix, 9);
    exact(resume[0], [0.0; 2]);
    exact(resume[8], [pcm(2048); 2]);
    steps(&resume, pcm(2048) / config.ramp_frames as f32);
    let expired = handle.play(Play::new(sound, 2));
    handle.set_emitter(
        looping,
        VoiceSettings {
            volume: 0.5,
            place: Some(place(2.0, 0.0, 8.0)),
            ..VoiceSettings::default()
        },
    );
    handle.set_listener(Listener {
        position: [2.0, 0.0, 0.0],
        ..Listener::default()
    });
    handle.set_volumes(Volumes {
        master: 0.5,
        effects: 0.5,
        music: 0.125,
    });
    handle.rebuild_mix(&mut mix).expect("own mix");
    assert_eq!(handle.record().epoch, 2);
    mix.start_pulling();
    let rebuilt = pull(&mut mix, 20);
    exact(rebuilt[0], [0.0; 2]);
    exact(rebuilt[19], [pcm(256); 2]);
    let outcomes = outcomes(&mut handle);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        (outcomes[0].number, outcomes[0].fate, outcomes[0].epoch),
        (expired, Fate::Interrupted, 1)
    );
    assert_eq!(handle.play(Play::new(sound, 3)).0, expired.0 + 1);
}

#[test]
fn resume_preserves_a_loop_even_when_the_handle_reassigns_its_slot() {
    let config = Config {
        emitters: 2,
        commands: 2,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let signal = mono(&mut handle, (1..=64).map(|i| i * 64).collect());
    let zero = mono(&mut handle, vec![0]);
    let a = handle
        .start_emitter(signal, VoiceSettings::default())
        .expect("valid emitter");
    let b = handle
        .start_emitter(zero, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 11));
    mix.stop_pulling();
    handle.move_emitter(a, Some(place(10.0, 0.0, 1.0)));
    handle.update();
    assert!(!handle.emitter_voiced(a));
    let c = handle
        .start_emitter(zero, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    assert!(handle.emitter_voiced(c));
    handle.end_emitter(b);
    handle.move_emitter(a, None);
    handle.update();
    assert!(handle.emitter_voiced(a));
    mix.start_pulling();
    let frames = pull(&mut mix, 9);
    exact(frames[1], [pcm(13 * 64) / 8.0; 2]);
    exact(frames[8], [pcm(20 * 64); 2]);
}

#[test]
fn an_emitter_added_while_stopped_uses_one_resume_ramp_and_can_end() {
    let config = Config {
        commands: 1,
        emitters: 1,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![8192]);
    mix.stop_pulling();
    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    mix.start_pulling();
    let frames = pull(&mut mix, 9);
    exact(frames[0], [0.0; 2]);
    exact(frames[8], [pcm(8192); 2]);
    steps(&frames, pcm(8192) / 8.0);
    handle.end_emitter(emitter);
    handle.update();
    let ending = pull(&mut mix, 9);
    exact(ending[0], [pcm(8192); 2]);
    exact(ending[8], [0.0; 2]);
    steps(&ending, pcm(8192) / 8.0);
    assert_eq!(handle.record().voices, 0);
}

#[test]
fn deferred_emitter_changes_survive_a_play_before_the_first_resumed_update() {
    for commands in [1, Config::default().commands] {
        for (held, end, while_stopped) in [
            (false, true, true),
            (false, true, false),
            (true, false, false),
            (false, false, true),
        ] {
            let (mut handle, mut mix) = audio(Config {
                commands,
                emitters: 1,
                ..config()
            });
            let signal = mono(&mut handle, vec![8192]);
            let silent = Play::new(mono(&mut handle, vec![0]), 0);
            let emitter = handle
                .start_emitter(signal, VoiceSettings::default())
                .expect("valid emitter");
            handle.hold_emitter(emitter, held);
            handle.update();
            drop(pull(&mut mix, 16));
            for _ in 0..commands {
                handle.play(silent);
            }
            handle.set_emitter(
                emitter,
                VoiceSettings {
                    volume: 0.5,
                    ..VoiceSettings::default()
                },
            );
            handle.update();
            mix.stop_pulling();
            if !while_stopped {
                mix.start_pulling();
            }
            if end {
                handle.end_emitter(emitter);
            } else if held {
                handle.hold_emitter(emitter, false);
            } else {
                handle.set_emitter(
                    emitter,
                    VoiceSettings {
                        volume: 0.25,
                        ..VoiceSettings::default()
                    },
                );
            }
            if while_stopped {
                mix.start_pulling();
            }
            handle.play(silent);
            handle.update();
            let expected = if end {
                0.0
            } else if held {
                pcm(4096)
            } else {
                pcm(2048)
            };
            let frames = pull(&mut mix, 32);
            for frame in &frames[16..] {
                exact(*frame, [expected; 2]);
            }
            assert_eq!(handle.record().voices, usize::from(!end));
            assert_eq!(handle.emitter_voiced(emitter), !end);
        }
    }
}

#[test]
fn deferred_state_is_restored_if_interrupted_again_before_the_queue_drains() {
    let (mut handle, mut mix) = audio(Config {
        commands: 1,
        emitters: 1,
        ..config()
    });
    let signal = mono(&mut handle, vec![8192]);
    let silent = Play::new(mono(&mut handle, vec![0]), 0);
    let emitter = handle
        .start_emitter(signal, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 16));
    let interrupted = handle.play(silent);
    handle.set_emitter(
        emitter,
        VoiceSettings {
            volume: 0.5,
            ..VoiceSettings::default()
        },
    );
    handle.update();
    mix.stop_pulling();
    mix.start_pulling();
    handle.set_emitter(
        emitter,
        VoiceSettings {
            volume: 0.25,
            ..VoiceSettings::default()
        },
    );
    // A stop request, like a play, gives the deferred emitter the open queue slot.
    assert!(!handle.stop_play(interrupted));
    mix.stop_pulling();
    mix.start_pulling();
    let frames = pull(&mut mix, 16);
    exact(frames[0], [0.0; 2]);
    exact(frames[8], [pcm(2048); 2]);
}

#[test]
fn dropping_the_mix_expires_queued_plays_and_closes_the_epoch() {
    let (mut handle, mut mix) = Handle::new(config()).expect("valid connection");
    let sound = mono(&mut handle, vec![8192]);
    mix.start_pulling();
    let play = Play {
        tick: Some(12),
        ..Play::new(sound, 53)
    };
    let queued = handle.play(play);
    drop(mix);
    assert!(!handle.record().pulling);
    assert_eq!(handle.record().epoch, 1);
    let absent = handle.play(play);
    let expected = Outcome {
        number: queued,
        event: 53,
        tick: Some(12),
        epoch: 0,
        fate: Fate::Interrupted,
    };
    assert_eq!(
        outcomes(&mut handle),
        [
            expected,
            Outcome {
                number: absent,
                epoch: 1,
                fate: Fate::NothingPulling,
                ..expected
            }
        ]
    );
    assert_eq!(handle.record().plays(Fate::Interrupted), 1);
    assert_eq!(outcomes(&mut handle), []);
}
