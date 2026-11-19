//! Shared decisions for a host's drawable acquisition.

use wgpu::{CurrentSurfaceTexture, SurfaceTexture};

/// What the shell should do with the result of an acquire.
#[derive(Debug)]
pub enum SurfaceAction {
    /// Use the drawable, and reconfigure after releasing it if requested.
    Draw {
        /// The acquired drawable.
        frame: SurfaceTexture,
        /// The drawable no longer matches the ideal surface configuration.
        reconfigure: bool,
    },
    /// Skip the frame and try again on a later callback.
    Skip {
        /// The window was occluded rather than waiting on a drawable.
        occluded: bool,
    },
    /// Skip this frame and retry surface configuration before acquiring again.
    Reconfigure {
        /// Acquire reported validation on an otherwise configured surface.
        validation: bool,
    },
}

/// Interpret a drawable acquisition independently of device-loss and window ownership.
pub fn surface_action(texture: CurrentSurfaceTexture) -> SurfaceAction {
    match texture {
        CurrentSurfaceTexture::Success(frame) => SurfaceAction::Draw {
            frame,
            reconfigure: false,
        },
        CurrentSurfaceTexture::Suboptimal(frame) => SurfaceAction::Draw {
            frame,
            reconfigure: true,
        },
        CurrentSurfaceTexture::Timeout => SurfaceAction::Skip { occluded: false },
        CurrentSurfaceTexture::Occluded => SurfaceAction::Skip { occluded: true },
        CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
            SurfaceAction::Reconfigure { validation: false }
        }
        CurrentSurfaceTexture::Validation => SurfaceAction::Reconfigure { validation: true },
    }
}
