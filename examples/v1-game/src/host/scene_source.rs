//! V1 fixture, camera and lighting policy for the engine frame runtime.

#[cfg(feature = "proving")]
mod cloud;
mod star_catalogue;
#[cfg(feature = "proving")]
pub(crate) mod test_bank;

use crate::{
    camera::{CameraRoute, SceneCamera},
    environment::DayCycle,
    host::{GameReconstruction, GameSetup},
    scene::{self, GameContent},
    simulation::PublishedPage,
};
use thrum_render::{
    abi::{InstanceRecord, MaterialRecord},
    atmosphere::AtmosphereFrame,
    error::RenderError,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameError, SceneFrame, SceneSource},
    synthetic::{
        DisplayOverlay, FrameLighting, SdrToneMap, SyntheticPointLight, SyntheticScene,
        SyntheticView,
    },
};

#[expect(
    clippy::cast_possible_truncation,
    reason = "validated haze inputs fit the GPU representation"
)]
fn aerosol(range: f64, exponent: f64) -> thrum_render::atmosphere::Aerosol {
    let extinction =
        thrum_cloud_model::aerosol_extinction(range, 11.6e-6).expect("committed visual range");
    thrum_render::atmosphere::Aerosol::new(extinction as f32, exponent as f32)
        .expect("committed aerosol")
}

fn atmosphere_lighting(
    sun: crate::environment::SunSample,
    camera_altitude_m: f32,
    haze: Option<thrum_render::atmosphere::Aerosol>,
    medium: Option<MediumSource>,
) -> FrameLighting {
    let frame = AtmosphereFrame::new(
        sun.toward_sun,
        sun.top_illuminance_lux,
        sun.colour,
        camera_altitude_m,
        sun.profile_epoch,
    )
    .expect("checked game camera and day cycle produce a valid atmosphere frame")
    .with_nightglow()
    .with_simulation_time(sun.simulation_time_seconds)
    .expect("the sampled game clock is finite");
    let frame = haze.map_or(frame, |aerosol| {
        frame.with_aerosol(aerosol).with_view_volume()
    });
    let frame = match medium {
        None => frame,
        Some(MediumSource::Ground) => frame
            .with_view_volume_medium(1)
            .with_overlay_exposure_adaptation(),
        #[cfg(feature = "proving")]
        Some(MediumSource::TestBank) => frame.with_view_volume_medium(1),
    };
    FrameLighting::Atmosphere(frame)
}

/// `(tick + 1 + φ)·P` in milliseconds at the game's 60 Hz clock.
pub(crate) fn route_step_ms(tick: u64, phi: f64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "route runs stay below 560 ticks past origin"
    )]
    let ticks = tick as f64 + 1.0 + phi;
    ticks * 1000.0 / 60.0
}

fn frame_phase(tick: u64, alpha: f32) -> f32 {
    if tick == 0 {
        return 0.0;
    }
    let cycle = u16::try_from(tick.saturating_sub(1) % 360).expect("phase is less than 360");
    (f32::from(cycle) + alpha) * (std::f32::consts::TAU / 360.0)
}

fn frame_view(
    tick: u64,
    alpha: f32,
    extent: [u32; 2],
    camera_pitch: f32,
    camera_yaw: f32,
    camera: Option<SceneCamera>,
) -> Result<SyntheticView, RenderError> {
    if let Some(camera) = camera {
        return camera
            .view(extent, camera_pitch, camera_yaw)
            .map(|view| view.with_sdr_tone_map(SdrToneMap::NeutralHighlights));
    }
    let phase = frame_phase(tick, alpha);
    SyntheticView::perspective(
        [phase.sin() * 0.08, (phase.cos() - 1.0) * 0.04, 2.0],
        [
            2.0 * camera_yaw.sin(),
            2.0 * camera_pitch.tan(),
            2.0 - 2.0 * camera_yaw.cos(),
        ],
        [0.0, 1.0, 0.0],
        // Keep the initial edge rows and the neutral subject inside the view.
        65.0_f32.to_radians(),
        extent[0] as f32 / extent[1] as f32,
        0.1,
        20.0,
    )
    .map(|view| view.with_sdr_tone_map(SdrToneMap::NeutralHighlights))
}

/// The flythrough camera with the player's pitch and yaw added to its heading.
#[cfg(feature = "proving")]
fn field_view(
    tick: u64,
    alpha: f32,
    extent: [u32; 2],
    camera_pitch: f32,
    camera_yaw: f32,
) -> Result<SyntheticView, RenderError> {
    let sample = crate::field::camera(tick, alpha);
    let heading = sample.heading + camera_yaw;
    let (pitch_sin, pitch_cos) = camera_pitch.sin_cos();
    let forward = [
        heading.cos() * pitch_cos,
        pitch_sin,
        heading.sin() * pitch_cos,
    ];
    let target = std::array::from_fn(|axis| sample.eye[axis] + forward[axis] * 10.0);
    SyntheticView::perspective(
        sample.eye,
        target,
        [0.0, 1.0, 0.0],
        65.0_f32.to_radians(),
        extent[0] as f32 / extent[1] as f32,
        0.1,
        1200.0,
    )
    .map(|view| {
        view.with_sdr_tone_map(SdrToneMap::NeutralHighlights)
            .with_history_epoch(sample.history_epoch)
    })
}

/// Share of field instance centres strictly inside the flythrough frustum at
/// `tick` (no player orientation), for labelling measured rows.
/// # Errors
/// Returns an invalid camera.
#[doc(hidden)]
#[cfg(feature = "proving")]
pub fn field_in_view_share(tick: u64, extent: [u32; 2]) -> Result<f64, String> {
    let m = field_view(tick, 1.0, extent, 0.0, 0.0)
        .map_err(|error| error.to_string())?
        .view_proj_columns();
    let inside = (0..crate::field::ENTITY_COUNT)
        .filter(|index| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "field coordinates round once into the f32 GPU domain"
            )]
            let p = crate::field::entity(*index)
                .position
                .map(|value| value as f32);
            let c: [f32; 4] = std::array::from_fn(|row| {
                m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row]
            });
            c[0].abs() < c[3] && c[1].abs() < c[3] && c[2] > 0.0 && c[2] < c[3]
        })
        .count();
    #[expect(clippy::cast_precision_loss, reason = "counts below 2^24")]
    Ok(inside as f64 / crate::field::ENTITY_COUNT as f64)
}

/// The field's prepared boxes, materials and Sun shadows; no local lights.
/// With `meshes`, props draw the mixed meshes and every fifth is mirrored.
#[cfg(feature = "proving")]
fn field_scene(extent: [u32; 2], meshes: bool) -> Result<SyntheticScene, FrameError> {
    let count = u32::try_from(crate::field::ENTITY_COUNT)
        .map_err(|_| FrameError::InvalidState("field exceeds u32 slots"))?;
    let mut scene = SyntheticScene::grid(count)?;
    scene.materials = crate::field::materials()
        .into_iter()
        .map(|material| MaterialRecord {
            base_rgba: material.base_rgba,
            surface: material.surface,
            ..Default::default()
        })
        .collect();
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        let entity = crate::field::entity(index);
        let (mut basis, mut normal) = crate::field::basis(&entity);
        if meshes && crate::field_meshes::mirrored(index) {
            basis[0] = basis[0].map(|value| -value);
            normal[0] = normal[0].map(|value| -value);
        }
        for axis in 0..3 {
            instance.current[axis][..3].copy_from_slice(&basis[axis]);
            instance.normal[axis][..3].copy_from_slice(&normal[axis]);
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "field coordinates are within ±420 m and round once into the f32 GPU domain"
        )]
        let position = entity.position.map(|value| value as f32);
        instance.current[3][..3].copy_from_slice(&position);
        instance.set_material(
            u32::try_from(entity.material)
                .map_err(|_| FrameError::InvalidState("field material row"))?,
        );
        instance.previous = instance.current;
    }
    scene.view = field_view(0, 0.0, extent, 0.0, 0.0)?;
    scene.sun_shadows = true;
    scene.meshes = meshes.then(crate::field_meshes::selection);
    Ok(scene)
}

