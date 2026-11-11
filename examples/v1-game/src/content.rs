//! One cold content activation shared by the live window and its capture mode.

use std::sync::{Arc, Mutex};

use crate::FrameHost;
use thrum_asset_gpu::CookedStats;

/// Select the host for `THRUM_SCENE=valley|diagnostic|field` and
/// `THRUM_CULLING=direct|frustum|occlusion`. The valley prepares its star field
/// unless `THRUM_STARS=off`; `THRUM_ROUTE_OVERLAY=proof|unmirrored|light-off` adds the
/// moving-route proof overlay or one of its controls.
/// The field draws mixed meshes with `THRUM_FIELD_MESHES=on`.
/// Without `THRUM_CULLING`, the field uses Frustum (a measured win along its
/// flythrough) and the fully visible grid keeps Direct.
/// # Errors
/// Refuses an unknown culling mode or route overlay.
pub fn selected_setup() -> Result<crate::GameSetup, String> {
    let scene = std::env::var("THRUM_SCENE");
    let content = if scene.as_deref() == Ok("field") {
        crate::GameContent::Field
    } else {
        crate::GameContent::Grid
    };
    let camera_culling = match std::env::var("THRUM_CULLING").as_deref() {
        Err(std::env::VarError::NotPresent) if content == crate::GameContent::Field => {
            thrum_render::synthetic::CullMode::Frustum
        }
        Err(std::env::VarError::NotPresent) | Ok("direct") => {
            thrum_render::synthetic::CullMode::Direct
        }
        Ok("frustum") => thrum_render::synthetic::CullMode::Frustum,
        Ok("occlusion") => thrum_render::synthetic::CullMode::Occlusion,
        _ => return Err("THRUM_CULLING must be direct, frustum or occlusion".to_owned()),
    };
    Ok(crate::GameSetup {
        diagnostic_clouds: std::env::var("THRUM_DIAGNOSTIC_CLOUDS").as_deref() == Ok("on"),
        content,
        camera_culling,
        valley_stars: scene.as_deref() == Ok("valley")
            && std::env::var("THRUM_STARS").as_deref() != Ok("off"),
        route_overlay: match (
            scene.as_deref(),
            std::env::var("THRUM_ROUTE_OVERLAY").as_deref(),
        ) {
            (_, Err(std::env::VarError::NotPresent)) => crate::RouteOverlay::Off,
            (Ok("valley"), Ok("proof")) => crate::RouteOverlay::Proof,
            (Ok("valley"), Ok("motion")) => crate::RouteOverlay::Motion,
            (Ok("valley"), Ok("unmirrored")) => crate::RouteOverlay::Unmirrored,
            (Ok("valley"), Ok("light-off")) => crate::RouteOverlay::LightOff,
            _ => {
                return Err(
                    "THRUM_ROUTE_OVERLAY must be proof, motion, unmirrored or light-off, with THRUM_SCENE=valley"
                        .to_owned(),
                );
            }
        },
        field_meshes: content == crate::GameContent::Field
            && std::env::var("THRUM_FIELD_MESHES").as_deref() == Ok("on"),
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
        Some(probe) => assets.with_draw_probe(probe),
        None => Ok(assets),
    })
}

fn attach_selected(
    host: &mut FrameHost,
    configure: impl FnOnce(
        thrum_asset_gpu::CookedAssets,
    ) -> Result<thrum_asset_gpu::CookedAssets, String>,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    if std::env::var("THRUM_SCENE").as_deref() == Ok("field") {
        return Ok(None);
    }
    let Some(store) = std::env::var_os("THRUM_ASSET_STORE") else {
        if std::env::var_os("THRUM_SCENE").is_some() {
            return Err("THRUM_SCENE requires THRUM_ASSET_STORE".to_owned());
        }
        return Ok(None);
    };
    let (camera, assets) = match std::env::var("THRUM_SCENE").as_deref() {
        Ok("valley") => {
            let (camera, assets) =
                crate::showcase::prepare_showcase(std::path::PathBuf::from(store).into())?;
            (Some(camera), assets)
        }
        Err(std::env::VarError::NotPresent) | Ok("diagnostic") => (
            None,
            thrum_asset_gpu::CookedAssets::new(
                if std::env::var("THRUM_COOKED_DRAW_DIAGNOSTIC").as_deref() == Ok("1") {
                    crate::cooked::qualification_config(std::path::PathBuf::from(store))
                } else {
                    crate::cooked::cooked_config(std::path::PathBuf::from(store))
                },
            )
            .map_err(|error| error.to_string())?,
        ),
        _ => return Err("THRUM_SCENE must be valley, diagnostic or field".to_owned()),
    };
    let assets = configure(assets)?;
    let stats = assets.stats();
    host.set_scene_contribution(Box::new(assets))?;
    if let Some(camera) = camera {
        host.set_scene_camera(camera)?;
    }
    Ok(Some(stats))
}
