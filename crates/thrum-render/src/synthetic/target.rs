//! Target preparation and late surface rebinding; no frame recording.
use super::{
    AirShadowTarget, GraphTextures, OffscreenTarget, PreToneViews, PreparedPreToneInput,
    PreparedSynthetic, RenderError, ShadowTarget, TargetResources, native_witness, temporal,
};
impl OffscreenTarget {
    pub(crate) fn bind_surface_graph(
        &mut self,
        prepared: &PreparedSynthetic,
        profile: crate::present::SurfaceProfile,
    ) -> Result<(), RenderError> {
        let compiled = prepared.compile_graph(
            self.width,
            self.height,
            self.row_bytes,
            self.capture_source,
            Some(profile),
        )?;
        self.textures
            .rebind(
                self.night_graphs
                    .as_ref()
                    .unwrap_or(&self.day_graphs)
                    .readback
                    .plan(),
                &compiled.night.as_ref().unwrap_or(&compiled.day).readback,
            )
            .map_err(RenderError::Graph)?;
        self.display = prepared.sdr_tone_stage().prepare_input(
            &self.device,
            compiled.ids.pre_tone.clone(),
            PreToneViews {
                colour: if compiled.ids.temporal.witness.as_ref()
                    == Some(&compiled.ids.pre_tone.colour)
                {
                    &self
                        .temporal
                        .witness
                        .as_ref()
                        .ok_or(RenderError::GraphPassMismatch)?
                        .0
                } else if let Some(native) = &self.native {
                    &native.provider.view
                } else {
                    &self.scene_view
                },
                motion: &self.motion_view,
                exposure: &prepared.exposure.samples,
            },
        )?;
        self.ids = compiled.ids;
        self.day_graphs = compiled.day.bind(prepared)?;
        self.night_graphs = compiled
            .night
            .map(|graphs| graphs.bind(prepared))
            .transpose()?;
        Ok(())
    }
}
impl PreparedSynthetic {
    pub(super) fn prepare_target_display(
        &self,
        device: &wgpu::Device,
        ids: &TargetResources,
        scene: &wgpu::TextureView,
        motion: &wgpu::TextureView,
        temporal: &temporal::TemporalTarget,
        native: Option<&native_witness::Binding>,
    ) -> Result<PreparedPreToneInput, RenderError> {
        self.sdr_tone_stage().prepare_input(
            device,
            ids.pre_tone.clone(),
            PreToneViews {
                colour: if ids.temporal.witness.as_ref() == Some(&ids.pre_tone.colour) {
                    &temporal
                        .witness
                        .as_ref()
                        .ok_or(RenderError::GraphPassMismatch)?
                        .0
                } else if let Some(native) = native {
                    &native.provider.view
                } else {
                    scene
                },
                motion,
                exposure: &self.exposure.samples,
            },
        )
    }

    pub(super) fn prepare_target_shadows(
        &self,
        device: &wgpu::Device,
        textures: &GraphTextures,
        ids: &TargetResources,
    ) -> Result<(Option<ShadowTarget>, Option<AirShadowTarget>), RenderError> {
        let shadow = match (&self.shadow, &ids.shadow) {
            (Some(_prepared_shadow), Some(id)) => Some(ShadowTarget::new(
                textures.texture(id).ok_or(RenderError::GraphPassMismatch)?,
            )),
            (None, None) => None,
            _ => return Err(RenderError::GraphPassMismatch),
        };
        let air_shadow = match (&self.air_shadow, &ids.air_shadow) {
            (Some(air), Some(id)) => Some(air.target(
                device,
                textures.texture(id).ok_or(RenderError::GraphPassMismatch)?,
                self.shadow.as_ref().zip(shadow.as_ref()),
                &self.directional_buffer,
            )?),
            (None, None) => None,
            _ => return Err(RenderError::GraphPassMismatch),
        };

        Ok((shadow, air_shadow))
    }
}
