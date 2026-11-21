//! V1 scene/rules facade over the engine-owned bounded frame runtime.

use crate::{
    environment::DayCycle,
    projection::{GameProjection, ProjectionGap},
    scene::{GameContent, RouteOverlay},
};
pub use runtime::{DriveReport, FrameConfig, FrameSample, SurfaceOutput};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use thrum_content_runtime::EnvironmentProduct;
use thrum_render::{
    capabilities::DeviceCapabilities,
    runtime,
    synthetic::{CullMode, DisplayOverlay, SunShadowMethod},
};

mod scene_source;
mod world;
pub(crate) use scene_source::GameScene;
pub use scene_source::field_in_view_share;
use world::WorldDriver;
pub(crate) use world::WorldPort;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;
#[cfg(not(target_arch = "wasm32"))]
mod world_thread;
#[cfg(not(target_arch = "wasm32"))]
use world_thread::WorldClient;
#[cfg(not(target_arch = "wasm32"))]
pub use world_thread::WorldControls;

pub(crate) type Runtime = runtime::FrameHost<WorldPort, GameScene>;

/// Content and camera culling chosen when the host is prepared.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MeshRowSource {
    /// Meshes authored directly in game code.
    #[default]
    Code,
    /// Named mesh products participating in cooked publications.
    Cooked,
}

impl From<bool> for MeshRowSource {
    fn from(cooked: bool) -> Self {
        if cooked { Self::Cooked } else { Self::Code }
    }
}

/// Visual selection for the exit scene's entity population.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExitEntities {
    /// Draw the populated scene.
    #[default]
    Draw,
    /// Diagnostic comparison with the same store and no drawn entities.
    Omit,
}

/// Content and camera culling chosen when the host is prepared.
#[derive(Clone, Copy, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent opt-in presentation selections"
)]
pub struct GameSetup {
    /// Select the sprite test card at this integer output scale.
    pub sprite_scale: Option<u16>,
    /// If selected with sprites, map this many simulated entities instead of the card.
    pub sprite_rows: Option<u32>,
    /// Valley entity meshes follow cooked publications instead of code-authored props.
    pub cooked_mesh_rows: MeshRowSource,
    /// Opt-in CPU Weather and GPU Shade, with no cloud layer. Off by default.
    pub clouds: crate::CloudSelection,
    /// Initial authoritative weather controls; ignored when clouds are off.
    pub cloud_weather: thrum_cloud_model::Parameters,
    /// Admit portable temporal reconstruction at native scale.
    pub reconstruction: thrum_render::synthetic::Reconstruction,
    /// Initial display diagnostic; TemporalInputs also prepares stored reconstruction inputs.
    pub display_overlay: DisplayOverlay,
    /// Main view's Sun shadow method.
    pub sun_shadow_method: SunShadowMethod,
    /// Trace hard shadows from the main view's local lights.
    pub lamp_shadows: bool,
    /// Opt-in analytic Sun-path optical depth, a diagnostic cloud-shade stand-in.
    pub diagnostic_clouds: bool,
    /// The grid or the dense flythrough field.
    pub content: GameContent,
    /// Draw the exit terrain and sample objects with its entity population suppressed.
    pub exit_entities: ExitEntities,
    /// How the camera's opaque draw is culled; Direct draws every instance.
    pub camera_culling: CullMode,
    /// Prepare the authored valley's star field. Ignored for the flythrough field.
    pub valley_stars: bool,
    /// Reseed the moving-route proof overlay (11 static grid slots at valley metres,
    /// one mirrored basis and one retuned local light). Grid content only; off by default.
    pub route_overlay: RouteOverlay,
    /// Draw the flythrough field's props as a rock, a column and a spruce,
    /// some mirrored, instead of boxes. Also selects meshes for valley entities. Occlusion uses
    /// the same mesh buckets with a class-matched depth prepass.
    pub field_meshes: bool,
}

impl Default for GameSetup {
    fn default() -> Self {
        Self {
            sprite_scale: None,
            sprite_rows: None,
            cooked_mesh_rows: MeshRowSource::Code,
            clouds: crate::CloudSelection::Off,
            cloud_weather: crate::clouds::initial_state().parameters(),
            reconstruction: thrum_render::synthetic::Reconstruction::Off,
            display_overlay: DisplayOverlay::Off,
            sun_shadow_method: SunShadowMethod::Maps,
            lamp_shadows: false,
            diagnostic_clouds: false,
            content: GameContent::Grid,
            exit_entities: ExitEntities::Draw,
            camera_culling: CullMode::Direct,
            valley_stars: false,
            route_overlay: RouteOverlay::Off,
            field_meshes: false,
        }
    }
}

impl GameSetup {
    /// Valley entities use conservative Frustum; promotion to Occlusion requires
    /// two paired moving-route runs winning both behind cover and in the open.
    pub fn valley_default_culling() -> CullMode {
        CullMode::Frustum
    }

    /// Two paired 1080p runs favour Occlusion on mixed meshes in cover and open
    /// views (CULLING's M2 record, .orchestrate/evidence/CULLING/M2.md). The cube field retains Frustum.
    pub(crate) fn field_default_culling(meshes: bool) -> CullMode {
        if meshes {
            CullMode::Occlusion
        } else {
            CullMode::Frustum
        }
    }
}

