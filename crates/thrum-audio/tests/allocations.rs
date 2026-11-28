use thrum_audio::{Config, Listener, Play, VoiceSettings, Volumes, diagnostics::ReferenceRun};
use thrum_test_support::{CountingAllocator, allocations::count};

use crate::support::{audio, config, mono, place};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn pulls_over_reference_and_an_interruption_do_not_allocate() {
    let mut script = ReferenceRun::default();
    let config = Config {
        one_shots: 1,
        ..config()
    };
    let (mut handle, mut mix) = audio(config);
    let sound = mono(&mut handle, vec![4096; 1024]);
    let looped = handle
        .start_emitter(sound, VoiceSettings::default())
        .expect("valid emitter");
    handle.update();
    let mut quiet = Play::new(sound, 1);
    quiet.settings.volume = 0.125;
    handle.play(quiet);
    let mut output = [0.0; 256];
    let ((), reference) = count(|| {
        while script.remaining() != 0 {
            let _ = script.pull(&mut output);
        }
    });
    let ((), first_pull) = count(|| mix.pull(&mut output));
    handle.play(Play::new(sound, 2));
    let ((), displaced) = count(|| mix.pull(&mut output));
    handle.play(Play::new(sound, 3));
    let ((), suspended) = count(|| {
        mix.stop_pulling();
        mix.pull(&mut output);
    });
    handle.hold_emitter(looped, true);
    handle.update();
    let ((), resumed) = count(|| {
        mix.start_pulling();
        mix.pull(&mut output);
    });
    handle.end_emitter(looped);
    handle.update();
    let ((), end) = count(|| mix.pull(&mut output));

    for allocations in [reference, first_pull, displaced, suspended, resumed, end] {
        assert_eq!(allocations.count, 0);
        assert_eq!(allocations.bytes, 0);
    }
    assert_eq!(handle.record().displaced, 1);
    assert_eq!(handle.record().interrupted_voices, 1);
}

#[test]
fn handle_frame_calls_do_not_allocate_after_scene_setup() {
    let (mut handle, mut mix) = audio(config());
    let sound = mono(&mut handle, vec![4096; 16]);
    let emitters: Vec<_> = (0..64)
        .map(|_| {
            handle
                .start_emitter(sound, VoiceSettings::default())
                .expect("valid emitter")
        })
        .collect();
    let mut frames = [0.0; 64];
    for _ in 0..4 {
        let ((), frame) = count(|| {
            for emitter in &emitters {
                handle.set_emitter(
                    *emitter,
                    VoiceSettings {
                        volume: 0.25,
                        pitch: 1.07,
                        place: None,
                    },
                );
                handle.move_emitter(*emitter, Some(place(1.0, 1.0, 8.0)));
                handle.hold_emitter(*emitter, true);
                handle.hold_emitter(*emitter, false);
            }
            handle.set_listener(Listener {
                position: [1.0, 0.0, 0.0],
                ..Listener::default()
            });
            handle.set_volumes(Volumes {
                master: 0.75,
                music: 0.5,
                effects: 0.25,
            });
            handle.update();
            handle.play(Play::new(sound, 123));
            handle.read_outcomes(|item| {
                std::hint::black_box(item);
            });
            std::hint::black_box(handle.record());
        });
        assert_eq!(frame.count, 0);
        mix.pull(&mut frames);
    }
}
