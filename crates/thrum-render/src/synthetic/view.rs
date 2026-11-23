//! Checked view input for the synthetic scene. The game/V2 side creates this
//! packet; V1 only packs and consumes it. Matrices stay column-major.

use crate::records::{ViewAuxRecord, ViewRecord};
use glam::{Mat3, Mat4, Vec3, Vec4};

use super::{
    DisplayOverlay, RenderError, SdrToneMap, checked_display_gain, exposure::ExposurePolicy,
};

pub(super) const VIEW_BYTES: usize = size_of::<ViewRecord>();
pub(super) const AUX_BYTES: usize = size_of::<ViewAuxRecord>();

/// Reconstruction selection fixed at generation preparation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Reconstruction {
    /// Spatial output with no temporal history consumer.
    #[default]
    Off,
    /// Registry-selected portable temporal resolve.
    Taau,
    /// Independent 32-offset box reference for held-state qualification.
    Reference,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Projection {
    OrthographicFixture,
    Perspective {
        view: Mat4,
        fx: f32,
        fy: f32,
        near: f32,
        far: f32,
    },
}

/// One checked, unjittered camera sample with an explicit cut flag. A future
/// V2 view packet owns the broader identity and history contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyntheticView {
    native_witness: super::NativeWitness,
    pub(super) projection: Projection,
    view_proj: Mat4,
    camera: Vec3,
    exposure: Option<ExposurePolicy>,
    sdr_tone_map: SdrToneMap,
    display_overlay: DisplayOverlay,
    history_reset: bool,
    history_epoch: u64,
    celestial: Option<Mat3>,
    celestial_epoch: u64,
    jitter_sample: Option<(u32, [u32; 2])>,
    raster_jitter: bool,
    temporal_inputs: bool,
    reconstruction: Reconstruction,
    output_extent: Option<[u32; 2]>,
    render_scale: super::RenderScale,
    sun_shadow_profile: Option<(u32, f32)>,
}

impl Default for SyntheticView {
    fn default() -> Self {
        // The historical orthographic GPU cards use clip-space coordinates
        // directly and a receiver-to-camera ray of -Z. Reverse depth without
        // changing their XY positions or near/far ordering.
        let reverse_z = Mat4::from_cols(
            Vec4::X,
            Vec4::Y,
            Vec4::new(0.0, 0.0, -1.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
        );
        Self {
            native_witness: super::NativeWitness::Off,
            projection: Projection::OrthographicFixture,
            view_proj: reverse_z,
            camera: Vec3::new(0.0, 0.0, -3.0),
            exposure: None,
            sdr_tone_map: SdrToneMap::default(),
            display_overlay: DisplayOverlay::Off,
            history_reset: false,
            history_epoch: 0,
            celestial: None,
            celestial_epoch: 0,
            jitter_sample: None,
            raster_jitter: false,
            temporal_inputs: false,
            reconstruction: Reconstruction::Off,
            output_extent: None,
            render_scale: super::RenderScale::Native,
            sun_shadow_profile: None,
        }
    }
}

impl SyntheticView {
    pub(crate) fn is_perspective(&self) -> bool {
        matches!(self.projection, Projection::Perspective { .. })
    }

    /// Select a labelled stateful copy/select diagnostic before preparation.
    /// Requires the FrameHost coordinator; ordinary scene baselines leave it off.
    #[must_use]
    pub fn with_native_witness(mut self, selection: super::NativeWitness) -> Self {
        self.native_witness = selection;
        self
    }

