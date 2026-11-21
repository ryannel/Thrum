//! Game content and controls for the water example. The engine contains no sea preset.
pub mod floating;
pub mod island;
pub mod settings;
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, Controls, CookedMaterial, Display, MeshPlacement, SkyGame};
use thrum_sea_model::{Forcing, Spectrum, Swell};
use thrum_water::{Profile, Water};
use thrum_water_model::{Bed, Body, BodyKind, Map, Stage};

/// The scene's map, shared with its stage control.
pub type SharedMap = Arc<Mutex<Map>>;
/// Preparation-only diagnostics retained after attaching the contribution.
pub type SharedReport = Arc<Mutex<thrum_water::WaterReport>>;

/// Authored inspection focus, on the actual long swell and wind-crest train.
pub const CAMERA_FOCUS: [f32; 2] = [132.0, 32.0];
/// Vertical field of view for the open-sea camera.
pub const CAMERA_FOV_DEGREES: f32 = 75.0;
/// Vertical drop over the camera's 100 m forward look vector (about 18 degrees).
pub const CAMERA_LOOK_DROP: f32 = 32.5;

/// Position the orbiting inspection camera at its height and compass bearing.
pub fn camera_position(height: f32, bearing_degrees: f32) -> [f32; 3] {
    let bearing = bearing_degrees.to_radians();
    let radius = 24.0 + (height - 2.0) * (16.0 / 38.0);
    [
        CAMERA_FOCUS[0] - bearing.sin() * radius,
        height,
        CAMERA_FOCUS[1] + bearing.cos() * radius,
    ]
}

/// The example's exposed trade wind and longer swell.
pub fn forcing() -> Forcing {
    Forcing {
        wind_speed: 8.0,
        wind_direction: 0.0,
        fetch: 100_000.0,
        swells: vec![Swell {
            significant_height: 2.5,
            wavelength: 200.0,
            direction: 0.25,
        }],
    }
}
/// This example's seawater and bounded long-wave deformation settings.
pub fn profile(spectrum: &Spectrum) -> Profile {
    let steepness =
        thrum_sea_model::contraction_bound(&spectrum.long, 1.0).unwrap_or(f64::INFINITY);
    let mut profile = Profile {
        deep_scatter: [0.004, 0.035, 0.065],
        ..Profile::default()
    };
    profile.waves.choppiness = (0.85 / steepness).min(4.0);
    profile
}
/// The scene's sea declaration. The empty bed is far below any wave.
/// # Errors
/// Returns invalid authored map or spectrum input.
pub fn water() -> Result<(Water, SharedMap), Box<dyn Error>> {
    let forcing = forcing();
    let map = Map::prepare(
        Bed::new([-1.0; 2], 2.0, [2, 2], vec![-1000.0; 4]).map_err(|e| format!("{e:?}"))?,
        vec![Body {
            name: "Trade sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: -2.0,
                max: 4.0,
                value: 0.0,
            },
            fetch: forcing.fetch,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 2.0,
            swash_ratio: 0.7,
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    let map = Arc::new(Mutex::new(map));
    let spectrum = Spectrum::build(&forcing).map_err(|e| format!("{e:?}"))?;
    let profile = profile(&spectrum);
    Ok((Water::new(Arc::clone(&map), 0, spectrum, profile)?, map))
}
/// Build the running scene with a 60 Hz world and Sun controls.
/// # Errors
/// Returns invalid controls, water declarations or clock settings.
pub fn build() -> Result<SkyGame, Box<dyn Error>> {
    build_with_report(false).map(|(game, _)| game)
}
/// Build the same scene with a preparation report for cost diagnostics.
/// # Errors
/// Returns the same authored-data failures as `build`.
/// # Panics
/// Panics if the scene's map lock is poisoned or its bounded stage control is rejected.
pub fn build_with_report(draw_float: bool) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    build_paired(draw_float, true)
}
/// Build one arm of the paired scene: only the water contribution differs.
/// # Errors
/// Returns the same authored-data failures as `build`.
/// # Panics
/// Panics if the scene's map lock is poisoned or its bounded stage control is rejected.
pub fn build_paired(
    draw_float: bool,
    draw_water: bool,
) -> Result<(SkyGame, SharedReport), Box<dyn Error>> {
    let mut controls = Controls::new();
    let sun = controls
        .slider("sun.elevation", "Sun elevation", -8.0..=80.0)
        .default(60.0)
        .display(Display::plain(1, "°"))
        .add()?;
    let azimuth = controls
        .slider("sun.azimuth", "Sun azimuth", 0.0..=360.0)
        .default(90.0)
        .add()?;
    let height = controls
        .slider("camera.height", "Camera height", 2.0..=40.0)
        .default(2.0)
        .add()?;
    let bearing = controls
        .slider("camera.bearing", "Camera bearing", 0.0..=360.0)
        .default(90.0)
        .add()?;
    let stage = controls
        .slider("sea.stage", "Sea stage", -2.0..=4.0)
        .default(0.0)
        .add()?;
    let (water, map) = water()?;
    let report = water.report();
    let game = SkyGame::new(
        controls,
        Camera::look_at(
            camera_position(2.0, 90.0),
            [
                camera_position(2.0, 90.0)[0] + 100.0,
                2.0 - CAMERA_LOOK_DROP,
                CAMERA_FOCUS[1],
            ],
            CAMERA_FOV_DEGREES,
        ),
        move |c, sky| {
            sky.exposure_compensation_ev = 0.7;
            sky.temporal_inputs = true;
            sky.sun_elevation_degrees = c.get(sun);
            sky.sun_azimuth_degrees = c.get(azimuth);
            let angle = c.get(bearing).to_radians();
            let h = c.get(height);
            let eye = camera_position(h, c.get(bearing));
            sky.camera = Camera::look_at(
                eye,
                [
                    eye[0] + angle.sin() * 100.0,
                    h - CAMERA_LOOK_DROP,
                    eye[2] - angle.cos() * 100.0,
                ],
                CAMERA_FOV_DEGREES,
            );
            map.lock()
                .expect("scene map")
                .set_stage(0, f64::from(c.get(stage)))
                .expect("bounded stage control");
        },
    )
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(Duration::from_secs_f64(1.0 / 60.0))?;
    let game = if draw_water {
        game.with_contribution(Box::new(water), || thrum_app::sky::ContentStatus::Drawn(1))
    } else {
        game
    };
    if draw_float {
        Ok((
            game.with_meshes(vec![
                MeshPlacement::new("meshes/float", "textures/white")
                    .at([CAMERA_FOCUS[0] - 12.0, -0.4, CAMERA_FOCUS[1]])
                    .scale(3.0)
                    .material(CookedMaterial {
                        base_colour: [0.18, 0.18, 0.18, 1.0],
                        roughness: 0.8,
                        metalness: 0.0,
                    }),
            ])?,
            report,
        ))
    } else {
        Ok((game, report))
    }
}
