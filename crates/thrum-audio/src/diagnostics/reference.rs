use std::{num::NonZeroUsize, sync::Arc};

use crate::{
    Config, Fate, Handle, Listener, Mix, Outcome, Place, Play, PlayNumber, SoundId, VoiceSettings,
    Volumes,
    mix::{Command, EmitterVoice, StateChange},
};

const LENGTH: usize = 8193;
const STEPS: &[(usize, Step)] = &[
    (
        0,
        Step::Emitter(0, 0, settings(0.625, 0.9, Some(at(-1.0, 0.25))), false),
    ),
    (37, Step::Play(0, settings(0.25, 1.07, None), None)),
    (70, Step::Play(1, settings(0.375, 0.5, None), None)),
    (84, Step::Play(2, settings(1.0, 1.1, None), Some(1))),
    (85, Step::Play(2, settings(1.0, 1.0, None), Some(1))),
    (
        127,
        Step::Listener(Listener {
            position: [-0.5, 0.0, 0.0],
            right: [0.0, 1.0, 0.0],
            near: 0.25,
        }),
    ),
    (
        174,
        Step::Emitter(0, 0, settings(0.875, 1.5, Some(at(1.75, 0.0))), false),
    ),
    (
        250,
        Step::Emitter(1, 1, settings(0.5, 0.85, Some(at(-1.5, 1.0))), false),
    ),
    (
        256,
        Step::Emitter(0, 0, settings(0.875, 1.5, Some(at(1.75, 0.0))), true),
    ),
    (
        413,
        Step::Emitter(0, 0, settings(0.875, 1.5, Some(at(1.75, 0.0))), false),
    ),
    (
        601,
        Step::Volumes(Volumes {
            master: 0.75,
            music: 0.5,
            effects: 0.75,
        }),
    ),
    (701, Step::Stop(3)),
    (911, Step::End(0)),
    (
        999,
        Step::Play(0, settings(0.5, 2.0, Some(at(0.0, 0.75))), None),
    ),
    (1200, Step::End(1)),
    (1501, Step::Play(1, settings(1.0, 0.5, None), None)),
];

const fn at(x: f32, y: f32) -> Place {
    Place {
        position: [x, y, 0.0],
        radius: 4.0,
    }
}

const fn settings(volume: f32, pitch: f32, place: Option<Place>) -> VoiceSettings {
    VoiceSettings {
        volume,
        pitch,
        place,
    }
}

#[derive(Clone, Copy)]
enum Step {
    Play(usize, VoiceSettings, Option<usize>),
    Emitter(usize, usize, VoiceSettings, bool),
    End(usize),
    Stop(u64),
    Listener(Listener),
    Volumes(Volumes),
}

/// The frame-addressed reference script. Set it up before pulling and use arbitrary buffer sizes.
pub struct ReferenceRun {
    mix: Mix,
    handle: Handle,
    sounds: [SoundId; 3],
    frame: usize,
    next: usize,
    number: u64,
}

impl Default for ReferenceRun {
    fn default() -> Self {
        let config = Config {
            one_shots: 2,
            emitters: 2,
            ..Config::default()
        };
        let (mut handle, mut mix) = Handle::new(config).expect("valid reference configuration");
        let mono: Arc<[i16]> = (0_i32..509)
            .map(|i| i16::try_from(i * 83 % 12_001 - 6_000).expect("bounded mono sample"))
            .collect();
        let stereo: Arc<[i16]> = (0_i32..1021)
            .flat_map(|i| {
                [
                    i16::try_from(i * 277 % 24_001 - 12_000).expect("bounded left sample"),
                    i16::try_from(i * 967 % 20_001 - 10_000).expect("bounded right sample"),
                ]
            })
            .collect();
        let full: Arc<[i16]> = (0..1901)
            .map(|i| if i % 23 < 11 { 32767 } else { -32768 })
            .collect();
        let sounds = [
            handle.add_sound(mono, 1).expect("reference mono"),
            handle.add_sound(stereo, 2).expect("reference stereo"),
            handle.add_sound(full, 1).expect("reference full scale"),
        ];
        mix.start_pulling();
        Self {
            mix,
            handle,
            sounds,
            frame: 0,
            next: 0,
            number: 0,
        }
    }
}

impl ReferenceRun {
    /// Remaining stereo frames.
    pub fn remaining(&self) -> usize {
        LENGTH - self.frame
    }

    /// Writes up to the requested stereo frames, returning the number written.
    pub fn pull(&mut self, output: &mut [f32]) -> usize {
        let count = self.remaining().min(output.len() / 2);
        let end = self.frame + count;
        let mut written = 0;
        while self.frame < end {
            while STEPS
                .get(self.next)
                .is_some_and(|&(at, _)| at == self.frame)
            {
                self.apply(STEPS[self.next].1);
                self.next += 1;
            }
            let until = STEPS.get(self.next).map_or(end, |&(at, _)| at.min(end));
            let frames = until - self.frame;
            self.mix
                .render(&mut output[written * 2..(written + frames) * 2]);
            written += frames;
            self.frame = until;
        }
        written
    }

    fn apply(&mut self, step: Step) {
        let command = match step {
            Step::Play(clip, settings, limit) => {
                self.number += 1;
                let sound = self.sounds[clip];
                let play = Play {
                    sound,
                    settings,
                    limit: limit.and_then(NonZeroUsize::new),
                    event: self.number,
                    tick: None,
                };
                let outcome = Outcome {
                    number: PlayNumber(self.number),
                    event: play.event,
                    tick: None,
                    epoch: 0,
                    fate: Fate::Started,
                };
                Command::Play(play, outcome, self.handle.diagnostic_samples(sound))
            }
            Step::Emitter(slot, clip, settings, held) => {
                let sound = self.sounds[clip];
                Command::State(
                    0,
                    StateChange::Emitter(
                        slot,
                        Some(EmitterVoice {
                            identity: slot as u64 + 1,
                            sound,
                            settings,
                            held,
                            samples: self.handle.diagnostic_samples(sound),
                        }),
                        None,
                    ),
                )
            }
            Step::End(slot) => Command::State(0, StateChange::Emitter(slot, None, None)),
            Step::Stop(number) => Command::Stop(PlayNumber(number)),
            Step::Listener(listener) => Command::State(0, StateChange::Listener(listener)),
            Step::Volumes(volumes) => Command::State(0, StateChange::Volumes(volumes)),
        };
        self.mix.apply(command);
    }
}

/// FNV-1a over little-endian IEEE 754 bits in interleaved sample order.
pub struct Digest(u64);

impl Default for Digest {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Digest {
    /// Folds consecutive samples into the digest.
    pub fn write(&mut self, samples: &[f32]) {
        for sample in samples {
            for byte in sample.to_bits().to_le_bytes() {
                self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
            }
        }
    }

    /// The folded digest so far.
    pub fn finish(&self) -> u64 {
        self.0
    }
}
