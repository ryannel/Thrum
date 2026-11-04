//! Checked view input for the synthetic scene. The game/V2 side creates this
//! packet; V1 only packs and consumes it. Matrices stay column-major.

use glam::{Mat4, Vec3, Vec4};

use super::RenderError;

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

/// One checked, unjittered camera sample. A future V2 view packet owns its
/// identity/history; this bounded fixture keeps only the spatial fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyntheticView {
    projection: Projection,
    view_proj: Mat4,
    camera: Vec3,
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
        })
    }

    pub(super) fn projection(&self) -> Projection {
        self.projection
    }

    pub(super) fn pack_view(&self, exposure: f32) -> [u8; VIEW_BYTES] {
        let mut bytes = [0; VIEW_BYTES];
        for (offset, value) in self.view_proj.to_cols_array().into_iter().enumerate() {
            bytes[offset * 4..offset * 4 + 4].copy_from_slice(&value.to_le_bytes());
            bytes[64 + offset * 4..68 + offset * 4].copy_from_slice(&value.to_le_bytes());
        }
        for (axis, value) in self.camera.to_array().into_iter().enumerate() {
            bytes[128 + axis * 4..132 + axis * 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[144..148].copy_from_slice(&super::PRE_EXPOSURE.to_le_bytes());
        bytes[148..152].copy_from_slice(&(1.0 / exposure).to_le_bytes());
        bytes
    }

    pub(super) fn pack_aux(&self, width: u32, height: u32) -> [u8; AUX_BYTES] {
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
}
