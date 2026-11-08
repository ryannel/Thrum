//! One cold content activation shared by the live window and its capture mode.

use std::sync::{Arc, Mutex};

use crate::FrameHost;
use thrum_asset_gpu::CookedStats;

/// Select the host for `THRUM_SCENE=valley|diagnostic|field` and
/// `THRUM_CULLING=direct|frustum|occlusion`. The valley prepares its star field
/// unless `THRUM_STARS=off`; `THRUM_ROUTE_OVERLAY=proof|unmirrored|light-off` adds the
/// moving-route proof overlay or one of its controls.
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
            (Ok("valley"), Ok("unmirrored")) => crate::RouteOverlay::Unmirrored,
            (Ok("valley"), Ok("light-off")) => crate::RouteOverlay::LightOff,
            _ => {
                return Err(
                    "THRUM_ROUTE_OVERLAY must be proof, unmirrored or light-off, with THRUM_SCENE=valley"
                        .to_owned(),
                );
            }
        },
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
            thrum_asset_gpu::CookedAssets::new(crate::cooked::cooked_config(
                std::path::PathBuf::from(store),
            ))
            .map_err(|error| error.to_string())?,
        ),
        _ => return Err("THRUM_SCENE must be valley, diagnostic or field".to_owned()),
    };
    let stats = assets.stats();
    host.set_scene_contribution(Box::new(assets))?;
    if let Some(camera) = camera {
        host.set_scene_camera(camera)?;
    }
    Ok(Some(stats))
}
