//! Thrum starter: one cooked rock under a physically based sky, a camera, and Sun sliders.
//!
//! Run it with `./dev starter`, which cooks `assets/`, watches them for edits and opens the
//! window. Edit a slider in the panel, `starter.params.json` (press L to reload it) or any
//! file under `assets/` while it runs. See README.md.

#[cfg(target_os = "macos")]
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(target_os = "macos")]
use thrum_app::{
    Config,
    sky::{self, Camera, ContentStatus, SkyGame},
};
#[cfg(target_os = "macos")]
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedMaterial, CookedStats, MeshPlacement};
#[cfg(target_os = "macos")]
use thrum_controls::{Controls, Display};

#[cfg(target_os = "macos")]
fn main() {
    let config = Config {
        name: "thrum-starter",
        frames_variable: "THRUM_STARTER_FRAMES",
        window_title: "Thrum starter",
        title: "Thrum starter",
        render_thread: "thrum-starter-render",
        panel_title: "Starter",
        window_size: [1600, 900],
    };
    sky::run(config, scene);
}

/// The starter's window runs on the Mac only.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn scene() -> Result<SkyGame, String> {
    let store = asset_store()?;
    let mut controls = Controls::new();
    let elevation = controls
        .slider("sun.elevation", "Sun elevation", -8.0..=60.0)
        .default(25.0)
        .step(0.5)
        .display(Display::plain(1, "°"))
        .add()
        .map_err(|error| error.to_string())?;
    let azimuth = controls
        .slider("sun.azimuth", "Sun azimuth", 0.0..=360.0)
        .default(150.0)
        .step(1.0)
        .display(Display::plain(0, "°"))
        .add()
        .map_err(|error| error.to_string())?;

    // One rock, its base resting on sea level (Y = 0), metres throughout.
    let rock = MeshPlacement {
        mesh: "meshes/rock".into(),
        texture: "textures/white".into(),
        transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.83, 0.0, 1.0],
        ],
        material: CookedMaterial {
            base_colour: [0.34, 0.29, 0.23, 1.0],
            roughness: 0.85,
            metalness: 0.0,
        },
    };
    let assets = CookedAssets::new(CookedConfig {
        source: store.into(),
        meshes: vec![rock],
        volume: None,
        upload_budget: 8 << 20,
        poll_interval: Duration::from_millis(20),
        texture_preference: None,
    });
    let stats = assets.stats();

    let camera = Camera::look_at([0.0, 1.6, 5.5], [0.0, 0.7, 0.0], 35.0);
    let game = SkyGame::new(controls, camera, move |controls, sky| {
        sky.sun_elevation_degrees = controls.get(elevation);
        sky.sun_azimuth_degrees = controls.get(azimuth);
    });
    Ok(game.with_contribution(Box::new(assets), move || status(&stats)))
}

/// What the cooked assets show: the drawn generation, or why nothing is drawn yet.
#[cfg(target_os = "macos")]
fn status(stats: &Arc<Mutex<CookedStats>>) -> ContentStatus {
    let Ok(stats) = stats.lock() else {
        return ContentStatus::Failed("cooked asset observations were poisoned".into());
    };
    match (stats.drawn, stats.errors.last()) {
        (Some(generation), _) => ContentStatus::Drawn(generation),
        (None, Some(error)) => ContentStatus::Failed(error.clone()),
        (None, None) => ContentStatus::Loading,
    }
}

/// The cooked store named by `THRUM_ASSET_STORE`, which `./dev starter` sets.
#[cfg(target_os = "macos")]
fn asset_store() -> Result<PathBuf, String> {
    let store = std::env::var_os("THRUM_ASSET_STORE")
        .map(PathBuf::from)
        .ok_or(
            "THRUM_ASSET_STORE is not set, so there are no cooked assets to draw; \
         run ./dev starter, which cooks examples/starter/assets and sets it",
        )?;
    if !store.join("HEAD").is_file() {
        return Err(format!(
            "asset store {} has no HEAD: nothing has been cooked there; run ./dev starter, or \
             cargo run -p thrum-cook -- cook examples/starter/assets {}",
            store.display(),
            store.display()
        ));
    }
    Ok(store)
}