/// Prepared V1 game, whose world clock and frame executor belong to the engine.
pub struct FrameHost {
    cooked_rows: Option<thrum_asset_gpu::rows::CookedRows>,
    inner: Runtime,
    cloud_input: Option<thrum_clouds::WeatherInput>,
    cloud_sky: Option<thrum_clouds::SkyProfile>,
    #[cfg(not(target_arch = "wasm32"))]
    visual_product: Option<EnvironmentProduct>,
}

/// One admitted callback, preserving the engine's exclusive frame token.
pub struct AdvancedFrame<'a>(runtime::AdvancedFrame<'a, WorldPort, GameScene>);
impl<'a> AdvancedFrame<'a> {
    /// Whether the engine can record a scene before drawable acquisition.
    pub fn can_acquire_serial(&self) -> bool {
        self.0.can_acquire_serial()
    }
    /// Submit the optional presentation of the already admitted world callback.
    /// # Errors
    /// Returns an engine frame failure without undoing world authority.
    pub fn submit(
        self,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, runtime::FrameError> {
        self.0.submit(surface)
    }
    /// Record the scene before the platform acquires its drawable.
    /// # Errors
    /// Returns an engine extraction or recording failure.
    pub fn encode_scene(self) -> Result<EncodedFrame<'a>, runtime::FrameError> {
        self.0.encode_scene().map(EncodedFrame)
    }
}
/// One unsubmitted engine scene token, held across late drawable acquisition.
pub struct EncodedFrame<'a>(runtime::EncodedFrame<'a, WorldPort, GameScene>);
impl EncodedFrame<'_> {
    /// Copy the caster view uniforms of this encoded diagnostic frame.
    #[cfg(feature = "draw-probe")]
    pub fn copy_shadow_view_witness(&mut self, output: &wgpu::Buffer) -> Option<u64> {
        self.0.copy_shadow_view_witness(output)
    }

    /// Whether the engine recorded a view that can use a drawable.
    pub fn can_acquire_surface(&self) -> bool {
        self.0.can_acquire_surface()
    }

    /// Actual resources and matrix of this encoded frame.
    pub fn frame_extent(&self) -> Option<runtime::FrameExtent> {
        self.0.frame_extent()
    }

    /// Skip a view while retaining its committed world update.
    pub fn skip(self) -> DriveReport {
        self.0.skip()
    }
    /// Cancel after foreground lease revocation, without queue work, and reset history.
    pub fn cancel_background(self) -> DriveReport {
        self.0.cancel_background()
    }

    /// Discard the view after the platform observes loss of its device,
    /// without the empty queue submit a healthy skip makes.
    pub fn abandon_lost_device(self) {
        self.0.abandon_lost_device();
    }
    /// Append the optional drawable consumer and submit through the engine.
    /// # Errors
    /// Returns an engine submission failure.
    pub fn submit(
        self,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, runtime::FrameError> {
        self.0.submit(surface)
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Prepared game product paired with engine-owned replacement resources.
pub struct PreparedEnvironment {
    product: Option<EnvironmentProduct>,
    inner: runtime::PreparedVisual,
}
#[cfg(not(target_arch = "wasm32"))]
/// Detached engine preparation inputs for a cooked game environment.
pub struct VisualPrepareInputs(runtime::VisualPrepareInputs);
#[cfg(not(target_arch = "wasm32"))]
impl VisualPrepareInputs {
    /// Prepare the cooked photometric view without borrowing the active game.
    /// # Errors
    /// Returns a GPU preparation failure.
    pub async fn prepare(self, product: EnvironmentProduct) -> Result<PreparedEnvironment, String> {
        let revision = Some(*product.revision().as_bytes());
        let lighting = thrum_render::synthetic::FrameLighting::Photometric(
            scene_source::photometric(&product),
        );
        let inner = self
            .0
            .prepare(lighting, 1.0, revision)
            .await
            .map_err(|error| error.to_string())?;
        Ok(PreparedEnvironment {
            product: Some(product),
            inner,
        })
    }
}

/// Native frame view after its authoritative world has moved to an autonomous
/// lane. This exposes presentation operations without exposing inline world
/// and projection methods that cannot run on the render owner.
#[cfg(not(target_arch = "wasm32"))]
pub struct ThreadedFrameHost {
    inner: FrameHost,
}

#[cfg(not(target_arch = "wasm32"))]
impl ThreadedFrameHost {
    /// Outcomes of the explicitly selected native copy/select witness.
    pub fn native_witness_report(&self) -> Option<thrum_render::synthetic::NativeWitnessReport> {
        self.inner.native_witness_report()
    }
    /// Culling decision of the published render generation.
    pub fn culling_selection(&self) -> thrum_render::synthetic::CullingSelection {
        self.inner.culling_selection()
    }
    /// Snapshot GPU reconstruction inputs without moving the resident world.
    /// # Errors
    /// Returns a failure while sampling the current camera or lighting policy.
    pub fn device_prepare_inputs(
        &self,
    ) -> Result<runtime::DevicePrepareInputs, runtime::FrameError> {
        self.inner.inner.device_prepare_inputs()
    }

    /// Install a host-bound replacement without polling the lost device.
    /// # Errors
    /// Refuses stale, consumed or foreign reconstruction resources.
    pub fn publish_device(
        &mut self,
        candidate: &mut runtime::PreparedDevice,
    ) -> Result<(), runtime::FrameError> {
        self.inner.inner.publish_device(candidate)
    }

    /// Set the next display target without advancing the fixed-step world.
    pub fn set_presentation_lead(&mut self, lead: Duration) {
        self.inner.set_presentation_lead(lead);
    }

    /// Select the game's visual time of day without changing simulation time.
    /// # Errors
    /// Refuses a fraction outside the normalized day.
    pub fn set_time_of_day(&mut self, fraction: f32) -> Result<(), String> {
        self.inner.set_time_of_day(fraction)
    }

    /// Committed solar time of the world lane's latest tick: midnight zero, noon one half.
    pub fn time_of_day(&self) -> f32 {
        self.inner.time_of_day()
    }

    /// The controlled actor's committed position in the world lane's latest published page.
    pub fn controlled_position(&self) -> Option<[f64; 3]> {
        self.inner.controlled_position()
    }

    /// Change the game camera elevation continuously.
    /// # Errors
    /// Refuses an unsupported or nonfinite pitch.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), String> {
        self.inner.set_camera_pitch(radians)
    }

    /// Turn the game camera continuously.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside ±π radians.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), String> {
        self.inner.set_camera_yaw(radians)
    }

    /// Draw a display diagnostic, such as exposure zones, in place of the tone-mapped scene.
    pub fn set_display_overlay(&mut self, overlay: DisplayOverlay) {
        self.inner.set_display_overlay(overlay);
    }

    /// Render the latest immutable world publication.
    /// # Errors
    /// Returns a render or authoritative-world fault.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, runtime::FrameError> {
        self.inner.inner.drive(now, surface)
    }

    /// As [`FrameHost::set_scene_contribution`].
    /// # Errors
    /// The contribution's refusal to draw into the current scene.
    pub fn set_scene_contribution(
        &mut self,
        contribution: Box<dyn thrum_render::contribution::SceneContribution>,
    ) -> Result<(), String> {
        self.inner.set_scene_contribution(contribution)
    }

    /// Admit the world observation before scene encoding and late drawable
    /// acquisition on the render owner.
    /// # Errors
    /// Returns a clock, publication or GPU fault without rolling back a tick.
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_>, runtime::FrameError> {
        self.inner.advance(now)
    }

    /// Whether a serial render view can use an available GPU slot.
    pub fn has_serial_view(&self) -> bool {
        self.inner.has_serial_view()
    }

    /// Whether the last committed page can be resampled.
    pub fn has_current_scene_for_display(&self) -> bool {
        self.inner.has_current_scene_for_display()
    }

    /// Number of GPU frames still in flight.
    pub fn pending_frames(&self) -> usize {
        self.inner.pending_frames()
    }

    /// Poll one completed GPU sample.
    /// # Errors
    /// Returns a device fault.
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, runtime::FrameError> {
        self.inner.inner.poll_completed()
    }

    /// Enable the renderer tap with finite allocation limits.
    /// # Errors
    /// Refuses an active session or invalid limits.
    pub fn enable_submitted_capture(
        &mut self,
        session: u64,
        limits: runtime::CaptureLimits,
    ) -> Result<(), String> {
        self.inner
            .inner
            .enable_submitted_capture_with_limits(session, limits)
            .map_err(|e| e.to_string())
    }

    /// Collect a submitted image without advancing the world.
    /// # Errors
    /// Refuses mapping failure or retired generation.
    pub fn try_submitted_image(&mut self) -> Result<Option<runtime::SubmittedImage>, String> {
        self.inner.try_submitted_image()
    }

    /// Return a CPU lease to its generation-bound pool.
    pub fn recycle_submitted_image(&mut self, image: runtime::SubmittedImage) {
        self.inner.recycle_submitted_image(image);
    }

    /// Stop new copies while pending images drain.
    pub fn stop_submitted_capture(&mut self) {
        self.inner.inner.stop_submitted_capture();
    }

    /// Cancel capture without submitting or waiting for GPU work.
    pub fn disable_submitted_capture(&mut self) {
        self.inner.disable_submitted_capture();
    }

    /// Samples omitted by the renderer's finite ring.
    pub fn submitted_capture_omissions(&self) -> u64 {
        self.inner.inner.submitted_capture_omissions()
    }

    /// Current staging and CPU leases.
    pub fn submitted_capture_usage(&self) -> Option<(usize, usize)> {
        self.inner.inner.submitted_capture_usage()
    }

    /// Resize the surface binding after a coordinated platform change.
    /// # Errors
    /// Refuses an invalid or unavailable surface.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), runtime::FrameError> {
        self.inner.inner.resize_surface(extent)
    }

    /// Resize the scene grid and presenter coherently on the render owner.
    /// # Errors
    /// Returns a preparation fault. Zero extents or busy slots defer.
    pub fn resize_scene_surface(&mut self, extent: [u32; 2]) -> Result<bool, runtime::FrameError> {
        self.inner.inner.resize_scene_surface(extent)
    }

    /// Resize with a final platform lease/epoch validity check.
    /// # Errors
    /// Returns a preparation fault. A stale epoch defers publication.
    pub fn resize_scene_surface_if(
        &mut self,
        extent: [u32; 2],
        current: impl FnOnce() -> bool,
    ) -> Result<bool, runtime::FrameError> {
        self.inner.inner.resize_scene_surface_if(extent, current)
    }

    /// Optional scene views skipped.
    pub fn dropped_views(&self) -> u64 {
        self.inner.dropped_views()
    }

    /// Completed views dropped after source expiry.
    pub fn dropped_stale_views(&self) -> u64 {
        self.inner.dropped_stale_views()
    }

    /// Surface views dropped.
    pub fn dropped_surface(&self) -> u64 {
        self.inner.dropped_surface()
    }

    /// Completed profiling rows dropped.
    pub fn dropped_reports(&self) -> u64 {
        self.inner.dropped_reports()
    }

    /// The engine runtime, for a platform shell that drives it directly.
    #[cfg(target_os = "macos")]
    pub(crate) fn into_runtime(self) -> Runtime {
        self.inner.inner
    }
}

