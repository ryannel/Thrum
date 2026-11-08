//! Checked authored camera policy for the V1 game's real-world scene.

use thrum_render::synthetic::SyntheticView;

/// A fixed perspective camera in world metres, with sea level at Y = zero.
/// Orientation remains adjustable through the game's pitch and yaw controls.
#[derive(Clone, Copy, Debug)]
pub struct SceneCamera {
    eye: [f32; 3],
    vertical_fov_radians: f32,
    near: f32,
    far: f32,
    focus_distance: f32,
    pitch: f32,
    yaw: f32,
}

impl SceneCamera {
    /// Validate an authored perspective camera using the renderer's view checks.
    /// The viewport supplies its current aspect ratio when a frame is sampled.
    /// # Errors
    /// Refuses invalid perspective inputs or eye altitude outside 0–100 km.
    pub fn perspective(
        eye: [f32; 3],
        target: [f32; 3],
        vertical_fov_radians: f32,
        near: f32,
        far: f32,
    ) -> Result<Self, String> {
        if !(0.0..=100_000.0).contains(&eye[1]) {
            return Err("scene camera altitude must be finite and within 0–100 km".to_owned());
        }
        SyntheticView::perspective(
            eye,
            target,
            [0.0, 1.0, 0.0],
            vertical_fov_radians,
            1.0,
            near,
            far,
        )
        .map_err(|error| format!("scene camera: {error:?}"))?;
        let forward = std::array::from_fn::<_, 3, _>(|axis| target[axis] - eye[axis]);
        let horizontal = forward[0].hypot(forward[2]);
        let focus_distance = horizontal.hypot(forward[1]);
        if !focus_distance.is_finite() {
            return Err("scene camera target distance must be representable".to_owned());
        }
        Ok(Self {
            eye,
            vertical_fov_radians,
            near,
            far,
            focus_distance,
            pitch: forward[1].atan2(horizontal),
            yaw: forward[0].atan2(-forward[2]),
        })
    }

    /// The same camera moved to `eye`, keeping its lens and orientation.
    /// # Errors
    /// Refuses an eye the authored camera checks would refuse.
    pub fn at(self, eye: [f32; 3]) -> Result<Self, String> {
        if !eye.iter().all(|value| value.is_finite()) || !(0.0..=100_000.0).contains(&eye[1]) {
            return Err("scene camera altitude must be finite and within 0–100 km".to_owned());
        }
        Ok(Self { eye, ..self })
    }

    /// The authored eye in world metres.
    #[must_use]
    pub fn eye(self) -> [f32; 3] {
        self.eye
    }

    pub(crate) fn orientation(self) -> [f32; 2] {
        [self.pitch, self.yaw]
    }

    pub(crate) fn view(
        self,
        extent: [u32; 2],
        pitch: f32,
        yaw: f32,
    ) -> Result<SyntheticView, String> {
        let (sin_pitch, cos_pitch) = pitch.sin_cos();
        let (sin_yaw, cos_yaw) = yaw.sin_cos();
        let direction = [cos_pitch * sin_yaw, sin_pitch, -cos_pitch * cos_yaw];
        let target =
            std::array::from_fn(|axis| self.eye[axis] + self.focus_distance * direction[axis]);
        SyntheticView::perspective(
            self.eye,
            target,
            [0.0, 1.0, 0.0],
            self.vertical_fov_radians,
            extent[0] as f32 / extent[1] as f32,
            self.near,
            self.far,
        )
        .map_err(|error| format!("scene camera: {error:?}"))
    }
}

mod route;
pub use route::{CameraRoute, RouteCheckpoint, RouteKey, RoutePose};

#[cfg(test)]
mod tests;
