//! One cold content activation shared by the live window and its capture mode.

use std::sync::{Arc, Mutex};

use crate::{GameHost, GameReconstruction, host::Runtime as Host};
use thrum_asset_gpu::CookedStats;

// This cold launch boundary owns environment interpretation; world construction
// receives checked typed controls and never reads process settings.
pub(crate) fn launch_value(name: &str) -> Result<String, std::env::VarError> {
    std::env::var(name)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn selected_rows(
    content: crate::GameContent,
    max_binding: u64,
) -> Result<thrum_asset_gpu::rows::CookedRows, thrum_asset_gpu::rows::CookedRowError> {
    let store = launch_value("THRUM_ASSET_STORE")
        .map_err(|_| thrum_asset_gpu::rows::CookedRowError::Missing("THRUM_ASSET_STORE".into()))?;
    thrum_asset_gpu::rows::CookedRows::from_store(
        &thrum_asset::AssetStore::new(store),
        crate::showcase::entity_mesh_names(content),
        max_binding,
    )
}

#[cfg(target_os = "macos")]
pub(crate) fn placed_sky() -> Result<Option<crate::PlacedSky>, String> {
    let store = match launch_value("THRUM_CLOUD_STORE") {
        Ok(store) => std::path::PathBuf::from(store),
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !store.join("HEAD").is_file() {
        return Err(format!(
            "Placed cloud store has no HEAD at {}",
            store.display()
        ));
    }
    let placement = launch_value("THRUM_CLOUD_PLACEMENT");
    let (directed, weather_share) = match placement.as_deref() {
        Ok("directed") => (true, None),
        Ok("weather") => (false, Some(0.36)),
        Ok("procedural") => (false, Some(0.0)),
        Ok("mixed") | Err(std::env::VarError::NotPresent) => (false, None),
        _ => {
            return Err(
                "THRUM_CLOUD_PLACEMENT must be directed, mixed, weather or procedural".to_owned(),
            );
        }
    };
    let sky = crate::PlacedSky::new(store.into(), directed).map_err(|error| error.to_string())?;
    Ok(Some(if let Some(share) = weather_share {
        sky.with_weather_placement(share)
    } else {
        sky
    }))
}

fn cloud_weather() -> Result<thrum_cloud_model::Parameters, String> {
    let number = |name: &str, fallback| match launch_value(name) {
        Ok(value) => value
            .parse::<f64>()
            .map_err(|_| format!("{name} must be a number")),
        Err(std::env::VarError::NotPresent) => Ok(fallback),
        Err(error) => Err(error.to_string()),
    };
    let preset = match launch_value("THRUM_CLOUD_PRESET") {
        Ok(value) => crate::clouds::WeatherPreset::named(&value)
            .ok_or("THRUM_CLOUD_PRESET must be clear, fair, broken or overcast")?,
        Err(std::env::VarError::NotPresent) => crate::clouds::WeatherPreset::Fair,
        Err(error) => return Err(error.to_string()),
    };
    let parameters = thrum_cloud_model::Parameters {
        wind: [number("THRUM_CLOUD_WIND", 10.0)?, 0.0],
        time_scale: number("THRUM_CLOUD_TIME_SCALE", 1.0)?,
        ..preset.parameters()
    };
    thrum_cloud_model::State::new(parameters, 5.0, 0).map_err(|e| e.to_string())?;
    Ok(parameters)
}

/// Apply optional launch quality settings through the prepared cloud controls.
/// # Errors
/// Refuses unsupported values or a scene without the requested cloud capability.
#[cfg(feature = "proving")]
pub fn apply_cloud_quality(host: &mut Host) -> Result<(), CloudQualityError> {
    for (variable, suffix) in [
        ("THRUM_CLOUD_REFRESH_STRIDE", "cloud refresh exponent"),
        ("THRUM_CLOUD_LIGHT_SAMPLES", "cloud local light samples"),
    ] {
        let Ok(text) = launch_value(variable) else {
            continue;
        };
        let invalid = || CloudQualityError::Value {
            setting: variable,
            value: text.clone(),
        };
        let mut value = text.parse::<u32>().map_err(|_| invalid())?;
        if suffix == "cloud refresh exponent" {
            if ![1, 2, 4].contains(&value) {
                return Err(invalid());
            }
            value = value.trailing_zeros();
        }
        let name = host
            .quality_knobs()
            .into_iter()
            .find(|q| q.name.ends_with(suffix))
            .ok_or_else(|| {
                CloudQualityError::Control(thrum_render::contribution::QualityError::UnknownKnob {
                    name: suffix.into(),
                })
            })?
            .name;
        host.set_game_quality_knob(&name, value)
            .map_err(CloudQualityError::Control)?;
    }
    Ok(())
}

/// A refused native cloud quality launch setting.
#[derive(Debug)]
#[cfg(feature = "proving")]
pub enum CloudQualityError {
    /// A setting is not an admitted unsigned quality value.
    Value {
        /// Launch setting name.
        setting: &'static str,
        /// Refused text.
        value: String,
    },
    /// The prepared scene refused the requested control.
    Control(thrum_render::contribution::QualityError),
}
#[cfg(feature = "proving")]
impl std::fmt::Display for CloudQualityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value { setting, value } => write!(f, "invalid {setting}: {value:?}"),
            Self::Control(error) => error.fmt(f),
        }
    }
}
#[cfg(feature = "proving")]
impl std::error::Error for CloudQualityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Control(error) => Some(error),
            Self::Value { .. } => None,
        }
    }
}