impl FrameHost {
    /// Observe Mesh BLAS builds before the first frame of the diagnostic run.
    #[cfg(all(feature = "draw-probe", target_os = "macos"))]
    #[must_use]
    pub fn observe_ray_mesh_builds(&mut self) -> Option<thrum_render::synthetic::RayBuildProbe> {
        self.inner.observe_ray_mesh_builds()
    }

    /// Inspect the actual scene structure from the latest recording.
    #[cfg(all(feature = "draw-probe", target_os = "macos"))]
    #[must_use]
    pub fn ray_structure_instances(
        &self,
    ) -> Option<Vec<thrum_render::synthetic::RayStructureInstance>> {
        self.inner.ray_structure_instances()
    }

    /// Bind the latest scene structure for a diagnostic query on the host's queue.
    #[cfg(all(feature = "draw-probe", target_os = "macos"))]
    #[must_use]
    pub fn ray_structure_probe_group(
        &self,
        layout: &wgpu::BindGroupLayout,
    ) -> Option<wgpu::BindGroup> {
        self.inner.ray_structure_probe_group(layout)
    }

    /// Optional submitted cloud transport and lighting products for inspection.
    pub fn cloud_sky_products(&self) -> Option<thrum_clouds::SkyProducts> {
        self.cloud_input.as_ref()?.sky_products()
    }
    /// Inspect the prepared scene graph without encoding another frame.
    /// # Errors
    /// Refuses a target during an incomplete scene transition.
    pub fn scene_graph(&self) -> Result<thrum_render::graph::CompiledGraph, runtime::FrameError> {
        self.inner.scene_graph()
    }
    /// Read-only temporal inputs of the latest in-flight submitted frame.
    pub fn submitted_temporal_witness(&self) -> Option<&wgpu::Texture> {
        self.inner.submitted_temporal_witness()
    }
    /// Read-only Sun depth layers of the latest submitted diagnostic frame.
    #[cfg(feature = "draw-probe")]
    #[must_use]
    pub fn submitted_shadow_witness(&self) -> Option<&wgpu::Texture> {
        self.inner.submitted_shadow_witness()
    }
    /// Discover the attached effects' runtime quality controls.
    pub fn quality_knobs(&self) -> Vec<thrum_render::contribution::QualityKnob> {
        self.inner.quality_knobs()
    }
    /// Stage a quality change for the next frame boundary.
    /// # Errors
    /// Refuses unknown names and values outside the declared range.
    pub fn set_quality_knob(
        &mut self,
        name: &str,
        value: u32,
    ) -> Result<(), thrum_render::contribution::QualityError> {
        self.inner.set_quality_knob(name, value)?;
        if let Some(profile) = &mut self.cloud_sky {
            if name.ends_with("cloud refresh exponent") {
                profile.refresh_stride = 1 << value;
            }
            if name.ends_with("cloud local light samples") {
                profile.light_samples = value;
            }
        }
        Ok(())
    }
    /// Current Weather and Shade product, for optional GPU inspection.
    pub fn cloud_ground_product(&self) -> Option<(wgpu::Texture, wgpu::Buffer)> {
        self.cloud_input.as_ref()?.ground_product()
    }

