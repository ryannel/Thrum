use std::{num::NonZeroUsize, sync::Arc};

use thrum_audio::{
    Config, Error, Fate, Listener, Outcome, Play, PlayNumber, VoiceSettings, Volumes,
};

use crate::support::{
    audio, config, exact, first, mono, nonfinite_call, outcomes, pcm, place, pull,
};

#[test]
fn every_fate_preserves_call_number_event_tick_and_epoch() {
    let config = Config {
        commands: 1,
        one_shots: 1,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![512; 64]);
    let mut play = Play::new(sound, 0);
    let mut expected = Vec::new();
    let mut call = |handle: &mut thrum_audio::Handle, mut play: Play, epoch, fate| {
        let number = PlayNumber(expected.len() as u64 + 1);
        play.event = 100 + number.0;
        play.tick = Some(10 + number.0);
        let returned = if fate == Fate::BadValue {
            nonfinite_call(|| handle.play(play))
        } else {
            Some(handle.play(play))
        };
        if let Some(returned) = returned {
            assert_eq!(returned, number);
        }
        expected.push(Outcome {
            number,
            event: play.event,
            tick: play.tick,
            epoch,
            fate,
        });
    };

    call(&mut handle, play, 0, Fate::Started);
    call(&mut handle, play, 0, Fate::QueueFull);
    exact(first(&mut mix), [pcm(512); 2]);
    play.limit = NonZeroUsize::new(1);
    call(&mut handle, play, 0, Fate::Limit);
    let _ = first(&mut mix);
    play.limit = None;
    call(&mut handle, play, 0, Fate::Pool);
    let _ = first(&mut mix);
    play.settings.place = Some(place(20.0, 0.0, 10.0));
    call(&mut handle, play, 0, Fate::OutOfRange);
    let _ = first(&mut mix);
    play.settings.place = None;
    call(&mut handle, play, 0, Fate::Interrupted);
    mix.stop_pulling();
    call(&mut handle, play, 1, Fate::NothingPulling);
    mix.start_pulling();
    drop(pull(&mut mix, 8));
    play.settings.volume = f32::NAN;
    call(&mut handle, play, 1, Fate::BadValue);
    play.settings.volume = 2.0;
    call(&mut handle, play, 1, Fate::Started);
    exact(first(&mut mix), [pcm(512); 2]);
    assert_eq!(outcomes(&mut handle), expected);
    assert_eq!(outcomes(&mut handle), []);
    let record = handle.record();
    assert_eq!(record.voices, 1);
    assert_eq!(record.peak_voices, 1);
    assert_eq!(record.interrupted_voices, 1);
    assert_eq!(record.outcomes_lost, 0);
    assert_eq!(record.clamped, 1);
    assert_eq!(record.bad_values, 1);
    assert_eq!(record.queue_high_water, 1);
    assert_eq!(record.frames, 13);
    assert_eq!(record.epoch, 1);
    assert!(record.pulling);
    for fate in [
        Fate::Started,
        Fate::QueueFull,
        Fate::Limit,
        Fate::Pool,
        Fate::OutOfRange,
        Fate::Interrupted,
        Fate::NothingPulling,
        Fate::BadValue,
    ] {
        assert_eq!(record.plays(fate), u64::from(fate == Fate::Started) + 1);
    }
}

#[test]
fn same_event_two_plays_are_matched_by_play_number() {
    let cases = [
        (1, None, Fate::Started, Fate::QueueFull, 256),
        (
            2,
            Some(place(4.0, 0.0, 2.0)),
            Fate::OutOfRange,
            Fate::Started,
            512,
        ),
        (2, None, Fate::Started, Fate::Started, 768),
    ];
    for (commands, first_place, expected_first, expected_second, sample) in cases {
        let (mut handle, mut mix) = audio(Config {
            commands,
            ..config()
        });
        let sound = mono(&mut handle, vec![1024]);
        let play = Play {
            sound,
            settings: VoiceSettings {
                volume: 0.25,
                place: first_place,
                ..VoiceSettings::default()
            },
            limit: None,
            event: 42,
            tick: Some(16),
        };
        let a = handle.play(play);
        let b = handle.play(Play {
            settings: VoiceSettings {
                volume: 0.5,
                place: None,
                ..play.settings
            },
            ..play
        });
        exact(first(&mut mix), [pcm(sample); 2]);
        let actual = outcomes(&mut handle);
        assert_eq!(
            actual,
            [
                Outcome {
                    number: a,
                    event: 42,
                    tick: Some(16),
                    epoch: 0,
                    fate: expected_first
                },
                Outcome {
                    number: b,
                    event: 42,
                    tick: Some(16),
                    epoch: 0,
                    fate: expected_second
                },
            ]
        );
    }
}

