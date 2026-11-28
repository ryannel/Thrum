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
    map_selection(forcing, store, false)
}

fn map_selection(
    forcing: &thrum_sea_model::Forcing,
    store: Option<&Path>,
    fall: bool,
) -> Result<Prepared, Box<dyn Error>> {
    let bed = crate::island::terrain_bed(TERRAIN)?;
    let files = files(&bed, store, fall)?;
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
    for file in &files {
        map.validate_falls(&file.fall_samples())
            .map_err(|error| model_error(&error))?;
    }
    map.set_forcing(forcing)
        .map_err(|error| model_error(&error))?;
    Ok((map, files))
}

#[cfg(not(target_arch = "wasm32"))]
fn files(bed: &Bed, store: Option<&Path>, fall: bool) -> Result<[RiverFile; 2], Box<dyn Error>> {
    store::load(bed, store.unwrap_or_else(|| Path::new(DEFAULT_STORE)), fall)
}

/// Generate the river files beside the cooked store for packaging.
/// # Errors
/// Returns invalid terrain, a refused solve or an I/O error.
#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_files(directory: &Path) -> Result<[String; 2], Box<dyn Error>> {
    let bed = crate::island::terrain_bed(TERRAIN)?;
    let files = store::load(&bed, directory, false)?;
    let (_, files) = prepare_map(&bed, files, &crate::channel::forcing(Wind::Calm))?;
    Ok(files.map(|file| key_hex(&file.key)))
}