/// Select the native game's authored scene and optional capabilities.
/// # Errors
/// Refuses unsupported launch settings.
#[cfg_attr(
    feature = "proving",
    expect(
        clippy::needless_update,
        reason = "the shipping build takes defaults for proving-only selections"
    )
)]
pub fn selected_setup() -> Result<crate::GameSetup, String> {
    let scene = launch_value("THRUM_SCENE");
    #[cfg(feature = "proving")]
    let interface = scene.as_deref() == Ok("interface");
    #[cfg(feature = "proving")]
    let (interface_scale, interface_rows) = selected_interface(interface)?;
    #[cfg(feature = "proving")]
    let scene = scene_with_interface_background(scene);
    #[cfg(feature = "proving")]
    let sprite_scale =
        selected_sprite_scale(matches!(scene.as_deref(), Ok("sprites" | "interface")))?;
    #[cfg(not(feature = "proving"))]
    if matches!(
        scene.as_deref(),
        Ok("field" | "sprites" | "interface" | "diagnostic")
    ) {
        return Err("the selected scene requires the proving build".into());
    }
    let content = match scene.as_deref() {
        #[cfg(feature = "proving")]
        Ok("field") => crate::GameContent::Field,
        Ok("exit") => crate::GameContent::Exit,
        _ => crate::GameContent::Grid,
    };
    let field_meshes = scene.as_deref() == Ok("valley")
        && launch_value("THRUM_VALLEY_MESHES").as_deref() == Ok("on");
    #[cfg(feature = "proving")]
    let field_meshes = field_meshes
        || (content == crate::GameContent::Field
            && launch_value("THRUM_FIELD_MESHES").as_deref() == Ok("on"));
    let camera_culling = match launch_value("THRUM_CULLING").as_deref() {
        #[cfg(feature = "proving")]
        Err(std::env::VarError::NotPresent) if content == crate::GameContent::Field => {
            crate::GameSetup::field_default_culling(field_meshes)
        }
        Err(std::env::VarError::NotPresent)
            if matches!(scene.as_deref(), Ok("valley" | "exit")) =>
        {
            crate::GameSetup::valley_default_culling()
        }
        Err(std::env::VarError::NotPresent) | Ok("direct") => {
            thrum_render::synthetic::CullMode::Direct
        }
        Ok("frustum") => thrum_render::synthetic::CullMode::Frustum,
        Ok("occlusion") => thrum_render::synthetic::CullMode::Occlusion,
        _ => return Err("THRUM_CULLING must be direct, frustum or occlusion".to_owned()),
    };
    let clouds = match launch_value("THRUM_CLOUDS").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("off") => crate::CloudSelection::Off,
        Ok("shade") => crate::CloudSelection::Shade,
        Ok("sky") => crate::CloudSelection::Sky,
        Ok("world-light") => crate::CloudSelection::WorldLight,
        _ => return Err("THRUM_CLOUDS must be off, shade, sky or world-light".to_owned()),
    };
    #[cfg(feature = "proving")]
    let cloud_sequence = match launch_value("THRUM_CLOUD_SEQUENCE").as_deref() {
        Err(std::env::VarError::NotPresent) => crate::clouds::WeatherSequence::Held,
        Ok("coverage-sweep") => crate::clouds::WeatherSequence::CoverageSweep,
        Ok("presets") => crate::clouds::WeatherSequence::Presets,
        Ok("wind-turn") => crate::clouds::WeatherSequence::WindTurn,
        _ => {
            return Err("THRUM_CLOUD_SEQUENCE must be presets, coverage-sweep or wind-turn".into());
        }
    };
    Ok(crate::GameSetup {
        #[cfg(feature = "proving")]
        interface_scale,
        #[cfg(feature = "proving")]
        interface_rows,
        #[cfg(feature = "proving")]
        interface_text: selected_interface_text(interface)?,
        #[cfg(feature = "proving")]
        sprite_scale,
        #[cfg(feature = "proving")]
        route_scale_props: false,
        #[cfg(feature = "proving")]
        sprite_probe: if sprite_scale.is_some() {
            match launch_value("THRUM_SPRITE_PROBE") {
                Ok(value) => Some(crate::sprite_probe::parse(&value)?),
                Err(std::env::VarError::NotPresent) => None,
                Err(error) => return Err(error.to_string()),
            }
        } else {
            None
        },
        #[cfg(feature = "proving")]
        sprite_rows: if sprite_scale.is_some() {
            match launch_value("THRUM_SPRITE_ROWS") {
                Ok(count) => Some(
                    count
                        .parse::<u32>()
                        .map_err(|_| "THRUM_SPRITE_ROWS must be a whole number")?,
                ),
                Err(std::env::VarError::NotPresent) => interface.then_some(10_000),
                Err(error) => return Err(error.to_string()),
            }
        } else {
            None
        },
        cloud_weather: if clouds == crate::CloudSelection::Off {
            crate::clouds::initial_state().parameters()
        } else {
            cloud_weather()?
        },
        #[cfg(feature = "proving")]
        cloud_sequence,
        clouds,
        reconstruction: selected_reconstruction(launch_value("THRUM_RECONSTRUCTION").as_deref())?,
        #[cfg(feature = "proving")]
        display_overlay: match launch_value("THRUM_TRACED_VIEW").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("off") => {
                thrum_render::synthetic::DisplayOverlay::Off
            }
            Ok("on") => thrum_render::synthetic::DisplayOverlay::TracedGeometry,
            _ => return Err("THRUM_TRACED_VIEW must be off or on".to_owned()),
        },
        #[cfg(feature = "proving")]
        sun_shadow_method: match launch_value("THRUM_SUN_SHADOW_METHOD").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("maps") => {
                thrum_render::synthetic::SunShadowMethod::Maps
            }
            Ok("traced") => thrum_render::synthetic::SunShadowMethod::Traced,
            _ => return Err("THRUM_SUN_SHADOW_METHOD must be maps or traced".to_owned()),
        },
        #[cfg(feature = "proving")]
        sun_penumbra: match launch_value("THRUM_SUN_PENUMBRA").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("off") => false,
            Ok("on") => true,
            _ => return Err("THRUM_SUN_PENUMBRA must be off or on".to_owned()),
        },
        #[cfg(feature = "proving")]
        sun_proof_geometry: match launch_value("THRUM_SUN_PROOF_GEOMETRY").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("off") => crate::SunProofGeometry::Off,
            Ok("grounded-balloon") if content == crate::GameContent::Exit => crate::SunProofGeometry::GroundedBalloon,
            Ok("ambient") if content == crate::GameContent::Exit => crate::SunProofGeometry::Ambient,
            Ok("fast") if content == crate::GameContent::Exit => crate::SunProofGeometry::Fast,
            Ok("joined") if content == crate::GameContent::Exit => crate::SunProofGeometry::Joined,
            Ok("separate") if content == crate::GameContent::Exit => crate::SunProofGeometry::Separate,
            _ => return Err("THRUM_SUN_PROOF_GEOMETRY requires off or an exit scene selection: fast, ambient, grounded-balloon, joined or separate".to_owned()),
        },
        #[cfg(feature = "proving")]
        lamp_shadows: match launch_value("THRUM_LAMP_SHADOWS").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("off") => false,
            Ok("on") => true,
            _ => return Err("THRUM_LAMP_SHADOWS must be off or on".to_owned()),
        },
        #[cfg(feature = "proving")]
        ambient_occlusion_radius: match launch_value("THRUM_AMBIENT_OCCLUSION").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("off") => None,
            Ok(value) => Some(value.parse::<f32>().ok().filter(|radius| radius.is_finite() && *radius > 0.0)
                .ok_or("THRUM_AMBIENT_OCCLUSION must be off or a positive radius in metres")?),
            Err(error) => return Err(error.to_string()),
        },
        #[cfg(feature = "proving")]
        diagnostic_clouds: launch_value("THRUM_DIAGNOSTIC_CLOUDS").as_deref() == Ok("on"),
        #[cfg(feature = "proving")]
        haze: crate::host::haze_selection(launch_value("THRUM_HAZE").ok().as_deref())?,
        #[cfg(feature = "proving")]
        test_bank: launch_value("THRUM_HAZE").as_deref()
            == Ok(crate::host::scene_source::test_bank::SELECTION),
        content,
        #[cfg(feature = "proving")]
        exit_entities: if content == crate::GameContent::Exit
            && launch_value("THRUM_EXIT_NO_ENTITIES").as_deref() == Ok("1")
        {
            crate::ExitEntities::Omit
        } else {
            crate::ExitEntities::Draw
        },
        camera_culling,
        valley_stars: matches!(scene.as_deref(), Ok("valley" | "exit"))
            && launch_value("THRUM_STARS").as_deref() != Ok("off"),
        #[cfg(feature = "proving")]
        route_overlay: match (
            scene.as_deref(),
            launch_value("THRUM_ROUTE_OVERLAY").as_deref(),
        ) {
            (_, Err(std::env::VarError::NotPresent) | Ok("off")) => crate::RouteOverlay::Off,
            (Ok("valley"), Ok("proof")) => crate::RouteOverlay::Proof,
            (Ok("valley"), Ok("motion")) => crate::RouteOverlay::Motion,
            (Ok("valley"), Ok("reconstruction")) => crate::RouteOverlay::Reconstruction,
            (Ok("valley"), Ok("unmirrored")) => crate::RouteOverlay::Unmirrored,
            (Ok("valley"), Ok("light-off")) => crate::RouteOverlay::LightOff,
            (Ok("valley"), Ok("lamp-stop")) => crate::RouteOverlay::LampStop,
            (Ok("valley"), Ok("night-lights")) => crate::RouteOverlay::NightLights,
            _ => {
                return Err(
                    "THRUM_ROUTE_OVERLAY must be off, proof, motion, reconstruction, unmirrored, light-off, lamp-stop or night-lights, with THRUM_SCENE=valley"
                        .to_owned(),
                );
            }
        },
        field_meshes,
        cooked_mesh_rows: (content == crate::GameContent::Exit
            || (field_meshes && scene.as_deref() == Ok("valley")))
        .into(),
        ..crate::GameSetup::default()
    })
}

