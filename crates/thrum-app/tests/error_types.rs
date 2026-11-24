//! The error types exported from each of the application crate's target surfaces.

use std::error::Error;

use thrum_app::{
    GameError,
    sky::{CookedSetupError, ReloadError, SkyBuildError, WorldClockError},
};
use thrum_render::runtime::FrameError;

fn assert_error<E: Error>() {}

#[test]
fn exported_errors_support_standard_reporting() {
    assert_error::<GameError>();
    assert_error::<CookedSetupError>();
    assert_error::<ReloadError>();
    assert_error::<SkyBuildError>();
    assert_error::<WorldClockError>();
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    assert_error::<thrum_app::capture_writer::WriterError>();
    #[cfg(target_os = "macos")]
    {
        assert_error::<thrum_app::offscreen::OffscreenError>();
        assert_error::<thrum_app::offscreen::clock::OpportunityRateError>();
        assert_error::<thrum_app::sky::HeadlessDeviceError>();
    }
    #[cfg(target_arch = "wasm32")]
    assert_error::<thrum_app::web::WebHostError>();
}

#[test]
fn callers_can_distinguish_device_loss_from_a_bad_scene_file() {
    let lost = GameError::from(FrameError::DeviceLost("device retired".into()));
    assert!(matches!(lost, GameError::Frame(FrameError::DeviceLost(_))));
    assert!(lost.source().unwrap().is::<FrameError>());
    assert!(lost.to_string().contains("device retired"));

    let bad_file = thrum_app::sky::Controls::new().apply_file("{").unwrap_err();
    let bad_scene = SkyBuildError::from(bad_file);
    assert!(matches!(bad_scene, SkyBuildError::Control(_)));
    assert!(
        bad_scene
            .source()
            .unwrap()
            .is::<thrum_controls::ControlError>()
    );
    assert!(bad_scene.to_string().contains("parameter file"));
}

#[cfg(target_os = "macos")]
#[test]
fn offscreen_preserves_device_loss_and_scene_refusal() {
    use thrum_app::offscreen::OffscreenError;

    fn classify(error: &OffscreenError) -> &'static str {
        match error {
            OffscreenError::Frame(FrameError::DeviceLost(_))
            | OffscreenError::Game(GameError::Frame(FrameError::DeviceLost(_))) => "recover device",
            OffscreenError::Game(GameError::InvalidScene(_)) | OffscreenError::Parameters(_) => {
                "fix scene input"
            }
            _ => "report other failure",
        }
    }

    let device = OffscreenError::Frame(FrameError::DeviceLost("device retired".into()));
    let parameters = thrum_app::sky::Controls::new().apply_file("{").unwrap_err();
    let scene = OffscreenError::Parameters(parameters);
    assert_eq!(classify(&device), "recover device");
    assert_eq!(classify(&scene), "fix scene input");
    assert!(device.source().unwrap().is::<FrameError>());
    assert!(scene.source().unwrap().is::<thrum_controls::ControlError>());
}