/// Generate the fall selection's independently sampled spring file for packaging.
/// # Errors
/// Returns invalid terrain, a refused solve or an I/O error.
#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_fall_file(directory: &Path) -> Result<String, Box<dyn Error>> {
    let bed = crate::island::terrain_bed(TERRAIN)?;
    let files = store::load(&bed, directory, true)?;
    let (_, files) = prepare_map(&bed, files, &crate::channel::forcing(Wind::Calm))?;
    Ok(key_hex(&files[0].key))
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
fn files(_: &Bed, _: Option<&Path>, fall: bool) -> Result<[RiverFile; 2], Box<dyn Error>> {
    [FILES[usize::from(fall) * 2], FILES[1]]
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
pub const VIEWS: [(&str, Camera); 19] = [
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
        Camera::look_at([175.7, 14.45, -19.8], [183.5, 12.4, -19.8], 72.0),
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
    (
        "Riffle along",
        Camera::look_at([175.5, 20.0, -90.0], [180.0, 18.1, -82.0], 66.0),
    ),
    (
        "Riffle across",
        Camera::look_at([186.0, 20.0, -84.0], [180.0, 18.0, -84.0], 64.0),
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
    /// Submitted river whitewater and control over chart-loss births.
    pub foam: Option<(thrum_water::foam::Tap, thrum_water::foam::Input)>,
    /// Submitted interaction waves around the plunge pool.
    pub wake: Option<thrum_water::interaction::WitnessTap>,
    /// The stream and outlet floats.
    pub floats: Box<dyn Fn() -> [crate::floating::Body; 2]>,
}

/// Build the river selection, with paired water-on and water-off rendering arms.
/// # Errors
/// Returns invalid assets, water or controls.
/// # Panics
/// Panics if the water map lock is poisoned or its bounded controls fail.
pub fn build(wind: Wind, draw_water: bool, store: Option<&Path>) -> Result<Scene, Box<dyn Error>> {
    build_with_flow(wind, draw_water, store, true)
}

/// Build the river selection with a switch for comparing its original chart surface.
/// # Errors
/// Returns invalid assets, water or controls.
/// # Panics
/// Panics if the water map lock is poisoned or its bounded controls fail.
pub fn build_with_flow(
    wind: Wind,
    draw_water: bool,
    store: Option<&Path>,
    flow: bool,
) -> Result<Scene, Box<dyn Error>> {
    build_selection(wind, draw_water, store, flow, false)
}

/// Build the river selection at its waterfall, with interaction windows on its plunge pool.
/// # Errors
/// Returns invalid assets, fall records, water or controls.
pub fn build_fall(
    wind: Wind,
    draw_water: bool,
    store: Option<&Path>,
) -> Result<Scene, Box<dyn Error>> {
    build_selection(wind, draw_water, store, true, true)
}

fn build_selection(
    wind: Wind,
    draw_water: bool,
    store: Option<&Path>,
    flow: bool,
    falls: bool,
) -> Result<Scene, Box<dyn Error>> {
    let mut forcing = crate::channel::forcing(wind);
    // The exposed floats take the prevailing wind alongside the map's current.
    forcing.wind_direction = 0.42;
    let (mut map, files) = map_selection(&forcing, store, falls)?;
    if flow {
        map.set_flow_bands(Some(
            thrum_water_model::FlowBands::new(thrum_water_model::FlowBandsProfile::default())
                .map_err(|error| model_error(&error))?,
        ));
    }
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
    let spring_samples = files[0].gauges();
    let fall = falls.then(|| FallViews::new(&files[0], &map)).transpose()?;
    let map = Arc::new(Mutex::new(map));
    let spectrum = thrum_sea_model::Spectrum::build(&forcing)?;
    let mut profile = crate::profile(&spectrum);
    profile.optics = Some(thrum_water::Optics::default());
    profile.query = Some(thrum_water::query::QueryLimits {
        points: 24,
        max_age: 8,
    });
    if falls {
        profile.interaction = Some(thrum_water::interaction::Limits {
            impulses: 64,
            ..Default::default()
        });
    }
    let fallback = crate::floating::fallback(&spectrum, profile.waves.choppiness)?;
    let mut water = thrum_water::Water::mapped(Arc::clone(&map), spectrum, profile)?;
    if falls {
        let records = files
            .iter()
            .enumerate()
            .filter_map(|(id, file)| {
                let samples = file.fall_samples();
                (samples.segment_capacity() != 0).then_some((id, samples))
            })
            .collect();
        water.enable_falls(
            records,
            thrum_water::FallsProfile {
                sheet: thrum_water_model::SheetProfile {
                    breakup: 3.75,
                    breakup_exponent: 0.20,
                    tear: 0.028,
                },
                plunge: thrum_water_model::PlungeProfile {
                    wave_share: 0.75,
                    ..Default::default()
                },
            },
        )?;
    }
    let wake = falls.then(|| water.tap_interaction());
    let follow = fall
        .as_ref()
        .zip(water.interaction_mailbox())
        .map(|(view, mailbox)| (view.pool, mailbox));
    let (z, x) = forcing.wind_direction.sin_cos();
    let wind = glam::DVec2::new(x, z) * forcing.wind_speed;
    let foam = if flow {
        let input = water.enable_foam(wind.as_vec2().to_array())?;
        Some((water.tap_foam(), input))
    } else {
        None
    };
    let query = water.query_mailbox().expect("river queries");
    let report = water.report();
    let mut controls = Controls::new();
    let camera = controls
        .slider(
            "camera.river",
            "River view",
            if falls { 0.0..=8.0 } else { 0.0..=18.0 },
        )
        .step(1.0)
        .add()?;
    let lake = controls
        .slider("lake.stage", "Lake stage (m)", 0.0..=1.0)
        .default(if falls { 0.0 } else { 0.5 })
        .add()?;
    let spring = controls
        .slider("spring.stage", "Spring stage", 0.0..=1.0)
        .default(0.5)
        .add()?;
    let tide = controls
        .slider("sea.stage", "Tide (m)", -0.5..=0.5)
        .default(0.0)
        .add()?;
    let sun = controls
        .slider("sun.elevation", "Sun elevation", -15.0..=90.0)
        .default(48.0)
        .add()?;
    let azimuth = falls
        .then(|| {
            controls
                .slider("sun.azimuth", "Sun azimuth", 0.0..=360.0)
                .default(165.0)
                .add()
        })
        .transpose()?;
    let stage_map = Arc::clone(&map);
    let initial = fall.as_ref().map_or(VIEWS[0].1, |fall| fall.cameras[0]);
    let game = SkyGame::new(controls, initial, move |values, sky| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded view index"
        )]
        let view = values.get(camera).round() as usize;
        sky.camera = fall
            .as_ref()
            .map_or_else(|| VIEWS[view].1, |fall| fall.cameras[view]);
        sky.sun_elevation_degrees = values.get(sun);
        sky.sun_azimuth_degrees = azimuth.map_or(165.0, |control| values.get(control));
        sky.exposure_compensation_ev = 0.3;
        sky.temporal_inputs = true;
        let mut map = stage_map.lock().expect("river map");
        map.set_stage(1, f64::from(values.get(lake)))
            .expect("bounded lake stage");
        map.set_stage(0, f64::from(values.get(tide)))
            .expect("bounded tide");
        let spring = f64::from(values.get(spring));
        let gauge = if falls {
            let a = spring_samples[1];
            let b = spring_samples[2];
            if spring < 0.25 {
                (a - low).mul_add(spring * 4.0, low)
            } else if spring < 0.5 {
                (b - a).mul_add((spring - 0.25) * 4.0, a)
            } else {
                (high - b).mul_add((spring - 0.5) * 2.0, b)
            }
        } else {
            (high - low).mul_add(spring, low)
        };
        if (map.gauge(0) - gauge).abs() > 1e-12 {
            map.set_gauge(0, gauge).expect("bounded spring stage");
        }
    })
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(Duration::from_nanos(16_666_667))?;
    let terrain = scenery(map.lock().expect("river map").bed());
    let material = crate::island::island_material(0.0);
    let (game, _) =
        game.with_meshes_and_texture_lease(terrain, Some(("river island", &material)), &[])?;
    let (game, floats) = crate::floating::install_river_floats(
        game,
        Arc::clone(&map),
        &water,
        fallback,
        wind,
        follow,
    );
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
        foam,
        wake,
        floats,
    })
}

