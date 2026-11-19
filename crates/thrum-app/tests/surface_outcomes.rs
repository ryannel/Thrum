//! Regression checks for shared no-drawable decisions.

use thrum_app::surface::{SurfaceAction, surface_action};
use wgpu::CurrentSurfaceTexture;

#[test]
fn surface_loss_outdated_and_validation_all_retry_configuration() {
    for outcome in [CurrentSurfaceTexture::Lost, CurrentSurfaceTexture::Outdated] {
        assert!(matches!(
            surface_action(outcome),
            SurfaceAction::Reconfigure { validation: false }
        ));
    }
    assert!(matches!(
        surface_action(CurrentSurfaceTexture::Validation),
        SurfaceAction::Reconfigure { validation: true }
    ));
}

#[test]
fn timeout_and_occlusion_skip_without_reconfiguring() {
    assert!(matches!(
        surface_action(CurrentSurfaceTexture::Timeout),
        SurfaceAction::Skip { occluded: false }
    ));
    assert!(matches!(
        surface_action(CurrentSurfaceTexture::Occluded),
        SurfaceAction::Skip { occluded: true }
    ));
}