    /// Prepared cloud resource payload bytes, excluding driver overhead.
    pub fn cloud_resource_bytes(&self) -> Vec<(String, u64)> {
        self.cloud_input
            .as_ref()
            .map_or_else(Vec::new, thrum_clouds::WeatherInput::resource_bytes)
    }
    /// Reconstruction provider actually prepared for the live scene generation.
    pub fn reconstruction(&self) -> thrum_render::synthetic::Reconstruction {
        self.inner.reconstruction()
    }
    /// Pipeline creations on this device generation, when creation counters are enabled.
    pub fn pipeline_creation_count(&self) -> Option<u64> {
        self.inner.pipeline_creation_count()
    }
    /// Existing generation ledger, including object deltas and pipeline events.
    /// This diagnostic does not introduce another creation authority.
    #[cfg(feature = "draw-probe")]
    #[must_use]
    pub fn creation_report(&self) -> runtime::CreationReport {
        self.inner.creation_report()
    }
    /// Inspect the renderer's prepared variant storage and resident texture bytes.
    #[must_use]
    pub fn storage_report(&self) -> String {
        self.inner.storage_report()
    }

    /// Outcomes of the explicitly selected native copy/select witness.
    #[must_use]
    pub fn native_witness_report(&self) -> Option<thrum_render::synthetic::NativeWitnessReport> {
        self.inner.native_witness_report()
    }

    /// Maximum submitted scene packets retained by the engine.
    pub const MAX_IN_FLIGHT: usize = Runtime::MAX_IN_FLIGHT;
    /// Initial generation for simple callers; live platforms own later generations.
    pub const DEVICE_GENERATION: u64 = thrum_render::capabilities::FIRST_DEVICE_GENERATION;

    /// Retain CPU presentation inputs for detached GPU reconstruction.
    /// # Errors
    /// Returns a current camera or lighting policy failure.
    pub fn device_prepare_inputs(
        &self,
    ) -> Result<runtime::DevicePrepareInputs, runtime::FrameError> {
        self.inner.device_prepare_inputs()
    }