fn selected_reconstruction(
    value: Result<&str, &std::env::VarError>,
) -> Result<GameReconstruction, String> {
    match value {
        Err(std::env::VarError::NotPresent) | Ok("off") => Ok(GameReconstruction::Off),
        Ok("taau") => Ok(GameReconstruction::Taau),
        Ok("metalfx") => Ok(GameReconstruction::MetalFx),
        #[cfg(feature = "proving")]
        Ok("reference") => Ok(GameReconstruction::Reference),
        #[cfg(feature = "proving")]
        _ => Err("THRUM_RECONSTRUCTION must be off, taau, metalfx or reference".to_owned()),
        #[cfg(not(feature = "proving"))]
        _ => Err("THRUM_RECONSTRUCTION must be off, taau or metalfx".to_owned()),
    }
}

#[cfg(feature = "proving")]
fn validate_sprite_launch() -> Result<(), String> {
    if !matches!(
        launch_value("THRUM_ASSET_STORE"),
        Err(std::env::VarError::NotPresent)
    ) {
        return Err("2D scene refuses cooked placements through THRUM_ASSET_STORE".into());
    }
    for (name, enabled) in [
        ("THRUM_FIELD_MESHES", "on"),
        ("THRUM_VALLEY_MESHES", "on"),
        ("THRUM_COOKED_DRAW_DIAGNOSTIC", "1"),
        ("THRUM_SUN_SHADOW_METHOD", "maps"),
    ] {
        if launch_value(name).as_deref() == Ok(enabled) {
            return Err(format!("2D scene refuses {name}={enabled}"));
        }
    }
    for name in ["THRUM_STARS", "THRUM_ROUTE_OVERLAY", "THRUM_NATIVE_WITNESS"] {
        if let Ok(request) = launch_value(name)
            && request != "off"
        {
            return Err(format!("2D scene refuses {name}={request}"));
        }
    }
    for name in ["THRUM_CULLING", "THRUM_EFFECT_GRAPH"] {
        if let Ok(request) = launch_value(name) {
            return Err(format!("2D scene refuses {name}={request}"));
        }
    }
    Ok(())
}

