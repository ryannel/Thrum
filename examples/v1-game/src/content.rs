//! One cold content activation shared by the live window and its capture mode.

use std::sync::{Arc, Mutex};

use crate::FrameHost;
use thrum_asset_gpu::CookedStats;

// This cold launch boundary owns environment interpretation; world construction
// receives checked typed controls and never reads process settings.
fn launch_value(name: &str) -> Result<String, std::env::VarError> {
    std::env::var(name)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn selected_rows(
    max_binding: u64,
) -> Result<thrum_asset_gpu::rows::CookedRows, thrum_asset_gpu::rows::CookedRowError> {
    let store = launch_value("THRUM_ASSET_STORE")
        .map_err(|_| thrum_asset_gpu::rows::CookedRowError::Missing("THRUM_ASSET_STORE".into()))?;
    thrum_asset_gpu::rows::CookedRows::from_store(
        &thrum_asset::AssetStore::new(store),
        crate::showcase::entity_mesh_names(),
        max_binding,
    )
}

fn cloud_weather() -> Result<thrum_cloud_model::Parameters, String> {
    let number = |name: &str, fallback| match launch_value(name) {
        Ok(value) => value
            .parse::<f64>()
            .map_err(|_| format!("{name} must be a number")),
        Err(std::env::VarError::NotPresent) => Ok(fallback),
        Err(error) => Err(error.to_string()),
    };
    let parameters = thrum_cloud_model::Parameters {
        wind: [number("THRUM_CLOUD_WIND", 10.0)?, 0.0],
        time_scale: number("THRUM_CLOUD_TIME_SCALE", 1.0)?,
        ..crate::clouds::initial_state().parameters()
    };
    thrum_cloud_model::State::new(parameters, 5.0, 0).map_err(|e| e.to_string())?;
    Ok(parameters)
}

/// Select the host for `THRUM_SCENE=valley|diagnostic|field` and
/// `THRUM_CULLING=direct|frustum|occlusion`. The valley prepares its star field
/// unless `THRUM_STARS=off`; `THRUM_ROUTE_OVERLAY=proof|unmirrored|light-off` adds the
/// moving-route proof overlay or one of its controls.
/// The field draws mixed meshes with `THRUM_FIELD_MESHES=on`; the valley uses
/// `THRUM_VALLEY_MESHES=on` for the same prop arena on its entities.
/// Without `THRUM_CULLING`, the mixed-mesh field uses measured Occlusion,
/// the cube field and valley use Frustum, and the fully visible grid keeps Direct.
/// # Errors
/// Refuses an unknown culling mode or route overlay.
pub fn selected_setup() -> Result<crate::GameSetup, String> {
    let scene = launch_value("THRUM_SCENE");
    let content = if scene.as_deref() == Ok("field") {
        crate::GameContent::Field
    } else {
        crate::GameContent::Grid
    };
    let field_meshes = (content == crate::GameContent::Field
        && launch_value("THRUM_FIELD_MESHES").as_deref() == Ok("on"))
        || (scene.as_deref() == Ok("valley")
            && launch_value("THRUM_VALLEY_MESHES").as_deref() == Ok("on"));
    let camera_culling = match launch_value("THRUM_CULLING").as_deref() {
        Err(std::env::VarError::NotPresent) if content == crate::GameContent::Field => {
            crate::GameSetup::field_default_culling(field_meshes)
        }
        Err(std::env::VarError::NotPresent) if scene.as_deref() == Ok("valley") => {
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
        _ => return Err("THRUM_CLOUDS must be off or shade".to_owned()),
    };
    Ok(crate::GameSetup {
        cloud_weather: if clouds == crate::CloudSelection::Shade {
            cloud_weather()?
        } else {
            crate::clouds::initial_state().parameters()
        },
        clouds,
        reconstruction: match launch_value("THRUM_RECONSTRUCTION").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("off") => {
                thrum_render::synthetic::Reconstruction::Off
            }
            Ok("taau") => thrum_render::synthetic::Reconstruction::Taau,
            Ok("reference") => thrum_render::synthetic::Reconstruction::Reference,
            _ => return Err("THRUM_RECONSTRUCTION must be off, taau or reference".to_owned()),
        },
        display_overlay: thrum_render::synthetic::DisplayOverlay::Off,
        diagnostic_clouds: launch_value("THRUM_DIAGNOSTIC_CLOUDS").as_deref() == Ok("on"),
        content,
        camera_culling,
        valley_stars: scene.as_deref() == Ok("valley")
            && launch_value("THRUM_STARS").as_deref() != Ok("off"),
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
            _ => {
                return Err(
                    "THRUM_ROUTE_OVERLAY must be off, proof, motion, unmirrored or light-off, with THRUM_SCENE=valley"
                        .to_owned(),
                );
            }
        },
        field_meshes,
        cooked_mesh_rows: (field_meshes && scene.as_deref() == Ok("valley")).into(),
    })
}

