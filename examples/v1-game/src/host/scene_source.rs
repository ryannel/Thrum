//! V1 fixture, camera and lighting policy for the engine frame runtime.

mod cloud;
mod star_catalogue;

use crate::{
    camera::{CameraRoute, SceneCamera},
    environment::DayCycle,
    field,
    host::GameSetup,
    scene::{self, GameContent},
    simulation::PublishedPage,
};
#[cfg(not(target_arch = "wasm32"))]
use thrum_content_runtime::EnvironmentProduct;
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::synthetic::PhotometricEnvironment;
use thrum_render::{
    abi::{InstanceRecord, MaterialRecord},
    atmosphere::AtmosphereFrame,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{SceneFrame, SceneSource},
    synthetic::{
        DisplayOverlay, FrameLighting, SdrToneMap, SyntheticPointLight, SyntheticScene,
        SyntheticView,
    },
};

fn atmosphere_lighting(
    sun: crate::environment::SunSample,
    camera_altitude_m: f32,
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
    FrameLighting::Atmosphere(frame)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn photometric(product: &EnvironmentProduct) -> PhotometricEnvironment {
    let values = product.values();
    PhotometricEnvironment {
        toward_sun: values.toward_sun(),
        sun_lux: values.sun_lux(),
        sun_colour: values.sun_colour(),
        ambient_lux: values.ambient_lux(),
        background_radiance: values.background_radiance(),
    }
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
) -> Result<SyntheticView, String> {
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
    .map_err(|error| format!("live camera: {error:?}"))
}

/// The flythrough camera with the player's pitch and yaw added to its heading.
fn field_view(
    tick: u64,
    alpha: f32,
    extent: [u32; 2],
    camera_pitch: f32,
    camera_yaw: f32,
) -> Result<SyntheticView, String> {
    let sample = field::camera(tick, alpha);
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
    .map_err(|error| format!("field camera: {error:?}"))
}

/// Share of field instance centres strictly inside the flythrough frustum at
/// `tick` (no player orientation), for labelling measured rows.
/// # Errors
/// Returns an invalid camera.
#[doc(hidden)]
pub fn field_in_view_share(tick: u64, extent: [u32; 2]) -> Result<f64, String> {
    let m = field_view(tick, 1.0, extent, 0.0, 0.0)?.view_proj_columns();
    let inside = (0..field::ENTITY_COUNT)
        .filter(|index| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "field coordinates round once into the f32 GPU domain"
            )]
            let p = field::entity(*index).position.map(|value| value as f32);
            let c: [f32; 4] = std::array::from_fn(|row| {
                m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row]
            });
            c[0].abs() < c[3] && c[1].abs() < c[3] && c[2] > 0.0 && c[2] < c[3]
        })
        .count();
    #[expect(clippy::cast_precision_loss, reason = "counts below 2^24")]
    Ok(inside as f64 / field::ENTITY_COUNT as f64)
}