/// Activate the selected game content from `THRUM_ASSET_STORE` and `THRUM_SCENE`.
/// `field` is prepared with the host (see [`selected_setup`]) and attaches nothing here.
/// `valley` uses the embedded metre-scale scene; `diagnostic` retains G5's test placements.
/// Unset variables retain the original simulation and camera.
/// # Errors
/// Returns an invalid selection, recipe or renderer contribution failure.
pub fn attach_cooked_scene(host: &mut Host) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    attach_cooked_scene_with_path(host, thrum_asset_gpu::CookedDrawPath::Auto)
}

pub(crate) fn attach_cooked_scene_with_path(
    host: &mut Host,
    path: thrum_asset_gpu::CookedDrawPath,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    attach_selected(host, path, Ok)
}

#[cfg(all(target_os = "macos", feature = "proving"))]
pub(crate) fn attach_content_preview(
    host: &mut Host,
) -> Result<crate::content_preview::PreviewRecord, String> {
    let store = launch_value("THRUM_ASSET_STORE").map_err(|error| error.to_string())?;
    let rows = host
        .cooked_rows()
        .ok_or("content preview needs cooked mesh rows")?;
    let (camera, assets) =
        crate::showcase::prepare_content_preview(std::path::PathBuf::from(store).into())?;
    let reader = rows.updates();
    let assets = assets.with_rows(rows).map_err(|error| error.to_string())?;
    let (contribution, record) =
        crate::content_preview::attach(assets, reader, host.reconstruction());
    host.set_game_contribution(contribution)?;
    host.set_scene_camera(camera)?;
    Ok(record)
}