/// Scale a grid instance's cold basis so its first axis has length `half_extent`.
fn scale_basis(instance: &mut InstanceRecord, half_extent: f32) {
    let basis_length = instance.current[0][..3]
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    let scale = half_extent / basis_length;
    for axis in 0..3 {
        for component in 0..3 {
            instance.current[axis][component] *= scale;
            instance.previous[axis][component] *= scale;
            instance.normal[axis][component] /= scale;
        }
    }
}

/// Give two existing static fixtures a small cross-section for the realtime
/// render-scale inspection. Their simulation positions and the other grid rows
/// remain the ordinary route's.
#[cfg(feature = "proving")]
fn apply_render_scale_props(scene: &mut SyntheticScene) {
    for slot in [5063, 5069] {
        let instance = &mut scene.instances[slot];
        for axis in [0, 2] {
            for component in 0..3 {
                instance.current[axis][component] *= 0.125;
                instance.previous[axis][component] *= 0.125;
                instance.normal[axis][component] *= 8.0;
            }
        }
    }
}

/// The moving-route proof overlay's retuned diagnostic light, above the mirrored pair.
#[cfg(feature = "proving")]
pub(crate) const ROUTE_OVERLAY_LIGHT: SyntheticPointLight = SyntheticPointLight {
    position: [0.0, 240.55, -3.2],
    radius: 4.0,
    candela: [8000.0, 6000.0, 4000.0],
};

/// Cold geometry for the moving-route overlay: valley-scale bases matching the
/// authoritative reseeds, one mirrored basis, and diagnostic light 0 moved beside them.
/// Two-sided state is all-or-nothing, so the mirrored basis switches the whole list.
#[cfg(feature = "proving")]
fn apply_route_overlay(scene: &mut SyntheticScene, overlay: scene::RouteOverlay) {
    for (slot, position) in scene::ROUTE_OVERLAY {
        let instance = &mut scene.instances[slot];
        scale_basis(instance, 0.18);
        instance.current[3][..3].copy_from_slice(&position);
        instance.previous[3][..3].copy_from_slice(&position);
    }
    if matches!(
        overlay,
        scene::RouteOverlay::Motion | scene::RouteOverlay::Reconstruction
    ) {
        let instance = &mut scene.instances[scene::MOTION_SUBJECT_SLOT];
        scale_basis(instance, 0.35);
        instance.current[3][..3].copy_from_slice(&scene::MOTION_SUBJECT_POSITION);
        instance.previous[3][..3].copy_from_slice(&scene::MOTION_SUBJECT_POSITION);
    }
    if overlay != scene::RouteOverlay::Unmirrored {
        let mirrored = &mut scene.instances[scene::ROUTE_OVERLAY_MIRRORED_SLOT];
        for component in 0..3 {
            mirrored.current[0][component] = -mirrored.current[0][component];
            mirrored.previous[0][component] = -mirrored.previous[0][component];
            mirrored.normal[0][component] = -mirrored.normal[0][component];
        }
    }
    scene.local_lights[0] = ROUTE_OVERLAY_LIGHT;
    if matches!(
        overlay,
        scene::RouteOverlay::LampStop | scene::RouteOverlay::NightLights
    ) {
        scene.local_lights = scene::NIGHT_LAMPS
            .map(|(position, candela)| SyntheticPointLight {
                position,
                candela,
                radius: 2.5,
            })
            .to_vec();
    }
    if overlay == scene::RouteOverlay::NightLights {
        for (offset, (position, colour)) in scene::NIGHT_LAMPS.into_iter().enumerate() {
            let instance = &mut scene.instances[scene::NIGHT_LAMP_FIRST_SLOT + offset];
            scale_basis(instance, 0.025);
            instance.current[3][..3].copy_from_slice(&position);
            instance.previous[3][..3].copy_from_slice(&position);
            // The night exposure amplifies these dim physical sources. Keep
            // their visible radiance within the coloured portion of the shoulder.
            let [r, g, b] = colour.map(|channel| channel * 0.09);
            scene.materials.push(MaterialRecord {
                base_rgba: [0.0, 0.0, 0.0, 1.0],
                emissive_cutoff: [r, g, b, 0.0],
                ..Default::default()
            });
        }
    }
    if overlay == scene::RouteOverlay::LightOff {
        scene.local_lights[0].candela = [0.0; 3];
    }
}

#[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
fn g3t_overlay_control(scene: &mut SyntheticScene, control: &str) -> Result<(), String> {
    match control {
        "mirror-omit" => {}
        "overflow" | "weak-off" => {
            scene.local_lights = vec![ROUTE_OVERLAY_LIGHT; 34];
            scene.local_lights[31].candela = [1000.0; 3];
            scene.local_lights[32].candela = [250.0; 3];
            scene.local_lights[33].candela = [100.0; 3];
            if control == "weak-off" {
                scene.local_lights[31].candela = [0.0; 3];
            }
        }
        _ => return Err("unknown G3t overlay control".to_owned()),
    }
    Ok(())
}

fn native_witness_selection() -> thrum_render::synthetic::NativeWitness {
    use thrum_render::synthetic::NativeWitness;
    #[cfg(all(feature = "proving", not(target_arch = "wasm32")))]
    {
        match std::env::var("THRUM_NATIVE_WITNESS").as_deref() {
            Ok("metal") => NativeWitness::Metal,
            Ok("wgsl") => NativeWitness::Wgsl,
            Ok("rejected") => NativeWitness::Rejected,
            Ok("no-commands") => NativeWitness::NoCommands,
            Ok("no-tile") => NativeWitness::NoTile,
            Ok("no-reset") => NativeWitness::NoReset,
            Ok("stale-source") => NativeWitness::StaleSource,
            Ok("stale-history") => NativeWitness::StaleHistory,
            _ => NativeWitness::Off,
        }
    }
    #[cfg(any(not(feature = "proving"), target_arch = "wasm32"))]
    {
        NativeWitness::Off
    }
}

#[cfg(feature = "draw-probe")]
fn omit_mirror(overlay: scene::RouteOverlay) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("THRUM_G3T_OVERLAY_CONTROL").as_deref() == Ok("mirror-omit") {
        return true;
    }
    overlay == scene::RouteOverlay::MirrorOmit
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MediumSource {
    Ground,
    #[cfg(feature = "proving")]
    TestBank,
}