/// Activate the selected game content from `THRUM_ASSET_STORE` and `THRUM_SCENE`.
/// `field` is prepared with the host (see [`selected_setup`]) and attaches nothing here.
/// `valley` uses the embedded metre-scale scene; `diagnostic` retains G5's test placements.
/// Unset variables retain the original simulation and camera.
/// # Errors
/// Returns an invalid selection, recipe or renderer contribution failure.
pub fn attach_cooked_scene(
    host: &mut FrameHost,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    attach_selected(host, Ok)
}

/// Attach selected content with an explicit compile-time-enabled qualification control.
/// # Errors
/// Refuses invalid content, incomplete reference arguments or a renderer failure.
#[cfg(feature = "draw-probe")]
pub fn attach_cooked_scene_with_probe(
    host: &mut FrameHost,
    probe: Option<thrum_asset_gpu::draw_probe::Probe>,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    attach_selected(host, |assets| match probe {
        Some(probe) => assets
            .with_draw_probe(probe)
            .map_err(|error| error.to_string()),
        None => Ok(assets),
    })
}

fn attach_selected(
    host: &mut FrameHost,
    configure: impl FnOnce(
        thrum_asset_gpu::CookedAssets,
    ) -> Result<thrum_asset_gpu::CookedAssets, String>,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    let scene = launch_value("THRUM_SCENE");
    if scene.as_deref() == Ok("field") {
        return Ok(None);
    }
    let Some(store) = std::env::var_os("THRUM_ASSET_STORE") else {
        if scene != Err(std::env::VarError::NotPresent) {
            return Err("THRUM_SCENE requires THRUM_ASSET_STORE".to_owned());
        }
        return Ok(None);
    };
    let (camera, assets) = match scene.as_deref() {
        Ok("valley") => {
            let (camera, assets) =
                crate::showcase::prepare_showcase(std::path::PathBuf::from(store).into())?;
            (Some(camera), assets)
        }
        Err(std::env::VarError::NotPresent) | Ok("diagnostic") => (
            None,
            thrum_asset_gpu::CookedAssets::new(
                if launch_value("THRUM_COOKED_DRAW_DIAGNOSTIC").as_deref() == Ok("1") {
                    crate::cooked::qualification_config(std::path::PathBuf::from(store))
                } else {
                    crate::cooked::cooked_config(std::path::PathBuf::from(store))
                },
            )
            .map_err(|error| error.to_string())?,
        ),
        _ => return Err("THRUM_SCENE must be valley, diagnostic or field".to_owned()),
    };
    let assets = if let Some(rows) = host.cooked_rows() {
        assets.with_rows(rows).map_err(|error| error.to_string())?
    } else {
        assets
    };
    let assets = configure(assets)?;
    let stats = assets.stats();
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
        host.set_scene_contribution(Box::new(
            thrum_render::contribution::SceneContributions::new(vec![
                Box::new(assets),
                Box::new(effect),
            ]),
        ))?;
    } else {
        host.set_scene_contribution(Box::new(assets))?;
    }
    let camera = if let (Some(camera), Ok(height)) = (camera, launch_value("THRUM_CAMERA_HEIGHT")) {
        let mut eye = camera.eye();
        eye[1] = height
            .parse()
            .map_err(|_| "THRUM_CAMERA_HEIGHT must be metres")?;
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
