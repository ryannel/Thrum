//! V1 scene/rules facade over the engine-owned bounded frame runtime.

use crate::{
    camera::{CameraRoute, SceneCamera},
    environment::DayCycle,
    scene::{GameContent, RouteOverlay},
};
use runtime::FrameConfig;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(any(feature = "proving", target_os = "macos", target_arch = "wasm32", test))]
use thrum_render::synthetic::DisplayOverlay;
use thrum_render::{
    capabilities::DeviceCapabilities,
    runtime,
    synthetic::{CullMode, SunShadowMethod},
};

#[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
pub(crate) mod haze_capture;

pub(crate) mod scene_source;
mod world;
pub use scene_source::GameScene;
#[cfg(feature = "proving")]
pub use scene_source::field_in_view_share;
pub use world::GameWorld;
use world::{WorldDriver, WorldPort};
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;
#[cfg(not(target_arch = "wasm32"))]
mod world_thread;
#[cfg(not(target_arch = "wasm32"))]
use world_thread::WorldClient;
#[cfg(not(target_arch = "wasm32"))]
pub use world_thread::WorldControls;

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
#[cfg(feature = "proving")]
pub enum ExitEntities {
    /// Draw the populated scene.
    #[default]
    Draw,
    /// Diagnostic comparison with the same store and no drawn entities.
    Omit,
}

/// Reconstruction modes available to the proving ground, including the reference.
#[cfg(feature = "proving")]
pub use thrum_render::synthetic::Reconstruction as GameReconstruction;

/// Reconstruction modes available to the game.
#[cfg(not(feature = "proving"))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GameReconstruction {
    /// Spatial output with no temporal history.
    #[default]
    Off,
    /// Portable temporal reconstruction.
    Taau,
    /// Apple temporal reconstruction with a portable twin.
    MetalFx,
}

/// Content and camera culling chosen when the host is prepared.
#[derive(Clone, Copy, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent opt-in presentation selections"
)]
pub struct GameSetup {
    /// Whole-number output scale of the panel card, when selected.
    #[cfg(feature = "proving")]
    pub interface_scale: Option<u8>,
    /// Use the empty or measured paint list in place of the panel card.
    #[cfg(feature = "proving")]
    pub interface_rows: Option<u32>,
    /// Select a cooked-font card in the interface scene.
    #[cfg(feature = "proving")]
    pub interface_text: Option<crate::InterfaceText>,
    /// Select the sprite test card at this integer output scale.
    #[cfg(feature = "proving")]
    pub sprite_scale: Option<u16>,
    /// If selected with sprites, map this many simulated entities instead of the card.
    #[cfg(feature = "proving")]
    pub sprite_rows: Option<u32>,
    /// Use a sprite scene authored for sampling inspection.
    #[cfg(feature = "proving")]
    pub sprite_probe: Option<crate::SpriteProbe>,
    /// Valley entity meshes follow cooked publications instead of code-authored props.
    pub cooked_mesh_rows: MeshRowSource,
    /// Opt-in CPU Weather and GPU Shade, with no cloud layer. Off by default.
    pub clouds: crate::CloudSelection,
    /// Initial authoritative weather controls; ignored when clouds are off.
    pub cloud_weather: thrum_cloud_model::Parameters,
    /// Optional tick-driven sequence for the selected weather.
    #[cfg(feature = "proving")]
    pub cloud_sequence: crate::clouds::WeatherSequence,
    /// Admit portable temporal reconstruction at native scale.
    pub reconstruction: GameReconstruction,
    /// Initial display diagnostic; TemporalInputs also prepares stored reconstruction inputs.
    #[cfg(feature = "proving")]
    pub display_overlay: DisplayOverlay,
    /// Main view's Sun shadow method.
    pub sun_shadow_method: SunShadowMethod,
    /// Prepare soft visibility for the traced Sun.
    pub sun_penumbra: bool,
    /// Opt-in geometry for Sun shadow captures in the exit scene.
    #[cfg(feature = "proving")]
    pub sun_proof_geometry: crate::SunProofGeometry,
    /// Trace hard shadows from the main view's local lights.
    pub lamp_shadows: bool,
    /// Maximum ambient occlusion ray distance in metres.
    pub ambient_occlusion_radius: Option<f32>,
    /// Opt-in analytic Sun-path optical depth, a diagnostic cloud-shade stand-in.
    #[cfg(feature = "proving")]
    pub diagnostic_clouds: bool,
    /// The diagnostic view volume's clear-air aerosol. Absent in ordinary scenes.
    #[cfg(feature = "proving")]
    pub haze: Option<thrum_render::atmosphere::Aerosol>,
    /// Opt-in analytic test bank stand-in for weather-provided fog.
    #[cfg(feature = "proving")]
    pub test_bank: bool,
    /// The grid or the dense flythrough field.
    pub content: GameContent,
    /// Draw the exit terrain and sample objects with its entity population suppressed.
    #[cfg(feature = "proving")]
    pub exit_entities: ExitEntities,
    /// How the camera's opaque draw is culled; Direct draws every instance.
    pub camera_culling: CullMode,
    /// Prepare the authored valley's star field. Ignored for the flythrough field.
    pub valley_stars: bool,
    /// Reseed the moving-route proof overlay (11 static grid slots at valley metres,
    /// one mirrored basis and one retuned local light). Grid content only; off by default.
    #[cfg(feature = "proving")]
    pub route_overlay: RouteOverlay,
    /// Narrow two overlay props for the realtime render-scale witness.
    #[cfg(feature = "proving")]
    pub route_scale_props: bool,
    /// Draw the flythrough field's props as a rock, a column and a spruce,
    /// some mirrored, instead of boxes. Also selects meshes for valley entities. Occlusion uses
    /// the same mesh buckets with a class-matched depth prepass.
    pub field_meshes: bool,
}

