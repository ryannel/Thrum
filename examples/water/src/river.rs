//! The island's solved spring stream and lake outlet.
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use thrum_water_model::{Bed, Body, BodyKind, Gauge, Map, RiverFile, Stage};

#[cfg(not(target_arch = "wasm32"))]
mod store;

const DEFAULT_STORE: &str = "target/scenes/water-scene/rivers";
const NAMES: [&str; 2] = ["Spring stream", "Lake river"];
const TERRAIN: &[u8] = include_bytes!("../assets/meshes/river-island.bin");

/// The spring releases a float on the valley floor.
pub const SPRING: [f64; 2] = [180.0, -95.0];

/// The selected island's map and the prepared stream gauge range.
pub type Prepared = (Map, [RiverFile; 2]);

/// Locate native river products beside the cooked store, unless a directory was selected.
pub fn store_directory(directory: Option<PathBuf>, cooked: Option<&Path>) -> PathBuf {
    directory.unwrap_or_else(|| {
        cooked.map_or_else(
            || PathBuf::from(DEFAULT_STORE),
            |store| store.with_file_name("rivers"),
        )
    })
}

/// Build the selected island from its prepared river files.
/// # Errors
/// Returns invalid terrain, river files or water declarations.
pub fn map(
    forcing: &thrum_sea_model::Forcing,
    store: Option<&Path>,
) -> Result<Prepared, Box<dyn Error>> {
    let bed = crate::island::terrain_bed(TERRAIN)?;
    let files = files(&bed, store)?;
    prepare_map(&bed, files, forcing)
}

fn prepare_map(
    bed: &Bed,
    files: [RiverFile; 2],
    forcing: &thrum_sea_model::Forcing,
) -> Result<Prepared, Box<dyn Error>> {
    let mut bodies = crate::island::bodies(forcing, 0.5);
    let ids: Vec<Vec<usize>> = files
        .iter()
        .scan(bodies.len(), |next, file| {
            let first = *next;
            *next += file.reaches.len();
            Some((first..*next).collect())
        })
        .collect();
    let names: Vec<_> = bodies
        .iter()
        .map(|body| body.name.clone())
        .chain(
            files
                .iter()
                .flat_map(|file| file.reaches.iter().map(|reach| reach.name.clone())),
        )
        .collect();
    for file in &files {
        for reach in &file.reaches {
            let chart = reach
                .chart(bed, |name| names.iter().position(|n| n == name))
                .map_err(|error| model_error(&error))?;
            bodies.push(Body {
                name: reach.name.clone(),
                kind: BodyKind::Chart(chart),
                level: 0.0,
                stage: Stage {
                    min: 0.0,
                    max: 0.0,
                    value: 0.0,
                },
                fetch: 100.0,
                gains: [0.0; 4],
            });
        }
    }
    let profile = thrum_water_model::Profile {
        margin_height: 1.25,
        ..crate::island::map_profile(false)
    };
    let mut map =
        Map::prepare(bed.clone(), bodies, profile).map_err(|error| model_error(&error))?;
    let [min, max] = files[0].gauge_range();
    let gauges = [
        Gauge::Stage(Stage {
            min,
            max,
            value: f64::midpoint(min, max),
        }),
        Gauge::Body(1),
    ];
    for (((file, bodies), name), gauge) in files.iter().zip(&ids).zip(NAMES).zip(gauges) {
        let river = file
            .river(bed, name, bodies, gauge)
            .map_err(|error| model_error(&error))?;
        map.add_river(river).map_err(|error| model_error(&error))?;
    }
    map.set_forcing(forcing)
        .map_err(|error| model_error(&error))?;
    Ok((map, files))
}

#[cfg(not(target_arch = "wasm32"))]
fn files(bed: &Bed, store: Option<&Path>) -> Result<[RiverFile; 2], Box<dyn Error>> {
    store::load(bed, store.unwrap_or_else(|| Path::new(DEFAULT_STORE)))
}

/// Generate the river files beside the cooked store for packaging.
/// # Errors
/// Returns invalid terrain, a refused solve or an I/O error.
#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_files(directory: &Path) -> Result<[String; 2], Box<dyn Error>> {
    let bed = crate::island::terrain_bed(TERRAIN)?;
    let files = store::load(&bed, directory)?;
    let (_, files) = prepare_map(&bed, files, &crate::channel::forcing(Wind::Calm))?;
    Ok(files.map(|file| key_hex(&file.key)))
}

fn key_hex(key: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut hex = String::with_capacity(64);
    for byte in key {
        write!(hex, "{byte:02x}").expect("write to string");
    }
    hex
}

#[cfg(target_arch = "wasm32")]
include!(concat!(env!("OUT_DIR"), "/river_package.rs"));

