//! Declaration and checked camera requests for opaque secondary views.

use crate::synthetic::SyntheticView;
use glam::Vec3;

/// Where the aerial transport of a secondary view starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewAir {
    /// At the requested camera, as in the main view.
    #[default]
    Camera,
    /// At each fragment ray's crossing of a level clip plane. The consumer
    /// composes the transport from its own camera to that plane separately.
    Plane,
}

/// A world-space plane whose nonnegative half-space is kept.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewClipPlane([f32; 4]);

impl ViewClipPlane {
    /// Normalize a finite plane `normal.xyz · position + offset = 0`.
    /// # Errors
    /// Refuses a zero or nonfinite normal or offset.
    pub fn new(equation: [f32; 4]) -> Result<Self, SecondaryViewError> {
        let normal = Vec3::from_array([equation[0], equation[1], equation[2]]);
        let length = normal.length();
        if !equation.iter().all(|value| value.is_finite())
            || !length.is_finite()
            || length <= f32::MIN_POSITIVE
        {
            return Err(SecondaryViewError::InvalidClipPlane);
        }
        let equation = equation.map(|value| value / length);
        if !equation.iter().all(|value| value.is_finite()) {
            return Err(SecondaryViewError::InvalidClipPlane);
        }
        Ok(Self(equation))
    }

    /// Unit-normal plane equation in world space.
    #[must_use]
    pub fn equation(self) -> [f32; 4] {
        self.0
    }

    fn level(self) -> bool {
        self.0[0].abs() < 1e-6 && self.0[2].abs() < 1e-6
    }

    fn at_least_five_cm_from(self, camera: [f32; 3]) -> Self {
        let dot = self.0[0] * camera[0] + self.0[1] * camera[1] + self.0[2] * camera[2];
        let distance = dot + self.0[3];
        let mut equation = self.0;
        if distance.abs() < 0.05 {
            // Exactly on the plane is treated as before the kept half-space.
            equation[3] = if distance > 0.0 { 0.05 } else { -0.05 } - dot;
        }
        Self(equation)
    }
}

/// One slot's maximum extent, relative to each side of the main render extent.
/// Slot count and maximum extent are preparation-time declarations. Asking for
/// a smaller viewport does not resize the slot's attachments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecondaryViewSlot {
    scale: f32,
}

impl SecondaryViewSlot {
    /// Declare a slot between one quarter and all of each render side.
    /// # Errors
    /// Refuses a nonfinite scale or one outside `0.25..=1.0`.
    pub fn new(scale: f32) -> Result<Self, SecondaryViewError> {
        if !scale.is_finite() || !(0.25..=1.0).contains(&scale) {
            return Err(SecondaryViewError::InvalidExtent);
        }
        Ok(Self { scale })
    }

    /// Largest viewport at the current render extent, rounded up to full pixels.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the admitted positive scale is at most one, so rounding up stays inside the u32 input extent"
    )]
    pub fn extent(self, render_extent: [u32; 2]) -> [u32; 2] {
        render_extent.map(|side| ((f64::from(side) * f64::from(self.scale)).ceil() as u32).max(1))
    }

    /// Validate all slots before any of their GPU resources are created.
    /// # Errors
    /// Refuses more than four slots or an extent beyond the device texture limit.
    pub fn validate(
        slots: &[Self],
        render_extent: [u32; 2],
        limits: &wgpu::Limits,
    ) -> Result<(), SecondaryViewError> {
        if slots.len() > 4 {
            return Err(SecondaryViewError::SlotLimit);
        }
        if render_extent.contains(&0)
            || slots.iter().any(|slot| {
                slot.extent(render_extent)
                    .iter()
                    .any(|side| *side > limits.max_texture_dimension_2d)
            })
        {
            return Err(SecondaryViewError::InvalidExtent);
        }
        Ok(())
    }
}