    /// Install complete GPU resources while keeping this game's resident world.
    /// # Errors
    /// Refuses a stale, foreign or consumed device candidate.
    pub fn publish_device(
        &mut self,
        candidate: &mut runtime::PreparedDevice,
    ) -> Result<(), runtime::FrameError> {
        self.inner.publish_device(candidate)
    }

    /// Prepare the fixed V1 scene using a platform-owned device and queue.
    /// # Errors
    /// Returns a scene, world or engine preparation failure.
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: FrameConfig,
    ) -> Result<Self, String> {
        let capabilities = DeviceCapabilities::capture_device(device, Self::DEVICE_GENERATION);
        Self::new_with_capabilities(device, queue, &capabilities, config).await
    }
    /// Prepare from the platform's immutable capability record.
    /// # Errors
    /// Refuses a mismatched device or game/engine preparation failure.
    pub async fn new_with_capabilities(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
    ) -> Result<Self, String> {
        Self::new_with_setup(device, queue, capabilities, config, GameSetup::default()).await
    }
    /// Prepare the selected content and camera culling mode.
    /// # Errors
    /// Refuses a mismatched device or game/engine preparation failure.
    pub async fn new_with_setup(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        setup: GameSetup,
    ) -> Result<Self, String> {
        Self::new_with_rows(device, queue, capabilities, config, setup, None)
            .await
            .map_err(|error| error.to_string())
    }

    /// Prepare entity geometry from cooked rows with stable slot choices.
    /// # Errors
    /// Returns the ordinary scene or device preparation failure.
    pub async fn new_with_rows(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        setup: GameSetup,
        rows: Option<thrum_asset_gpu::rows::CookedRows>,
    ) -> Result<Self, runtime::FrameError> {
        use runtime::FrameError;

        if !capabilities.matches(device) {
            return Err(FrameError::InvalidState(
                "capability record does not match the host device",
            ));
        }
        if let Some(scale) = setup.sprite_scale {
            let temporal_sampling = match setup.display_overlay {
                DisplayOverlay::Off => false,
                DisplayOverlay::TemporalInputs => true,
                DisplayOverlay::TracedGeometry => {
                    return Err(FrameError::InvalidState(
                        "2D scene refuses traced geometry view (THRUM_TRACED_VIEW=on)",
                    ));
                }
                DisplayOverlay::Motion => {
                    return Err(FrameError::InvalidState("2D scene refuses motion view"));
                }
                DisplayOverlay::ExposureZones => {
                    return Err(FrameError::InvalidState(
                        "2D scene refuses exposure zones view",
                    ));
                }
            };
            if setup.sun_shadow_method == SunShadowMethod::Traced {
                return Err(FrameError::InvalidState(
                    "2D scene refuses traced sun shadows",
                ));
            }
            if setup.lamp_shadows {
                return Err(FrameError::InvalidState("2D scene refuses lamp shadows"));
            }
            if rows.is_some() || setup.cooked_mesh_rows == MeshRowSource::Cooked {
                return Err(FrameError::InvalidState(
                    "2D scene refuses cooked placements",
                ));
            }
            match setup.clouds {
                crate::CloudSelection::Off => {}
                crate::CloudSelection::Shade => {
                    return Err(FrameError::InvalidState("2D scene refuses cloud shade"));
                }
                crate::CloudSelection::Sky => {
                    return Err(FrameError::InvalidState("2D scene refuses cloud sky"));
                }
            }
            for (requested, message) in [
                (
                    setup.diagnostic_clouds,
                    "2D scene refuses diagnostic clouds",
                ),
                (setup.valley_stars, "2D scene refuses valley stars"),
                (
                    setup.field_meshes,
                    "2D scene refuses field or valley meshes",
                ),
                (
                    setup.content == GameContent::Field,
                    "2D scene refuses the flythrough field",
                ),
                (
                    setup.content == GameContent::Exit,
                    "2D scene refuses the exit scene",
                ),
                (
                    setup.route_overlay != RouteOverlay::Off,
                    "2D scene refuses the route overlay",
                ),
            ] {
                if requested {
                    return Err(FrameError::InvalidState(message));
                }
            }
            let world = WorldPort::Inline(Box::new(
                match setup.sprite_rows {
                    Some(count) => WorldDriver::sprite_field(count),
                    None => WorldDriver::new(setup.content, RouteOverlay::Off),
                }
                .map_err(FrameError::World)?,
            ));
            let inner = Runtime::new_2d(
                capabilities,
                queue,
                config,
                world,
                GameScene::new(setup),
                match setup.sprite_rows {
                    Some(count) => crate::sprite_field::scene(count),
                    None => crate::sprite_card::scene(scale)?,
                },
                runtime::SpriteSceneRequests {
                    temporal_sampling,
                    culling: (setup.camera_culling != CullMode::Direct)
                        .then_some(setup.camera_culling),
                    reconstruction: setup.reconstruction,
                },
            )
            .await?;
            return Ok(Self {
                cooked_rows: None,
                inner,
                cloud_input: None,
                cloud_sky: None,
                #[cfg(not(target_arch = "wasm32"))]
                visual_product: None,
            });
        }
        let mut scene = GameScene::scene(config.render_extent, setup).map_err(FrameError::Scene)?;
        #[cfg(not(target_arch = "wasm32"))]
        let rows = if rows.is_none() && setup.cooked_mesh_rows == MeshRowSource::Cooked {
            Some(
                crate::content::selected_rows(
                    setup.content,
                    device.limits().max_storage_buffer_binding_size,
                )
                .map_err(|error| FrameError::Scene(error.to_string()))?,
            )
        } else {
            rows
        };
        if let Some(rows) = &rows {
            let slots = scene
                .meshes
                .as_ref()
                .ok_or(FrameError::InvalidState(
                    "cooked rows require a mesh selection",
                ))?
                .slots
                .clone();
            scene.meshes = Some(rows.selection(slots));
        }
        let cloud_sky = setup.clouds.sky_profile(config.render_extent);
        let world = WorldPort::Inline(Box::new(
            if setup.clouds == crate::CloudSelection::Off {
                WorldDriver::new(setup.content, setup.route_overlay)
            } else {
                WorldDriver::selected(
                    setup.content,
                    setup.route_overlay,
                    Some(setup.cloud_weather),
                )
            }
            .map_err(FrameError::World)?,
        ));
        let shade = if setup.clouds == crate::CloudSelection::Off {
            None
        } else {
            let shade = thrum_clouds::Shade::new(crate::clouds::shade_config())
                .map_err(|error| FrameError::Scene(error.to_string()))?;
            Some(if let Some(profile) = cloud_sky {
                shade
                    .with_sky(profile)
                    .map_err(|error| FrameError::Scene(error.to_string()))?
            } else {
                shade
            })
        };
        let cloud_input = shade.as_ref().map(thrum_clouds::Shade::input);
        let mut source = GameScene::new(setup);
        source.cloud_input.clone_from(&cloud_input);
        let inner = Runtime::new(
            capabilities,
            queue,
            config,
            world,
            source,
            scene,
            1.0 / 16_000.0,
        )
        .await?;
        let mut host = Self {
            cooked_rows: rows,
            inner,
            cloud_input,
            cloud_sky,
            #[cfg(not(target_arch = "wasm32"))]
            visual_product: None,
        };
        if let Some(shade) = shade {
            host.inner.attach_scene_contribution(Box::new(shade))?;
        }
        Ok(host)
    }

    /// Cooked entity rows selected at preparation, shared with residency.
    pub fn cooked_rows(&self) -> Option<thrum_asset_gpu::rows::CookedRows> {
        self.cooked_rows.clone()
    }
    /// Move the existing clock/rules into the engine's resident world lane.
    /// # Errors
    /// Returns world-lane startup or initial-publication failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_threaded(
        mut self,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(ThreadedFrameHost, WorldControls), String> {
        let WorldPort::Inline(driver) =
            std::mem::replace(self.inner.world_mut(), WorldPort::Starting)
        else {
            return Err("world is already threaded".to_owned());
        };
        let (client, controls) = WorldClient::start(*driver, origin, wake)?;
        *self.inner.world_mut() = WorldPort::Threaded(Box::new(client));
        Ok((ThreadedFrameHost { inner: self }, controls))
    }
    /// Select the platform's next display target without advancing authority.
    pub fn set_presentation_lead(&mut self, lead: Duration) {
        self.inner.set_presentation_lead(lead);
    }
    /// Change the game-authored day phase and reset presentation history.
    /// # Errors
    /// Refuses a nonfinite or out-of-range normalized time.
    pub fn set_time_of_day(&mut self, fraction: f32) -> Result<(), String> {
        set_time_of_day(&mut self.inner, fraction)
    }
    /// Current committed solar time: midnight is zero, noon is one half.
    /// This observes the world's clock; display interpolation may lag one tick.
    pub fn time_of_day(&self) -> f32 {
        time_of_day(&self.inner)
    }
    /// Solar time in the current tick's display interval at the submitted frame's alpha.
    /// Read it before advancing the host or editing the solar phase.
    pub fn rendered_time_of_day(&self, alpha: f32) -> f32 {
        rendered_time_of_day(self.inner.source(), self.tick(), alpha)
    }

    /// Change the game camera elevation continuously.
    /// # Errors
    /// Refuses a nonfinite pitch outside the supported scene range.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), String> {
        set_camera_pitch(&mut self.inner, radians)
    }

    /// Turn the game camera continuously.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside ±π radians.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), String> {
        set_camera_yaw(&mut self.inner, radians)
    }

    /// Draw a display diagnostic, such as exposure zones, in place of the tone-mapped scene.
    pub fn set_display_overlay(&mut self, overlay: DisplayOverlay) {
        set_display_overlay(&mut self.inner, overlay);
    }
    /// Inspect whether a native scene view can use a free engine slot.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_serial_view(&self) -> bool {
        self.inner.has_serial_view()
    }
    /// Whether the retained committed tick pair can be sampled for display.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_current_scene_for_display(&self) -> bool {
        self.inner.has_current_scene_for_display()
    }
    /// Prepare detached cooked visual resources using the engine executor.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visual_prepare_inputs(&self) -> VisualPrepareInputs {
        VisualPrepareInputs(self.inner.visual_prepare_inputs())
    }
    /// Install a host-bound candidate after old GPU packets retire.
    /// # Errors
    /// Returns an engine ownership/binding/upload failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn publish_environment(
        &mut self,
        candidate: &mut PreparedEnvironment,
    ) -> Result<bool, String> {
        let published = self
            .inner
            .publish_visual(&mut candidate.inner)
            .map_err(|error| error.to_string())?;
        if published {
            self.visual_product = candidate.product.take();
        }
        Ok(published)
    }
    /// Delay before the next inline world deadline.
    pub fn next_due_in(&self) -> Option<Duration> {
        self.inner.world().next_due_in()
    }
    /// Stop inline wall-time admission, retaining accepted ticks.
    /// Native threaded transitions use `WorldControls` requests and replies.
    /// # Errors
    /// Refuses a threaded world instead of presenting enqueueing as completion.
    pub fn pause(&mut self) -> Result<(), String> {
        self.inner.world_mut().pause()
    }
    /// Resume the drained inline world from a new monotonic origin.
    /// # Errors
    /// Returns a clock state/timestamp failure, or refuses a threaded world.
    pub fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.inner.world_mut().resume(now)
    }
    /// Latest committed authority tick.
    pub fn tick(&self) -> u64 {
        self.inner.world().tick()
    }
    /// Validate and replace continuous local input.
    /// # Errors
    /// Refuses invalid controls or a stopped owner.
    pub fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        self.inner.source().route_guard()?;
        self.inner.world_mut().set_controls(axes)
    }
    /// Position of the game fixture's controlled actor.
    pub fn controlled_position(&self) -> Option<[f64; 3]> {
        self.inner.world().controlled_position()
    }
    /// Enable the game's optional detached projection outbox.
    /// # Errors
    /// Returns a world publication failure.
    pub fn enable_projection(&mut self) -> Result<(), String> {
        self.inner.world_mut().enable_projection()
    }
    /// Consume game projection output in committed order.
    /// # Errors
    /// Reports an explicit overwritten-tick gap requiring a baseline.
    pub fn poll_projection(
        &mut self,
        consume: impl FnOnce(&GameProjection),
    ) -> Result<Option<bool>, ProjectionGap> {
        self.inner.world_mut().poll_projection(consume)
    }
    /// Reserved bytes in the game's detached projection outbox.
    pub fn projection_reserved_bytes(&self) -> usize {
        self.inner.world().projection_reserved_bytes()
    }
    /// Replace projection deltas with a fresh game baseline.
    /// # Errors
    /// Returns a disabled-output or publication failure.
    pub fn reset_projection_baseline(&mut self) -> Result<(), String> {
        self.inner.world_mut().reset_projection_baseline()
    }
    /// Draw an engine contribution (for example cooked G5 products) inside the scene's forward
    /// pass with the scene's selected lighting. It is rebuilt across visual and device
    /// republication.
    /// # Errors
    /// The contribution's refusal to draw into the current scene.
    pub fn set_scene_contribution(
        &mut self,
        contribution: Box<dyn thrum_render::contribution::SceneContribution>,
    ) -> Result<(), String> {
        self.inner.set_scene_contribution(crate::clouds::with_shade(
            contribution,
            self.cloud_input.as_ref(),
            self.cloud_sky,
        )?)
    }
    /// Drive authority and optional presentation through the engine.
    /// # Errors
    /// Returns a world, presentation or GPU fault.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, String> {
        self.inner
            .drive(now, surface)
            .map_err(|error| error.to_string())
    }
    /// Admit or observe the world before late surface acquisition.
    /// # Errors
    /// Returns a world or GPU polling failure.
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_>, runtime::FrameError> {
        self.inner.advance(now).map(AdvancedFrame)
    }
    /// Advance the same world without polling, preparing or submitting GPU work.
    /// # Errors
    /// Returns an authoritative world or clock failure.
    pub fn advance_without_rendering(&mut self, now: Duration) -> Result<DriveReport, String> {
        self.inner
            .advance_without_rendering(now)
            .map_err(|error| error.to_string())
    }
    /// Submitted GPU packets not yet fully retired.
    pub fn pending_frames(&self) -> usize {
        self.inner.pending_frames()
    }
    /// The camera culling mode the prepared renderer runs.
    pub fn camera_culling(&self) -> CullMode {
        self.inner.camera_culling()
    }
    /// Requested/resolved camera mode and capability reason for this generation.
    pub fn culling_selection(&self) -> thrum_render::synthetic::CullingSelection {
        self.inner.culling_selection()
    }
    /// Optional scene views shed by the engine.
    pub fn dropped_views(&self) -> u64 {
        self.inner.dropped_views()
    }
    /// Stale source views rejected by the engine.
    pub fn dropped_stale_views(&self) -> u64 {
        self.inner.dropped_stale_views()
    }
    /// Undrained profiler rows discarded by the engine.
    pub fn dropped_reports(&self) -> u64 {
        self.inner.dropped_reports()
    }
    /// Invalid presentation rows omitted during explicit capture.
    pub fn dropped_capture_rows(&self) -> u64 {
        self.inner.dropped_capture_rows()
    }
    /// Surface presentations omitted by the engine.
    pub fn dropped_surface(&self) -> u64 {
        self.inner.dropped_surface()
    }
    /// Resize prepared surface bindings after coordinated platform configuration.
    /// # Errors
    /// Returns a prepared-surface extent or device failure.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), String> {
        self.inner
            .resize_surface(extent)
            .map_err(|error| error.to_string())
    }
    /// Drain one completed engine sample in submission order.
    /// # Errors
    /// Returns a GPU polling failure.
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, String> {
        self.inner
            .poll_completed()
            .map_err(|error| error.to_string())
    }
    /// Enable the renderer-owned tap for ordinary submitted sequence frames.
    /// # Errors
    /// Refuses an already active capture session.
    pub fn enable_submitted_capture(&mut self, session: u64) -> Result<(), String> {
        self.inner
            .enable_submitted_capture(session)
            .map_err(|error| error.to_string())
    }
    /// Enable a declared engine stage through the same submitted-image tap.
    /// # Errors
    /// Refuses active capture, pending frames during a stage change or preparation failure.
    pub fn enable_submitted_capture_from(
        &mut self,
        session: u64,
        source: thrum_render::synthetic::CaptureSource,
    ) -> Result<(), String> {
        self.inner
            .enable_submitted_capture_from(session, runtime::CaptureLimits::default(), source)
            .map_err(|error| error.to_string())
    }

    /// Collect an ordinary submitted frame without redrawing an endpoint.
    /// # Errors
    /// Returns mapping or retired-generation failure.
    pub fn try_submitted_image(&mut self) -> Result<Option<runtime::SubmittedImage>, String> {
        self.inner
            .try_submitted_image()
            .map_err(|error| error.to_string())
    }
    /// Return the sequence writer's CPU image lease.
    pub fn recycle_submitted_image(&mut self, image: runtime::SubmittedImage) {
        self.inner.recycle_submitted_image(image);
    }
    /// End the sequence observer without waiting for GPU work.
    pub fn disable_submitted_capture(&mut self) {
        self.inner.disable_submitted_capture();
    }
    /// Capture the latest committed endpoint for untimed visual inspection.
    /// # Errors
    /// Returns an engine recording, completion or readback failure.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, String> {
        self.inner
            .capture_pixels()
            .map_err(|error| error.to_string())
    }
    /// Submit a capture of the latest committed endpoint without waiting; the
    /// browser finishes it from a later animation frame.
    /// # Errors
    /// Returns an engine recording or submission failure.
    pub fn begin_capture(&mut self) -> Result<runtime::PendingCapture, String> {
        self.inner
            .begin_capture()
            .map_err(|error| error.to_string())
    }
    /// Nonblocking: the capture's RGBA pixels once its readback completes.
    /// # Errors
    /// Returns a retired device generation or readback failure.
    pub fn try_finish_capture(
        &mut self,
        pending: &mut runtime::PendingCapture,
    ) -> Result<Option<Vec<u8>>, String> {
        self.inner
            .try_finish_capture(pending)
            .map_err(|error| error.to_string())
    }
}