/// The game's scene: its content, camera, solar day and display choices, which
/// the engine's frame host samples for each frame.
pub struct GameScene {
    #[cfg(feature = "proving")]
    interface: Option<crate::interface_card::Card>,
    #[cfg(feature = "proving")]
    pub(super) font_card: Option<Box<crate::interface_text::FontCard>>,
    #[cfg(feature = "proving")]
    font_tick: std::cell::Cell<u64>,
    #[cfg(feature = "proving")]
    sprite_scale: Option<u16>,
    #[cfg(feature = "proving")]
    sprite_probe: Option<crate::sprite_probe::Scene>,
    #[cfg(feature = "proving")]
    sprite_rows: Option<u32>,
    #[cfg(feature = "proving")]
    pub(super) field_sheets: Option<crate::sprite_field::FieldSheets>,
    #[cfg(feature = "proving")]
    pub(super) sheet_stats: Option<std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>>,
    pub(super) cloud_input: Option<thrum_clouds::WeatherInput>,
    pub(super) weather_edits: Option<super::WeatherEdits>,
    pub(super) cloud_sky: Option<thrum_clouds::SkyProfile>,
    pub(super) cloud_selection: crate::CloudSelection,
    pub(super) cloud_ground: Option<thrum_clouds::GroundConfig>,
    pub(super) cloud_shapes: Option<crate::PlacedSky>,
    #[cfg(feature = "proving")]
    haze: Option<thrum_render::atmosphere::Aerosol>,
    medium: Option<MediumSource>,
    pub(super) cooked_rows: Option<thrum_asset_gpu::rows::CookedRows>,
    native_witness: thrum_render::synthetic::NativeWitness,
    content: GameContent,
    #[cfg(feature = "proving")]
    exit_entities: crate::ExitEntities,
    valley_stars: bool,
    pub(super) day_phase_offset: f32,
    ground_solar: std::cell::Cell<Option<crate::simulation::SolarTime>>,
    ground_aerosol: std::cell::Cell<Option<thrum_render::atmosphere::Aerosol>>,
    #[cfg(all(any(target_os = "macos", target_arch = "wasm32"), feature = "proving"))]
    pub(crate) solar_sweep: Option<SolarSweep>,
    #[cfg(all(target_os = "macos", feature = "proving"))]
    pub(crate) chart_jump: Option<(u64, f32)>,
    #[cfg(feature = "proving")]
    pub(super) capture_solar_tick: Option<u64>,
    pub(super) camera_pitch: f32,
    pub(super) camera_yaw: f32,
    pub(super) display_overlay: DisplayOverlay,
    #[cfg(feature = "proving")]
    temporal_inputs: bool,
    #[cfg(feature = "creation-counters")]
    pub(super) forced_storage_scale: Option<f32>,
    reconstruction: GameReconstruction,
    #[cfg(feature = "proving")]
    temporal_pan: (u64, f32),
    #[cfg(feature = "proving")]
    temporal_star_edit: u64,
    camera: Option<SceneCamera>,
    route_overlay: scene::RouteOverlay,
    route: Option<CameraRoute>,
    #[cfg(feature = "draw-probe")]
    omit_mirror: bool,
}
#[cfg(all(any(target_os = "macos", target_arch = "wasm32"), feature = "proving"))]
pub(crate) type SolarSweep = ([u64; 2], [f32; 2]);

impl GameScene {
    pub(super) fn ground_time_of_day(&self, alpha: f32) -> Option<f32> {
        if self.held_solar_tick().is_some() || self.route.is_some() {
            return None;
        }
        let solar = self.ground_solar.get()?;
        if let Some(time) = self
            .weather_edits
            .as_ref()
            .and_then(|edits| edits.pending().solar)
        {
            return Some(time);
        }
        Some(solar.sample(alpha))
    }

