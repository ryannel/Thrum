//! Declaration and checked camera requests for opaque secondary views.

use crate::synthetic::SyntheticView;
use glam::Vec3;
use std::sync::{Arc, Mutex};

mod record;
pub use record::SecondaryViewRecord;

/// One asked camera, delivered after every contribution's ordinary upload.
/// The index is stable until the scene surface is replaced. Producers use
/// these exact raster matrices for their own frustum, clip and size tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecondaryViewFrame {
    /// Index of the matching [`SecondarySceneSurface`].
    pub index: usize,
    /// This frame's camera, viewport, effective clip and least size.
    pub record: SecondaryViewRecord,
    /// Requested finite Sun cascade count, from zero to two.
    pub shadow_count: u32,
    /// Requested cascade receiver reach in world metres.
    pub shadow_reach: f32,
}

/// Pipeline inputs for an opted-in opaque producer. This lends no slot
/// textures: only the contribution that declares a slot receives its handle.
#[derive(Clone)]
pub struct SecondarySceneSurface<'a> {
    /// Stable index used by upload and draw callbacks.
    pub index: usize,
    /// The scene's shading, with the view fragment entries and clipping.
    pub shader_source: &'a str,
    /// View-local camera, shadow and reconstruction inputs at group 0.
    pub view_layout: &'a wgpu::BindGroupLayout,
    /// View-local light clusters and aerial product at group 1.
    pub light_layout: &'a wgpu::BindGroupLayout,
    /// Shared scene materials at group 2.
    pub material_layout: &'a wgpu::BindGroupLayout,
    /// The entry that calls the scene's shading and writes opaque coverage.
    pub fragment_entry: &'static str,
    /// Single scene-encoded colour attachment, with coverage in alpha.
    pub colour_format: wgpu::TextureFormat,
    /// Reverse-Z depth attachment, cleared to zero and compared with Greater.
    pub depth_format: wgpu::TextureFormat,
    /// This view's Sun caster pipeline inputs, if Sun shadows were prepared.
    pub shadow: Option<super::SceneShadowSurface<'a>>,
}

/// A declared slot lent only to its owning contribution. Physical attachments
/// are allocated at preparation; asking changes no pipeline or resource.
#[derive(Clone)]
pub struct SecondaryView {
    pub(crate) colour: wgpu::Texture,
    pub(crate) depth: wgpu::Texture,
    pub(crate) colour_view: wgpu::TextureView,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) record: wgpu::Buffer,
    pub(crate) request: Arc<Mutex<ViewRequestState>>,
    pub(crate) maximum_extent: [u32; 2],
    pub(crate) generation: u64,
}

pub(crate) struct ViewRequestState {
    pub(crate) camera: [f32; 3],
    pub(crate) asked: Option<SecondaryViewRequest>,
}

impl SecondaryView {
    /// Ask for this slot in the contribution's upload callback. The request
    /// lasts for that recording alone, including a discarded recording or capture.
    /// Each upload starts idle. A second ask in that upload replaces the first.
    /// Copy the main camera's horizontal coordinates from [`super::ContributionFrame::camera`]
    /// when constructing the camera. Any height, direction and field of view is accepted.
    /// # Errors
    /// Refuses a viewport larger than the prepared slot. A camera off the main
    /// camera's vertical is refused by name and leaves the slot idle for this frame.
    pub fn request(&self, request: &SecondaryViewRequest) -> Result<(), SecondaryViewError> {
        let mut state = self
            .request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let camera = request.camera.camera_position();
        if [0, 2]
            .into_iter()
            .any(|axis| camera[axis].to_bits() != state.camera[axis].to_bits())
        {
            state.asked = None;
            return Err(SecondaryViewError::CameraOffVertical);
        }
        if request
            .extent
            .iter()
            .zip(self.maximum_extent)
            .any(|(&side, maximum)| side > maximum)
        {
            return Err(SecondaryViewError::InvalidExtent);
        }
        state.asked = Some(*request);
        Ok(())
    }

    /// Scene-encoded RGBA16Float opaque colour. Coverage is alpha; uncovered
    /// colour is zero. Read only in a frame where this slot was asked for.
    #[must_use]
    pub fn colour(&self) -> &wgpu::TextureView {
        &self.colour_view
    }

    /// Standard reverse-Z Depth32Float, cleared to zero. Its world reconstruction
    /// uses the inverse jittered matrix in this slot's record.
    #[must_use]
    pub fn depth(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Uniform containing [`SecondaryViewRecord`], updated before this view draws.
    /// Bind using [`Self::record_wgsl`] in the consumer's shader.
    #[must_use]
    pub fn record(&self) -> &wgpu::Buffer {
        &self.record
    }

    /// WGSL declaration of the record. The consumer chooses its group and binding.
    #[must_use]
    pub fn record_wgsl() -> &'static str {
        include_str!("views/record.wgsl")
    }

    /// Largest viewport and physical attachment extent.
    #[must_use]
    pub fn maximum_extent(&self) -> [u32; 2] {
        self.maximum_extent
    }

    /// Device generation which owns all resources of this handle.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn asked(&self) -> Option<SecondaryViewRequest> {
        self.request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .asked
    }

    pub(crate) fn begin(&self, camera: [f32; 3]) {
        *self
            .request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = ViewRequestState {
            camera,
            asked: None,
        };
    }
}

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
        let scale = normal.abs().max_element();
        if !equation.iter().all(|value| value.is_finite()) || scale <= 0.0 {
            return Err(SecondaryViewError::InvalidClipPlane);
        }
        let length = (normal / scale).length();
        let equation = equation.map(|value| (value / scale) / length);
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
    /// Maximum declarations across the whole scene in the current profile.
    pub const MAX_COUNT: usize = 4;
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
        if slots.len() > Self::MAX_COUNT {
            return Err(SecondaryViewError::SlotLimit);
        }
        if !slots.is_empty()
            && (limits.max_bindings_per_bind_group < 7
                || limits.max_uniform_buffer_binding_size < size_of::<SecondaryViewRecord>() as u64)
        {
            return Err(SecondaryViewError::RecordLimit);
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
    /// The device cannot bind the reconstruction record alongside view inputs.
    RecordLimit,
    /// The per-view local-light table cannot be prepared within the device profile.
    LightingLimit,
    /// The view's extra stage bindings exceed the device's shader profile.
    ShaderLimit,
    /// A scale, viewport or device texture extent is invalid.
    InvalidExtent,
    /// A request must use a checked rigid perspective camera.
    InvalidCamera,
    /// The declared horizontal coordinates must exactly match the frame's main camera.
    CameraOffVertical,
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
            Self::RecordLimit => "secondary view record exceeds device binding limits",
            Self::LightingLimit => "secondary view local-light table exceeds its profile",
            Self::ShaderLimit => "secondary view shader inputs exceed device stage limits",
            Self::InvalidExtent => "invalid secondary view extent",
            Self::InvalidCamera => "secondary view camera must be rigid and perspective",
            Self::CameraOffVertical => "secondary view camera is off the main camera's vertical",
            Self::InvalidClipPlane => "invalid secondary view clip plane",
            Self::AirNeedsLevelPlane => "secondary view air from plane requires a level clip plane",
            Self::InvalidLeastSize => "invalid secondary view least size",
            Self::InvalidShadowProfile => "invalid secondary view shadow profile",
        })
    }
}

impl std::error::Error for SecondaryViewError {}
