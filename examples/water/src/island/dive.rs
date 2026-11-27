use super::{
    Wind, island_material, placements, prepare_island_map_at, surf_spectrum, surf_water_profile,
};
use crate::SharedReport;
use std::{
    error::Error,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, CookedMaterial, MeshPlacement, SkyGame};
use thrum_sea_model::{Cascade, LongWaves, Spectrum};
use thrum_water::{Optics, Water};
use thrum_water_model::Field;

// Keep the inspection camera in the sky shell's altitude range while retaining
// the island's relative sea, seabed and camera elevations.
const SEA_LEVEL: f32 = 16.0;
const DEEP_SEA_LEVEL: f32 = 26.0;
const LAKE_LEVEL: f32 = 12.0;
const POND_LEVEL: f32 = 10.6;
const HIGH_LAKE_STAGE: f32 = 2.75;
const STEP_SECONDS: f64 = 1.0 / 60.0;
const ADAPTATION_SHARE: f64 = 0.55;
const SURFACE_TRANSMISSION: f64 = 0.96;

/// An island waterline the inspection camera can cross.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiveSite {
    /// The deep part of the sheltered island lake.
    Lake,
    /// The sheltered lake at a stage deep enough for the 5 m view.
    HighLake,
    /// The sheltered pond, viewed through its shallow surface.
    Pond,
    /// The exposed sea over the offshore shelf.
    Sea,
    /// Deep water off the southwest side of the island.
    DeepSea,
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
            Self::Pond => "pond",
            Self::Sea => "sea",
            Self::DeepSea => "deep-sea",
            Self::Swash => "swash",
        }
    }

    /// The distance above and below mean water used in the crossing.
    #[must_use]
    pub const fn crossing_depth(self) -> f64 {
        match self {
            Self::Lake | Self::HighLake | Self::Sea | Self::DeepSea => 0.6,
            Self::Swash | Self::Pond => 0.35,
        }
    }
}

fn camera([x, level, z]: [f32; 3], depth: f32, pitch: f32, bearing: f32, fov: f32) -> Camera {
    let y = level - depth;
    let (rise, forward) = pitch.to_radians().sin_cos();
    let (east, south) = bearing.to_radians().sin_cos();
    Camera::look_at(
        [x, y, z],
        [
            x + 10.0 * forward * east,
            y + 10.0 * rise,
            z - 10.0 * forward * south,
        ],
        fov,
    )
}

// Strengthen a short group of travelling wind modes in the exposed lake
// inspection. Its compression drives the ordinary whitewater births.
#[expect(
    clippy::cast_possible_truncation,
    reason = "indices recovered from the 256-square cascade lattice"
)]
fn lake_crest_group(spectrum: &mut Spectrum, eye: [f32; 3]) -> Result<(), Box<dyn Error>> {
    let band = &spectrum.cascades[0];
    let length = band.length();
    let step = std::f64::consts::TAU / length;
    // The focused modes compress enough to seed foam while retaining a
    // positive horizontal stretch of their own at the crest.
    let train = [(65, -9, 0.0060), (68, 8, 0.0042), (58, 22, 0.0031)];
    let mut modes: Vec<_> = band
        .components()
        .iter()
        .filter_map(|wave| {
            let [x, z] = wave.wave_vector.map(|k| (k / step).round() as i32);
            (!train.iter().any(|&(a, b, _)| (x, z) == (a, b))).then_some((
                x,
                z,
                wave.amplitude.powi(2) * 0.5,
                wave.phase,
            ))
        })
        .collect();
    modes.extend(train.map(|(x, z, power)| {
        let kx = f64::from(x) * step;
        let kz = f64::from(z) * step;
        let omega = (9.81 * kx.hypot(kz)).sqrt();
        let phase = omega * 10.0 - kx * f64::from(eye[0]) - kz * f64::from(eye[2]);
        (x, z, power, phase)
    }));
    spectrum.cascades[0] = Cascade::new(length, &modes)?;
    Ok(())
}

/// An inspection camera that can cross the selected waterline.
/// # Errors
/// Returns invalid map, spectrum, control or cooked asset declarations.
pub fn build_dive(
    wind: Wind,
    site: DiveSite,
    draw_water: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    build(wind, site, draw_water, true, true)
}