    fn sample_sun(&self, tick: u64, alpha: f32) -> crate::environment::SunSample {
        let offset = self.day_phase_offset;
        #[cfg(all(any(target_os = "macos", target_arch = "wasm32"), feature = "proving"))]
        let offset = self
            .solar_sweep
            .map_or(offset, |([first, last], [from, to])| {
                #[cfg(target_os = "macos")]
                if self.chart_jump.is_some() {
                    if tick < first {
                        return offset;
                    }
                    if tick > last {
                        return DayCycle::offset_for_time(last, to / 24.0).unwrap_or(offset);
                    }
                }
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a proof sweep fits within the route's bounded tick range"
                )]
                let progress = (tick.saturating_sub(first).min(last - first) as f32
                    + if tick < first { 0.0 } else { alpha })
                    / (last - first) as f32;
                let solar = from + (to - from) * progress.clamp(0.0, 1.0);
                DayCycle::offset_for_time(tick, solar / 24.0).unwrap_or(offset)
            });
        #[cfg(all(target_os = "macos", feature = "proving"))]
        let offset = self
            .chart_jump
            .filter(|(jump, _)| tick >= *jump)
            .and_then(|(_, hour)| DayCycle::offset_for_time(tick, hour / 24.0))
            .unwrap_or(offset);
        self.ground_time_of_day(alpha).map_or_else(
            || DayCycle::sample(tick, alpha, offset),
            |time| crate::clouds::ground::sun(time, tick, alpha),
        )
    }

    #[cfg(all(target_arch = "wasm32", feature = "proving"))]
    pub(super) fn set_sprite_sweep_step(&mut self, step: u32) -> Result<(), String> {
        self.sprite_probe
            .as_mut()
            .ok_or("sprite camera sweep requires a prepared probe")?
            .set_sweep_step(step)
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn is_exit(&self) -> bool {
        self.content == GameContent::Exit
    }
    /// Select a checked authored camera and its orientation.
    /// # Errors
    /// Refuses while a moving route is installed.
    pub(super) fn select_camera(&mut self, camera: SceneCamera) -> Result<(), String> {
        self.route_guard()?;
        [self.camera_pitch, self.camera_yaw] = camera.orientation();
        self.camera = Some(camera);
        Ok(())
    }

    /// The authored scene camera, when content selected one.
    pub(super) fn authored_camera(&self) -> Option<SceneCamera> {
        self.camera
    }

    #[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
    pub(super) fn move_capture_eye(&mut self, eye: [f32; 3]) -> Result<(), String> {
        self.route_guard()?;
        let camera = self
            .camera
            .ok_or("haze capture needs the cooked valley camera")?;
        self.camera = Some(camera.at(eye)?);
        Ok(())
    }

    /// The installed route's pose for `tick` at `alpha`.
    pub(super) fn route_pose(&self, tick: u64, alpha: f32) -> Option<crate::RoutePose> {
        self.route.as_ref().map(|route| route.pose(tick, alpha))
    }

    /// Refuse camera, time and control edits while a moving route owns the camera.
    pub(super) fn route_guard(&self) -> Result<(), String> {
        match self.route {
            Some(_) => Err(
                "a moving route is installed; camera, time and control edits are refused"
                    .to_owned(),
            ),
            None => Ok(()),
        }
    }

    /// Install `route` when the committed tick is the one before its origin.
    /// It sets the day phase so the origin shows the route's solar hour, and the
    /// caller resets history. Returns the day-phase offset applied.
    /// # Errors
    /// Refuses a second route, no authored camera, an overlay other than the one this
    /// game was prepared with, the wrong tick, or a key 0 that is not the current pose.
    pub(super) fn install_route(&mut self, route: CameraRoute, tick: u64) -> Result<(), String> {
        self.route_guard()?;
        let camera = self
            .camera
            .ok_or("a moving route needs the authored valley camera")?;
        if route.overlay != self.route_overlay {
            return Err("the route's overlay differs from the game's setup".to_owned());
        }
        if route.origin_tick == 0 || tick != route.origin_tick - 1 {
            return Err(format!(
                "install the route at committed tick {}, not {tick}",
                route.origin_tick.saturating_sub(1)
            ));
        }
        route.check_start(camera, self.camera_pitch, self.camera_yaw)?;
        self.day_phase_offset =
            DayCycle::offset_for_time(route.origin_tick, route.solar_hours / 24.0)
                .ok_or("route solar hour is outside the day")?;
        self.route = Some(route);
        Ok(())
    }

    pub(super) fn held_solar_tick(&self) -> Option<u64> {
        #[cfg(feature = "proving")]
        if let Some(tick) = self.capture_solar_tick {
            return Some(tick);
        }
        self.route
            .as_ref()
            .filter(|route| route.hold_solar)
            .map(|route| route.origin_tick)
    }

    /// Residency observations for the optional cooked sprite field.
    #[cfg(feature = "proving")]
    pub fn sprite_sheet_stats(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>> {
        self.sheet_stats.clone()
    }

    #[cfg(all(
        feature = "proving",
        any(target_os = "macos", target_os = "ios", target_arch = "wasm32")
    ))]
    pub(crate) fn set_menu(&mut self, menu: crate::interface_menu::MenuView) {
        if let Some(card) = self.font_card.as_mut() {
            card.set_menu(menu);
        }
    }

    #[cfg(all(
        feature = "proving",
        any(target_os = "macos", target_os = "ios", target_arch = "wasm32")
    ))]
    pub(crate) fn submitted_menu(&self) -> Option<crate::interface_menu::MenuFrame> {
        self.font_card.as_ref()?.submitted_menu()
    }

    #[cfg(all(
        feature = "proving",
        any(target_os = "macos", target_os = "ios", target_arch = "wasm32")
    ))]
    pub(crate) fn set_interface_output(&mut self, density: f64, insets: [f64; 4]) {
        if let Some(card) = &mut self.font_card {
            card.output(density, insets);
        }
    }

    /// Measure layout and paint builds with a monotonic millisecond clock.
    /// # Errors
    /// Returns a refused workload declaration.
    #[cfg(feature = "proving")]
    pub fn status_layout_costs(
        now_ms: impl Fn() -> f64,
    ) -> Result<Vec<[f64; 3]>, thrum_ui::TreeError> {
        crate::interface_status::measure_layout(now_ms)
    }

    #[cfg(all(feature = "proving", target_os = "ios"))]
    pub(crate) fn status_bounds(&self) -> Option<serde_json::Value> {
        self.font_card.as_ref()?.status_bounds()
    }

    /// Counts of actual retained status layout and paint work.
    #[cfg(feature = "proving")]
    pub fn status_stats(&self) -> Option<thrum_ui::TreeStats> {
        self.font_card.as_ref()?.status_stats()
    }

    /// Drive each status value at the simulation tick rate for the cost run.
    #[cfg(feature = "proving")]
    pub fn measure_status_updates(&mut self) {
        if let Some(card) = &mut self.font_card {
            card.measure_updates();
        }
    }

    #[cfg(all(feature = "proving", target_os = "macos"))]
    pub(crate) fn hold_status_capture(&mut self, status: crate::StatusCapture) {
        if let Some(card) = &mut self.font_card {
            card.hold_status_capture(status);
        }
    }

    /// Request a different cooked interface scale.
    #[cfg(feature = "proving")]
    pub fn set_interface_scale(&self, scale: u32) -> bool {
        self.font_card
            .as_ref()
            .is_some_and(|card| card.set_scale(scale))
    }

    #[cfg(feature = "proving")]
    pub(super) fn uses_test_bank(&self) -> bool {
        self.medium == Some(MediumSource::TestBank)
    }

    pub(super) fn new(setup: GameSetup) -> Self {
        #[cfg(feature = "proving")]
        let temporal: Option<serde_json::Value> =
            (setup.display_overlay == DisplayOverlay::TemporalInputs).then(|| {
                serde_json::from_str(thrum_render::synthetic::TEMPORAL_WITNESS_JSON)
                    .expect("checked-in temporal fixture")
            });
        Self {
            #[cfg(feature = "proving")]
            font_card: None,
            #[cfg(feature = "proving")]
            font_tick: std::cell::Cell::new(0),
            #[cfg(feature = "proving")]
            interface: setup.interface_scale.map(|scale| {
                crate::interface_card::Card::new(
                    scale,
                    setup.interface_rows,
                    setup.sprite_scale.is_some(),
                )
            }),
            #[cfg(feature = "proving")]
            sprite_scale: setup.sprite_scale,
            #[cfg(feature = "proving")]
            sprite_probe: setup.sprite_probe.map(crate::sprite_probe::Scene::new),
            #[cfg(feature = "proving")]
            sprite_rows: setup.sprite_rows,
            #[cfg(feature = "proving")]
            field_sheets: None,
            #[cfg(feature = "proving")]
            sheet_stats: None,
            cloud_input: None,
            weather_edits: None,
            cloud_sky: None,
            cloud_shapes: None,
            cloud_selection: setup.clouds,
            cloud_ground: None,
            #[cfg(feature = "proving")]
            haze: setup.haze,
            medium: if matches!(setup.clouds, crate::CloudSelection::Ground(_)) {
                Some(MediumSource::Ground)
            } else {
                #[cfg(feature = "proving")]
                {
                    setup.test_bank.then_some(MediumSource::TestBank)
                }
                #[cfg(not(feature = "proving"))]
                None
            },
            cooked_rows: None,
            content: setup.content,
            #[cfg(feature = "proving")]
            exit_entities: setup.exit_entities,
            valley_stars: !setup.content.is_field() && setup.valley_stars,
            day_phase_offset: 0.0,
            ground_solar: std::cell::Cell::new(
                matches!(setup.clouds, crate::CloudSelection::Ground(_)).then(Default::default),
            ),
            ground_aerosol: std::cell::Cell::new(match setup.clouds {
                crate::CloudSelection::Ground(preset) => {
                    let targets = preset.targets();
                    Some(aerosol(targets.visual_range, targets.angstrom_exponent))
                }
                _ => None,
            }),
            #[cfg(all(any(target_os = "macos", target_arch = "wasm32"), feature = "proving"))]
            solar_sweep: None,
            #[cfg(all(target_os = "macos", feature = "proving"))]
            chart_jump: None,
            #[cfg(feature = "proving")]
            capture_solar_tick: None,
            camera_pitch: 0.0,
            camera_yaw: 0.0,
            #[cfg(feature = "proving")]
            display_overlay: setup.display_overlay,
            #[cfg(not(feature = "proving"))]
            display_overlay: DisplayOverlay::Off,
            #[cfg(feature = "proving")]
            temporal_inputs: temporal.is_some(),
            native_witness: native_witness_selection(),
            reconstruction: setup.reconstruction,
            #[cfg(feature = "creation-counters")]
            forced_storage_scale: None,
            #[cfg(feature = "proving")]
            temporal_star_edit: temporal.as_ref().map_or(u64::MAX, |f| {
                f["star_edit_tick"].as_u64().expect("fixture star tick")
            }),
            #[cfg(feature = "proving")]
            temporal_pan: temporal.as_ref().map_or((u64::MAX, 0.0), |f| {
                (
                    f["pan_tick"].as_u64().expect("fixture pan tick"),
                    serde_json::from_value(f["pan_yaw"].clone()).expect("fixture pan yaw"),
                )
            }),
            camera: None,
            route_overlay: setup.route_overlay(),
            route: None,
            #[cfg(feature = "draw-probe")]
            omit_mirror: omit_mirror(setup.route_overlay),
        }
    }
    pub(super) fn scene(extent: [u32; 2], setup: GameSetup) -> Result<SyntheticScene, FrameError> {
        let mut scene = match setup.content {
            GameContent::Grid => {
                let mut scene = Self::grid_scene(extent)?;
                if setup.valley_stars {
                    scene.star_catalogue = Some(&star_catalogue::HARVARD);
                }
                if setup.field_meshes {
                    scene.meshes = Some(crate::field_meshes::selection_for(
                        scene::ENTITY_COUNT,
                        false,
                    ));
                }
                scene
            }
            #[cfg(feature = "proving")]
            GameContent::Field => field_scene(extent, setup.field_meshes)?,
            GameContent::Exit => {
                let mut scene = crate::exit_scene::scene()?;
                if setup.valley_stars {
                    scene.star_catalogue = Some(&star_catalogue::HARVARD);
                }
                scene
            }
        };
        if setup.cooked_mesh_rows == super::MeshRowSource::Cooked
            && let Some(selection) = scene.meshes.as_mut()
        {
            let selection = std::sync::Arc::make_mut(selection);
            selection.updates = Some(thrum_render::synthetic::mesh_rows::MeshRows::new(
                selection.meshes.clone(),
            ));
        }
        scene.view = scene
            .view
            .with_native_witness(native_witness_selection())
            .with_reconstruction(setup.reconstruction != GameReconstruction::Off);
        if setup.reconstruction == GameReconstruction::MetalFx {
            scene.view = scene.view.with_metalfx();
        }
        #[cfg(feature = "proving")]
        {
            scene.view = scene
                .view
                .with_reconstruction_reference(
                    setup.reconstruction == GameReconstruction::Reference,
                )
                .with_display_overlay(setup.display_overlay)
                .with_temporal_inputs(setup.display_overlay == DisplayOverlay::TemporalInputs);
            scene.ray_structure = setup.display_overlay == DisplayOverlay::TracedGeometry;
        }
        scene.sun_shadow_method = setup.sun_shadow_method;
        scene.sun_penumbra = setup.sun_penumbra;
        scene.lamp_shadows = setup.lamp_shadows;
        scene.ambient_occlusion_radius = setup.ambient_occlusion_radius;
        scene.camera_culling = setup.camera_culling;
        #[cfg(feature = "proving")]
        if setup.test_bank {
            if setup.clouds != crate::CloudSelection::Off || setup.diagnostic_clouds {
                return Err(FrameError::InvalidState(
                    "the test bank replaces other Sun-path providers",
                ));
            }
            scene.sun_optical_depth = Some(test_bank::sun_path()?);
        }
        scene.sky_overlay = setup.clouds.world_light();
        if setup.clouds != crate::CloudSelection::Off {
            scene.sun_optical_depth = Some(thrum_render::atmosphere::SunOpticalDepth::gpu_target(
                [crate::clouds::GROUND_SIZE; 2],
                1,
            )?);
        }
        #[cfg(feature = "proving")]
        if setup.clouds == crate::CloudSelection::Off && setup.diagnostic_clouds {
            scene.sun_optical_depth = Some(if setup.haze.is_some() {
                cloud::haze_banks()?
            } else {
                cloud::diagnostic()?
            });
        }
        #[cfg(feature = "proving")]
        if setup.route_overlay != scene::RouteOverlay::Off {
            if setup.content != GameContent::Grid {
                return Err(FrameError::InvalidState(
                    "the route overlay needs grid content",
                ));
            }
            apply_route_overlay(&mut scene, setup.route_overlay);
            if setup.route_scale_props {
                apply_render_scale_props(&mut scene);
            }
            #[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
            if let Ok(control) = std::env::var("THRUM_G3T_OVERLAY_CONTROL") {
                g3t_overlay_control(&mut scene, &control).map_err(FrameError::Scene)?;
            }
        }
        Ok(scene)
    }

    fn grid_scene(extent: [u32; 2]) -> Result<SyntheticScene, FrameError> {
        let count = u32::try_from(scene::ENTITY_COUNT)
            .map_err(|_| FrameError::InvalidState("fixture exceeds u32 slots"))?;
        let mut scene = SyntheticScene::grid(count)?;
        scene.materials[0].base_rgba = [0.18, 0.18, 0.18, 1.0];
        scene.materials[0].surface = [1.0, 1.0, 0.0, 1.0];
        scene.materials.push(MaterialRecord {
            base_rgba: [0.65, 0.65, 0.65, 1.0],
            surface: [1.0, 0.32, 1.0, 1.0],
            ..Default::default()
        });
        // Static authored subjects are outside the moving prefix and four
        // controlled slots. Their rotated faces expose lighting direction.
        // A 0.18 halfextent also stays outside the coloured local-light spheres.
        for slot in [scene::NEUTRAL_SUBJECT_SLOT, scene::REFLECTION_SUBJECT_SLOT] {
            scale_basis(&mut scene.instances[slot], 0.18);
        }
        // This cold geometry matches the authoritative spawn. Publication owns
        // position and material after extraction; no GPU identity override.
        let reflection = &mut scene.instances[scene::REFLECTION_SUBJECT_SLOT];
        reflection.current[3][..3].copy_from_slice(&scene::REFLECTION_SUBJECT_POSITION);
        reflection.previous[3][..3].copy_from_slice(&scene::REFLECTION_SUBJECT_POSITION);
        scene.view = frame_view(0, 0.0, extent, 0.0, 0.0, None)?;
        scene.sun_shadows = true;
        scene.local_lights = [
            ([-0.9, -0.9, 1.0], [5.0, 0.8, 0.4]),
            ([0.9, -0.9, 1.0], [0.4, 4.0, 0.8]),
            ([-0.9, 0.9, 1.0], [0.7, 1.0, 5.0]),
            ([0.9, 0.9, 1.0], [4.0, 3.0, 0.4]),
        ]
        .map(|(position, candela)| SyntheticPointLight {
            position,
            radius: 0.6,
            candela,
        })
        .to_vec();
        Ok(scene)
    }
}