/// Attach selected content with an explicit compile-time-enabled qualification control.
/// # Errors
/// Refuses invalid content, incomplete reference arguments or a renderer failure.
#[cfg(feature = "draw-probe")]
pub fn attach_cooked_scene_with_probe(
    host: &mut Host,
    probe: Option<thrum_asset_gpu::draw_probe::Probe>,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    attach_selected(
        host,
        thrum_asset_gpu::CookedDrawPath::Auto,
        |assets| match probe {
            Some(probe) => assets
                .with_draw_probe(probe)
                .map_err(|error| error.to_string()),
            None => Ok(assets),
        },
    )
}

#[cfg(feature = "proving")]
fn scene_with_interface_background(
    scene: Result<String, std::env::VarError>,
) -> Result<String, std::env::VarError> {
    if scene.as_deref() == Ok("interface")
        && launch_value("THRUM_INTERFACE_BACKGROUND").as_deref() == Ok("valley")
    {
        Ok("valley".into())
    } else {
        scene
    }
}

#[cfg(feature = "proving")]
fn selected_sprite_scale(enabled: bool) -> Result<Option<u16>, String> {
    if !enabled {
        return Ok(None);
    }
    validate_sprite_launch()?;
    let scale = match launch_value("THRUM_SPRITE_SCALE") {
        Ok(scale) => scale
            .parse()
            .map_err(|_| "THRUM_SPRITE_SCALE must be an integer from 1 to 6")?,
        Err(std::env::VarError::NotPresent) => 2,
        Err(error) => return Err(error.to_string()),
    };
    Ok(Some(scale))
}