struct FallViews {
    cameras: [Camera; 9],
    pool: [f64; 2],
}

impl FallViews {
    fn new(file: &RiverFile, map: &Map) -> Result<Self, Box<dyn Error>> {
        use glam::{DVec2, DVec3};

        let falls = file.falls_at(file.gauge_range()[1]);
        let fall = falls
            .iter()
            .max_by_key(|fall| fall.segments.len())
            .ok_or("spring river has no fall")?;
        let path = &fall.segments[fall.segments.len() / 2].path;
        let lip = path[0];
        let foot = path[path.len() - 1];
        let start = DVec2::from_array(lip.place);
        let stop = DVec2::from_array(foot.place);
        let forward = (stop - start).normalize_or(DVec2::Y);
        let across = forward.perp();
        let world = |xz: DVec2, y: f64| DVec3::new(xz.x, y, xz.y).as_vec3().to_array();
        let pool = stop + forward * 2.0;
        let pool_level = map
            .sample(pool.to_array())
            .map_or(foot.height.max(12.0), |sample| sample.row.level.max(12.0));
        let middle_height = f64::midpoint(lip.height, pool_level);
        let centre = start.lerp(stop, 0.35);
        let bank_offset = fall
            .segments
            .iter()
            .flat_map(|segment| segment.lip)
            .map(|point| (DVec2::from_array(point) - stop).dot(across))
            .fold(0.0, f64::max);
        let bank_target = stop + 0.45 * forward;
        let bank_eye = pool_level + 2.0;
        let bank_aim = pool_level + 0.3;
        let bank_position =
            std::iter::successors(Some(bank_offset + 1.4), |&offset| Some(offset - 0.25))
                .take_while(|&offset| offset >= 0.5)
                .map(|offset| bank_target + 1.15 * forward + offset * across)
                .find(|&position| {
                    (0..=10).all(|step| {
                        let t = f64::from(step) * 0.1;
                        let place = position.lerp(bank_target, t).to_array();
                        let ray = (bank_aim - bank_eye).mul_add(t, bank_eye);
                        map.bed()
                            .height(place)
                            .is_some_and(|rock| rock < ray - 0.35)
                    })
                })
                .unwrap_or(bank_target + 1.15 * forward + 2.0 * across);
        let look = |position: DVec2, eye, target, target_height, angle| {
            let base = map.bed().height(position.to_array()).unwrap_or(eye);
            Camera::look_at(
                world(position, eye.max(base + 0.28)),
                world(target, target_height),
                angle,
            )
        };
        Ok(Self {
            cameras: [
                look(
                    centre + 5.0 * forward + across,
                    lip.height + 0.25,
                    centre,
                    middle_height,
                    59.0,
                ),
                look(
                    centre + 2.0 * forward + 0.2 * across,
                    lip.height + 0.22,
                    centre,
                    middle_height,
                    74.0,
                ),
                look(
                    centre + 3.0 * across + forward,
                    lip.height + 0.25,
                    centre,
                    middle_height,
                    65.0,
                ),
                look(
                    centre - 0.55 * forward + 0.7 * across,
                    lip.height + 0.35,
                    centre + 0.4 * forward,
                    middle_height,
                    84.0,
                ),
                look(
                    stop + 60.0 * forward + 4.0 * across,
                    lip.height + 8.0,
                    centre,
                    middle_height,
                    31.0,
                ),
                look(
                    stop + 200.0 * forward + 6.0 * across,
                    lip.height + 28.0,
                    centre,
                    middle_height,
                    26.0,
                ),
                look(bank_position, bank_eye, bank_target, bank_aim, 62.0),
                look(
                    pool + 3.0 * forward + across,
                    pool_level + 15.0,
                    pool,
                    pool_level,
                    50.0,
                ),
                look(
                    centre + 7.2 * forward + across,
                    lip.height + 0.5,
                    centre,
                    middle_height,
                    63.0,
                ),
            ],
            pool: pool.to_array(),
        })
    }
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
    // Expose the solved bed's rock shoulders at their wetted edges.
    // Sink each mesh into the bed so it reads as part of the same obstacle.
    for [x, z, scale, inset] in [
        [175.0, -43.0, 1.05, 0.34],
        [174.5, -41.5, 0.70, 0.48],
        [182.2, -43.1, 1.05, 0.32],
        [182.6, -40.0, 0.64, 0.46],
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
