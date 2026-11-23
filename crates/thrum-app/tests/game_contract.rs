//! The same game meets the window and offscreen adapter requirements.

#[test]
fn dom_physical_keys_keep_a_platform_neutral_identity() {
    use thrum_app::KeyCode;

    assert_eq!(KeyCode::from_code("KeyW"), Some(KeyCode::KeyW));
    assert_eq!(KeyCode::from_code("MetaLeft"), Some(KeyCode::SuperLeft));
    assert_eq!(KeyCode::from_code("Unidentified"), None);
}

#[cfg(target_os = "macos")]
#[test]
fn game_receives_portable_keys_frame_edits_and_offscreen_readback() {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicU16, Ordering},
        },
        time::Duration,
    };

    use thrum_app::{
        Game, Gpu, KeyCode, KeyResponse, Prepared, WindowGame,
        offscreen::Offscreen,
        sky::{Camera, Controls, SkyEdits, SkyGame, SkyScene, StillWorld},
    };
    use thrum_render::{capabilities::DeviceProfile, runtime::FrameHost};

    type Host = FrameHost<StillWorld, SkyScene>;

    struct GameProbe {
        sky: SkyGame,
        observed: Vec<Option<u64>>,
        pressed: bool,
    }

    impl Game for GameProbe {
        type World = StillWorld;
        type Scene = SkyScene;
        type Edits = SkyEdits;
        type Readback = Option<u64>;

        async fn prepare<'a>(
            &'a mut self,
            gpu: Gpu<'a>,
            wake: Box<dyn Fn() + Send>,
        ) -> Result<Prepared<StillWorld, SkyScene>, String> {
            self.sky.prepare(gpu, wake).await
        }

        fn controls(&self) -> &Controls {
            self.sky.controls()
        }

        fn controls_mut(&mut self) -> &mut Controls {
            self.sky.controls_mut()
        }

        fn take_edits(&mut self) -> (SkyEdits, Option<bool>) {
            self.sky.take_edits()
        }

        fn frame_edits(&mut self) -> SkyEdits {
            self.sky.frame_edits()
        }

        fn replay(&self) -> SkyEdits {
            self.sky.replay()
        }

        fn apply(host: &mut Host, edits: SkyEdits) -> Result<(), String> {
            SkyGame::apply(host, edits)
        }

        fn readback(host: &Host) -> Option<u64> {
            host.last_submission()
        }

        fn observe(&mut self, submission: Option<u64>) {
            self.observed.push(submission);
        }

        fn key(&mut self, code: KeyCode, pressed: bool) -> Result<KeyResponse, String> {
            if code == KeyCode::Space {
                self.pressed = pressed;
                Ok(KeyResponse::Input)
            } else {
                Ok(KeyResponse::Ignored)
            }
        }
    }

    fn offscreen<G: WindowGame>(game: G, gpu: &thrum_test_support::Opened) -> Offscreen<G> {
        Offscreen::new(game, &gpu.device, &gpu.capabilities, &gpu.queue, [64, 64]).unwrap()
    }

    let Some(gpu) = thrum_test_support::open(
        "game_receives_portable_keys_frame_edits_and_offscreen_readback",
        Some(wgpu::Backend::Metal),
        DeviceProfile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };

    let camera = Camera::look_at([0.0, 2.0, 5.0], [0.0, 0.0, 0.0], 55.0);
    let updates = Arc::new(AtomicU16::new(0));
    let count = Arc::clone(&updates);
    let sky = SkyGame::new(Controls::new(), camera, move |_, sky| {
        sky.sun_elevation_degrees = 5.0 + f32::from(count.fetch_add(1, Ordering::Relaxed));
    });
    let mut game = GameProbe {
        sky,
        observed: Vec::new(),
        pressed: false,
    };
    assert_ne!(
        std::any::TypeId::of::<KeyCode>(),
        std::any::TypeId::of::<winit::keyboard::KeyCode>()
    );
    let space = KeyCode::from_code("Space").unwrap();
    assert_eq!(game.key(space, true).unwrap(), KeyResponse::Input);
    assert!(game.pressed);
    assert_eq!(game.key(space, false).unwrap(), KeyResponse::Input);
    assert!(!game.pressed);

    let mut run = offscreen(game, &gpu);
    let prepared = updates.load(Ordering::Relaxed);
    assert!(run.step_at(Duration::ZERO).unwrap().submitted);
    assert!(run.step_at(Duration::from_millis(16)).unwrap().submitted);
    assert_eq!(updates.load(Ordering::Relaxed), prepared + 2);
    let [Some(first), Some(second)] = run.game().observed.as_slice() else {
        panic!("expected both frame replies, got {:?}", run.game().observed);
    };
    assert!(first < second);
    assert_eq!(Some(*second), run.last_submission());
}