#[cfg(feature = "proving")]
fn selected_interface_text(enabled: bool) -> Result<Option<crate::InterfaceText>, String> {
    if !enabled {
        return Ok(None);
    }
    match launch_value("THRUM_INTERFACE_TEXT").as_deref() {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Ok("type") => Ok(Some(crate::InterfaceText::Type)),
        Ok("counting") => Ok(Some(crate::InterfaceText::Counting)),
        Ok("status") => Ok(Some(crate::InterfaceText::Status)),
        Ok("specimen") => Ok(Some(crate::InterfaceText::Specimen)),
        _ => Err("THRUM_INTERFACE_TEXT must be type, counting, specimen or status".into()),
    }
}

#[cfg(feature = "proving")]
fn selected_interface(enabled: bool) -> Result<(Option<u8>, Option<u32>), String> {
    if !enabled {
        return Ok((None, None));
    }
    let scale = match launch_value("THRUM_INTERFACE_SCALE") {
        Err(std::env::VarError::NotPresent) => None,
        Ok(value) => Some(
            value
                .parse::<u8>()
                .ok()
                .filter(|scale| (1..=3).contains(scale))
                .ok_or("THRUM_INTERFACE_SCALE must be an integer from 1 to 3")?,
        ),
        Err(error) => return Err(error.to_string()),
    };
    let rows = match launch_value("THRUM_INTERFACE_ROWS") {
        Err(std::env::VarError::NotPresent) => None,
        Ok(value) => Some(
            value
                .parse::<u32>()
                .ok()
                .filter(|rows| matches!(rows, 0 | 2000))
                .ok_or("THRUM_INTERFACE_ROWS must be 0 or 2000")?,
        ),
        Err(error) => return Err(error.to_string()),
    };
    Ok((
        scale.or_else(|| {
            (!matches!(
                selected_interface_text(enabled),
                Ok(Some(crate::InterfaceText::Status))
            ))
            .then_some(1)
        }),
        rows,
    ))
}