/// The field's prepared boxes, materials and Sun shadows; no local lights.
/// With `meshes`, props draw the mixed meshes and every fifth is mirrored.
fn field_scene(extent: [u32; 2], meshes: bool) -> Result<SyntheticScene, String> {
    let count = u32::try_from(field::ENTITY_COUNT).map_err(|_| "field exceeds u32 slots")?;
    let mut scene =
        SyntheticScene::grid(count).map_err(|error| format!("field scene: {error:?}"))?;
    scene.materials = field::materials();
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        let entity = field::entity(index);
        let (mut basis, mut normal) = field::basis(&entity);
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
        instance.ids[1] = u32::try_from(entity.material).map_err(|_| "field material row")?;
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

/// The moving-route proof overlay's retuned diagnostic light, above the mirrored pair.
pub(crate) const ROUTE_OVERLAY_LIGHT: SyntheticPointLight = SyntheticPointLight {
    position: [0.0, 240.55, -3.2],
    radius: 4.0,
    candela: [8000.0, 6000.0, 4000.0],
};

/// Cold geometry for the moving-route overlay: valley-scale bases matching the
/// authoritative reseeds, one mirrored basis, and diagnostic light 0 moved beside them.
/// Two-sided state is all-or-nothing, so the mirrored basis switches the whole list.
fn apply_route_overlay(scene: &mut SyntheticScene, overlay: scene::RouteOverlay) {
    for (slot, position) in scene::ROUTE_OVERLAY {
        let instance = &mut scene.instances[slot];
        scale_basis(instance, 0.18);
        instance.current[3][..3].copy_from_slice(&position);
        instance.previous[3][..3].copy_from_slice(&position);
    }
    if overlay == scene::RouteOverlay::Motion {
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::env::var("THRUM_NATIVE_WITNESS").as_deref() {
            Ok("metal") => NativeWitness::Metal,
            Ok("wgsl") => NativeWitness::Wgsl,
            Ok("no-commands") => NativeWitness::NoCommands,
            Ok("no-tile") => NativeWitness::NoTile,
            Ok("no-reset") => NativeWitness::NoReset,
            Ok("stale-source") => NativeWitness::StaleSource,
            Ok("stale-history") => NativeWitness::StaleHistory,
            _ => NativeWitness::Off,
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        NativeWitness::Off
    }
}

pub(crate) struct GameScene {
    native_witness: thrum_render::synthetic::NativeWitness,
    content: GameContent,
    valley_stars: bool,
    pub(super) day_phase_offset: f32,
    pub(super) camera_pitch: f32,
    pub(super) camera_yaw: f32,
    pub(super) display_overlay: DisplayOverlay,
    temporal_inputs: bool,
    temporal_pan: (u64, f32),
    temporal_star_edit: u64,
    camera: Option<SceneCamera>,
    route_overlay: scene::RouteOverlay,
    route: Option<CameraRoute>,
    #[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
    omit_mirror: bool,
}
impl GameScene {
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

    pub(super) fn new(setup: GameSetup) -> Self {
        let temporal: Option<serde_json::Value> =
            (setup.display_overlay == DisplayOverlay::TemporalInputs).then(|| {
                serde_json::from_str(include_str!(
                    "../../../../crates/thrum-render/src/synthetic/temporal/witness.json"
                ))
                .expect("checked-in temporal fixture")
            });
        Self {
            content: setup.content,
            valley_stars: setup.content == GameContent::Grid && setup.valley_stars,
            day_phase_offset: 0.0,
            camera_pitch: 0.0,
            camera_yaw: 0.0,
            display_overlay: setup.display_overlay,
            temporal_inputs: temporal.is_some(),
            native_witness: native_witness_selection(),
            temporal_star_edit: temporal.as_ref().map_or(u64::MAX, |f| {
                f["star_edit_tick"].as_u64().expect("fixture star tick")
            }),
            temporal_pan: temporal.as_ref().map_or((u64::MAX, 0.0), |f| {
                (
                    f["pan_tick"].as_u64().expect("fixture pan tick"),
                    serde_json::from_value(f["pan_yaw"].clone()).expect("fixture pan yaw"),
                )
            }),
            camera: None,
            route_overlay: setup.route_overlay,
            route: None,
            #[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
            omit_mirror: std::env::var("THRUM_G3T_OVERLAY_CONTROL").as_deref() == Ok("mirror-omit"),
        }
    }
    pub(super) fn scene(extent: [u32; 2], setup: GameSetup) -> Result<SyntheticScene, String> {
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
            GameContent::Field => field_scene(extent, setup.field_meshes)?,
        };
        scene.view = scene
            .view
            .with_native_witness(native_witness_selection())
            .with_temporal_inputs(setup.display_overlay == DisplayOverlay::TemporalInputs)
            .with_display_overlay(setup.display_overlay);
        scene.camera_culling = setup.camera_culling;
        if setup.diagnostic_clouds {
            scene.sun_optical_depth = Some(cloud::diagnostic()?);
        }
        if setup.route_overlay != scene::RouteOverlay::Off {
            if setup.content != GameContent::Grid {
                return Err("the route overlay needs grid content".to_owned());
            }
            apply_route_overlay(&mut scene, setup.route_overlay);
            #[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
            if let Ok(control) = std::env::var("THRUM_G3T_OVERLAY_CONTROL") {
                g3t_overlay_control(&mut scene, &control)?;
            }
        }
        Ok(scene)
    }

    fn grid_scene(extent: [u32; 2]) -> Result<SyntheticScene, String> {
        let count = u32::try_from(scene::ENTITY_COUNT).map_err(|_| "fixture exceeds u32 slots")?;
        let mut scene =
            SyntheticScene::grid(count).map_err(|error| format!("synthetic scene: {error:?}"))?;
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

impl super::FrameHost {
    /// Current presentation camera pitch and yaw, in radians.
    #[must_use]
    pub fn camera_orientation(&self) -> [f32; 2] {
        let source = self.inner.source();
        [source.camera_pitch, source.camera_yaw]
    }

    /// Select a checked authored camera and reset presentation history.
    /// # Errors
    /// Refuses while a moving route is installed.
    pub fn set_scene_camera(&mut self, camera: SceneCamera) -> Result<(), String> {
        self.inner.source_mut().select_camera(camera)?;
        self.inner.invalidate_history();
        Ok(())
    }

    /// Install a moving camera route when the committed tick is `origin_tick − 1`.
    /// From then on the authored camera follows the route and camera, time and
    /// control edits are refused. History resets so both targets align at the origin.
    /// # Errors
    /// See the route's installation refusals.
    pub fn set_camera_route(&mut self, route: CameraRoute) -> Result<(), String> {
        let tick = self.inner.world().tick();
        self.inner.source_mut().install_route(route, tick)?;
        self.inner.invalidate_history();
        Ok(())
    }

    /// The authored scene camera, when content selected one.
    #[must_use]
    pub fn scene_camera(&self) -> Option<SceneCamera> {
        self.inner.source().authored_camera()
    }

    /// The installed route's pose and history epoch for `tick` presented at `alpha`.
    #[must_use]
    pub fn route_pose(&self, tick: u64, alpha: f32) -> Option<crate::RoutePose> {
        self.inner.source().route_pose(tick, alpha)
    }

    /// World time, in milliseconds from the clock origin, of the next tick boundary
    /// plus `phi` of a period: `(tick + 1 + φ)·P` for the committed tick.
    #[must_use]
    pub fn route_step_ms(&self, phi: f64) -> f64 {
        route_step_ms(self.inner.world().tick(), phi)
    }
}

impl SceneSource<PublishedPage> for GameScene {
    fn extract(&self, into: &mut RenderExtractor, page: &PublishedPage) -> ExtractReport {
        #[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
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
    ) -> Result<SceneFrame, String> {
        let temporal_yaw = if self.temporal_inputs && source.tick >= self.temporal_pan.0 {
            self.temporal_pan.1
        } else {
            self.camera_yaw
        };
        let field = self.content == GameContent::Field && self.camera.is_none();
        let routed = self.route.as_ref().zip(self.camera);
        let view = if let Some((route, camera)) = routed {
            let pose = route.pose(source.tick, alpha);
            camera
                .at(pose.eye)?
                .view(extent, pose.pitch, pose.yaw)?
                .with_sdr_tone_map(SdrToneMap::NeutralHighlights)
                .with_history_epoch(pose.epoch)
        } else if field {
            field_view(source.tick, alpha, extent, self.camera_pitch, temporal_yaw)?
        } else {
            frame_view(
                source.tick,
                alpha,
                extent,
                self.camera_pitch,
                temporal_yaw,
                self.camera,
            )?
        }
        .with_temporal_inputs(self.temporal_inputs)
        .with_native_witness(self.native_witness)
        .with_display_overlay(self.display_overlay)
        .with_jitter_sequence(0, [640, 360]);
        // The diagnostic fixture's local Y zero is a two-metre reference datum.
        // Authored scene and field coordinates already measure altitude above sea level.
        let camera_altitude_m = view.camera_position()[1]
            + if self.camera.is_some() || field {
                0.0
            } else {
                2.0
            };
        let sun = DayCycle::sample(source.tick, alpha, self.day_phase_offset);
        let lighting = lighting.unwrap_or_else(|| atmosphere_lighting(sun, camera_altitude_m));
        let view = if self.valley_stars
            && self.camera.is_some()
            && matches!(lighting, FrameLighting::Atmosphere(_))
        {
            view.with_celestial_epoch(u64::from(
                self.temporal_inputs && source.tick >= self.temporal_star_edit,
            ))
            .with_celestial_basis(sun.equatorial_to_world)
            .map_err(|error| format!("celestial view: {error:?}"))?
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
        }
        .map_err(|error| format!("view exposure: {error:?}"))?;
        Ok(SceneFrame { view, lighting })
    }
}

#[cfg(test)]
mod tests {
    use super::{GameScene, RenderExtractor, SceneSource, SourceTick, frame_view, scene};
    use crate::{camera::SceneCamera, environment::DayCycle};
    use thrum_core::StableId;
    use thrum_render::synthetic::{DisplayOverlay, FrameLighting, SdrToneMap, SyntheticView};

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
        let mut policy = GameScene::new(crate::GameSetup {
            route_overlay: scene::RouteOverlay::Proof,
            ..crate::GameSetup::default()
        });
        assert!(
            policy.install_route(parse("proof"), 49).is_err(),
            "needs a camera"
        );
        policy.select_camera(camera).unwrap();
        assert!(
            policy.install_route(parse("off"), 49).is_err(),
            "overlay mismatch"
        );
        assert!(
            policy.install_route(parse("proof"), 48).is_err(),
            "wrong tick"
        );
        policy.install_route(parse("proof"), 49).unwrap();
        assert!(
            policy.install_route(parse("proof"), 49).is_err(),
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
            native_witness: thrum_render::synthetic::NativeWitness::Off,
            content: scene::GameContent::Grid,
            valley_stars: true,
            day_phase_offset: 0.1,
            camera_pitch,
            camera_yaw,
            display_overlay: DisplayOverlay::Off,
            temporal_inputs: false,
            temporal_pan: (30, -0.05),
            temporal_star_edit: 15,
            camera: Some(camera),
            route_overlay: scene::RouteOverlay::Off,
            route: None,
            #[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
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
                u64::from(subject.ids[1]),
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
