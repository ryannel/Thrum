use std::sync::Arc;

use thrum_audio::{Config, Fate, Handle, Mix, Outcome, Place, PlayNumber, SoundId};

pub(crate) fn config() -> Config {
    Config {
        ramp_frames: 8,
        glide_step: 1.0 / 8.0,
        ..Config::default()
    }
}

pub(crate) fn audio(config: Config) -> (Handle, Mix) {
    let (handle, mut mix) = Handle::new(config).expect("test configuration");
    mix.start_pulling();
    (handle, mix)
}

pub(crate) fn mono(handle: &mut Handle, samples: Vec<i16>) -> SoundId {
    handle
        .add_sound(Arc::from(samples), 1)
        .expect("valid mono PCM")
}

pub(crate) fn stereo(handle: &mut Handle, samples: Vec<i16>) -> SoundId {
    handle
        .add_sound(Arc::from(samples), 2)
        .expect("valid stereo PCM")
}

pub(crate) fn pcm(sample: i16) -> f32 {
    f32::from(sample) / 32768.0
}

pub(crate) fn pull(mix: &mut Mix, frames: usize) -> Vec<[f32; 2]> {
    let mut output = vec![[f32::NAN; 2]; frames];
    mix.pull(output.as_flattened_mut());
    output
}

pub(crate) fn first(mix: &mut Mix) -> [f32; 2] {
    let mut output = [f32::NAN; 2];
    mix.pull(&mut output);
    output
}

pub(crate) fn outcomes(handle: &mut Handle) -> Vec<Outcome> {
    let mut result = Vec::new();
    handle.read_outcomes(|outcome| result.push(outcome));
    result.sort_unstable_by_key(|outcome| outcome.number);
    result
}

pub(crate) fn fate(handle: &mut Handle, number: PlayNumber, expected: Fate) {
    let actual = outcomes(handle);
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].number, number);
    assert_eq!(actual[0].fate, expected);
}

pub(crate) fn place(x: f32, y: f32, radius: f32) -> Place {
    Place {
        position: [x, y, 0.0],
        radius,
    }
}

pub(crate) fn exact(got: [f32; 2], expected: [f32; 2]) {
    assert_eq!(got.map(f32::to_bits), expected.map(f32::to_bits));
}

pub(crate) fn close(got: f32, expected: f32, tolerance: f32) {
    assert!(
        (got - expected).abs() <= tolerance,
        "{got} vs {expected}, tolerance {tolerance}"
    );
}

pub(crate) fn steps(frames: &[[f32; 2]], max: f32) {
    for pair in frames.windows(2) {
        for channel in 0..2 {
            assert!(
                (pair[0][channel] - pair[1][channel]).abs() <= max + 2.0 * f32::EPSILON,
                "step larger than {max}: {pair:?}"
            );
        }
    }
}