fn attach_selected(
    host: &mut Host,
    path: thrum_asset_gpu::CookedDrawPath,
    configure: impl FnOnce(
        thrum_asset_gpu::CookedAssets,
    ) -> Result<thrum_asset_gpu::CookedAssets, String>,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    let scene = launch_value("THRUM_SCENE");
    #[cfg(feature = "proving")]
    let scene = scene_with_interface_background(scene);
    if scene.as_deref() == Ok("field") {
        return Ok(None);
    }
    let store = std::env::var_os("THRUM_ASSET_STORE");
    if matches!(scene.as_deref(), Ok("sprites" | "interface")) {
        if store.is_some() {
            return Err("2D scene refuses cooked placements through THRUM_ASSET_STORE".into());
        }
        return Ok(None);
    }
    let Some(store) = store else {
        if scene != Err(std::env::VarError::NotPresent) {
            return Err("THRUM_SCENE requires THRUM_ASSET_STORE".to_owned());
        }
        return Ok(None);
    };
    let (camera, assets) = match scene.as_deref() {
        Ok("valley" | "exit") => {
            let (camera, assets) = crate::showcase::prepare_showcase_with_path(
                std::path::PathBuf::from(store).into(),
                path,
            )?;
            (Some(camera), assets)
        }
        #[cfg(feature = "proving")]
        Err(std::env::VarError::NotPresent) | Ok("diagnostic") => (None, {
            let store = std::path::PathBuf::from(store);
            let config = if launch_value("THRUM_COOKED_DRAW_DIAGNOSTIC").as_deref() == Ok("1") {
                crate::cooked::qualification_config(store)
            } else {
                crate::cooked::cooked_config(store)
            };
            thrum_asset_gpu::CookedAssets::new(config).map_err(|error| error.to_string())?
        }),
        _ => {
            return Err(
                "THRUM_SCENE must be valley, exit, diagnostic, field or sprites".to_owned(),
            );
        }
    };
    let assets = if let Some(rows) = host.cooked_rows() {
        assets.with_rows(rows).map_err(|error| error.to_string())?
    } else {
        assets
    };
    let assets = configure(assets)?;
    let stats = assets.stats();
    #[cfg(feature = "proving")]
    if let Ok(selection) = launch_value("THRUM_EFFECT_GRAPH") {
        if !matches!(
            selection.as_str(),
            "1" | "slab" | "depth-only" | "simulation-only" | "slab-empty" | "depth-only-empty"
        ) {
            return Err("unknown effect diagnostic".into());
        }
        let (effect, witness) = crate::effect::DiagnosticEffect::new();
        witness
            .lock()
            .map_err(|error| error.to_string())?
            .controls
            .ground_height = 240.0;
        let effect = if selection == "1" {
            effect.scripted()
        } else if selection == "simulation-only" {
            effect.simulation_only()
        } else {
            witness
                .lock()
                .map_err(|error| error.to_string())?
                .controls
                .slab_origin_scale = [0., 240., -6., 2.];
            witness
                .lock()
                .map_err(|error| error.to_string())?
                .controls
                .slab_draw = !selection.ends_with("-empty");
            effect.with_slab(selection.starts_with("slab"))
        };
        host.set_game_contribution(Box::new(
            thrum_render::contribution::SceneContributions::new(vec![
                Box::new(assets),
                Box::new(effect),
            ]),
        ))?;
    } else {
        let contribution: Box<dyn thrum_render::contribution::SceneContribution> = Box::new(assets);
        #[cfg(target_os = "macos")]
        let contribution = match launch_value("THRUM_GOVERNOR_LOAD") {
            Ok(rounds) => {
                let rounds = rounds
                    .parse::<u32>()
                    .map_err(|_| "invalid debug load rounds")?;
                if rounds > crate::governor_load::DebugLoad::MAX_ROUNDS {
                    return Err("debug load rounds exceed the qualification limit".to_owned());
                }
                crate::governor_load::DebugLoad::new(230..320, rounds).alongside(contribution)
            }
            Err(std::env::VarError::NotPresent) => contribution,
            Err(error) => return Err(error.to_string()),
        };
        host.set_game_contribution(contribution)?;
    }
    #[cfg(not(feature = "proving"))]
    host.set_game_contribution(Box::new(assets))?;
    #[cfg(feature = "proving")]
    let camera = if let (Some(camera), Ok(height)) = (camera, launch_value("THRUM_CAMERA_HEIGHT")) {
        let mut eye = camera.eye();
        eye[1] = height
            .parse()
            .map_err(|_| "THRUM_CAMERA_HEIGHT must be metres")?;
        if let Ok(x) = launch_value("THRUM_CAMERA_X") {
            eye[0] = x.parse().map_err(|_| "THRUM_CAMERA_X must be metres")?;
        }
        if let Ok(z) = launch_value("THRUM_CAMERA_Z") {
            eye[2] = z.parse().map_err(|_| "THRUM_CAMERA_Z must be metres")?;
        }
        Some(camera.at(eye)?)
    } else {
        camera
    };
    if let Some(camera) = camera {
        host.set_scene_camera(camera)?;
    }
    Ok(Some(stats))
}

#[cfg(test)]
mod reconstruction_tests {
    use super::{GameReconstruction, selected_reconstruction};

    #[test]
    fn native_launch_admits_only_the_reconstruction_modes_in_this_build() {
        assert_eq!(
            selected_reconstruction(Err(&std::env::VarError::NotPresent)),
            Ok(GameReconstruction::Off)
        );
        assert_eq!(
            selected_reconstruction(Ok("off")),
            Ok(GameReconstruction::Off)
        );
        assert_eq!(
            selected_reconstruction(Ok("taau")),
            Ok(GameReconstruction::Taau)
        );
        assert!(selected_reconstruction(Ok("invalid")).is_err());
        #[cfg(feature = "proving")]
        assert_eq!(
            selected_reconstruction(Ok("reference")),
            Ok(GameReconstruction::Reference)
        );
        #[cfg(not(feature = "proving"))]
        assert!(selected_reconstruction(Ok("reference")).is_err());
    }
}