impl SceneSource<PublishedPage> for GameScene {
    #[cfg(feature = "proving")]
    fn interface_config(&self) -> Option<thrum_ui::InterfaceConfig> {
        if let Some(card) = &self.font_card {
            return Some(card.config());
        }
        self.interface
            .as_ref()
            .map(crate::interface_card::Card::config)
    }

    #[cfg(feature = "proving")]
    fn paint_lists(&mut self, extent: [u32; 2]) -> [Option<thrum_ui::PaintList>; 2] {
        if let Some(card) = &mut self.font_card {
            return card.paint(extent, self.font_tick.get());
        }
        [self.interface.as_mut().map(|card| card.paint(extent)), None]
    }

    #[cfg(feature = "proving")]
    fn extract_sprites(
        &self,
        into: &mut thrum_render::sprite::SpriteSink<'_>,
        page: &PublishedPage,
    ) {
        if let Some(probe) = &self.sprite_probe {
            probe.extract(
                into,
                SourceTick {
                    epoch: page.id().epoch.0,
                    timeline: page.timeline(),
                    tick: page.id().tick,
                },
            );
        } else if let Some(count) = self.sprite_rows {
            let player = self
                .font_card
                .as_ref()
                .is_some_and(|card| card.interactive())
                .then_some(count - 1);
            crate::sprite_field::extract(into, page, self.field_sheets.as_ref(), player);
        }
    }