/// A checked rigid perspective camera and the work requested for one frame.
/// It carries no history, reflection matrix, exposure or independent jitter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecondaryViewRequest {
    pub(crate) camera: SyntheticView,
    pub(crate) extent: [u32; 2],
    pub(crate) clip: Option<ViewClipPlane>,
    pub(crate) air: ViewAir,
    pub(crate) least_pixels: f32,
    pub(crate) cascades: u32,
    pub(crate) shadow_reach: f32,
}

impl SecondaryViewRequest {
    /// Request an opaque view. Construct `camera` with [`SyntheticView::perspective`].
    /// Its exposure and temporal settings are ignored: these come from the frame.
    /// A clip plane nearer than 5 cm is held at 5 cm before the request is accepted.
    /// Default shadow profile: one cascade to 128 m.
    /// # Errors
    /// Refuses a non-perspective camera, zero extent, or plane air without a level plane.
    pub fn new(
        camera: &SyntheticView,
        extent: [u32; 2],
        clip: Option<ViewClipPlane>,
        air: ViewAir,
    ) -> Result<Self, SecondaryViewError> {
        if !camera.is_perspective() {
            return Err(SecondaryViewError::InvalidCamera);
        }
        if extent.contains(&0) {
            return Err(SecondaryViewError::InvalidExtent);
        }
        if air == ViewAir::Plane && !clip.is_some_and(ViewClipPlane::level) {
            return Err(SecondaryViewError::AirNeedsLevelPlane);
        }
        Ok(Self {
            camera: *camera,
            extent,
            clip: clip.map(|plane| plane.at_least_five_cm_from(camera.camera_position())),
            air,
            least_pixels: 0.0,
            cascades: 1,
            shadow_reach: 128.0,
        })
    }

    /// Set the smallest projected instance worth drawing, in view pixels.
    /// # Errors
    /// Refuses a negative or nonfinite size.
    pub fn with_least_pixels(mut self, pixels: f32) -> Result<Self, SecondaryViewError> {
        if !pixels.is_finite() || pixels < 0.0 {
            return Err(SecondaryViewError::InvalidLeastSize);
        }
        self.least_pixels = pixels;
        Ok(self)
    }

    /// Select zero to two Sun cascades and a finite positive shadow reach.
    /// Zero cascades keeps the scene's height-map shadows.
    /// # Errors
    /// Refuses a count above two or a nonpositive/nonfinite reach.
    pub fn with_shadows(mut self, count: u32, reach: f32) -> Result<Self, SecondaryViewError> {
        if count > 2 || !reach.is_finite() || reach <= 0.0 {
            return Err(SecondaryViewError::InvalidShadowProfile);
        }
        self.cascades = count;
        self.shadow_reach = reach;
        Ok(self)
    }

    /// Effective plane after the 5 cm camera-distance clamp.
    #[must_use]
    pub fn clip_plane(self) -> Option<ViewClipPlane> {
        self.clip
    }
}

/// Why a secondary-view declaration or request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecondaryViewError {
    /// The profile permits at most four slots for the whole scene.
    SlotLimit,
    /// A scale, viewport or device texture extent is invalid.
    InvalidExtent,
    /// A request must use a checked rigid perspective camera.
    InvalidCamera,
    /// The clip plane is nonfinite or degenerate.
    InvalidClipPlane,
    /// Air from a plane requires a level clip plane.
    AirNeedsLevelPlane,
    /// Least size must be finite and nonnegative.
    InvalidLeastSize,
    /// Cascade count or reach is outside the supported profile.
    InvalidShadowProfile,
}

impl std::fmt::Display for SecondaryViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SlotLimit => "secondary view slot limit (four) exceeded",
            Self::InvalidExtent => "invalid secondary view extent",
            Self::InvalidCamera => "secondary view camera must be rigid and perspective",
            Self::InvalidClipPlane => "invalid secondary view clip plane",
            Self::AirNeedsLevelPlane => "secondary view air from plane requires a level clip plane",
            Self::InvalidLeastSize => "invalid secondary view least size",
            Self::InvalidShadowProfile => "invalid secondary view shadow profile",
        })
    }
}

impl std::error::Error for SecondaryViewError {}
