//! The same kit game type meets the window and offscreen adapter requirements.

#[cfg(target_os = "macos")]
#[test]
fn sky_game_is_hosted_offscreen_and_accepts_window_input() {
    use std::time::Duration;

    use thrum_app::{
        KeyCode, KeyResponse, WindowGame,
        offscreen::Offscreen,
        sky::{Camera, Controls, SkyGame},
    };
    use thrum_render::capabilities::DeviceProfile;

    fn offscreen(mut game: impl WindowGame, gpu: &thrum_test_support::Opened) {
        assert_eq!(
            game.key(KeyCode::Space, true).unwrap(),
            KeyResponse::Ignored
        );
        let mut run =
            Offscreen::new(game, &gpu.device, &gpu.capabilities, &gpu.queue, [64, 64]).unwrap();
        assert!(run.step_at(Duration::ZERO).unwrap().submitted);
    }

    let Some(gpu) = thrum_test_support::open(
        "sky_game_is_hosted_offscreen_and_accepts_window_input",
        Some(wgpu::Backend::Metal),
        DeviceProfile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };

    let camera = Camera::look_at([0.0, 2.0, 5.0], [0.0, 0.0, 0.0], 55.0);
    offscreen(SkyGame::new(Controls::new(), camera, |_, _| {}), &gpu);
}
