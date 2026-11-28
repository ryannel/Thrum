use thrum_audio::{Fate, Play};

use crate::support::{
    audio, close, config, exact, fate, first, mono, nonfinite_call, pcm, pull, stereo,
};

#[test]
fn silence_and_entire_mono_sounds() {
    const PULL: usize = 128;
    let (mut handle, mut mix) = audio(config());
    for frame in pull(&mut mix, PULL) {
        exact(frame, [0.0; 2]);
    }

    for length in [1, PULL - 1, PULL * 4 + 7] {
        let samples: Vec<_> = (0..length)
            .map(|i| i16::try_from(i).expect("small input") - 19)
            .collect();
        let sound = mono(&mut handle, samples.clone());
        let number = handle.play(Play::new(sound, 7));
        let mut rendered = Vec::new();
        for _ in 0..length.div_ceil(PULL) {
            rendered.extend(pull(&mut mix, PULL));
        }
        assert_eq!(handle.record().voices, 0);
        rendered.extend(pull(&mut mix, PULL));
        for (i, frame) in rendered.into_iter().enumerate() {
            exact(frame, [pcm(samples.get(i).copied().unwrap_or(0)); 2]);
        }
        fate(&mut handle, number, Fate::Started);
    }
}

#[test]
fn stereo_channel_order_two_starts_and_volume() {
    let (mut handle, mut mix) = audio(config());
    let stereo = stereo(&mut handle, vec![1024, -4096]);
    handle.play(Play::new(stereo, 1));
    exact(first(&mut mix), [pcm(1024), pcm(-4096)]);

    let sound = mono(&mut handle, vec![2048; 700]);
    let mut play = Play::new(sound, 8);
    play.settings.volume = 0.375;
    handle.play(play);
    for frame in pull(&mut mix, 480) {
        exact(frame, [pcm(2048) * 0.375; 2]);
    }
    handle.play(Play::new(sound, 9));
    for (i, frame) in pull(&mut mix, 702).into_iter().enumerate() {
        let expected = f32::from(i < 220) * 0.375 + f32::from(i < 700);
        exact(frame, [pcm(2048) * expected; 2]);
    }
}

#[test]
fn integer_and_half_pitch() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![64, 192, 320, 448]);
    let mut play = Play::new(sound, 1);
    play.settings.pitch = 2.0;
    handle.play(play);
    for (frame, sample) in pull(&mut mix, 4).into_iter().zip([64, 320, 0, 0]) {
        exact(frame, [pcm(sample); 2]);
    }
    play.settings.pitch = 0.5;
    handle.play(play);
    let frames = pull(&mut mix, 10);
    for (frame, sample) in frames.iter().step_by(2).zip([64, 192, 320, 448, 0]) {
        exact(*frame, [pcm(sample); 2]);
    }
    assert_ne!(frames[7][0].to_bits(), 0);
    exact(frames[9], [0.0; 2]);
}

#[test]
fn fractional_pitch_follows_cubic_phase_without_drift() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, (1..=4800).collect());
    for pitch in [0.9, 1.07, 1.1] {
        let mut play = Play::new(sound, 1);
        play.settings.pitch = pitch;
        handle.play(play);
        let frames = pull(&mut mix, 5500);
        let length = frames
            .iter()
            .position(|frame| frame[0].to_bits() == 0)
            .expect("sound ends");
        assert!((length as f64 - 4800.0 / f64::from(pitch)).abs() <= 1.0);
        for (k, frame) in frames[..length].iter().enumerate() {
            let position = k as f32 * pitch;
            if (2.0..4797.0).contains(&position) {
                for sample in frame {
                    close(*sample, (1.0 + position) / 32768.0, 2.0 / 32768.0);
                }
            }
        }
    }
}

#[test]
fn invalid_plays_drop_and_finite_extremes_clamp() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![128, 256, 512, 1024]);
    let mut play = Play::new(sound, 1);
    play.settings.volume = 3.0;
    play.settings.pitch = 3.0;
    handle.play(play);
    for (frame, sample) in pull(&mut mix, 4).into_iter().zip([128, 512, 0, 0]) {
        exact(frame, [pcm(sample); 2]);
    }
    play.settings.volume = 1.0;
    play.settings.pitch = 0.0;
    handle.play(play);
    let lower_pitch = pull(&mut mix, 9);
    for (frame, sample) in lower_pitch.iter().step_by(2).zip([128, 256, 512, 1024, 0]) {
        exact(*frame, [pcm(sample); 2]);
    }
    assert_ne!(lower_pitch[7][0].to_bits(), 0);
    play.settings.volume = -1.0;
    play.settings.pitch = 1.0;
    handle.play(play);
    assert_eq!(pull(&mut mix, 5), [[0.0; 2]; 5]);
    assert_eq!(handle.record().clamped, 4);
    drop(crate::support::outcomes(&mut handle));
    for (volume, pitch) in [(f32::NAN, 1.0), (1.0, f32::INFINITY)] {
        play.settings.volume = volume;
        play.settings.pitch = pitch;
        let number = nonfinite_call(|| handle.play(play));
        let outcome = crate::support::outcomes(&mut handle)
            .pop()
            .expect("invalid play outcome");
        assert_eq!(outcome.fate, Fate::BadValue);
        if let Some(number) = number {
            assert_eq!(outcome.number, number);
        }
    }
    exact(first(&mut mix), [0.0; 2]);
    assert_eq!(handle.record().plays(Fate::BadValue), 2);
    assert_eq!(handle.record().bad_values, 2);
}

#[test]
fn reference_frames_are_bit_exact_across_pull_sizes_and_match_retained_digest() {
    use thrum_audio::diagnostics::{Digest, ReferenceRun};

    fn render(sizes: &[usize]) -> Vec<f32> {
        let mut run = ReferenceRun::default();
        let mut result = Vec::with_capacity(run.remaining() * 2);
        let mut output = vec![0.0; 8192];
        for request in sizes.iter().cycle() {
            if run.remaining() == 0 {
                break;
            }
            let count = run.pull(&mut output[..request * 2]);
            result.extend_from_slice(&output[..count * 2]);
        }
        result
    }

    let baseline = render(&[1]);
    for sizes in [&[37][..], &[128], &[512], &[4096], &[1, 37, 128, 512, 4096]] {
        let actual = render(sizes);
        assert_eq!(actual.len(), baseline.len());
        for (index, (&sample, &expected)) in actual.iter().zip(&baseline).enumerate() {
            assert_eq!(
                sample.to_bits(),
                expected.to_bits(),
                "sample {index}, pulls {sizes:?}"
            );
        }
    }
    let mut digest = Digest::default();
    digest.write(&baseline);
    assert_eq!(
        format!("{:016x}", digest.finish()),
        include_str!("reference.digest").trim()
    );
}