#[test]
fn recent_outcomes_are_retained_across_both_producers() {
    let (mut handle, mut mix) = audio(Config {
        outcomes: 2,
        commands: 1,
        ..config()
    });
    let sound = mono(&mut handle, vec![128]);
    let a = handle.play(Play::new(sound, 1));
    let b = handle.play(Play::new(sound, 2));
    let _ = first(&mut mix);
    let c = handle.play(Play::new(sound, 3));
    let d = handle.play(Play::new(sound, 4));
    let _ = first(&mut mix);
    assert_eq!(handle.record().outcomes_lost, 2);
    let actual = outcomes(&mut handle);
    assert_eq!(
        actual.iter().map(|entry| entry.number).collect::<Vec<_>>(),
        [c, d]
    );
    assert_ne!(a, b);
    assert_eq!(handle.record().outcomes_lost, 2);
    assert_eq!(handle.record().plays(Fate::Started), 2);
    assert_eq!(handle.record().plays(Fate::QueueFull), 2);
    assert_eq!(outcomes(&mut handle), []);
}

#[test]
fn queue_pressure_keeps_only_the_latest_state() {
    let (mut handle, mut mix) = audio(Config {
        commands: 1,
        emitters: 1,
        ..config()
    });
    let sound = mono(&mut handle, vec![4096]);
    let emitter = handle
        .start_emitter(
            sound,
            VoiceSettings {
                place: Some(place(1.0, 0.0, 8.0)),
                ..VoiceSettings::default()
            },
        )
        .expect("valid emitter");
    handle.update();
    let denied = handle.play(Play::new(sound, 3));
    handle.set_volumes(Volumes {
        effects: 0.1,
        ..Volumes::default()
    });
    handle.set_volumes(Volumes {
        effects: 0.5,
        ..Volumes::default()
    });
    handle.set_listener(Listener {
        position: [5.0, 0.0, 0.0],
        ..Listener::default()
    });
    handle.set_listener(Listener {
        position: [2.0, 0.0, 0.0],
        ..Listener::default()
    });
    handle.move_emitter(emitter, Some(place(6.0, 0.0, 8.0)));
    handle.set_emitter(
        emitter,
        VoiceSettings {
            volume: 0.5,
            place: Some(place(2.0, 0.0, 8.0)),
            ..VoiceSettings::default()
        },
    );
    drop(pull(&mut mix, 9));
    assert_eq!(outcomes(&mut handle)[0].number, denied);
    assert_eq!(handle.record().plays(Fate::QueueFull), 1);
    for _ in 0..3 {
        handle.update();
        drop(pull(&mut mix, 9));
    }
    exact(first(&mut mix), [pcm(1024); 2]);
    let record = handle.record();
    assert_eq!(
        (
            record.emitters,
            record.eligible_emitters,
            record.voiced_emitters
        ),
        (1, 1, 1)
    );
}

#[test]
fn sound_table_errors_are_typed_and_leave_existing_sample_intact() {
    let (mut handle, mut mix) = audio(Config {
        sounds: 1,
        ..config()
    });
    let sound = mono(&mut handle, vec![1024]);
    assert_eq!(handle.add_sound(Arc::from([]), 1), Err(Error::EmptySound));
    assert_eq!(handle.add_sound(Arc::from([1]), 3), Err(Error::Channels));
    assert_eq!(handle.add_sound(Arc::from([1]), 0), Err(Error::Channels));
    assert_eq!(handle.add_sound(Arc::from([1]), 2), Err(Error::Frames));
    assert_eq!(
        handle.add_sound(Arc::from([1]), 1),
        Err(Error::SoundTableFull)
    );
    handle.play(Play::new(sound, 1));
    exact(first(&mut mix), [pcm(1024); 2]);
}