#[cfg(target_arch = "wasm32")]
fn files(_: &Bed, _: Option<&Path>) -> Result<[RiverFile; 2], Box<dyn Error>> {
    FILES
        .into_iter()
        .zip(NAMES)
        .map(|(entry, name)| {
            let (key, bytes) =
                entry.ok_or_else(|| format!("{name}: river file missing from page package"))?;
            RiverFile::read(bytes, &key).map_err(|error| format!("{name}: {error:?}").into())
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?
        .try_into()
        .map_err(|_| "expected two packaged river files".into())
}

fn model_error(error: &thrum_water_model::WaterError) -> String {
    format!("{error:?}")
}

use crate::island::Wind;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};

/// The outlet float starts beyond the source join.
pub const OUTLET_RELEASE: [f64; 2] = [218.0, 5.0];

/// Inspection lenses at the pool, sill, narrows, joins and both river overlooks.
pub const VIEWS: [(&str, Camera); 17] = [
    (
        "Pool along",
        Camera::look_at([175.0, 19.0, -72.0], [180.0, 17.1, -58.0], 66.0),
    ),
    (
        "Pool across",
        Camera::look_at([170.5, 19.0, -66.0], [180.0, 17.1, -66.0], 66.0),
    ),
    (
        "Sill along",
        Camera::look_at([172.8, 19.7, -61.0], [181.0, 16.8, -48.0], 72.0),
    ),
    (
        "Sill across",
        Camera::look_at([187.0, 19.6, -55.6], [178.0, 16.9, -51.2], 72.0),
    ),
    (
        "Narrows along",
        Camera::look_at([176.5, 18.7, -48.4], [180.1, 16.4, -40.0], 70.0),
    ),
    (
        "Narrows across",
        Camera::look_at([173.5, 18.8, -38.0], [181.5, 16.4, -42.5], 70.0),
    ),
    (
        "Lake inlet along",
        Camera::look_at([180.0, 14.4, -7.0], [181.0, 12.7, -22.0], 68.0),
    ),
    (
        "Lake inlet across",
        Camera::look_at([172.0, 14.5, -15.0], [184.4, 12.4, -20.5], 72.0),
    ),
    (
        "Stream overlook",
        Camera::look_at([181.0, 57.0, -61.0], [181.0, 16.0, -57.0], 110.0),
    ),
    ("Outlet along", crate::channel::VIEWS[0]),
    (
        "Outlet across",
        Camera::look_at([216.5, 14.5, 12.7], [208.0, 12.2, 4.0], 65.0),
    ),
    ("River bank", crate::channel::VIEWS[2]),
    ("Sea mouth", crate::channel::VIEWS[3]),
    (
        "River overlook",
        Camera::look_at([280.0, 46.0, 7.0], [280.0, 5.0, 5.0], 115.0),
    ),
    (
        "Both rivers",
        Camera::look_at([239.0, 97.0, -31.0], [238.0, 10.0, -29.0], 105.0),
    ),
    (
        "River across",
        Camera::look_at([257.0, 9.05, 12.0], [257.0, 7.2, 3.0], 61.0),
    ),
    (
        "Sea mouth across",
        Camera::look_at([356.0, 2.15, 17.0], [345.5, -0.05, 2.0], 70.0),
    ),
];

/// Selected river scene and the observations used by its target checks.
pub struct Scene {
    /// The normal window and web scene.
    pub game: SkyGame,
    /// Latest water resource report.
    pub report: crate::SharedReport,
    /// The shared map sampled by float queries and water.
    pub map: crate::SharedMap,
    /// Prepared files, including each sample's fall records.
    pub files: [RiverFile; 2],
    /// Queries and replies on the submitted water.
    pub query: thrum_water::query::QueryMailbox,
    /// The stream and outlet floats.
    pub floats: Box<dyn Fn() -> [crate::floating::Body; 2]>,
}

/// Build the river selection, with paired water-on and water-off rendering arms.
/// # Errors
/// Returns invalid assets, water or controls.
/// # Panics
/// Panics if the water map lock is poisoned or its bounded controls fail.
pub fn build(wind: Wind, draw_water: bool, store: Option<&Path>) -> Result<Scene, Box<dyn Error>> {
    let mut forcing = crate::channel::forcing(wind);
    // The exposed floats also take the prevailing wind. Water currents
    // provide the transport in the channel. Wind carries them across joins.
    forcing.wind_direction = 0.42;
    let (map, files) = map(&forcing, store)?;
    #[cfg(target_arch = "wasm32")]
    {
        let loaded = js_sys::Array::new();
        for (name, file) in NAMES.into_iter().zip(&files) {
            let record = js_sys::Array::new();
            record.push(&name.into());
            record.push(&key_hex(&file.key).into());
            loaded.push(&record);
        }
        drop(js_sys::Reflect::set(
            &js_sys::global(),
            &"waterRiverFiles".into(),
            &loaded,
        ));
    }
    let [low, high] = files[0].gauge_range();
    let map = Arc::new(Mutex::new(map));
    let spectrum = thrum_sea_model::Spectrum::build(&forcing)?;
    let mut profile = crate::profile(&spectrum);
    profile.optics = Some(thrum_water::Optics::default());
    profile.query = Some(thrum_water::query::QueryLimits {
        points: 24,
        max_age: 8,
    });
    let fallback = crate::floating::fallback(&spectrum, profile.waves.choppiness)?;
    let water = thrum_water::Water::mapped(Arc::clone(&map), spectrum, profile)?;
    let query = water.query_mailbox().expect("river queries");
    let report = water.report();
    let mut controls = Controls::new();
    let camera = controls
        .slider("camera.river", "River view", 0.0..=16.0)
        .step(1.0)
        .add()?;
    let lake = controls
        .slider("lake.stage", "Lake stage (m)", 0.0..=1.0)
        .default(0.5)
        .add()?;
    let spring = controls
        .slider("spring.stage", "Spring stage", 0.0..=1.0)
        .default(0.5)
        .add()?;
    let tide = controls
        .slider("sea.stage", "Tide (m)", -0.5..=0.5)
        .default(0.0)
        .add()?;
    let stage_map = Arc::clone(&map);
    let game = SkyGame::new(controls, VIEWS[0].1, move |values, sky| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded view index"
        )]
        let view = values.get(camera).round() as usize;
        sky.camera = VIEWS[view].1;
        sky.sun_elevation_degrees = 48.0;
        sky.sun_azimuth_degrees = 165.0;
        sky.exposure_compensation_ev = 0.3;
        sky.temporal_inputs = true;
        let mut map = stage_map.lock().expect("river map");
        map.set_stage(1, f64::from(values.get(lake)))
            .expect("bounded lake stage");
        map.set_stage(0, f64::from(values.get(tide)))
            .expect("bounded tide");
        let gauge = (high - low).mul_add(f64::from(values.get(spring)), low);
        if (map.gauge(0) - gauge).abs() > 1e-12 {
            map.set_gauge(0, gauge).expect("bounded spring stage");
        }
    })
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(Duration::from_nanos(16_666_667))?;
    let terrain = scenery(map.lock().expect("river map").bed());
    let (game, _) = game.with_meshes_and_texture_lease(
        terrain,
        Some(("river island", include_str!("../island.wgsl"))),
        &[],
    )?;
    let (z, x) = forcing.wind_direction.sin_cos();
    let wind = glam::DVec2::new(x, z) * forcing.wind_speed;
    let (game, floats) =
        crate::floating::install_river_floats(game, Arc::clone(&map), &water, fallback, wind);
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok(Scene {
        game,
        report,
        map,
        files,
        query,
        floats,
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded island placements use GPU metres"
)]
fn scenery(bed: &Bed) -> Vec<MeshPlacement> {
    let mut meshes = vec![MeshPlacement::new("meshes/river-island", "textures/white")];
    let trees = [
        [168.8, -80.0, 0.75],
        [165.2, -78.0, 1.05],
        [190.4, -73.0, 0.91],
        [165.5, -64.0, 0.85],
        [191.0, -62.0, 0.69],
        [169.3, -54.0, 0.73],
        [188.3, -46.6, 0.84],
        [191.1, -45.2, 0.67],
        [169.9, -41.0, 1.06],
        [167.1, -36.5, 0.72],
        [190.0, -33.0, 0.99],
        [168.0, -14.0, 1.09],
        [208.0, 18.0, 1.0],
        [218.0, 19.0, 0.75],
        [251.0, -4.0, 0.64],
        [260.0, -5.0, 0.86],
        [272.0, 18.0, 0.95],
    ];
    for [x, z, scale] in trees {
        let height = bed
            .height([f64::from(x), f64::from(z)])
            .expect("island tree");
        for (mesh, colour) in [
            ("meshes/pine-bark", [0.17, 0.088, 0.048, 1.0]),
            ("meshes/pine-needles", [0.049, 0.103, 0.035, 1.0]),
        ] {
            meshes.push(
                MeshPlacement::new(mesh, "textures/white")
                    .at([x, height as f32, z])
                    .scale(scale)
                    .material(thrum_app::sky::CookedMaterial {
                        base_colour: colour,
                        roughness: 0.9,
                        metalness: 0.0,
                    }),
            );
        }
    }
    // Exposed rock on the solved shoulders. The lower part of each mesh
    // is buried in terrain above the stream's highest waterline.
    for [x, z, scale, inset] in [
        [175.0, -43.0, 1.05, 0.34],
        [174.5, -41.5, 0.70, 0.48],
        [183.7, -42.3, 1.30, 0.43],
        [184.6, -40.0, 0.77, 0.50],
        [174.4, -52.3, 0.81, 0.60],
        [185.4, -52.6, 0.91, 0.55],
    ] {
        let height = bed
            .height([f64::from(x), f64::from(z)])
            .expect("rock shoulder");
        meshes.push(
            MeshPlacement::new("meshes/rock", "textures/white")
                .at([x, height as f32 - inset, z])
                .scale(scale)
                .material(thrum_app::sky::CookedMaterial {
                    base_colour: [0.25, 0.235, 0.203, 1.0],
                    roughness: 0.97,
                    metalness: 0.0,
                }),
        );
    }
    meshes
}
