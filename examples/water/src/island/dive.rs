use super::{Wind, island_material, placements, prepare_island_map_at};
use crate::SharedReport;
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, CookedMaterial, MeshPlacement, SkyGame};
use thrum_sea_model::Spectrum;
use thrum_water::{Optics, Water};

// Keep the inspection camera in the sky shell's altitude range while retaining
// the island's relative sea, seabed and camera elevations.
const SEA_LEVEL: f32 = 16.0;

fn camera([x, level, z]: [f32; 3], depth: f32, pitch: f32) -> Camera {
    let y = level - depth;
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
    build_dive(wind, draw_water, true)
}

/// Offshore inspection above the island shelf at a raised sea level.
/// # Errors
/// Returns invalid map, spectrum, control or cooked asset declarations.
pub fn build_sea_dive(
    wind: Wind,
    draw_water: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    build_dive(wind, draw_water, false)
}

fn build_dive(
    wind: Wind,
    draw_water: bool,
    lake: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    let (eye_at_surface, vertical_offset, max_depth) = if lake {
        ([183.0, 12.0, 5.0], 0.0, 3.4)
    } else {
        ([-293.0, SEA_LEVEL, 20.0], SEA_LEVEL, 10.0)
    };
    let forcing = wind.forcing();
    let map = Arc::new(Mutex::new(prepare_island_map_at(
        &forcing,
        None,
        None,
        f64::from(vertical_offset),
    )?));
    let spectrum = Spectrum::build(&forcing)?;
    let mut profile = crate::profile(&spectrum);
    let mut optics = Optics::default();
    if lake {
        optics = Optics {
            extinction: [0.56, 0.29, 0.38],
            downwelling: [0.24, 0.12, 0.16],
            reflection_reach: 48.0,
            ..Optics::default()
        };
        profile.deep_scatter = [0.017, 0.049, 0.026];
    }
    profile.optics = Some(optics);
    profile.underwater = true;
    let mut water = Water::mapped(Arc::clone(&map), spectrum, profile)?;
    if !lake && wind != Wind::Calm {
        water.enable_foam(wind.velocity())?;
    }
    let report = water.report();
    let mut controls = Controls::new();
    let depth = controls
        .slider("camera.depth", "Depth (m)", -2.0..=max_depth)
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
    let game = SkyGame::new(
        controls,
        camera(eye_at_surface, 1.0, 45.0),
        move |controls, sky| {
            sky.camera = camera(eye_at_surface, controls.get(depth), controls.get(pitch));
            sky.sun_elevation_degrees = controls.get(sun);
            sky.sun_azimuth_degrees = 228.0;
            sky.temporal_inputs = true;
        },
    )
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(Duration::from_secs_f64(1.0 / 60.0))?;
    let mut meshes = placements(&map, lake, vertical_offset);
    let [x, level, z] = eye_at_surface;
    meshes.push(
        MeshPlacement::new("meshes/float", "textures/white")
            .at([x + 2.5, level - 1.8, z])
            .scale(0.38),
    );
    if !lake {
        meshes.push(
            MeshPlacement::new("meshes/float", "textures/white")
                .at([x + 2.5, level, z + 0.7])
                .scale(1.8)
                .material(CookedMaterial {
                    base_colour: [0.76, 0.48, 0.13, 1.0],
                    roughness: 0.68,
                    metalness: 0.0,
                }),
        );
    }
    let (game, _) = game.with_meshes_and_texture_lease(
        meshes,
        Some(("water island", &island_material(vertical_offset))),
        &[],
    )?;
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok((game, report))
}