impl GameSetup {
    pub(crate) fn route_overlay(&self) -> RouteOverlay {
        #[cfg(feature = "proving")]
        {
            self.route_overlay
        }
        #[cfg(not(feature = "proving"))]
        {
            let _ = self;
            RouteOverlay::Off
        }
    }
}

impl Default for GameSetup {
    fn default() -> Self {
        Self {
            #[cfg(feature = "proving")]
            interface_scale: None,
            #[cfg(feature = "proving")]
            interface_rows: None,
            #[cfg(feature = "proving")]
            interface_text: None,
            #[cfg(feature = "proving")]
            sprite_scale: None,
            #[cfg(feature = "proving")]
            sprite_rows: None,
            #[cfg(feature = "proving")]
            sprite_probe: None,
            cooked_mesh_rows: MeshRowSource::Code,
            clouds: crate::CloudSelection::Off,
            cloud_weather: crate::clouds::initial_state().parameters(),
            #[cfg(feature = "proving")]
            cloud_sequence: crate::clouds::WeatherSequence::Held,
            reconstruction: GameReconstruction::Off,
            #[cfg(feature = "proving")]
            display_overlay: DisplayOverlay::Off,
            sun_shadow_method: SunShadowMethod::Maps,
            sun_penumbra: false,
            #[cfg(feature = "proving")]
            sun_proof_geometry: crate::SunProofGeometry::Off,
            lamp_shadows: false,
            ambient_occlusion_radius: None,
            #[cfg(feature = "proving")]
            diagnostic_clouds: false,
            #[cfg(feature = "proving")]
            haze: None,
            #[cfg(feature = "proving")]
            test_bank: false,
            content: GameContent::Grid,
            #[cfg(feature = "proving")]
            exit_entities: ExitEntities::Draw,
            camera_culling: CullMode::Direct,
            valley_stars: false,
            #[cfg(feature = "proving")]
            route_overlay: RouteOverlay::Off,
            #[cfg(feature = "proving")]
            route_scale_props: false,
            field_meshes: false,
        }
    }
}

#[cfg(feature = "proving")]
pub(crate) fn haze_selection(
    value: Option<&str>,
) -> Result<Option<thrum_render::atmosphere::Aerosol>, String> {
    use thrum_render::atmosphere::Aerosol;
    let Some(value) = value else {
        return Ok(None);
    };
    if value == scene_source::test_bank::SELECTION {
        return Ok(Some(Aerosol::default()));
    }
    if value == "default" {
        return Ok(Some(Aerosol::default()));
    }
    let (range, kind) = value
        .split_once(',')
        .ok_or("haze takes visual range in km and Ångström exponent")?;
    let range = range.parse::<f32>().map_err(|error| error.to_string())?;
    let kind = kind.parse::<f32>().map_err(|error| error.to_string())?;
    if !(3.0..=300.0).contains(&range) {
        return Err("haze visual range must be within 3–300 km".to_owned());
    }
    let aerosol = thrum_cloud_model::aerosol_extinction(f64::from(range) * 1000.0, 11.6e-6)
        .map_err(|error| format!("haze extinction: {error:?}"))?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the admitted visual range has a bounded f32 GPU extinction"
    )]
    let aerosol = aerosol as f32;
    Aerosol::new(aerosol, kind)
        .map(Some)
        .map_err(|error| format!("haze profile: {error:?}"))
}