    pub(super) fn native_witness(&self) -> super::NativeWitness {
        self.native_witness
    }
    /// Build a finite right-handed perspective view with WebGPU [0,1]
    /// reverse depth. Near maps to one and far to zero.
    /// # Errors
    /// Refuses nonfinite/degenerate camera, FOV, aspect or depth interval.
    pub fn perspective(
        camera: [f32; 3],
        target: [f32; 3],
        up: [f32; 3],
        fov_y_radians: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Result<Self, RenderError> {
        if !camera
            .iter()
            .chain(target.iter())
            .chain(up.iter())
            .all(|v| v.is_finite())
            || !fov_y_radians.is_finite()
            || !(0.0..std::f32::consts::PI).contains(&fov_y_radians)
            || !aspect.is_finite()
            || aspect <= 0.0
            || !near.is_finite()
            || near <= 0.0
            || !far.is_finite()
            || far <= near
            || !(far / near).is_finite()
            || !(near * far).is_finite()
        {
            return Err(RenderError::InvalidView);
        }
        let eye = Vec3::from_array(camera);
        let forward = Vec3::from_array(target) - eye;
        let up = Vec3::from_array(up);
        if forward.length_squared() <= 1e-12
            || up.length_squared() <= 1e-12
            || forward.normalize().cross(up.normalize()).length_squared() <= 1e-12
        {
            return Err(RenderError::InvalidView);
        }
        let view =
            glam::camera::rh::view::look_at_mat4(eye, Vec3::from_array(target), up.normalize());
        let standard =
            glam::camera::rh::proj::directx::perspective(fov_y_radians, aspect, near, far);
        let reverse_z = Mat4::from_cols(
            Vec4::X,
            Vec4::Y,
            Vec4::new(0.0, 0.0, -1.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
        );
        let view_proj = reverse_z * standard * view;
        let cols = view_proj.to_cols_array();
        if !cols.iter().all(|v| v.is_finite())
            || !view_proj
                .inverse()
                .to_cols_array()
                .iter()
                .all(|v| v.is_finite())
        {
            return Err(RenderError::InvalidView);
        }
        let fy = 1.0 / (fov_y_radians * 0.5).tan();
        let fx = fy / aspect;
        Ok(Self {
            native_witness: super::NativeWitness::Off,
            projection: Projection::Perspective {
                view,
                fx,
                fy,
                near,
                far,
            },
            view_proj,
            camera: eye,
            exposure: None,
            sdr_tone_map: SdrToneMap::default(),
            display_overlay: DisplayOverlay::Off,
            history_reset: false,
            history_epoch: 0,
            celestial: None,
            celestial_epoch: 0,
            jitter_sample: None,
            raster_jitter: false,
            temporal_inputs: false,
            reconstruction: Reconstruction::Off,
            output_extent: None,
            render_scale: super::RenderScale::Native,
            sun_shadow_profile: None,
        })
    }

    /// Prepare the ordinary scene with a secondary view's Sun profile.
    /// Zero cascades uses only the scene's distant height map; one uses a
    /// single full-reach cascade, and two use the ordinary near/far fit.
    /// The default remains two cascades to the original 512 m reach.
    /// # Errors
    /// Refuses more than two cascades or a nonpositive, nonfinite reach.
    pub fn with_sun_shadow_profile(mut self, count: u32, reach: f32) -> Result<Self, RenderError> {
        if count > 2 || !reach.is_finite() || reach <= 0.0 {
            return Err(RenderError::InvalidView);
        }
        self.sun_shadow_profile = Some((count, reach));
        Ok(self)
    }

    pub(super) fn sun_shadow_profile(&self) -> Option<(u32, f32)> {
        self.sun_shadow_profile
    }

    /// Select one checked scene-to-display exposure for this view. The
    /// prepared exposure remains the fallback when no override is selected.
    /// # Errors
    /// Refuses a nonpositive, nonfinite or unrepresentable exposure.
    pub fn with_exposure(mut self, exposure: f32) -> Result<Self, RenderError> {
        checked_display_gain(exposure)?;
        self.exposure = Some(ExposurePolicy::Fixed {
            inverse: 1.0 / exposure,
        });
        Ok(self)
    }

    /// Select a flat horizontal incident-light camera meter for physical atmosphere.
    /// Positive compensation brightens the camera; bounds are EV at ISO 100.
    /// Other lighting profiles use the prepared fixed fallback, flagged unmetered.
    /// # Errors
    /// Refuses nonfinite settings, reversed bounds or EV settings outside -24..=24.
    pub fn with_incident_exposure(
        mut self,
        compensation_ev: f32,
        min_ev100: f32,
        max_ev100: f32,
    ) -> Result<Self, RenderError> {
        self.exposure = Some(ExposurePolicy::incident(
            compensation_ev,
            min_ev100,
            max_ev100,
        )?);
        Ok(self)
    }

    /// Darken the incident meter's target below daylight, following the
    /// Krawczyk et al. (2005) key-value curve, so twilight reads dim rather
    /// than metered back to mid-grey. A smooth shoulder moderates additional
    /// gain at deep twilight and keeps the dim HDR storage scale. Daylight
    /// above 1,000 lux is unchanged.
    /// # Errors
    /// Refuses a view without an incident meter.
    pub fn with_low_light_adaptation(mut self) -> Result<Self, RenderError> {
        match &mut self.exposure {
            Some(ExposurePolicy::IncidentHorizontal { low_light_key, .. }) => {
                *low_light_key = true;
                Ok(self)
            }
            _ => Err(RenderError::InvalidExposure),
        }
    }

    pub(super) fn exposure_policy(
        &self,
        fallback: f32,
        atmosphere: bool,
    ) -> [u8; size_of::<crate::records::Policy>()] {
        self.exposure
            .unwrap_or(ExposurePolicy::Fixed {
                inverse: 1.0 / fallback,
            })
            .pack(
                1.0 / fallback,
                atmosphere,
                self.sdr_tone_map,
                self.display_overlay,
            )
    }

    pub(super) fn star_camera_inverse_floor(&self, direct_lux: f32) -> Option<f64> {
        use crate::shader_constants::{
            ADAPTED_LUX, CAMERA_EXPOSURE_NORMALIZATION, INCIDENT_METER_LUX_SCALE,
        };

        let Some(ExposurePolicy::IncidentHorizontal {
            compensation_ev,
            max_ev100,
            ..
        }) = self.exposure
        else {
            return None;
        };
        if self.sdr_tone_map != SdrToneMap::NeutralHighlights
            || self.display_overlay != DisplayOverlay::Off
            || direct_lux < ADAPTED_LUX
        {
            return None;
        }
        // The fixed Earth LUTs produce finite nonnegative sky lux for checked
        // atmosphere inputs. Their bright incident meter branch has no key.
        // The EV ceiling may still increase camera gain.
        let direct = (CAMERA_EXPOSURE_NORMALIZATION * INCIDENT_METER_LUX_SCALE)
            * f64::from(direct_lux)
            * (-f64::from(compensation_ev)).exp2();
        Some(direct.min(CAMERA_EXPOSURE_NORMALIZATION * f64::from(max_ev100).exp2()))
    }

    /// Select the SDR display transform independently of the camera exposure.
    #[must_use]
    pub fn with_sdr_tone_map(mut self, tone_map: SdrToneMap) -> Self {
        self.sdr_tone_map = tone_map;
        self
    }

    /// Draw a display diagnostic in place of the tone-mapped scene. The overlay reads
    /// exposed luminance before the tone scale, so it does not depend on the tone map.
    #[must_use]
    pub fn with_display_overlay(mut self, overlay: DisplayOverlay) -> Self {
        self.display_overlay = overlay;
        self
    }

    pub(super) fn traced_geometry(self) -> bool {
        self.display_overlay == DisplayOverlay::TracedGeometry
    }

    /// Mark a camera cut or another invalid correspondence for this sample.
    #[must_use]
    pub fn with_history_reset(mut self, reset: bool) -> Self {
        self.history_reset = reset;
        self
    }

    /// Set the camera continuity identity. Keep it stable through continuous
    /// motion and change it at each discontinuity. The frame host compares it
    /// with the last submitted scene even if samples were skipped or discarded.
    #[must_use]
    pub fn with_history_epoch(mut self, epoch: u64) -> Self {
        self.history_epoch = epoch;
        self
    }

    /// Prepare stored, sampleable reconstruction inputs and their GPU diagnostic.
    /// This selection is fixed when the scene generation is prepared.
    #[must_use]
    pub fn with_temporal_inputs(mut self, enabled: bool) -> Self {
        self.temporal_inputs = enabled;
        self
    }

    /// Select the registry's portable temporal reconstructor at scene preparation.
    #[must_use]
    pub fn with_reconstruction(mut self, enabled: bool) -> Self {
        self.reconstruction = if enabled {
            Reconstruction::Taau
        } else {
            Reconstruction::Off
        };
        self
    }

    pub(super) fn reconstruction(&self) -> bool {
        self.reconstruction != Reconstruction::Off
    }

    /// Select the independent, held-state 32-sample linear reference accumulator.
    #[must_use]
    pub fn with_reconstruction_reference(mut self, enabled: bool) -> Self {
        if enabled {
            self = self.with_reconstruction(true);
            self.reconstruction = Reconstruction::Reference;
        } else if self.reconstruction == Reconstruction::Reference {
            self.reconstruction = Reconstruction::Taau;
        }
        self
    }

    pub(super) fn reconstruction_reference(&self) -> bool {
        self.reconstruction == Reconstruction::Reference
    }

    pub(super) fn temporal_diagnostic(&self) -> bool {
        self.display_overlay == DisplayOverlay::TemporalInputs
    }

    pub(super) fn temporal_inputs(&self) -> bool {
        self.temporal_inputs || self.reconstruction()
    }

    /// Set the reconstruction output grid independently of render scale.
    #[must_use]
    pub fn with_output_extent(mut self, extent: [u32; 2]) -> Self {
        self.output_extent = Some(extent.map(|n| n.max(1)));
        self
    }

    pub(super) fn output_extent(&self, render: [u32; 2]) -> [u32; 2] {
        self.output_extent.unwrap_or(render)
    }

    pub(crate) fn history_epoch(&self) -> u64 {
        self.history_epoch
    }

    /// Supply the authored proper rotation from equatorial directions to world directions.
    /// # Errors
    /// Refuses reflections, nonfinite or nonorthonormal axes and nonperspective views.
    pub fn with_celestial_basis(mut self, columns: [[f32; 3]; 3]) -> Result<Self, RenderError> {
        let basis = Mat3::from_cols_array_2d(&columns);
        let orthogonal = basis.transpose() * basis;
        if !matches!(self.projection, Projection::Perspective { .. })
            || !orthogonal.abs_diff_eq(Mat3::IDENTITY, 1e-4)
            || basis.determinant() <= 0.0
        {
            return Err(RenderError::InvalidView);
        }
        self.celestial = Some(basis);
        Ok(self)
    }

    /// Identify a discontinuous celestial phase edit independently of camera motion.
    #[must_use]
    pub fn with_celestial_epoch(mut self, epoch: u64) -> Self {
        self.celestial_epoch = epoch;
        self
    }

    pub(super) fn celestial_epoch(&self) -> u64 {
        self.celestial_epoch
    }

    pub(super) fn celestial_basis(&self) -> Option<Mat3> {
        self.celestial
    }

    pub(crate) fn history_reset(&self) -> bool {
        self.history_reset
    }

    pub(super) fn projection(&self) -> Projection {
        self.projection
    }

    /// Column-major view-projection (reverse-Z, WebGPU clip space).
    pub fn view_proj_columns(&self) -> [[f32; 4]; 4] {
        self.view_proj.to_cols_array_2d()
    }

    /// World-space camera position.
    pub fn camera_position(&self) -> [f32; 3] {
        self.camera.to_array()
    }

    /// Select jittered rasterization for a temporal consumer or explicit jitter witness.
    /// The host advances the 32-phase sequence only after submission.
    #[must_use]
    pub fn with_jitter_phase(self, phase: u32, extent: [u32; 2]) -> Self {
        let mut view = self.at_jitter_phase(phase, extent);
        view.raster_jitter = true;
        view
    }

    /// Retain temporal sample metadata while presenting spatial, unjittered output.
    /// A reconstructor selects [`Self::with_jitter_phase`] when it consumes the sample.
    #[must_use]
    pub fn with_jitter_sequence(self, phase: u32, extent: [u32; 2]) -> Self {
        let mut view = self.at_jitter_phase(phase, extent);
        view.raster_jitter = false;
        view
    }

    /// Select the next drawn subrectangle inside the prepared output grid.
    #[must_use]
    pub fn with_render_scale(mut self, scale: super::RenderScale) -> Self {
        self.render_scale = scale;
        self
    }

    pub(super) fn render_scale(&self) -> super::RenderScale {
        self.render_scale
    }

    pub(super) fn at_jitter_phase(mut self, phase: u32, extent: [u32; 2]) -> Self {
        self.jitter_sample = Some((phase % 32, extent.map(|axis| axis.max(1))));
        self
    }

    pub(crate) fn without_jitter(mut self) -> Self {
        self.jitter_sample = None;
        self
    }

    pub(super) fn jitter_extent(&self) -> Option<[u32; 2]> {
        self.jitter_sample.map(|(_, extent)| extent)
    }

    pub(super) fn jitter_enabled(&self) -> bool {
        self.jitter_sample.is_some()
    }

    pub(super) fn raster_jitter_enabled(&self) -> bool {
        (self.raster_jitter || self.reconstruction()) && self.jitter_enabled()
    }

    pub(super) fn raster_phase(&self) -> u32 {
        self.jitter_sample.map_or(0, |(phase, _)| phase)
    }

    pub(super) fn raster_projection(&self) -> Mat4 {
        if self.raster_jitter_enabled() {
            self.jittered_projection()
        } else {
            self.view_proj
        }
    }

    pub(super) fn jittered_projection(&self) -> Mat4 {
        let Some((phase, extent)) = self.jitter_sample else {
            return self.view_proj;
        };
        let jitter = halton_jitter(phase);
        let shift = Mat4::from_cols(
            Vec4::X,
            Vec4::Y,
            Vec4::Z,
            Vec4::new(
                2.0 * jitter[0] / extent[0] as f32,
                -2.0 * jitter[1] / extent[1] as f32,
                0.0,
                1.0,
            ),
        );
        shift * self.view_proj
    }

    pub(crate) fn pack_view(&self, previous: &Self, exposure: f32) -> [u8; VIEW_BYTES] {
        bytemuck::cast(self.view_record(previous, exposure))
    }

    pub(super) fn view_record(&self, previous: &Self, exposure: f32) -> ViewRecord {
        // The separately bound exposure sample is authoritative for metered exposure.
        let inverse = match self.exposure {
            Some(ExposurePolicy::Fixed { inverse }) => inverse,
            Some(ExposurePolicy::IncidentHorizontal { .. }) => 0.0,
            None => 1.0 / exposure,
        };
        let (phase, [jx, jy]) = self
            .jitter_sample
            .map_or((0, [0.0; 2]), |(phase, _)| (phase, halton_jitter(phase)));
        ViewRecord {
            view_proj: self.raster_projection().to_cols_array_2d(),
            previous_view_proj: previous.view_proj.to_cols_array_2d(),
            camera_xyz_time: self.camera.extend(0.0).to_array(),
            pre_exposure_inv_exposure_jitter: [super::PRE_EXPOSURE, inverse, jx, jy],
            unjittered_view_proj: self.view_proj.to_cols_array_2d(),
            mip_bias: -1.0,
            jitter_phase: phase,
            padding: [0; 2],
        }
    }

    pub(crate) fn inverse_view_proj_columns(&self) -> [[f32; 4]; 4] {
        self.view_proj.inverse().to_cols_array_2d()
    }

    pub(crate) fn pack_aux(&self, width: u32, height: u32) -> [u8; AUX_BYTES] {
        let (width, height) = (width as f32, height as f32);
        let (near, far, flag) = match self.projection {
            Projection::OrthographicFixture => (0.0, 1.0, 0_u32),
            Projection::Perspective { near, far, .. } => (near, far, 1_u32),
        };
        bytemuck::cast(ViewAuxRecord {
            inverse_view_proj: self.inverse_view_proj_columns(),
            viewport_inv_viewport: [width, height, 1.0 / width, 1.0 / height],
            near_far: [near, far, 0.0, 0.0],
            time: [0.0; 4],
            flags: [flag, 0, 0, 0],
            subrectangle: [0.0, 0.0, width, height],
        })
    }
}

/// Samples 1..=32: index zero would introduce a corner sample and is never used.
fn halton_jitter(phase: u32) -> [f32; 2] {
    fn radical_inverse(mut index: u32, base: u32) -> f32 {
        let mut weight = 1.0;
        let mut value = 0.0;
        while index != 0 {
            weight /= base as f32;
            value += (index % base) as f32 * weight;
            index /= base;
        }
        value - 0.5
    }
    let index = phase % 32 + 1;
    [radical_inverse(index, 2), radical_inverse(index, 3)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_reverse_depth_and_inverse_round_trip() {
        let camera = SyntheticView::perspective(
            [0.0, 0.0, 3.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            16.0 / 9.0,
            0.1,
            20.0,
        )
        .expect("valid camera");
        for (distance, depth) in [(0.1, 1.0), (20.0, 0.0)] {
            let clip = camera.view_proj * Vec4::new(0.0, 0.0, 3.0 - distance, 1.0);
            assert!((clip.z / clip.w - depth).abs() < 1e-5);
        }
        let point = Vec4::new(0.3, -0.2, 0.0, 1.0);
        let clip = camera.view_proj * point;
        let ndc = clip / clip.w;
        let restored = camera.view_proj.inverse() * ndc;
        assert!((restored.truncate() / restored.w - point.truncate()).length() < 1e-4);
    }

    #[test]
    fn degenerate_camera_refuses() {
        assert!(
            SyntheticView::perspective([0.0; 3], [0.0; 3], [0.0, 1.0, 0.0], 1.0, 1.0, 0.1, 10.0)
                .is_err()
        );
        assert!(
            SyntheticView::perspective(
                [0.0; 3],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
                1.0,
                1.0,
                0.1,
                10.0
            )
            .is_err()
        );
    }

    #[test]
    fn celestial_basis_accepts_rotation_and_refuses_reflection() {
        let view = SyntheticView::perspective(
            [0.0; 3],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            10.0,
        )
        .expect("perspective camera");
        let basis = Mat3::from_rotation_y(0.4);
        assert!(view.with_celestial_basis(basis.to_cols_array_2d()).is_ok());
        let reflected = Mat3::from_cols(basis.x_axis, basis.y_axis, -basis.z_axis);
        assert!(
            view.with_celestial_basis(reflected.to_cols_array_2d())
                .is_err()
        );
    }

    #[test]
    fn view_exposure_is_checked_and_packed_without_replacing_the_camera() {
        let view = SyntheticView::default();
        for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(view.with_exposure(invalid).is_err());
        }
        let selected = view.with_exposure(0.25).expect("finite exposure");
        assert_eq!(selected.view_proj, view.view_proj);
        let packed = selected.pack_view(&view, 1.0);
        assert_eq!(
            f32::from_le_bytes(packed[148..152].try_into().unwrap()),
            4.0
        );
        let fallback = view.pack_view(&view, 1.0);
        assert_eq!(
            f32::from_le_bytes(fallback[148..152].try_into().unwrap()),
            1.0
        );
    }

    #[test]
    fn live_view_packet_retains_the_last_submitted_projection() {
        let before = SyntheticView::perspective(
            [0.0, 0.0, 3.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .expect("first view");
        let after = SyntheticView::perspective(
            [0.3, 0.0, 3.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .expect("second view");
        let packet = after.pack_view(&before, 1.0);
        assert_ne!(&packet[..64], &packet[64..128]);
        assert_eq!(&packet[64..128], &before.pack_view(&before, 1.0)[..64]);
    }
}

#[cfg(test)]
mod temporal_tests {
    use super::*;

    #[test]
    fn spatial_output_retains_the_sequence_and_both_matrices() {
        let base = SyntheticView::default();
        for phase in 0..96 {
            let spatial = base.with_jitter_sequence(phase, [64, 32]);
            let temporal = base.with_jitter_phase(phase, [64, 32]);
            let record = spatial.pack_view(&base, 1.0);
            let jittered = temporal.pack_view(&base, 1.0);
            assert_eq!(&record[64..], &jittered[64..]);
            assert_eq!(&record[..64], &record[160..224]);
            assert_ne!(&jittered[..64], &jittered[160..224]);
            assert_ne!(&record[152..160], &[0; 8]);
            assert_eq!(&record[228..232], &(phase % 32).to_le_bytes());
            assert_eq!(spatial.raster_projection(), base.view_proj);
            assert_ne!(temporal.raster_projection(), base.view_proj);
            let next = spatial.at_jitter_phase(phase + 1, [128, 64]);
            assert!(!next.raster_jitter_enabled());
        }
    }

    #[test]
    fn continuous_halton_values_and_exact_record_layout() {
        let x = [
            0, -16, 16, -24, 8, -8, 24, -28, 4, -12, 20, -20, 12, -4, 28, -30, 2, -14, 18, -22, 10,
            -6, 26, -26, 6, -10, 22, -18, 14, -2, 30, -31,
        ];
        let y = [
            27, 54, 9, 36, 63, 18, 45, 72, 3, 30, 57, 12, 39, 66, 21, 48, 75, 6, 33, 60, 15, 42,
            69, 24, 51, 78, 1, 28, 55, 10, 37, 64,
        ];
        for phase in 0..96_u32 {
            let sample = halton_jitter(phase);
            let index = phase as usize % 32;
            assert_eq!(sample[0], x[index] as f32 / 64.0);
            assert!((sample[1] - (y[index] as f32 / 81.0 - 0.5)).abs() < 1e-7);
            assert_ne!(sample, [0.0; 2]);
        }
        let previous = SyntheticView::default();
        assert_eq!(
            previous.view_record(&previous, 1.0).mip_bias.to_bits(),
            (-1.0_f32).to_bits()
        );
        let current = previous
            .with_reconstruction(true)
            .with_jitter_phase(7, [64, 32]);
        let record = current.pack_view(&previous, 1.0);
        let mut expected = [0_u8; 240];
        let mut matrix = previous.view_proj;
        matrix.w_axis.x = -0.875 / 64.0;
        matrix.w_axis.y = -(8.0 / 9.0 - 0.5) * 2.0 / 32.0;
        for (offset, values) in [
            (0, matrix.to_cols_array()),
            (64, previous.view_proj.to_cols_array()),
            (160, previous.view_proj.to_cols_array()),
        ] {
            for (i, value) in values.into_iter().enumerate() {
                expected[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        for (offset, value) in [
            (128, 0.0_f32),
            (132, 0.0),
            (136, -3.0),
            (144, super::super::PRE_EXPOSURE),
            (148, 1.0),
            (152, -0.4375),
            (156, 8.0 / 9.0 - 0.5),
            (224, -1.0),
        ] {
            expected[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        expected[228..232].copy_from_slice(&7_u32.to_le_bytes());
        assert_eq!(record, expected);
        let source = super::super::scene_source::compose(false, false);
        let module = wgpu::naga::front::wgsl::parse_str(&source).unwrap();
        for (name, span, offsets) in [
            (
                "ViewRecord",
                240,
                &[0, 64, 128, 144, 160, 224, 228, 232][..],
            ),
            ("ViewAuxRecord", 144, &[0, 64, 80, 96, 112, 128][..]),
        ] {
            let ty = module
                .types
                .iter()
                .find(|(_, ty)| ty.name.as_deref() == Some(name))
                .unwrap()
                .1;
            let wgpu::naga::TypeInner::Struct {
                members,
                span: actual,
            } = &ty.inner
            else {
                panic!("record");
            };
            assert_eq!(*actual, span);
            assert_eq!(
                members
                    .iter()
                    .map(|member| member.offset)
                    .collect::<Vec<_>>(),
                offsets
            );
        }
        let aux = current.pack_aux(64, 32);
        assert_eq!(&aux[128..136], &[0; 8]);
        assert_eq!(&aux[136..140], &64.0_f32.to_le_bytes());
        assert_eq!(&aux[140..144], &32.0_f32.to_le_bytes());
    }
}