/// The unchanged opaque scene used by the paired above-water cost record.
/// # Errors
/// Returns invalid map, spectrum, control or cooked asset declarations.
pub fn build_dive_cost(
    wind: Wind,
    site: DiveSite,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    build(wind, site, true, false, false)
}

/// Paired cost arm with the ordinary craft, optics and identical camera route.
/// # Errors
/// Returns invalid scene inputs and unavailable cooked assets.
pub fn build_dive_light_cost(
    wind: Wind,
    site: DiveSite,
    under_light: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    build(wind, site, true, true, under_light)
}

/// The sea or lake under an authored opaque cloud-shade patch.
/// # Errors
/// Propagates scene preparation and the optical-depth field's validation errors.
pub fn build_dive_shade(
    wind: Wind,
    site: DiveSite,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    let (game, report) = build_dive(wind, site, true)?;
    let shade = thrum_render::atmosphere::SunOpticalDepth::new(
        [2, 2],
        &[16.0; 4],
        100.0,
        [-4096.0, -4096.0, 4096.0, 4096.0],
        0.02,
        1,
    )?;
    Ok((game.with_sun_optical_depth(shade), report))
}

fn build(
    wind: Wind,
    site: DiveSite,
    draw_water: bool,
    draw_craft: bool,
    under_light: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    let (eye_at_surface, vertical_offset, initial_depth, max_depth) = match site {
        DiveSite::Lake => ([183.0, LAKE_LEVEL, 5.0], 0.0, 1.0, 3.4),
        DiveSite::HighLake => ([190.0, LAKE_LEVEL + HIGH_LAKE_STAGE, 0.0], 0.0, 1.0, 5.3),
        DiveSite::Pond => ([104.0, POND_LEVEL, 93.0], 0.0, -2.0, 0.4),
        DiveSite::Sea => ([-293.0, SEA_LEVEL, 20.0], SEA_LEVEL, 1.0, 10.0),
        DiveSite::DeepSea => ([-300.0, DEEP_SEA_LEVEL, 190.0], DEEP_SEA_LEVEL, 1.0, 20.0),
        DiveSite::Swash => ([-112.0, SEA_LEVEL, 0.0], SEA_LEVEL, 0.1, 0.4),
    };
    let surf = site == DiveSite::Swash;
    let lake = matches!(site, DiveSite::Lake | DiveSite::HighLake);
    let lake_whitecaps = site == DiveSite::HighLake && draw_craft && wind == Wind::Trade;
    let lake_stage = if site == DiveSite::HighLake {
        HIGH_LAKE_STAGE
    } else {
        0.0
    };
    let surface_wind = if lake && draw_craft && wind == Wind::Trade {
        // Strong Trade provides breaking crests after the lake's fetch attenuation.
        Wind::Strong
    } else {
        wind
    };
    let forcing = if surf {
        wind.surf_forcing()
    } else {
        surface_wind.forcing()
    };
    let mut map = prepare_island_map_at(
        &forcing,
        surf.then_some(true),
        None,
        f64::from(vertical_offset),
        f64::from(lake_stage),
    )?;
    if lake_whitecaps {
        // The authored short crest group is local to this basin. Keep the
        // fetch-derived protection of the other bands, including the swell.
        let mut gains = map.body(1).ok_or("the dive lake is missing")?.gains;
        gains[1] = 1.0;
        map.set_wave_gains(1, gains)
            .map_err(|error| format!("{error:?}"))?;
    }
    if matches!(site, DiveSite::HighLake | DiveSite::DeepSea) {
        let [x, level, z] = eye_at_surface.map(f64::from);
        let sample = map
            .sample([x, z])
            .ok_or("deep dive inspection has no water body")?;
        let bed = sample
            .bed
            .ok_or("deep dive inspection has no surveyed bed")?;
        if (sample.row.level - level).abs() > 0.001 || level - bed <= f64::from(max_depth) {
            return Err("deep dive inspection camera must clear the surveyed bed".into());
        }
    }
    let map = Arc::new(Mutex::new(map));
    let (spectrum, mut profile) = if surf {
        let spectrum = surf_spectrum(wind, &forcing)?;
        let profile = surf_water_profile(&spectrum);
        (spectrum, profile)
    } else {
        let mut spectrum = Spectrum::build(&forcing)?;
        if lake_whitecaps {
            lake_crest_group(&mut spectrum, eye_at_surface)?;
        }
        let profile = crate::profile(&spectrum);
        (spectrum, profile)
    };
    let mut optics = if draw_craft && matches!(site, DiveSite::Sea | DiveSite::DeepSea) {
        // The shelf and offshore station share one clear-sea water type.
        Optics {
            extinction: [0.37, 0.105, 0.054],
            downwelling: [0.22, 0.075, 0.042],
            ..Optics::default()
        }
    } else {
        Optics::default()
    };
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
    profile.caustics = under_light;
    profile.shafts = under_light;
    let meter_field = if draw_water && under_light {
        let [outer, middle, inner] = &spectrum.cascades;
        Some(Field {
            layers: [
                LongWaves::new(spectrum.long.clone(), profile.waves.choppiness)?,
                LongWaves::new(outer.components().to_vec(), 0.0)?,
                LongWaves::new(middle.components().to_vec(), 0.0)?,
                LongWaves::new(inner.components().to_vec(), 0.0)?,
            ],
        })
    } else {
        None
    };
    let mut water = Water::mapped(Arc::clone(&map), spectrum, profile)?;
    if wind != Wind::Calm && (!lake || draw_craft) {
        water.enable_foam(surface_wind.velocity())?;
    }
    let report = water.report();
    let game = dive_camera_game(
        eye_at_surface,
        initial_depth,
        max_depth,
        &map,
        meter_field,
        optics,
    )?;
    let meshes = dive_meshes(&map, site, eye_at_surface, vertical_offset, draw_craft);
    let [x, level, z] = eye_at_surface;
    let (game, _) = game.with_meshes_and_texture_lease(
        meshes,
        Some(("water island", &island_material(vertical_offset))),
        &[],
    )?;
    let game = if draw_craft && !surf && site != DiveSite::Pond {
        crate::floating::install_moored_craft(
            game,
            &water,
            [f64::from(x) + 1.7, f64::from(z)],
            f64::from(level),
            std::f64::consts::FRAC_PI_2,
            0.85,
        )
    } else {
        game
    };
    let game = if draw_water {
        game.with_sun_path(water.sun_path_provider())
            .with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok((game, report))
}

fn dive_camera_game(
    eye_at_surface: [f32; 3],
    initial_depth: f32,
    max_depth: f32,
    map: &Arc<Mutex<thrum_water_model::Map>>,
    meter_field: Option<Field>,
    optics: Optics,
) -> Result<SkyGame, Box<dyn Error>> {
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
    let bearing = controls
        .slider("camera.bearing", "Camera bearing", 0.0..=360.0)
        .default(90.0)
        .add()?;
    let fov = controls
        .slider("camera.fov", "Camera field of view", 42.0..=105.0)
        .default(105.0)
        .add()?;
    let setback = controls
        .slider(
            "camera.setback",
            "Distance back from dive station (m)",
            -70.0..=24.0,
        )
        .default(0.0)
        .add()?;
    let surface_follow = controls
        .slider("camera.surface_follow", "Follow local surface", 0.0..=1.0)
        .default(0.0)
        .add()?;
    let sun = controls
        .slider("sun.elevation", "Sun elevation", -18.0..=80.0)
        .default(60.0)
        .add()?;
    let step = Duration::from_secs_f64(STEP_SECONDS);
    let initial_camera = camera(eye_at_surface, initial_depth, 45.0, 90.0, 105.0);
    let presentation_camera = Arc::new(Mutex::new((initial_camera, 0.0)));
    let edited_camera = Arc::clone(&presentation_camera);
    let game = SkyGame::new(controls, initial_camera, move |controls, sky| {
        let heading = controls.get(bearing);
        let (east, south) = heading.to_radians().sin_cos();
        let distance = controls.get(setback);
        let [x, y, z] = eye_at_surface;
        sky.camera = camera(
            [x - east * distance, y, z + south * distance],
            controls.get(depth),
            controls.get(pitch),
            heading,
            controls.get(fov),
        );
        *edited_camera.lock().expect("dive camera") = (sky.camera, controls.get(surface_follow));
        sky.sun_elevation_degrees = controls.get(sun);
        sky.sun_azimuth_degrees = 228.0;
        sky.temporal_inputs = true;
    })
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(step)?;
    if let Some(field) = meter_field {
        let wave_tick = Arc::new(AtomicU64::new(1));
        let field = Arc::new(field);
        let camera_field = Arc::clone(&field);
        let camera_map = Arc::clone(map);
        let camera_tick = Arc::clone(&wave_tick);
        let map = Arc::clone(map);
        let downwelling = optics.downwelling.map(f64::from);
        Ok(game
            .with_presentation_camera(move |alpha| {
                let (pose, follow) = *presentation_camera.lock().expect("dive camera");
                if follow == 0.0 {
                    return pose;
                }
                let time = display_time(camera_tick.load(Ordering::Relaxed), alpha, step);
                follow_surface(pose, follow, &camera_map, &camera_field, time)
            })
            .with_meter_medium(move |camera, source, alpha| {
                let [x, y, z] = camera.position.map(f64::from);
                let time = display_time(source.tick, alpha, step);
                let Some(surface) = map
                    .lock()
                    .ok()
                    .and_then(|map| map.surface(&field, [x, z], time).ok())
                    .filter(|surface| surface.wet)
                else {
                    return [1.0; 3];
                };
                thrum_water_model::meter_light_factor(
                    surface.sample.height - y,
                    downwelling,
                    SURFACE_TRANSMISSION,
                )
                .map(adapt_transmission)
            })
            .and_fixed_update(move |tick, _| wave_tick.store(tick, Ordering::Relaxed)))
    } else {
        Ok(game)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "presentation camera coordinates use f32 metres"
)]
fn follow_surface(
    mut pose: Camera,
    follow: f32,
    map: &Mutex<thrum_water_model::Map>,
    field: &Field,
    time: f64,
) -> Camera {
    let eye = glam::Vec3::from_array(pose.position);
    let forward = (glam::Vec3::from_array(pose.target) - eye).normalize();
    // The centre of the lens cut is on the sky camera's 10 cm near plane.
    let [x, _, z] = (eye + forward * 0.1).to_array().map(f64::from);
    let height = map.lock().ok().and_then(|map| {
        let level = map.sample([x, z])?.row.level;
        let surface = map.surface(field, [x, z], time).ok()?;
        surface.wet.then_some(surface.sample.height - level)
    });
    if let Some(height) = height {
        let offset = height as f32 * follow;
        pose.position[1] += offset;
        pose.target[1] += offset;
    }
    pose
}

fn dive_meshes(
    map: &Arc<Mutex<thrum_water_model::Map>>,
    site: DiveSite,
    eye_at_surface: [f32; 3],
    vertical_offset: f32,
    draw_craft: bool,
) -> Vec<MeshPlacement> {
    let mut meshes = placements(
        map,
        matches!(site, DiveSite::Lake | DiveSite::HighLake),
        vertical_offset,
    );
    let [x, level, z] = eye_at_surface;
    if !matches!(site, DiveSite::Swash | DiveSite::Pond) {
        // Keep the top of the sunk float in the shallow rungs, with the
        // curved sides reaching into the softer light below.
        meshes.push(
            MeshPlacement::new("meshes/float", "textures/white")
                .at([x + 0.65, level - 1.30, z + 3.4])
                .scale(1.65),
        );
    }
    if site == DiveSite::DeepSea {
        let map = map.lock().expect("dive map");
        for (forward, across, scale) in [(5.0, 0.8, 1.4), (10.5, -3.1, 2.2), (18.0, 3.0, 2.9)] {
            let world = [x + forward, z + across];
            if let Some(bed) = map
                .sample(world.map(f64::from))
                .and_then(|column| column.bed)
            {
                meshes.push(
                    MeshPlacement::new("meshes/rock", "textures/white")
                        .at([world[0], bed_for_placement(bed), world[1]])
                        .scale(scale)
                        .on_ground(),
                );
            }
        }
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
    meshes
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "cooked placements use f32 world positions"
)]
fn bed_for_placement(height: f64) -> f32 {
    height as f32
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the display clock converts ticks to interpolated seconds"
)]
fn display_time(tick: u64, alpha: f32, step: Duration) -> f64 {
    (tick as f64 - 1.0 + f64::from(alpha)) * step.as_secs_f64()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the camera request carries rounded f32 transmissions"
)]
fn adapt_transmission(factor: f64) -> f32 {
    factor.powf(ADAPTATION_SHARE) as f32
}