impl GameSetup {
    /// Valley entities use conservative Frustum; promotion to Occlusion requires
    /// two paired moving-route runs winning both behind cover and in the open.
    pub fn valley_default_culling() -> CullMode {
        CullMode::Frustum
    }

    /// Two paired 1080p runs favour Occlusion on mixed meshes in cover and open
    /// views (CULLING's M2 record, .orchestrate/evidence/CULLING/M2.md). The cube field retains Frustum.
    #[cfg(feature = "proving")]
    pub(crate) fn field_default_culling(meshes: bool) -> CullMode {
        if meshes {
            CullMode::Occlusion
        } else {
            CullMode::Frustum
        }
    }
}

/// Prepared V1 game held by the engine's frame host, whose world clock and frame executor belong
/// to the engine.
pub(crate) type Runtime = runtime::FrameHost<GameWorld, GameScene>;

impl GameSetup {
    /// Prepare the selected content on the engine's frame host, from the platform's capability
    /// record.
    /// # Errors
    /// Refuses a mismatched device or game/engine preparation failure.
    pub async fn prepare(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
    ) -> Result<Runtime, String> {
        let mut host = self
            .prepare_startup(device, queue, capabilities, config, None)
            .await?;
        host.finish_preparation();
        Ok(host)
    }

    pub(crate) async fn prepare_startup(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        shapes: Option<crate::PlacedSky>,
    ) -> Result<Runtime, String> {
        #[cfg(all(feature = "proving", not(target_arch = "wasm32")))]
        let sheet_source = if self.sprite_scale.is_some() {
            let store = if self.interface_text.is_some() {
                "THRUM_FONT_STORE"
            } else {
                "THRUM_SPRITE_SHEETS_STORE"
            };
            match crate::content::launch_value(store) {
                Ok(store) => Some(std::path::PathBuf::from(store).into()),
                Err(std::env::VarError::NotPresent) if self.interface_text.is_none() => None,
                Err(error) => return Err(error.to_string()),
            }
        } else {
            None
        };
        #[cfg(not(all(feature = "proving", not(target_arch = "wasm32"))))]
        let sheet_source = None;
        Box::pin(self.prepare_scene(
            device,
            queue,
            capabilities,
            config,
            None,
            shapes,
            sheet_source,
        ))
        .await
        .map_err(|error| error.to_string())
    }

    /// Prepare on a device whose first generation the caller opened, recording its
    /// capabilities from the device alone.
    /// # Errors
    /// Returns a scene, world or engine preparation failure.
    pub async fn prepare_on(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: FrameConfig,
    ) -> Result<Runtime, String> {
        let capabilities = DeviceCapabilities::capture_device(
            device,
            thrum_render::capabilities::FIRST_DEVICE_GENERATION,
        );
        self.prepare(device, queue, &capabilities, config).await
    }

    /// Prepare entity geometry from cooked rows with stable slot choices.
    /// # Errors
    /// Returns the ordinary scene or device preparation failure.
    pub async fn prepare_with_rows(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        rows: Option<thrum_asset_gpu::rows::CookedRows>,
    ) -> Result<Runtime, runtime::FrameError> {
        let mut host =
            Box::pin(self.prepare_scene(device, queue, capabilities, config, rows, None, None))
                .await?;
        host.finish_preparation();
        Ok(host)
    }

    /// Prepare the same scene with the game's opt-in placed sky.
    ///
    /// # Errors
    /// Refuses incompatible scenes, invalid content or an unavailable device.
    pub async fn prepare_placed(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        sky: crate::PlacedSky,
    ) -> Result<Runtime, runtime::FrameError> {
        let mut host = Box::pin(self.prepare_scene(
            device,
            queue,
            capabilities,
            config,
            None,
            Some(sky),
            None,
        ))
        .await?;
        host.finish_preparation();
        Ok(host)
    }

