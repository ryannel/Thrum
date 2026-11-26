use super::{
    Wind, island_material, placements, prepare_island_map_at, surf_spectrum, surf_water_profile,
};
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
const LAKE_LEVEL: f32 = 12.0;
const HIGH_LAKE_STAGE: f32 = 2.0;

/// An island waterline the inspection camera can cross.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiveSite {
    /// The deep part of the sheltered island lake.
    Lake,
    /// The sheltered lake at a stage deep enough for the 5 m view.
    HighLake,
    /// The exposed sea over the offshore shelf.
    Sea,
    /// The seaward side of the beach's swash.
    Swash,
}

impl DiveSite {
    /// The location name used by the scene query and capture receipts.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lake => "lake",
            Self::HighLake => "lake-high-stage",
            Self::Sea => "sea",
            Self::Swash => "swash",
        }
    }

    /// The distance above and below mean water used in the crossing.
    #[must_use]
    pub const fn crossing_depth(self) -> f64 {
        match self {
            Self::Lake | Self::HighLake | Self::Sea => 0.6,
            Self::Swash => 0.35,
        }
    }
}

fn camera([x, level, z]: [f32; 3], depth: f32, pitch: f32) -> Camera {
    let y = level - depth;
    let (rise, forward) = pitch.to_radians().sin_cos();
    Camera::look_at([x, y, z], [x + 10.0 * forward, y + 10.0 * rise, z], 105.0)
}

/// An inspection camera that can cross the selected waterline.
/// # Errors
/// Returns invalid map, spectrum, control or cooked asset declarations.
pub fn build_dive(
    wind: Wind,
    site: DiveSite,
    draw_water: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    build(wind, site, draw_water, true)
}

/// The unchanged opaque scene used by the paired above-water cost record.
/// # Errors
/// Returns invalid map, spectrum, control or cooked asset declarations.
pub fn build_dive_cost(
    wind: Wind,
    site: DiveSite,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    build(wind, site, true, false)
}

fn build(
    wind: Wind,
    site: DiveSite,
    draw_water: bool,
    draw_craft: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    let (eye_at_surface, vertical_offset, initial_depth, max_depth) = match site {
        DiveSite::Lake => ([183.0, LAKE_LEVEL, 5.0], 0.0, 1.0, 3.4),
        DiveSite::HighLake => ([183.0, LAKE_LEVEL + HIGH_LAKE_STAGE, 5.0], 0.0, 1.0, 5.3),
        DiveSite::Sea => ([-293.0, SEA_LEVEL, 20.0], SEA_LEVEL, 1.0, 10.0),
        DiveSite::Swash => ([-112.0, SEA_LEVEL, 0.0], SEA_LEVEL, 0.1, 0.4),
    };
    let surf = site == DiveSite::Swash;
    let lake = matches!(site, DiveSite::Lake | DiveSite::HighLake);
    let lake_stage = if site == DiveSite::HighLake {
        HIGH_LAKE_STAGE
    } else {
        0.0
    };
    let forcing = if surf {
        wind.surf_forcing()
    } else {
        wind.forcing()
    };
    let map = prepare_island_map_at(
        &forcing,
        surf.then_some(true),
        None,
        f64::from(vertical_offset),
        f64::from(lake_stage),
    )?;
    if site == DiveSite::HighLake {
        let [x, level, z] = eye_at_surface.map(f64::from);
        let sample = map
            .sample([x, z])
            .ok_or("high-stage inspection has no water body")?;
        let bed = sample
            .bed
            .ok_or("high-stage inspection has no surveyed bed")?;
        if (sample.row.level - level).abs() > 0.001 || level - bed <= f64::from(max_depth) {
            return Err("high-stage inspection camera is outside the lake".into());
        }
    }
    let map = Arc::new(Mutex::new(map));
    let (spectrum, mut profile) = if surf {
        let spectrum = surf_spectrum(wind, &forcing)?;
        let profile = surf_water_profile(&spectrum);
        (spectrum, profile)
    } else {
        let spectrum = Spectrum::build(&forcing)?;
        let profile = crate::profile(&spectrum);
        (spectrum, profile)
    };
    let mut optics = Optics::default();
    if lake {
        optics = Optics {
            extinction: [0.37, 0.18, 0.25],
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
        .default(initial_depth)
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
        camera(eye_at_surface, initial_depth, 45.0),
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
    if !surf {
        meshes.push(
            MeshPlacement::new("meshes/float", "textures/white")
                .at([x + 2.5, level - 1.8, z])
                .scale(0.38),
        );
    }
    if site == DiveSite::Sea && !draw_craft {
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
    let game = if draw_craft && !surf {
        crate::floating::install_moored_craft(
            game,
            &water,
            [f64::from(x) + 1.7, f64::from(z)],
            f64::from(level),
            std::f64::consts::FRAC_PI_2,
        )
    } else {
        game
    };
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok((game, report))
}