#[test]
fn mixed_script_accounts_for_all_counts_even_with_outcome_overflow() {
    let config = Config {
        commands: 1,
        one_shots: 1,
        emitters: 1,
        outcomes: 2,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let calm = mono(&mut handle, vec![512; 128]);
    let loud = mono(&mut handle, vec![32767; 128]);
    handle
        .start_emitter(calm, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    handle.play(Play::new(calm, 1));
    drop(pull(&mut mix, 9));
    let quiet = Play {
        settings: VoiceSettings {
            volume: 0.25,
            ..VoiceSettings::default()
        },
        ..Play::new(calm, 2)
    };
    handle.play(quiet);
    exact(first(&mut mix), [pcm(640); 2]);
    handle.play(Play {
        limit: NonZeroUsize::new(1),
        ..quiet
    });
    let _ = first(&mut mix);
    handle.play(quiet);
    let _ = first(&mut mix);
    handle.play(Play {
        settings: VoiceSettings {
            place: Some(place(9.0, 0.0, 1.0)),
            ..quiet.settings
        },
        ..quiet
    });
    let _ = first(&mut mix);
    handle.play(Play {
        settings: VoiceSettings {
            volume: 2.0,
            ..VoiceSettings::default()
        },
        ..Play::new(loud, 6)
    });
    for frame in pull(&mut mix, 9) {
        assert!(frame[0] > config.knee && frame[0] < 1.0);
    }
    handle.play(quiet);
    mix.stop_pulling();
    handle.play(quiet);
    nonfinite_call(|| {
        handle.play(Play {
            settings: VoiceSettings {
                pitch: f32::NAN,
                ..quiet.settings
            },
            ..quiet
        })
    });
    mix.start_pulling();
    drop(pull(&mut mix, 9));
    let retained = outcomes(&mut handle);
    assert_eq!(retained.len(), 2);
    assert_eq!(
        [retained[0].fate, retained[1].fate],
        [Fate::NothingPulling, Fate::BadValue]
    );
    let record = handle.record();
    assert_eq!((record.voices, record.peak_voices), (1, 3));
    assert_eq!(
        (
            record.emitters,
            record.eligible_emitters,
            record.voiced_emitters
        ),
        (1, 1, 1)
    );
    assert_eq!(
        (
            record.displaced,
            record.interrupted_voices,
            record.outcomes_lost
        ),
        (1, 1, 7)
    );
    assert_eq!(
        (record.clamped, record.bad_values, record.clipped_frames),
        (1, 1, 9)
    );
    assert_eq!(
        (
            record.queue_high_water,
            record.frames,
            record.epoch,
            record.pulling
        ),
        (1, 31, 1, true)
    );
    for fate in [
        Fate::Started,
        Fate::Limit,
        Fate::Pool,
        Fate::OutOfRange,
        Fate::NothingPulling,
        Fate::Interrupted,
        Fate::QueueFull,
        Fate::BadValue,
    ] {
        assert_eq!(record.plays(fate), 1 + u64::from(fate == Fate::Started));
    }
}

#[test]
fn continuous_control_writes_leave_room_for_pending_emitters() {
    let (mut handle, mut mix) = audio(Config {
        commands: 1,
        emitters: 2,
        ..config()
    });
    let sound = mono(&mut handle, vec![2048]);
    let emitters = [(); 2].map(|()| {
        handle
            .start_emitter(sound, VoiceSettings::default())
            .expect("valid emitter")
    });
    for frame in 0..120_i16 {
        handle.set_listener(Listener {
            position: [f32::from(frame), 0.0, 0.0],
            ..Listener::default()
        });
        handle.set_volumes(Volumes {
            effects: if frame < 112 { 1.0 } else { 0.5 },
            ..Volumes::default()
        });
        handle.update();
        let frames = pull(&mut mix, 400);
        if frame == 7 {
            exact(frames[399], [pcm(4096); 2]);
        }
        if frame == 119 {
            exact(frames[399], [pcm(2048); 2]);
        }
    }
    assert!(emitters.into_iter().all(|id| handle.emitter_voiced(id)));
    assert_eq!(handle.record().voices, 2);
}

#[test]
fn continuous_plays_leave_room_for_a_master_mute() {
    let (mut handle, mut mix) = audio(Config {
        commands: 1,
        emitters: 1,
        ..config()
    });
    let loop_sound = mono(&mut handle, vec![4096]);
    let silent_shot = mono(&mut handle, vec![0]);
    handle
        .start_emitter(loop_sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    let settled = pull(&mut mix, 9);
    exact(settled[8], [pcm(4096); 2]);

    for call in 0..120 {
        let play = Play {
            tick: Some(call + 17),
            ..Play::new(silent_shot, call + 50)
        };
        let number = handle.play(play);
        if call == 0 {
            handle.set_volumes(Volumes {
                master: 0.0,
                ..Volumes::default()
            });
            // The setter must register the retry even before the next update.
        } else {
            handle.update();
        }
        let frames = pull(&mut mix, 1200);
        exact(
            frames[1199],
            if call == 0 { [pcm(4096); 2] } else { [0.0; 2] },
        );
        assert_eq!(
            outcomes(&mut handle),
            [Outcome {
                number,
                tick: play.tick,
                event: play.event,
                epoch: 0,
                fate: if call == 1 {
                    Fate::QueueFull
                } else {
                    Fate::Started
                },
            }]
        );
    }
    assert_eq!(handle.record().plays(Fate::QueueFull), 1);
    assert_eq!(handle.record().plays(Fate::Started), 119);
}

#[test]
fn a_releasing_voice_counts_until_its_last_ramp_frame() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![4096; 64]);
    let number = handle.play(Play::new(sound, 1));
    exact(first(&mut mix), [pcm(4096); 2]);
    assert!(handle.stop_play(number));
    let releasing = pull(&mut mix, 2);
    exact(releasing[1], [pcm(3584); 2]);
    assert_eq!(
        (handle.record().voices, handle.record().peak_voices),
        (1, 1)
    );
    drop(pull(&mut mix, 6));
    assert_eq!(
        (handle.record().voices, handle.record().peak_voices),
        (0, 1)
    );
}

#[test]
fn handle_and_mix_can_exchange_commands_and_outcomes_on_distinct_threads() {
    let (mut handle, mut mix) = audio(config());
    let original = Arc::from([512, 1024]);
    let released = Arc::downgrade(&original);
    let sound = handle.add_sound(original, 1).expect("valid PCM");
    std::thread::scope(|scope| {
        let (pull_request, requests) = std::sync::mpsc::sync_channel::<()>(0);
        let (response, frames) = std::sync::mpsc::sync_channel(0);
        scope.spawn(move || {
            while requests.recv().is_ok() {
                response
                    .send(first(&mut mix))
                    .expect("handle receives the pull");
            }
        });
        let pull = || {
            pull_request.send(()).expect("mix receives the request");
            frames.recv().expect("mix returns a frame")
        };

        let old = handle.play(Play::new(sound, 44));
        exact(pull(), [pcm(512); 2]);
        handle
            .replace_sound(sound, Arc::from([2048]), 1)
            .expect("registered sound");
        let new = handle.play(Play::new(sound, 45));
        exact(pull(), [pcm(3072); 2]);
        assert!(released.upgrade().is_some());
        let outcomes = outcomes(&mut handle);
        assert_eq!([outcomes[0].number, outcomes[1].number], [old, new]);
        assert_eq!([outcomes[0].fate, outcomes[1].fate], [Fate::Started; 2]);
        assert!(released.upgrade().is_none());
    });
}
