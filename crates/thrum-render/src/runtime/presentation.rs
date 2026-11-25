//! Display headroom and late drawable encoding on the render owner.
use super::{
    DriveReport, FrameHost, FrameSlot, FrameTarget, FrameWorld, PreparedRenderer, SceneSource,
    SurfaceFault, SurfaceOutput,
};
use crate::{error::RenderError, present::SurfacePresenter};

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Headroom consumed by the current display mode, with 1.0 as SDR white.
    #[must_use]
    pub fn display_headroom(&self) -> f32 {
        if self.is_edr() {
            self.display_headroom
        } else {
            1.0
        }
    }

    /// Whether the admitted generation writes an extended-linear float drawable.
    #[must_use]
    pub fn is_edr(&self) -> bool {
        self.presenter
            .as_ref()
            .is_some_and(|p| p.profile().format == wgpu::TextureFormat::Rgba16Float)
    }

    /// Publish the platform's current headroom before recording its next frame.
    pub fn set_display_headroom(&mut self, headroom: f32) {
        let headroom = if headroom.is_finite() {
            headroom.clamp(1.0, 65_504.0)
        } else {
            1.0
        };
        if self.display_headroom.to_bits() != headroom.to_bits() {
            self.prepared.initialize_display(&self.queue, headroom);
            self.display_headroom = headroom;
        }
    }

    pub(super) fn install_prepared_for_display(&mut self, prepared: PreparedRenderer) {
        prepared.initialize_display(&self.queue, self.display_headroom);
        self.prepared = prepared;
    }
}

impl PreparedRenderer {
    pub(super) fn initialize_display(&self, queue: &wgpu::Queue, headroom: f32) {
        // The platform record can change while a replacement is preparing.
        // Initialize its new uniform from the render owner's latest value.
        match self {
            Self::ThreeD(scene) => scene.write_display_headroom(queue, headroom),
            Self::TwoD(scene) => scene.write_display_headroom(queue, headroom),
        }
    }
}
// Validate binding and timing before opening the drawable pass. The portable
// scene can still be submitted independently when one of those checks fails.
pub(super) fn encode_surface_pass(
    prepared: &PreparedRenderer,
    interface: Option<&crate::sprite::interface::PreparedInterface>,
    presenter: Option<&SurfacePresenter>,
    slot: &mut FrameSlot,
    surface: Option<&SurfaceOutput<'_>>,
    encoder: &mut wgpu::CommandEncoder,
    timing_requested: bool,
    readback: bool,
    report: &mut DriveReport,
) -> (bool, Option<RenderError>) {
    match (presenter, slot.presentation.as_ref(), surface) {
        (Some(presenter), Some(binding), Some(surface)) => {
            let surface = (presenter, binding, surface.view, surface.extent);
            let timings = timing_requested.then_some(&mut slot.timing.queries);
            let encoded = match (prepared, &slot.target) {
                (PreparedRenderer::ThreeD(scene), FrameTarget::ThreeD(target)) => {
                    scene.encode_surface(target, encoder, surface, timings, readback, interface)
                }
                (PreparedRenderer::TwoD(scene), FrameTarget::TwoD(target)) => {
                    scene.present(encoder, target, timings, readback, surface, interface)
                }
                _ => Err(RenderError::StaleTarget),
            };
            match encoded {
                Ok(()) => (true, None),
                Err(error) => {
                    let fault = SurfaceFault::from_encode(&error);
                    report.surface_fault = Some(fault);
                    (false, (!fault.is_transient()).then_some(error))
                }
            }
        }
        (Some(_), None, Some(_)) => {
            report.surface_fault = Some(SurfaceFault::MissingBinding);
            (false, None)
        }
        _ => (false, None),
    }
}
