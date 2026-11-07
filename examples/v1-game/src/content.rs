//! One cold content activation shared by the live window and its capture mode.

use std::sync::{Arc, Mutex};

use crate::FrameHost;
use thrum_asset_gpu::CookedStats;

/// Activate the selected game content from `THRUM_ASSET_STORE` and `THRUM_SCENE`.
/// `valley` uses the embedded metre-scale scene; `diagnostic` retains G5's test placements.
/// Unset variables retain the original simulation and camera.
/// # Errors
/// Returns an invalid selection, recipe or renderer contribution failure.
pub fn attach_cooked_scene(
    host: &mut FrameHost,
) -> Result<Option<Arc<Mutex<CookedStats>>>, String> {
    let Some(store) = std::env::var_os("THRUM_ASSET_STORE") else {
        if std::env::var_os("THRUM_SCENE").is_some() {
            return Err("THRUM_SCENE requires THRUM_ASSET_STORE".to_owned());
        }
        return Ok(None);
    };
    let (camera, assets) = match std::env::var("THRUM_SCENE").as_deref() {
        Ok("valley") => {
            let (camera, assets) = crate::showcase::prepare_showcase(store.into())?;
            (Some(camera), assets)
        }
        Err(std::env::VarError::NotPresent) | Ok("diagnostic") => (
            None,
            thrum_asset_gpu::CookedAssets::new(crate::cooked::cooked_config(store.into())),
        ),
        _ => return Err("THRUM_SCENE must be valley or diagnostic".to_owned()),
    };
    let stats = assets.stats();
    host.set_scene_contribution(Box::new(assets))?;
    if let Some(camera) = camera {
        host.set_scene_camera(camera);
    }
    Ok(Some(stats))
}
