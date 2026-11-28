use std::{hint::black_box, sync::Arc};

use crate::{Config, Error, Handle, Listener, Mix, Place, Play, SAMPLE_RATE, VoiceSettings};

const PITCH: f32 = 1.07;
const PULL: usize = 128;
const SECONDS: usize = 60;

/// One minute with every default voice slot occupied by a placed, pitched sound.
pub struct FullPool {
    handle: Handle,
    mix: Mix,
    output: [f32; PULL * 2],
}

impl FullPool {
    /// Prepares PCM and starts the voices before timing begins.
    ///
    /// # Errors
    /// Returns audio setup errors.
    ///
    /// # Panics
    /// The scene did not fill the voice pool.
    pub fn new() -> Result<Self, Error> {
        let config = Config::default();
        let (mut handle, mut mix) = Handle::new(config)?;
        let rate = SAMPLE_RATE as usize;
        let pcm = (0..rate * 66)
            .map(|i| {
                let value = i16::try_from(i % 997).expect("bounded PCM");
                (value - 498) * 2
            })
            .collect::<Arc<[i16]>>();
        let sound = handle.add_sound(pcm, 1)?;
        mix.start_pulling();
        let settings = VoiceSettings {
            pitch: PITCH,
            volume: 0.375,
            place: Some(Place {
                position: [1.0, 2.0, 0.0],
                radius: 8.0,
            }),
        };
        for index in 0..config.one_shots {
            handle.play(Play {
                settings,
                ..Play::new(sound, index as u64)
            });
        }
        for _ in 0..config.emitters {
            handle.start_emitter(sound, settings)?;
        }
        handle.update();
        let mut output = [0.0; PULL * 2];
        mix.pull(&mut output);
        assert_eq!(handle.record().voices, config.one_shots + config.emitters);
        Ok(Self {
            handle,
            mix,
            output,
        })
    }

    /// Renders 60 seconds. A witness samples each pull's output so the work cannot be elided.
    ///
    /// # Panics
    /// A pool slot fell silent during the run, or this prepared scene was already rendered.
    pub fn render(&mut self) -> u32 {
        let mut witness = 0_u32;
        for i in 0..(SAMPLE_RATE as usize * SECONDS).div_ceil(PULL) {
            let x = if i % 2 == 0 { -2.0 } else { 0.5 };
            self.handle.set_listener(Listener {
                position: [x, 0.0, 0.0],
                ..Listener::default()
            });
            self.mix.pull(black_box(&mut self.output));
            witness = witness.rotate_left(1) ^ self.output[130].to_bits();
        }
        let config = Config::default();
        assert_eq!(
            self.handle.record().voices,
            config.one_shots + config.emitters
        );
        witness
    }
}

/// A deterministic 10,000-emitter scene for measuring one moved-listener update.
pub struct MovingEmitters {
    handle: Handle,
    _mix: Mix,
}

impl MovingEmitters {
    /// Builds the scene, settles its initial voices, then moves 2,000 emitters and the listener.
    ///
    /// # Errors
    /// Returns audio setup errors.
    pub fn new() -> Result<Self, Error> {
        let (mut handle, mut mix) = Handle::new(Config::default())?;
        let sound = handle.add_sound(Arc::from([512]), 1)?;
        let mut moved = Vec::with_capacity(2000);
        for index in 0..10_000 {
            let xy = [
                ((index * 389) % 547) as f32 / 8.0 - 32.0,
                ((index * 739) % 443) as f32 / 8.0 - 32.0,
            ];
            let place = Place {
                position: [xy[0], xy[1], 0.0],
                radius: 21.0,
            };
            let settings = VoiceSettings {
                volume: (index % 1024) as f32 / 1024.0,
                place: Some(place),
                ..VoiceSettings::default()
            };
            let emitter = handle.start_emitter(sound, settings)?;
            if index % 5 == 0 {
                moved.push((emitter, place));
            }
        }
        mix.start_pulling();
        handle.update();
        mix.pull(&mut [0.0; 2]);
        for (emitter, mut place) in moved {
            place.position[0] += 3.0;
            handle.move_emitter(emitter, Some(place));
        }
        handle.set_listener(Listener {
            position: [3.0, -2.0, 0.0],
            ..Listener::default()
        });
        Ok(Self { handle, _mix: mix })
    }

    /// Runs the update and returns the count that was eligible.
    pub fn update(&mut self) -> usize {
        self.handle.update();
        self.handle.record().eligible_emitters
    }
}