/// Change the game-authored day phase and reset presentation history.
pub(crate) fn set_time_of_day(runtime: &mut Runtime, fraction: f32) -> Result<(), String> {
    if let Some(offset) = day_phase_edit(runtime.world().tick(), runtime.source(), fraction)? {
        runtime.source_mut().day_phase_offset = offset;
        runtime.invalidate_history();
    }
    Ok(())
}

/// Committed solar time of the runtime's latest world tick.
pub(crate) fn time_of_day(runtime: &Runtime) -> f32 {
    let source = runtime.source();
    let tick = source.held_solar_tick().unwrap_or(runtime.world().tick());
    DayCycle::time_of_day(tick, source.day_phase_offset)
}

fn rendered_time_of_day(source: &GameScene, tick: u64, alpha: f32) -> f32 {
    let (tick, alpha) = source
        .held_solar_tick()
        .map_or((tick, alpha), |held| (held, 1.0));
    DayCycle::rendered_time_of_day(tick, alpha, source.day_phase_offset)
}

/// Change the game camera elevation continuously.
pub(crate) fn set_camera_pitch(runtime: &mut Runtime, radians: f32) -> Result<(), String> {
    if let Some(pitch) = camera_pitch_edit(runtime.source(), radians)? {
        runtime.continuous_source_mut().camera_pitch = pitch;
    }
    Ok(())
}

