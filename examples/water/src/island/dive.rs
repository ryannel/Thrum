use super::{Wind, placements, prepare_island_map};
use crate::SharedReport;
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_sea_model::Spectrum;
use thrum_water::{Optics, Water};

const LAKE_LEVEL: f32 = 12.0;

fn camera(depth: f32, pitch: f32) -> Camera {
    let [x, z] = [183.0, 5.0];
    let y = LAKE_LEVEL - depth;
    let (rise, forward) = pitch.to_radians().sin_cos();
    Camera::look_at([x, y, z], [x + 10.0 * forward, y + 10.0 * rise, z], 105.0)
}

/// A camera that can dive through the island lake's waterline.
/// # Errors
/// Returns invalid map, spectrum, control or cooked asset declarations.
pub fn build_lake_dive(
    wind: Wind,
    draw_water: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    let forcing = wind.forcing();
    let map = Arc::new(Mutex::new(prepare_island_map(&forcing, None, None)?));
    let spectrum = Spectrum::build(&forcing)?;
    let mut profile = crate::profile(&spectrum);
    profile.optics = Some(Optics {
        extinction: [0.56, 0.29, 0.38],
        downwelling: [0.24, 0.12, 0.16],
        reflection_reach: 48.0,
        ..Optics::default()
    });
    profile.deep_scatter = [0.017, 0.049, 0.026];
    profile.underwater = true;
    let water = Water::mapped(Arc::clone(&map), spectrum, profile)?;
    let report = water.report();
    let mut controls = Controls::new();
    let depth = controls
        .slider("camera.depth", "Depth (m)", -2.0..=3.4)
        .step(0.01)
        .default(1.0)
        .add()?;
    let pitch = controls
        .slider("camera.pitch", "Pitch (degrees)", -80.0..=89.99)
        .default(45.0)
        .add()?;
    let sun = controls
        .slider("sun.elevation", "Sun elevation", -8.0..=80.0)
        .default(60.0)
        .add()?;
    let game = SkyGame::new(controls, camera(1.0, 45.0), move |controls, sky| {
        sky.camera = camera(controls.get(depth), controls.get(pitch));
        sky.sun_elevation_degrees = controls.get(sun);
        sky.sun_azimuth_degrees = 228.0;
        sky.temporal_inputs = true;
    })
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(Duration::from_secs_f64(1.0 / 60.0))?;
    let mut meshes = placements(&map, true);
    meshes.push(
        MeshPlacement::new("meshes/float", "textures/white")
            .at([185.5, 10.2, 5.0])
            .scale(0.38),
    );
    let (game, _) = game.with_meshes_and_texture_lease(
        meshes,
        Some(("water island", include_str!("../../island.wgsl"))),
        &[],
    )?;
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok((game, report))
}