    /// Prepare the moving sprite field with cooked art followed through a content lease.
    ///
    /// # Errors
    /// Refuses an incompatible scene or any failure opening the renderer or attaching residency.
    #[cfg(feature = "proving")]
    pub async fn prepare_with_sheet_source(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        source: thrum_asset_gpu::CookedSource,
    ) -> Result<Runtime, runtime::FrameError> {
        let mut host = Box::pin(self.prepare_scene(
            device,
            queue,
            capabilities,
            config,
            None,
            None,
            Some(source),
        ))
        .await?;
        host.finish_preparation();
        Ok(host)
    }

    #[cfg(feature = "proving")]
    async fn prepare_sprite_scene(
        self,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        scale: u16,
        has_cooked_rows: bool,
        sheet_source: Option<thrum_asset_gpu::CookedSource>,
    ) -> Result<Runtime, runtime::FrameError> {
        use runtime::FrameError;
        let setup = self;
        if setup.sprite_probe.is_some() && setup.sprite_rows.is_some() {
            return Err(FrameError::InvalidState(
                "a sprite sampling probe and a sprite field cannot be selected together",
            ));
        }
        let temporal_sampling = match setup.display_overlay {
            DisplayOverlay::Off | DisplayOverlay::Undithered => false,
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
        if has_cooked_rows || setup.cooked_mesh_rows == MeshRowSource::Cooked {
            return Err(FrameError::InvalidState(
                "2D scene refuses cooked placements",
            ));
        }
        match setup.clouds {
            crate::CloudSelection::Off => {}
            crate::CloudSelection::Shade => {
                return Err(FrameError::InvalidState("2D scene refuses cloud shade"));
            }
            crate::CloudSelection::Sky | crate::CloudSelection::WorldLight => {
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
        let world = GameWorld(WorldPort::Inline(Box::new(
            match setup.sprite_rows {
                Some(count) => WorldDriver::sprite_field(count),
                None => WorldDriver::new(setup.content, RouteOverlay::Off),
            }
            .map_err(FrameError::World)?,
        )));
        let mut game = GameScene::new(setup);
        let mut scene = match (setup.sprite_probe, setup.sprite_rows) {
            (Some(probe), _) => crate::sprite_probe::assets(probe),
            (None, Some(count)) => crate::sprite_field::scene(count),
            (None, None) => crate::sprite_card::scene(scale)?,
        };
        #[cfg(target_os = "ios")]
        if setup.sprite_rows.is_some() && setup.sprite_probe.is_none() {
            crate::sprite_card::add_to_mobile_field(&mut scene)?;
        }
        let assets = if let Some(mode) = setup.interface_text {
            let source = sheet_source.ok_or(FrameError::InvalidState(
                "interface text needs a cooked font store",
            ))?;
            let (card, assets) = crate::interface_text::FontCard::attach(
                source,
                mode,
                setup.interface_scale.unwrap_or(1),
            )?;
            game.font_card = Some(Box::new(card));
            game.sheet_stats = Some(assets.stats());
            Some(assets)
        } else if let Some(source) = sheet_source {
            if setup.sprite_rows.is_none() {
                return Err(FrameError::InvalidState(
                    "cooked sprite sheets require the moving sprite field",
                ));
            }
            let (sheets, assets) = crate::sprite_field::cooked_sheets(&mut scene, source)?;
            game.field_sheets = Some(sheets);
            game.sheet_stats = Some(assets.stats());
            Some(assets)
        } else {
            None
        };
        let mut inner = Runtime::new_2d_preparing(
            capabilities,
            queue,
            config,
            world,
            game,
            scene,
            runtime::SpriteSceneRequests {
                temporal_sampling,
                culling: (setup.camera_culling != CullMode::Direct).then_some(setup.camera_culling),
                reconstruction: setup.reconstruction,
            },
        )
        .await?;
        if let Some(assets) = assets {
            inner.set_scene_contribution(Box::new(assets))?;
        }
        Ok(inner)
    }

    async fn prepare_scene(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        rows: Option<thrum_asset_gpu::rows::CookedRows>,
        shapes: Option<crate::PlacedSky>,
        sheet_source: Option<thrum_asset_gpu::CookedSource>,
    ) -> Result<Runtime, runtime::FrameError> {
        use runtime::FrameError;
        let setup = self;
        #[cfg(feature = "proving")]
        let sprites = setup.sprite_scale.is_some();
        #[cfg(not(feature = "proving"))]
        let sprites = false;
        if sheet_source.is_some() && !sprites {
            return Err(FrameError::InvalidState(
                "cooked sprite sheets require a 2D scene",
            ));
        }
        if shapes.is_some()
            && (setup.clouds == crate::CloudSelection::Off
                || setup.content == GameContent::Exit
                || sprites)
        {
            return Err(FrameError::InvalidState(
                "placed skies require an opt-in 3D cloud selection",
            ));
        }

        if !capabilities.matches(device) {
            return Err(FrameError::InvalidState(
                "capability record does not match the host device",
            ));
        }
        #[cfg(feature = "proving")]
        if let Some(scale) = setup.sprite_scale {
            return setup
                .prepare_sprite_scene(
                    queue,
                    capabilities,
                    config,
                    scale,
                    rows.is_some(),
                    sheet_source,
                )
                .await;
        }
        let mut scene = GameScene::scene(config.render_extent, setup)?;
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
        #[cfg(feature = "proving")]
        if setup.content == GameContent::Exit {
            setup.sun_proof_geometry.prepare(&mut scene);
        }
        let cloud_sky = cloud_sky_profile(setup.clouds, config.render_extent, capabilities);
        let weather = clouded_weather(setup, shapes.as_ref())?;
        let world = GameWorld(WorldPort::Inline(Box::new(
            WorldDriver::for_setup(&setup, weather).map_err(FrameError::World)?,
        )));
        let shade = cloud_shade(setup.clouds, shapes.as_ref(), cloud_sky)?;
        if let Some(shade) = &shade {
            scene.sun_optical_depth = Some(shade.provider()?);
        }
        let cloud_input = shade.as_ref().map(thrum_clouds::Shade::input);
        let mut source = GameScene::new(setup);
        source.cloud_input = cloud_input;
        source.cloud_sky = cloud_sky;
        source.cloud_shapes.clone_from(&shapes);
        source.cooked_rows = rows;
        let mut inner = Runtime::new_preparing(
            capabilities,
            queue,
            config,
            world,
            source,
            scene,
            1.0 / 16_000.0,
        )
        .await?;
        #[cfg(feature = "proving")]
        if setup.test_bank {
            inner.attach_scene_contribution(Box::new(
                scene_source::test_bank::TestBank::default(),
            ))?;
        }
        if let Some(shade) = shade {
            let contribution: Box<dyn thrum_render::contribution::SceneContribution> =
                if let Some(shapes) = shapes {
                    let (assets, shade) = shapes.shade(shade).map_err(FrameError::Scene)?;
                    Box::new(thrum_render::contribution::SceneContributions::new(vec![
                        Box::new(assets),
                        Box::new(shade),
                    ]))
                } else {
                    Box::new(shade)
                };
            inner.attach_scene_contribution(contribution)?;
        }
        Ok(inner)
    }
}

fn cloud_sky_profile(
    selection: crate::CloudSelection,
    extent: [u32; 2],
    capabilities: &DeviceCapabilities,
) -> Option<thrum_clouds::SkyProfile> {
    let mut sky = selection.sky_profile(extent)?;
    if matches!(
        capabilities.recipe(),
        Ok(thrum_render::capabilities::Recipe::AppleTile)
    ) {
        // Trace one member of each 2×2 block and reproject the other three
        // with the cloud's world-space travel between submissions.
        sky.refresh_stride = 2;
        sky.step = 22.5;
    }
    Some(sky)
}

fn clouded_weather(
    setup: GameSetup,
    shapes: Option<&crate::PlacedSky>,
) -> Result<Option<crate::simulation::Weather>, runtime::FrameError> {
    if setup.clouds == crate::CloudSelection::Off {
        return Ok(None);
    }
    #[cfg(feature = "proving")]
    let initial = setup.cloud_sequence.target(setup.cloud_weather, 0.0);
    #[cfg(not(feature = "proving"))]
    let initial = setup.cloud_weather;
    let state = shapes.map_or_else(
        || thrum_cloud_model::State::new(initial, 5.0, 0),
        |placed| placed.state(initial),
    );
    state
        .map(|state| {
            let weather =
                crate::simulation::Weather::new(state, shapes.and_then(crate::PlacedSky::demo));
            #[cfg(feature = "proving")]
            let weather = weather.with_sequence(setup.cloud_sequence, initial);
            Some(weather)
        })
        .map_err(|error| runtime::FrameError::World(error.to_string()))
}

fn cloud_shade(
    selection: crate::CloudSelection,
    shapes: Option<&crate::PlacedSky>,
    sky: Option<thrum_clouds::SkyProfile>,
) -> Result<Option<thrum_clouds::Shade>, runtime::FrameError> {
    if selection == crate::CloudSelection::Off {
        return Ok(None);
    }
    let config = shapes.map_or_else(crate::clouds::shade_config, crate::PlacedSky::profile);
    let shade = thrum_clouds::Shade::new(config)
        .map_err(|error| runtime::FrameError::Scene(error.to_string()))?;
    let Some(sky) = sky else {
        return Ok(Some(shade));
    };
    shade
        .with_sky(sky)
        .map(Some)
        .map_err(|error| runtime::FrameError::Scene(error.to_string()))
}

/// The game's own operations on the engine's frame host: its solar day, camera, moving route,
/// display diagnostics, controls, clouds and cooked content. Everything else a shell needs it
/// calls on the frame host itself.
pub trait GameHost {
    /// Change the game-authored day phase and reset presentation history.
    /// # Errors
    /// Refuses a nonfinite or out-of-range normalized time, or an installed route.
    fn set_time_of_day(&mut self, fraction: f32) -> Result<(), String>;
    /// Current committed solar time: midnight is zero, noon one half.
    /// This observes the world's clock; display interpolation may lag one tick.
    fn time_of_day(&self) -> f32;
    /// Solar time in the current tick's display interval at the submitted frame's alpha.
    /// Read it before advancing the host or editing the solar phase.
    fn rendered_time_of_day(&self, alpha: f32) -> f32;
    /// Change the game camera elevation continuously.
    /// # Errors
    /// Refuses a nonfinite pitch outside the supported scene range.
    fn set_camera_pitch(&mut self, radians: f32) -> Result<(), String>;
    /// Turn the game camera continuously.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside ±π radians.
    fn set_camera_yaw(&mut self, radians: f32) -> Result<(), String>;
    /// Current presentation camera pitch and yaw, in radians.
    fn camera_orientation(&self) -> [f32; 2];
    /// Draw a display diagnostic, such as exposure zones, in place of the tone-mapped scene.
    #[cfg(feature = "proving")]
    fn set_display_overlay(&mut self, overlay: DisplayOverlay);
    /// The authored scene camera, when content selected one.
    fn scene_camera(&self) -> Option<SceneCamera>;
    /// Select a checked authored camera and reset presentation history.
    /// # Errors
    /// Refuses while a moving route is installed.
    fn set_scene_camera(&mut self, camera: SceneCamera) -> Result<(), String>;
    /// Install a moving camera route when the committed tick is `origin_tick − 1`.
    /// From then on the authored camera follows the route and camera, time and
    /// control edits are refused. History resets so both targets align at the origin.
    /// # Errors
    /// See the route's installation refusals.
    fn set_camera_route(&mut self, route: CameraRoute) -> Result<(), String>;
    /// The installed route's pose and history epoch for `tick` presented at `alpha`.
    fn route_pose(&self, tick: u64, alpha: f32) -> Option<crate::RoutePose>;
    /// World time, in milliseconds from the clock origin, of the next tick boundary
    /// plus `phi` of a period: `(tick + 1 + φ)·P` for the committed tick.
    fn route_step_ms(&self, phi: f64) -> f64;
    /// Validate and replace continuous local input.
    /// # Errors
    /// Refuses invalid controls, an installed route or a stopped owner.
    fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String>;
    /// Draw an engine contribution (for example cooked G5 products) inside the scene's forward
    /// pass with the scene's selected lighting, sharing the game's cloud Shade when clouds are
    /// selected. It is rebuilt across visual and device republication.
    /// # Errors
    /// The contribution's refusal to draw into the current scene.
    fn set_game_contribution(
        &mut self,
        contribution: Box<dyn thrum_render::contribution::SceneContribution>,
    ) -> Result<(), String>;
    /// Stage a quality change for the next frame boundary, keeping the game's cloud sky
    /// profile in step with the cloud knobs.
    /// # Errors
    /// Refuses unknown names and values outside the declared range.
    fn set_game_quality_knob(
        &mut self,
        name: &str,
        value: u32,
    ) -> Result<(), thrum_render::contribution::QualityError>;
    /// Cooked entity rows selected at preparation, shared with residency.
    fn cooked_rows(&self) -> Option<thrum_asset_gpu::rows::CookedRows>;
    /// Optional submitted cloud transport and lighting products for inspection.
    #[cfg(feature = "proving")]
    fn cloud_sky_products(&self) -> Option<thrum_clouds::SkyProducts>;
    /// Current Weather and Shade product, for optional GPU inspection.
    #[cfg(feature = "proving")]
    fn cloud_ground_product(&self) -> Option<(wgpu::Texture, wgpu::Buffer)>;
    /// Submitted cloud products and their placement reports for a placed sky.
    #[cfg(feature = "proving")]
    fn cloud_placed_input(&self) -> Option<&thrum_clouds::WeatherInput>;
    /// Prepared cloud resource payload bytes, excluding driver overhead.
    #[cfg(feature = "proving")]
    fn cloud_resource_bytes(&self) -> Vec<(String, u64)>;
    /// Move the inline clock and rules onto the engine's world lane. The frame host then
    /// renders the lane's publications, and the returned controls drive the world.
    /// # Errors
    /// Returns world-lane startup or initial-publication failure.
    #[cfg(not(target_arch = "wasm32"))]
    fn into_threaded(
        self,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Runtime, WorldControls), String>;
}

impl GameHost for Runtime {
    fn set_time_of_day(&mut self, fraction: f32) -> Result<(), String> {
        set_time_of_day(self, fraction)
    }
    fn time_of_day(&self) -> f32 {
        time_of_day(self)
    }
    fn rendered_time_of_day(&self, alpha: f32) -> f32 {
        rendered_time_of_day(self.source(), self.world().tick(), alpha)
    }
    fn set_camera_pitch(&mut self, radians: f32) -> Result<(), String> {
        set_camera_pitch(self, radians)
    }
    fn set_camera_yaw(&mut self, radians: f32) -> Result<(), String> {
        set_camera_yaw(self, radians)
    }
    fn camera_orientation(&self) -> [f32; 2] {
        let source = self.source();
        [source.camera_pitch, source.camera_yaw]
    }
    #[cfg(feature = "proving")]
    fn set_display_overlay(&mut self, overlay: DisplayOverlay) {
        set_display_overlay(self, overlay);
    }
    fn scene_camera(&self) -> Option<SceneCamera> {
        self.source().authored_camera()
    }
    fn set_scene_camera(&mut self, camera: SceneCamera) -> Result<(), String> {
        self.source_mut().select_camera(camera)?;
        self.invalidate_history();
        Ok(())
    }
    fn set_camera_route(&mut self, route: CameraRoute) -> Result<(), String> {
        let tick = self.world().tick();
        self.source_mut().install_route(route, tick)?;
        self.invalidate_history();
        Ok(())
    }
    fn route_pose(&self, tick: u64, alpha: f32) -> Option<crate::RoutePose> {
        self.source().route_pose(tick, alpha)
    }
    fn route_step_ms(&self, phi: f64) -> f64 {
        scene_source::route_step_ms(self.world().tick(), phi)
    }
    fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        self.source().route_guard()?;
        self.world_mut().0.set_controls(axes)
    }
    fn set_game_contribution(
        &mut self,
        contribution: Box<dyn thrum_render::contribution::SceneContribution>,
    ) -> Result<(), String> {
        let source = self.source();
        let contribution = crate::clouds::with_shade(
            contribution,
            source.cloud_input.as_ref(),
            source.cloud_sky,
            source.cloud_shapes.as_ref(),
        )?;
        #[cfg(feature = "proving")]
        let contribution = if source.medium_epoch.is_some() {
            scene_source::test_bank::alongside(contribution)
        } else {
            contribution
        };
        self.set_scene_contribution(contribution)
            .map_err(String::from)
    }
    fn set_game_quality_knob(
        &mut self,
        name: &str,
        value: u32,
    ) -> Result<(), thrum_render::contribution::QualityError> {
        self.set_quality_knob(name, value)?;
        if let Some(profile) = &mut self.continuous_source_mut().cloud_sky {
            if name.ends_with("cloud refresh exponent") {
                profile.refresh_stride = 1 << value;
            }
            if name.ends_with("cloud local light samples") {
                profile.light_samples = value;
            }
        }
        Ok(())
    }
    fn cooked_rows(&self) -> Option<thrum_asset_gpu::rows::CookedRows> {
        self.source().cooked_rows.clone()
    }
    #[cfg(feature = "proving")]
    fn cloud_sky_products(&self) -> Option<thrum_clouds::SkyProducts> {
        self.source().cloud_input.as_ref()?.sky_products()
    }
    #[cfg(feature = "proving")]
    fn cloud_ground_product(&self) -> Option<(wgpu::Texture, wgpu::Buffer)> {
        self.source().cloud_input.as_ref()?.ground_product()
    }
    #[cfg(feature = "proving")]
    fn cloud_placed_input(&self) -> Option<&thrum_clouds::WeatherInput> {
        self.source()
            .cloud_input
            .as_ref()
            .filter(|_| self.source().cloud_shapes.is_some())
    }
    #[cfg(feature = "proving")]
    fn cloud_resource_bytes(&self) -> Vec<(String, u64)> {
        self.source()
            .cloud_input
            .as_ref()
            .map_or_else(Vec::new, thrum_clouds::WeatherInput::resource_bytes)
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn into_threaded(
        mut self,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Runtime, WorldControls), String> {
        let WorldPort::Inline(driver) =
            std::mem::replace(&mut self.world_mut().0, WorldPort::Starting)
        else {
            return Err("world is already threaded".to_owned());
        };
        let (client, controls) = WorldClient::start(*driver, origin, wake)?;
        self.world_mut().0 = WorldPort::Threaded(Box::new(client));
        Ok((self, controls))
    }
}

impl GameWorld {
    /// Latest committed authority tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.0.tick()
    }
    #[cfg(all(target_os = "macos", feature = "proving"))]
    pub(crate) fn inline_authority_hash(&self) -> Option<u64> {
        self.0.inline_authority_hash()
    }

    /// Delay before the next inline world deadline.
    #[must_use]
    pub fn next_due_in(&self) -> Option<Duration> {
        self.0.next_due_in()
    }
    /// Stop inline wall-time admission, retaining accepted ticks.
    /// Native threaded transitions use `WorldControls` requests and replies.
    /// # Errors
    /// Refuses a threaded world instead of presenting enqueueing as completion.
    pub fn pause(&mut self) -> Result<(), String> {
        self.0.pause()
    }
    /// Resume the drained inline world from a new monotonic origin.
    /// # Errors
    /// Returns a clock state/timestamp failure, or refuses a threaded world.
    pub fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.0.resume(now)
    }
    /// Position of the game fixture's controlled actor in the latest published page.
    #[must_use]
    pub fn controlled_position(&self) -> Option<[f64; 3]> {
        self.0.controlled_position()
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

#[cfg(all(target_os = "macos", feature = "proving"))]
pub(crate) fn hold_capture_sun(runtime: &mut Runtime) {
    let tick = runtime.world().tick();
    runtime.continuous_source_mut().capture_solar_tick = Some(tick);
}

#[cfg(all(target_os = "macos", feature = "proving"))]
pub(crate) fn scrub_haze_sun(runtime: &mut Runtime, fraction: f32) -> Result<(), String> {
    if let Some(offset) = day_phase_edit(runtime.world().tick(), runtime.source(), fraction)? {
        runtime.continuous_source_mut().day_phase_offset = offset;
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

#[cfg(all(target_os = "macos", feature = "proving"))]
pub(crate) fn set_haze_eye(runtime: &mut Runtime, eye: [f32; 3], cut: bool) -> Result<(), String> {
    if runtime
        .source()
        .authored_camera()
        .is_some_and(|camera| camera.eye().map(f32::to_bits) == eye.map(f32::to_bits))
    {
        return Ok(());
    }
    runtime.continuous_source_mut().move_capture_eye(eye)?;
    if cut {
        runtime.invalidate_history();
    }
    Ok(())
}

/// Select the display diagnostic. It replaces only the display encode, so history stays valid.
#[cfg(any(feature = "proving", target_os = "macos", target_arch = "wasm32", test))]
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