    #[cfg(feature = "proving")]
    fn sprites(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
    ) -> Result<thrum_render::sprite::SpriteFrame<'_>, FrameError> {
        if self.font_card.is_some() {
            self.font_tick.set(source.tick);
        }
        let scale = self.sprite_scale.ok_or(RenderError::MissingSpriteSource)?;
        Ok(if let Some(probe) = &self.sprite_probe {
            probe.frame(source, alpha, extent)
        } else if self.sprite_rows.is_some() {
            #[cfg(target_os = "ios")]
            {
                crate::sprite_card::mobile_field(extent)
            }
            #[cfg(not(target_os = "ios"))]
            {
                crate::sprite_field::frame(extent, self.field_sheets.map(|_| (source.tick, alpha)))
            }
        } else {
            crate::sprite_card::frame(scale, extent)
        })
    }

    #[cfg(feature = "proving")]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded sinusoidal fixture position is converted from game-clock f64 to renderer f32"
    )]
    fn sample_local_lights(
        &self,
        source: SourceTick,
        alpha: f32,
        lights: &mut [SyntheticPointLight],
    ) {
        if self.route_overlay == scene::RouteOverlay::Reconstruction {
            let time =
                DayCycle::sample(source.tick, alpha, self.day_phase_offset).simulation_time_seconds;
            if let Some(light) = lights.first_mut() {
                light.position[0] = (time * 1.5).sin() as f32 * 3.0;
            }
        }
    }
    fn extract(&self, into: &mut RenderExtractor, page: &PublishedPage) -> ExtractReport {
        if let Some(input) = &self.cloud_input
            && let Some(state) = page.weather()
        {
            self.ground_solar.set(page.weather_solar());
            if self.ground_solar.get().is_some() {
                let ground = state.ground();
                self.ground_aerosol.set(Some(aerosol(
                    ground.visual_range(),
                    ground.angstrom_exponent(),
                )));
            }
            if let Err(error) = input.publish(state) {
                debug_assert!(false, "cloud publication: {error}");
            }
        }
        #[cfg(feature = "proving")]
        if self.content == GameContent::Exit && self.exit_entities == crate::ExitEntities::Omit {
            return into.extract(
                SourceTick {
                    epoch: page.id().epoch.0,
                    timeline: page.timeline(),
                    tick: page.id().tick,
                },
                [0.0; 3],
                |_| {},
            );
        }
        #[cfg(feature = "draw-probe")]
        if self.omit_mirror {
            return scene::extract_without_mirror(into, page);
        }
        scene::extract(into, page)
    }
    fn frame(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        let yaw = self.camera_yaw;
        #[cfg(feature = "proving")]
        let yaw = if self.temporal_inputs && source.tick >= self.temporal_pan.0 {
            self.temporal_pan.1
        } else {
            yaw
        };
        let field = self.content.is_field() && self.camera.is_none();
        let routed = self.route.as_ref().zip(self.camera);
        let view = match routed {
            Some((route, camera)) => {
                let pose = route.pose(source.tick, alpha);
                camera
                    .at(pose.eye)
                    .map_err(FrameError::Scene)?
                    .view(extent, pose.pitch, pose.yaw)?
                    .with_sdr_tone_map(SdrToneMap::NeutralHighlights)
                    .with_history_epoch(pose.epoch)
            }
            #[cfg(feature = "proving")]
            None if field => field_view(source.tick, alpha, extent, self.camera_pitch, yaw)?,
            None => frame_view(
                source.tick,
                alpha,
                extent,
                self.camera_pitch,
                yaw,
                self.camera,
            )?,
        }
        .with_native_witness(self.native_witness)
        .with_display_overlay(self.display_overlay)
        .with_jitter_sequence(0, [640, 360])
        .with_reconstruction(self.reconstruction != GameReconstruction::Off);
        #[cfg(all(target_os = "macos", feature = "proving"))]
        let view = if self.chart_jump.is_some()
            && self
                .solar_sweep
                .is_some_and(|([first, last], _)| (first..=last).contains(&source.tick))
        {
            view.with_phase_history_reset()
        } else {
            view
        };
        let view = if self.reconstruction == GameReconstruction::MetalFx {
            view.with_metalfx()
        } else {
            view
        };
        #[cfg(feature = "proving")]
        let view = view
            .with_reconstruction_reference(self.reconstruction == GameReconstruction::Reference)
            .with_temporal_inputs(self.temporal_inputs);
        #[cfg(feature = "creation-counters")]
        let view = view.with_storage_scale_probe(self.forced_storage_scale)?;
        // The diagnostic fixture's local Y zero is a two-metre reference datum.
        // Authored scene and field coordinates already measure altitude above sea level.
        let camera_altitude_m = view.camera_position()[1]
            + if self.camera.is_some() || field {
                0.0
            } else {
                2.0
            };
        let sun = self.sample_sun(source.tick, alpha);
        let sun = if let Some(tick) = self.held_solar_tick() {
            crate::environment::SunSample {
                simulation_time_seconds: sun.simulation_time_seconds,
                ..DayCycle::sample(tick, 1.0, self.day_phase_offset)
            }
        } else {
            sun
        };
        let haze = self.ground_aerosol.get();
        #[cfg(feature = "proving")]
        let haze = self.haze.or(haze);
        let lighting = lighting
            .unwrap_or_else(|| atmosphere_lighting(sun, camera_altitude_m, haze, self.medium));
        let view = if self.valley_stars
            && self.camera.is_some()
            && matches!(lighting, FrameLighting::Atmosphere(_))
        {
            #[cfg(feature = "proving")]
            let view = view.with_celestial_epoch(u64::from(
                self.temporal_inputs && source.tick >= self.temporal_star_edit,
            ));
            view.with_celestial_basis(sun.equatorial_to_world)?
        } else {
            view
        };
        let view = match lighting {
            FrameLighting::Atmosphere(_) => view
                .with_incident_exposure(0.0, -12.0, 18.0)
                .and_then(SyntheticView::with_low_light_adaptation),
            FrameLighting::Diagnostic(_) | FrameLighting::Photometric(_) => {
                view.with_exposure(1.0 / 16_000.0)
            }
        }?;
        Ok(SceneFrame { view, lighting })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GameReconstruction, GameScene, RenderExtractor, SceneSource, SourceTick, frame_view, scene,
    };
    use crate::{camera::SceneCamera, environment::DayCycle};
    use thrum_core::StableId;
    use thrum_render::synthetic::{DisplayOverlay, FrameLighting, SdrToneMap, SyntheticView};

    #[test]
    #[cfg(feature = "proving")]
    fn reconstruction_light_follows_the_sampled_game_endpoint_and_holds_when_paused() {
        let policy = GameScene::new(crate::GameSetup {
            route_overlay: scene::RouteOverlay::Reconstruction,
            ..crate::GameSetup::default()
        });
        let mut lights = [super::ROUTE_OVERLAY_LIGHT];
        let source = |tick| SourceTick {
            epoch: 1,
            timeline: 0,
            tick,
        };
        policy.sample_local_lights(source(0), 1.0, &mut lights);
        let initial = lights[0].position;
        policy.sample_local_lights(source(61), 0.5, &mut lights);
        let moved = lights[0].position;
        assert!(moved[0] - initial[0] > 2.0);
        policy.sample_local_lights(source(61), 0.5, &mut lights);
        assert_eq!(lights[0].position, moved);
    }

    #[test]
    fn installed_route_moves_the_authored_camera_and_refuses_edits() {
        let camera = SceneCamera::perspective(
            [0.0, 244.92, 16.0],
            [0.0, 240.92, -60.0],
            46_f32.to_radians(),
            0.1,
            30_000.0,
        )
        .unwrap();
        let [pitch, yaw] = camera.orientation();
        let route_text = |overlay: &str| {
            serde_json::json!({
                "format": "thrum-v1-moving-route", "version": 1, "cooked_root": "root",
                "cooked_generation": 1, "tick_hz": 60, "phase": [1, 2], "origin_tick": 50,
                "start": {"solar_hours": 17.3}, "render_extent": [640, 360],
                "keys": [
                    {"tick": 0, "eye": [0.0, 244.92, 16.0], "yaw_degrees": yaw.to_degrees(), "pitch_degrees": pitch.to_degrees()},
                    {"tick": 10, "eye": [2.0, 244.92, 6.0], "yaw_degrees": 20.0, "pitch_degrees": -10.0},
                    {"tick": 10, "eye": [4.0, 244.92, 0.0], "yaw_degrees": 30.0, "pitch_degrees": -10.0},
                    {"tick": 20, "eye": [4.0, 244.92, -4.0], "yaw_degrees": 30.0, "pitch_degrees": -10.0},
                ],
                "cuts": [10], "checkpoints": [{"tick": 5, "name": "a"}], "overlay": overlay,
            })
            .to_string()
        };
        let parse = |overlay| crate::CameraRoute::parse(&route_text(overlay), camera, 60).unwrap();
        let mut policy = GameScene::new(crate::GameSetup::default());
        assert!(
            policy.install_route(parse("off"), 49).is_err(),
            "needs a camera"
        );
        policy.select_camera(camera).unwrap();
        #[cfg(feature = "proving")]
        assert!(
            policy.install_route(parse("proof"), 49).is_err(),
            "overlay mismatch"
        );
        assert!(
            policy.install_route(parse("off"), 48).is_err(),
            "wrong tick"
        );
        policy.install_route(parse("off"), 49).unwrap();
        assert!(
            policy.install_route(parse("off"), 49).is_err(),
            "second route"
        );
        assert!(policy.select_camera(camera).is_err());
        assert!(policy.route_guard().is_err());
        let solar = DayCycle::time_of_day(50, policy.day_phase_offset) * 24.0;
        assert!((solar - 17.3).abs() < 1e-4, "{solar}");
        let view = |tick, alpha| {
            policy
                .frame(
                    SourceTick {
                        epoch: 1,
                        tick,
                        timeline: 0,
                    },
                    alpha,
                    [640, 360],
                    None,
                )
                .unwrap()
                .view
        };
        assert_eq!(view(50, 0.5).camera_position(), [0.0, 244.92, 16.0]);
        let moved = view(55, 0.5).camera_position();
        assert!((moved[0] - 0.9).abs() < 1e-4, "{moved:?}");
        assert!((view(62, 0.5).camera_position()[0] - 4.0).abs() < 1e-4);
    }

    #[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
    #[test]
    fn g3t_overflow_control_removes_only_the_selected_weak_source() {
        let setup = crate::GameSetup {
            route_overlay: scene::RouteOverlay::Proof,
            ..crate::GameSetup::default()
        };
        let mut positive = GameScene::scene([640, 360], setup).unwrap();
        let mut negative = GameScene::scene([640, 360], setup).unwrap();
        super::g3t_overlay_control(&mut positive, "overflow").unwrap();
        super::g3t_overlay_control(&mut negative, "weak-off").unwrap();
        assert_eq!(positive.local_lights.len(), 34);
        assert_eq!(negative.local_lights.len(), 34);
        for (index, (a, b)) in positive
            .local_lights
            .iter()
            .zip(&negative.local_lights)
            .enumerate()
        {
            assert_eq!(a.position, b.position);
            assert_eq!(a.radius, b.radius);
            if index == 31 {
                assert_eq!(a.candela, [1000.0; 3]);
                assert_eq!(b.candela, [0.0; 3]);
            } else {
                assert_eq!(a.candela, b.candela);
            }
        }
    }

    #[test]
    #[cfg(feature = "proving")]
    fn route_overlay_changes_only_its_bases_and_light_zero() {
        let off = GameScene::scene([640, 360], crate::GameSetup::default()).unwrap();
        let on = GameScene::scene(
            [640, 360],
            crate::GameSetup {
                route_overlay: scene::RouteOverlay::Proof,
                ..crate::GameSetup::default()
            },
        )
        .unwrap();
        let overlay: Vec<usize> = scene::ROUTE_OVERLAY.iter().map(|(slot, _)| *slot).collect();
        for (slot, (a, b)) in off.instances.iter().zip(&on.instances).enumerate() {
            if !overlay.contains(&slot) {
                assert_eq!(format!("{a:?}"), format!("{b:?}"), "slot {slot}");
            }
        }
        let determinant = |m: [[f32; 4]; 4]| {
            m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[1][0] * (m[0][1] * m[2][2] - m[0][2] * m[2][1])
                + m[2][0] * (m[0][1] * m[1][2] - m[0][2] * m[1][1])
        };
        for (slot, position) in scene::ROUTE_OVERLAY {
            let instance = &on.instances[slot];
            assert_eq!(instance.current[3][..3], position);
            let mirrored = slot == scene::ROUTE_OVERLAY_MIRRORED_SLOT;
            assert_eq!(determinant(instance.current) < 0.0, mirrored, "slot {slot}");
        }
        assert_eq!(
            format!("{:?}", &off.local_lights[1..]),
            format!("{:?}", &on.local_lights[1..])
        );
        assert_eq!(
            format!("{:?}", on.local_lights[0]),
            format!("{:?}", super::ROUTE_OVERLAY_LIGHT)
        );
    }

    #[test]
    fn rejected_camera_keeps_the_renderer_cause() {
        use std::error::Error;
        use thrum_render::{error::RenderError, runtime::FrameError};
        let source = SourceTick {
            epoch: 1,
            tick: 0,
            timeline: 0,
        };
        for content in [
            scene::GameContent::Grid,
            #[cfg(feature = "proving")]
            scene::GameContent::Field,
        ] {
            let policy = GameScene::new(crate::GameSetup {
                content,
                ..Default::default()
            });
            let Err(error) = policy.frame(source, 0.0, [0, 480], None) else {
                panic!("zero-width camera accepted");
            };
            assert!(matches!(
                error,
                FrameError::Render(RenderError::InvalidView)
            ));
            assert!(matches!(
                error
                    .source()
                    .and_then(|cause| cause.downcast_ref::<RenderError>()),
                Some(RenderError::InvalidView)
            ));
        }
    }

    #[test]
    fn diagnostic_camera_keeps_its_original_pose_and_bob() {
        let extent = [640, 360];
        let actual = frame_view(0, 0.0, extent, 0.0, 0.0, None).unwrap();
        let expected = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            65.0_f32.to_radians(),
            640.0 / 360.0,
            0.1,
            20.0,
        )
        .unwrap()
        .with_sdr_tone_map(SdrToneMap::NeutralHighlights);
        assert_eq!(actual, expected);
        let bobbed = frame_view(91, 1.0, extent, 0.0, 0.0, None).unwrap();
        assert_ne!(actual.camera_position(), bobbed.camera_position());
        assert!((1.92..=2.0).contains(&(2.0 + bobbed.camera_position()[1])));
    }

    #[test]
    fn authored_camera_is_fixed_across_ticks_and_preserves_the_world_sun() {
        // The supported atmosphere ceiling exercises the same view's altitude:
        // applying the legacy two-metre datum would exceed the profile bounds.
        let camera = SceneCamera::perspective(
            [0.0, 100_000.0, 16.0],
            [0.0, 100_000.0, -60.0],
            46.0_f32.to_radians(),
            0.1,
            30_000.0,
        )
        .unwrap();
        let [camera_pitch, camera_yaw] = camera.orientation();
        let policy = GameScene {
            #[cfg(feature = "proving")]
            font_card: None,
            #[cfg(feature = "proving")]
            font_tick: std::cell::Cell::new(0),
            #[cfg(feature = "proving")]
            interface: None,
            #[cfg(feature = "proving")]
            haze: None,
            medium: None,
            #[cfg(feature = "proving")]
            sprite_scale: None,
            #[cfg(feature = "proving")]
            sprite_probe: None,
            #[cfg(feature = "proving")]
            sprite_rows: None,
            #[cfg(feature = "proving")]
            field_sheets: None,
            #[cfg(feature = "proving")]
            sheet_stats: None,
            cloud_input: None,
            weather_edits: None,
            cloud_sky: None,
            cloud_shapes: None,
            cloud_selection: crate::CloudSelection::Off,
            cloud_ground: None,
            cooked_rows: None,
            native_witness: thrum_render::synthetic::NativeWitness::Off,
            content: scene::GameContent::Grid,
            #[cfg(feature = "proving")]
            exit_entities: crate::ExitEntities::Draw,
            valley_stars: true,
            day_phase_offset: 0.1,
            ground_solar: std::cell::Cell::default(),
            ground_aerosol: std::cell::Cell::default(),
            #[cfg(all(any(target_os = "macos", target_arch = "wasm32"), feature = "proving"))]
            solar_sweep: None,
            #[cfg(all(target_os = "macos", feature = "proving"))]
            chart_jump: None,
            #[cfg(feature = "proving")]
            capture_solar_tick: None,
            camera_pitch,
            camera_yaw,
            display_overlay: DisplayOverlay::Off,
            #[cfg(feature = "proving")]
            temporal_inputs: false,
            reconstruction: GameReconstruction::Off,
            #[cfg(feature = "creation-counters")]
            forced_storage_scale: None,
            #[cfg(feature = "proving")]
            temporal_pan: (30, -0.05),
            #[cfg(feature = "proving")]
            temporal_star_edit: 15,
            camera: Some(camera),
            route_overlay: scene::RouteOverlay::Off,
            route: None,
            #[cfg(feature = "draw-probe")]
            omit_mirror: false,
        };
        let first = policy
            .frame(
                SourceTick {
                    epoch: 1,
                    tick: 1,
                    timeline: 0,
                },
                0.5,
                [640, 360],
                None,
            )
            .unwrap();
        for (tick, extent) in [(91, [640, 360]), (181, [360, 640])] {
            let frame = policy
                .frame(
                    SourceTick {
                        epoch: 1,
                        tick,
                        timeline: 0,
                    },
                    0.5,
                    extent,
                    None,
                )
                .unwrap();
            assert_eq!(first.view.camera_position(), frame.view.camera_position());
            let FrameLighting::Atmosphere(atmosphere) = frame.lighting else {
                panic!("authored camera retains atmosphere lighting");
            };
            let expected = DayCycle::sample(tick, 0.5, 0.1).toward_sun;
            assert!(
                atmosphere
                    .toward_sun()
                    .into_iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 1e-6)
            );
            if extent == [640, 360] {
                assert_eq!(
                    first.view.view_proj_columns(),
                    frame.view.view_proj_columns()
                );
            } else {
                assert_ne!(
                    first.view.view_proj_columns(),
                    frame.view.view_proj_columns()
                );
            }
        }
    }

    #[test]
    fn reflection_subject_keeps_authoritative_material_and_static_history() {
        let mut world = scene::simulation(scene::GameContent::Grid, scene::RouteOverlay::Off)
            .expect("actual 10k fixture");
        let mut render = GameScene::scene([640, 360], crate::GameSetup::default())
            .expect("authored lighting subjects");
        let mut extractor = RenderExtractor::new(scene::ENTITY_COUNT, scene::ENTITY_COUNT);
        let handle = world
            .handle(StableId(scene::REFLECTION_SUBJECT_SLOT as u128))
            .expect("authored subject exists");
        let slot = usize::try_from(handle.slot).expect("bounded physical slot");
        let policy = GameScene::new(crate::GameSetup::default());
        let mut changed = Vec::new();
        for _ in 0..2 {
            let page = world.published_page().expect("committed game page");
            assert_eq!(
                page.visual(handle),
                Some(scene::REFLECTION_SUBJECT_MATERIAL)
            );
            policy.extract(&mut extractor, &page);
            let report = render.apply_render_columns(extractor.frame(), &mut changed);
            assert_eq!(report.dropped_invalid, 0);
            let subject = &render.instances[slot];
            assert_eq!(
                u64::from(subject.material()),
                scene::REFLECTION_SUBJECT_MATERIAL
            );
            assert_eq!(subject.current, subject.previous);
            for (basis, normal) in subject.current[..3].iter().zip(&subject.normal) {
                let inverse_product: f32 = basis[..3]
                    .iter()
                    .zip(&normal[..3])
                    .map(|(a, b)| a * b)
                    .sum();
                assert!((inverse_product - 1.0).abs() < 1e-6);
            }
            for index in [scene::NEUTRAL_SUBJECT_SLOT, slot] {
                let subject = &render.instances[index];
                let radius = subject.current[..3]
                    .iter()
                    .flat_map(|axis| &axis[..3])
                    .map(|value| value * value)
                    .sum::<f32>()
                    .sqrt();
                for light in &render.local_lights {
                    let distance = subject.current[3][..3]
                        .iter()
                        .zip(light.position)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f32>()
                        .sqrt();
                    assert!(
                        distance > radius + light.radius,
                        "lighting subject intersects a local-light sphere"
                    );
                }
            }
            drop(page);
            world
                .step(&[])
                .expect("same authoritative workload advances");
        }
    }
}
