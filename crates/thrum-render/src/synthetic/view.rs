//! Checked view input for the synthetic scene. The game/V2 side creates this
//! packet; V1 only packs and consumes it. Matrices stay column-major.

use glam::{Mat3, Mat4, Vec3, Vec4};

use super::{
    DisplayOverlay, RenderError, SdrToneMap, checked_display_gain, exposure::ExposurePolicy,
};

const VIEW_BYTES: usize = 160;
pub(super) const AUX_BYTES: usize = 128;

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
    projection: Projection,
    view_proj: Mat4,
    camera: Vec3,
    exposure: Option<ExposurePolicy>,
    sdr_tone_map: SdrToneMap,
    display_overlay: DisplayOverlay,
    history_reset: bool,
    celestial: Option<Mat3>,
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
            projection: Projection::OrthographicFixture,
            view_proj: reverse_z,
            camera: Vec3::new(0.0, 0.0, -3.0),
            exposure: None,
            sdr_tone_map: SdrToneMap::default(),
            display_overlay: DisplayOverlay::Off,
            history_reset: false,
            celestial: None,
        }
    }
}

impl SyntheticView {
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
            celestial: None,
        })
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

    pub(super) fn exposure_policy(&self, fallback: f32, atmosphere: bool) -> [u8; 32] {
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
            || direct_lux < 1_000.0
        {
            return None;
        }
        // The fixed Earth LUTs produce finite nonnegative sky lux for checked
        // atmosphere inputs. Their bright incident meter branch has no key.
        // The EV ceiling may still increase camera gain.
        let direct = 0.48 * f64::from(direct_lux) * (-f64::from(compensation_ev)).exp2();
        Some(direct.min(1.2 * f64::from(max_ev100).exp2()))
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

    /// Mark a camera cut or another invalid correspondence for this sample.
    #[must_use]
    pub fn with_history_reset(mut self, reset: bool) -> Self {
        self.history_reset = reset;
        self
    }

    /// Supply the authored orthonormal map from equatorial directions to world directions.
    /// # Errors
    /// Refuses nonfinite or nonorthonormal axes and nonperspective views.
    pub fn with_celestial_basis(mut self, columns: [[f32; 3]; 3]) -> Result<Self, RenderError> {
        let basis = Mat3::from_cols_array_2d(&columns);
        let orthogonal = basis.transpose() * basis;
        if !matches!(self.projection, Projection::Perspective { .. })
            || !orthogonal.abs_diff_eq(Mat3::IDENTITY, 1e-4)
        {
            return Err(RenderError::InvalidView);
        }
        self.celestial = Some(basis);
        Ok(self)
    }

    pub(super) fn celestial_basis(&self) -> Option<Mat3> {
        self.celestial
    }

    pub(super) fn history_reset(&self) -> bool {
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

    pub(crate) fn pack_view(&self, previous: &Self, exposure: f32) -> [u8; VIEW_BYTES] {
        let mut bytes = [0; VIEW_BYTES];
        for (offset, value) in self.view_proj.to_cols_array().into_iter().enumerate() {
            bytes[offset * 4..offset * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (offset, value) in previous.view_proj.to_cols_array().into_iter().enumerate() {
            bytes[64 + offset * 4..68 + offset * 4].copy_from_slice(&value.to_le_bytes());
        }
        for (axis, value) in self.camera.to_array().into_iter().enumerate() {
            bytes[128 + axis * 4..132 + axis * 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[144..148].copy_from_slice(&super::PRE_EXPOSURE.to_le_bytes());
        // This legacy field describes only a CPU-known fixed request. Metered
        // exposure is GPU-authoritative in the separately bound sample record.
        let inverse = match self.exposure {
            Some(ExposurePolicy::Fixed { inverse }) => inverse,
            Some(ExposurePolicy::IncidentHorizontal { .. }) => 0.0,
            None => 1.0 / exposure,
        };
        bytes[148..152].copy_from_slice(&inverse.to_le_bytes());
        bytes
    }

    pub(crate) fn pack_aux(&self, width: u32, height: u32) -> [u8; AUX_BYTES] {
        let mut bytes = [0; AUX_BYTES];
        for (offset, value) in self
            .view_proj
            .inverse()
            .to_cols_array()
            .into_iter()
            .enumerate()
        {
            bytes[offset * 4..offset * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (axis, value) in [
            width as f32,
            height as f32,
            1.0 / width as f32,
            1.0 / height as f32,
        ]
        .into_iter()
        .enumerate()
        {
            bytes[64 + axis * 4..68 + axis * 4].copy_from_slice(&value.to_le_bytes());
        }
        let (near, far, flag) = match self.projection {
            Projection::OrthographicFixture => (0.0, 1.0, 0_u32),
            Projection::Perspective { near, far, .. } => (near, far, 1_u32),
        };
        bytes[80..84].copy_from_slice(&near.to_le_bytes());
        bytes[84..88].copy_from_slice(&far.to_le_bytes());
        bytes[112..116].copy_from_slice(&flag.to_le_bytes());
        bytes
    }
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
