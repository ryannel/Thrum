use std::num::NonZeroUsize;

use thrum_audio::{Config, Fate, Play, VoiceSettings};

use crate::support::{audio, config, exact, fate, first, mono, outcomes, pcm, pull, steps};

#[test]
fn every_frame_and_fate_over_forty_plays() {
    let cases: [(Option<usize>, usize, Vec<usize>); 5] = [
        (Some(4), 48_000, vec![0, 1, 2, 3, 40]),
        (Some(4), 14_400, (0..40).filter(|i| i % 12 < 4).collect()),
        (Some(4), 480, (0..40).collect()),
        (Some(1), 14_400, vec![0, 12, 24, 36]),
        (None, 48_000, (0..40).collect()),
    ];

    for (limit, length, started) in cases {
        let (mut handle, mut mix) = audio(Config {
            one_shots: 40,
            ..config()
        });
        let sound = mono(&mut handle, vec![128; length]);
        let mut rendered = Vec::new();
        let calls = if limit == Some(4) && length == 48_000 {
            41
        } else {
            40
        };
        for i in 0..calls {
            let mut play = Play::new(sound, i as u64);
            play.limit = limit.and_then(NonZeroUsize::new);
            let number = handle.play(play);
            rendered.extend(pull(&mut mix, 1200));
            fate(
                &mut handle,
                number,
                if started.contains(&i) {
                    Fate::Started
                } else {
                    Fate::Limit
                },
            );
        }
        let end = started.last().expect("at least one play") * 1200 + length + 1200;
        rendered.extend(pull(&mut mix, end - rendered.len()));
        for (frame, actual) in rendered.into_iter().enumerate() {
            let covering = started
                .iter()
                .filter(|&&i| (i * 1200..i * 1200 + length).contains(&frame))
                .count();
            exact(actual, [pcm(128) * covering as f32; 2]);
        }
        assert_eq!(handle.record().plays(Fate::Started), started.len() as u64);
        assert_eq!(
            handle.record().plays(Fate::Limit),
            (calls - started.len()) as u64
        );
        assert_eq!(handle.record().voices, 0);
    }
}

#[test]
fn a_play_limit_counts_a_sounding_emitter_and_its_release() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![2048]);
    let emitter = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    drop(pull(&mut mix, 9));
    let play = Play {
        limit: NonZeroUsize::new(1),
        ..Play::new(sound, 1)
    };
    let rejected = handle.play(play);
    exact(first(&mut mix), [pcm(2048); 2]);
    fate(&mut handle, rejected, Fate::Limit);

    handle.end_emitter(emitter);
    handle.update();
    let rejected = handle.play(play);
    exact(first(&mut mix), [pcm(2048); 2]);
    fate(&mut handle, rejected, Fate::Limit);
    drop(pull(&mut mix, 7));
    let started = handle.play(play);
    exact(first(&mut mix), [pcm(2048); 2]);
    fate(&mut handle, started, Fate::Started);
}

#[test]
fn displacement_ramps_even_with_one_source_frame_left() {
    let (mut handle, mut mix) = audio(Config {
        one_shots: 1,
        ..config()
    });
    let short = mono(&mut handle, vec![8192; 2]);
    let replacement = mono(&mut handle, vec![0; 9]);
    handle.play(Play {
        settings: VoiceSettings {
            volume: 0.5,
            ..VoiceSettings::default()
        },
        ..Play::new(short, 1)
    });
    exact(first(&mut mix), [pcm(4096); 2]);
    handle.play(Play::new(replacement, 2));
    let frames = pull(&mut mix, 9);
    exact(frames[0], [pcm(4096); 2]);
    exact(frames[7], [pcm(512); 2]);
    exact(frames[8], [0.0; 2]);
    steps(&frames, pcm(4096) / 8.0);
    assert_eq!(handle.record().displaced, 1);
}

#[test]
fn limits_are_per_sound_and_shares_stay_separate() {
    let (mut handle, mut mix) = audio(Config {
        one_shots: 5,
        emitters: 1,
        ..config()
    });
    let one = mono(&mut handle, vec![128; 64]);
    let two = mono(&mut handle, vec![256; 64]);
    let mut play = Play::new(one, 1);
    play.limit = NonZeroUsize::new(4);
    for _ in 0..4 {
        handle.play(play);
    }
    play.sound = two;
    let other = handle.play(play);
    exact(first(&mut mix), [pcm(4 * 128 + 256); 2]);
    assert_eq!(
        outcomes(&mut handle).last().expect("five outcomes").number,
        other
    );

    let emitter = handle
        .start_emitter(two, VoiceSettings::default())
        .expect("valid loop");
    let unvoiced = handle
        .start_emitter(two, VoiceSettings::default())
        .expect("valid loop");
    handle.update();
    drop(pull(&mut mix, 8));
    assert!(handle.emitter_voiced(emitter));
    assert!(!handle.emitter_voiced(unvoiced));
    play.limit = None;
    let rejected = handle.play(play);
    exact(first(&mut mix), [pcm(4 * 128 + 256 + 256); 2]);
    fate(&mut handle, rejected, Fate::Pool);
    assert!(handle.emitter_voiced(emitter));
    assert_eq!(handle.record().voices, 6);

    let zero_shots = Config {
        one_shots: 0,
        emitters: 1,
        ..config()
    };
    let (mut handle, mut mix) = audio(zero_shots);
    let sound = mono(&mut handle, vec![256]);
    let pending = handle.play(Play::new(sound, 1));
    exact(first(&mut mix), [0.0; 2]);
    fate(&mut handle, pending, Fate::Pool);
}

#[test]
fn pool_replaces_only_a_quieter_voice_and_ramps_its_tail() {
    let config = Config {
        one_shots: 1,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![4096; 64]);
    let mut play = Play::new(sound, 1);
    play.settings.volume = 0.25;
    handle.play(play);
    exact(first(&mut mix), [pcm(1024); 2]);
    drop(outcomes(&mut handle));
    let equal = handle.play(play);
    exact(first(&mut mix), [pcm(1024); 2]);
    fate(&mut handle, equal, Fate::Pool);
    play.settings.volume = 0.5;
    let louder = handle.play(play);
    let tail: Vec<_> = pull(&mut mix, 9)
        .into_iter()
        .map(|frame| frame.map(|sample| sample - pcm(2048)))
        .collect();
    exact(tail[0], [pcm(1024); 2]);
    exact(tail[8], [0.0; 2]);
    steps(&tail, pcm(1024) / config.ramp_frames as f32);
    fate(&mut handle, louder, Fate::Started);
    assert_eq!(handle.record().displaced, 1);
}