/// Turn the game camera continuously.
pub(crate) fn set_camera_yaw(runtime: &mut Runtime, radians: f32) -> Result<(), String> {
    if let Some(yaw) = camera_yaw_edit(runtime.source(), radians)? {
        runtime.continuous_source_mut().camera_yaw = yaw;
    }
    Ok(())
}

/// Select the display diagnostic. It replaces only the display encode, so history stays valid.
pub(crate) fn set_display_overlay(runtime: &mut Runtime, overlay: DisplayOverlay) {
    runtime.source_mut().display_overlay = overlay;
}

/// The day phase offset that shows `fraction` at `tick`, or `None` when it is already shown.
fn day_phase_edit(tick: u64, scene: &GameScene, fraction: f32) -> Result<Option<f32>, String> {
    scene.route_guard()?;
    let offset = DayCycle::offset_for_time(tick, fraction)
        .ok_or_else(|| "time of day must be finite and in [0, 1]".to_owned())?;
    Ok((scene.day_phase_offset.to_bits() != offset.to_bits()).then_some(offset))
}

/// A checked camera pitch, or `None` when it is already selected.
fn camera_pitch_edit(scene: &GameScene, radians: f32) -> Result<Option<f32>, String> {
    scene.route_guard()?;
    if !radians.is_finite() || !(-0.6..=0.6).contains(&radians) {
        return Err("camera pitch must be finite and within ±0.6 rad".to_owned());
    }
    Ok((scene.camera_pitch.to_bits() != radians.to_bits()).then_some(radians))
}

/// A checked camera yaw, or `None` when it is already selected.
fn camera_yaw_edit(scene: &GameScene, radians: f32) -> Result<Option<f32>, String> {
    scene.route_guard()?;
    if !radians.is_finite() || !(-std::f32::consts::PI..=std::f32::consts::PI).contains(&radians) {
        return Err("camera yaw must be finite and within ±π rad".to_owned());
    }
    Ok((scene.camera_yaw.to_bits() != radians.to_bits()).then_some(radians))
}

#[cfg(test)]
mod tests;
