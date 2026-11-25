//! Post-display paint preparation and per-frame sampling on the render owner.

use super::{CreationPhase, FrameError, FrameHost, FrameTarget, FrameWorld, SceneSource};
use crate::{capabilities::DeviceCapabilities, present::SurfacePresenter};

pub(super) async fn prepare_interface(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    config: Option<&crate::sprite::InterfaceConfig>,
) -> Result<Option<Box<crate::sprite::interface::PreparedInterface>>, FrameError> {
    let Some(config) = config else {
        return Ok(None);
    };
    let _region = capabilities
        .site()
        .ledger()
        .region(CreationPhase::Build, "interface");
    crate::sprite::interface::PreparedInterface::prepare(capabilities, queue, config)
        .await
        .map(Box::new)
        .map(Some)
        .map_err(|error| FrameError::gpu("interface preparation", error))
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn interface_inputs(&self) -> Option<crate::sprite::interface::SpriteInputs> {
        self.interface
            .as_deref()
            .map(crate::sprite::interface::PreparedInterface::inputs)
    }

    pub(super) fn update_interface(
        &mut self,
        sdr_extent: Option<[u32; 2]>,
        float_extent: Option<[u32; 2]>,
    ) -> Option<crate::sprite::InterfaceFrameStats> {
        let interface = self.interface.as_mut()?;
        let extent = float_extent
            .or(sdr_extent)
            .expect("prepared display target");
        let paint = self.source.paint_list(extent);
        Some(interface.upload(&self.queue, paint.as_ref(), sdr_extent, float_extent))
    }

    pub(super) fn sample_interface(
        &mut self,
        slot_index: usize,
    ) -> Option<crate::sprite::InterfaceFrameStats> {
        self.interface.as_ref()?;
        let float_extent = self
            .presenter
            .as_ref()
            .filter(|presenter| presenter.profile().format == wgpu::TextureFormat::Rgba16Float)
            .map(SurfacePresenter::extent);
        let sdr = float_extent.is_none()
            || self
                .submitted_capture
                .as_ref()
                .is_some_and(|tap| tap.source == crate::synthetic::CaptureSource::Sdr);
        let sdr_extent = sdr.then(|| match &self.slots[slot_index].target {
            FrameTarget::TwoD(target) => target.extent,
            FrameTarget::ThreeD(target) => {
                let texture = target.output_view().texture();
                [texture.width(), texture.height()]
            }
        });
        self.update_interface(sdr_extent, float_extent)
    }
}
