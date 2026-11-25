//! Target preparation and late surface rebinding; no frame recording.
use super::{
    AirShadowTarget, GraphTextures, OffscreenTarget, PreToneViews, PreparedPreToneInput,
    PreparedSynthetic, RenderError, ShadowTarget, TargetResources, temporal,
};
impl OffscreenTarget {
    pub(crate) fn bind_surface_graph(
        &mut self,
        prepared: &PreparedSynthetic,
        profile: crate::present::SurfaceProfile,
    ) -> Result<(), RenderError> {
        self.bind_surface_graph_with_inputs(&prepared.target_inputs(), profile)
    }

    pub(crate) fn bind_surface_graph_with_inputs(
        &mut self,
        prepared: &super::SyntheticTargetInputs,
        profile: crate::present::SurfaceProfile,
    ) -> Result<(), RenderError> {
        let compiled =
            prepared.compile_graph(self.width, self.height, self.capture_source, Some(profile))?;
        if self.capture_layout != compiled.capture_layout {
            return Err(RenderError::GraphPassMismatch);
        }
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
        self.display = prepared.display.prepare_input(
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
                    native.output_view()
                } else if let Some(taau) = &self.taau {
                    &taau.output
                } else {
                    &self.scene_view
                },
                motion: &self.motion_view,
                exposure: self
                    .temporal
                    .normalized
                    .as_ref()
                    .map_or(&prepared.exposure_samples, |target| &target.exposure),
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
impl super::SyntheticTargetInputs {
    pub(super) fn prepare_target_display(
        &self,
        device: &wgpu::Device,
        ids: &TargetResources,
        scene: &wgpu::TextureView,
        motion: &wgpu::TextureView,
        temporal: &temporal::TemporalTarget,
        native: Option<&crate::apple_tile::NativeBinding>,
        taau: Option<&super::taau::TaauTarget>,
    ) -> Result<PreparedPreToneInput, RenderError> {
        self.display.prepare_input(
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
                    native.output_view()
                } else if let Some(taau) = taau {
                    &taau.output
                } else {
                    scene
                },
                motion,
                exposure: temporal
                    .normalized
                    .as_ref()
                    .map_or(&self.exposure_samples, |target| &target.exposure),
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

    pub(super) fn prepare_target_volume(
        &self,
        device: &wgpu::Device,
        shadow: Option<&ShadowTarget>,
    ) -> Option<wgpu::BindGroup> {
        self.atmosphere.as_ref()?.prepare_volume_scene_group(
            device,
            self.shadow
                .as_ref()
                .zip(shadow.map(|target| &target.array_view)),
            self.cloud.as_ref(),
        )
    }
}
