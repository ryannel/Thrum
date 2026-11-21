//! The island's terrain, water declarations and inspection cameras.
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_sea_model::{Forcing, Spectrum};
use thrum_water::Water;
use thrum_water_model::{Bed, Body, BodyKind, Map, Profile, Stage};

use crate::{SharedMap, SharedReport};

/// Weather selected when the island is loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wind {
    /// Motionless water for shoreline inspection.
    Calm,
    /// Wind and swell approaching the beach.
    Trade,
    /// Strong breeze with the trade swell.
    Strong,
    /// High wind waves and the prevailing swell.
    Gale,
}

impl Wind {
    /// Interpret the scene's startup option.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "calm" => Some(Self::Calm),
            "trade" => Some(Self::Trade),
            "strong" => Some(Self::Strong),
            "gale" => Some(Self::Gale),
            _ => None,
        }
    }

    /// Forcing shared by the island's map and wave field.
    pub fn forcing(self) -> Forcing {
        let mut forcing = crate::forcing();
        match self {
            Self::Calm => {
                forcing.wind_speed = 0.0;
                forcing.swells.clear();
            }
            Self::Trade | Self::Strong => {
                if matches!(self, Self::Strong) {
                    forcing.wind_speed = 13.0;
                }
                forcing.swells[0].significant_height = 1.1;
                forcing.swells[0].wavelength = 84.0;
            }
            Self::Gale => {
                forcing.wind_speed = 19.0;
                forcing.swells[0].significant_height = 2.6;
                forcing.swells[0].wavelength = 108.0;
            }
        }
        forcing
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "authored wind uses bounded metres per second"
    )]
    pub(crate) fn velocity(self) -> [f32; 2] {
        let forcing = self.forcing();
        let (z, x) = forcing.wind_direction.sin_cos();
        [x, z].map(|axis| (axis * forcing.wind_speed) as f32)
    }
}

const SIZE: [usize; 2] = [241, 181];
// The first two glTF buffer views emitted by make_island.py.
const COARSE_MESH: [usize; 2] = [300, 252];
const POND_CELLS: [usize; 2] = [40, 32];
const POND_SUBDIVISIONS: usize = 4;
const POSITIONS_BYTES: usize = (COARSE_MESH[0] * COARSE_MESH[1]
    + (POND_CELLS[0] * POND_SUBDIVISIONS + 1) * (POND_CELLS[1] * POND_SUBDIVISIONS + 1))
    * 12;
const INDEX_BYTES: usize = ((COARSE_MESH[0] - 1) * (COARSE_MESH[1] - 1)
    + POND_CELLS[0] * POND_CELLS[1] * (POND_SUBDIVISIONS * POND_SUBDIVISIONS - 1))
    * 6
    * 4;

/// The bed and bodies, using the same indexed triangles as the opaque terrain.
/// # Errors
/// Returns invalid map or spectrum authoring.
pub fn map(wind: Wind) -> Result<Map, Box<dyn Error>> {
    let mesh = include_bytes!("../assets/meshes/island.bin");
    let positions: Vec<[f64; 3]> = mesh[..POSITIONS_BYTES]
        .as_chunks::<12>()
        .0
        .iter()
        .map(|p| {
            let (components, _) = p.as_chunks::<4>();
            std::array::from_fn(|i| f64::from(f32::from_le_bytes(components[i])))
        })
        .collect();
    let triangles: Vec<_> = mesh[POSITIONS_BYTES..POSITIONS_BYTES + INDEX_BYTES]
        .as_chunks::<12>()
        .0
        .iter()
        .map(|indices| {
            let (indices, _) = indices.as_chunks::<4>();
            std::array::from_fn(|i| positions[u32::from_le_bytes(indices[i]) as usize])
        })
        .collect();
    let bed = Bed::from_triangles([-200.0, -225.0], 2.5, SIZE, -12.0, &triangles)
        .map_err(|e| format!("{e:?}"))?;
    let forcing = wind.forcing();
    let mut map = Map::prepare(
        bed,
        vec![
            Body {
                name: "Sea".into(),
                kind: BodyKind::Sea,
                level: 0.0,
                stage: Stage {
                    min: 0.0,
                    max: 0.0,
                    value: 0.0,
                },
                fetch: forcing.fetch,
                gains: [1.0; 4],
            },
            Body {
                name: "Lake".into(),
                kind: BodyKind::Basin { seed: [183.0, 5.0] },
                level: 12.0,
                stage: Stage {
                    min: 0.0,
                    max: 1.0,
                    value: 0.0,
                },
                fetch: 1_000.0,
                gains: [0.0; 4],
            },
            Body {
                name: "Pond".into(),
                kind: BodyKind::Basin {
                    seed: [104.0, 93.0],
                },
                level: 10.6,
                stage: Stage {
                    min: 0.0,
                    max: 0.0,
                    value: 0.0,
                },
                fetch: 65.0,
                gains: [0.0; 4],
            },
        ],
        Profile {
            margin_height: 3.5,
            swash_ratio: 0.7,
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    map.set_forcing(&forcing).map_err(|e| format!("{e:?}"))?;
    Ok(map)
}

/// Island checkpoint cameras. The close views measure two metres from still water.
pub const CAMERAS: [Camera; 6] = [
    Camera::look_at([-112.0, 2.0, 30.0], [-105.0, -1.0, -7.0], 65.0),
    Camera::look_at([171.0, 14.0, 34.0], [187.0, 11.2, 12.0], 64.0),
    Camera::look_at([95.0, 12.6, 106.0], [105.0, 10.0, 94.0], 63.0),
    Camera::look_at([83.0, 40.0, 140.0], [83.0, 2.0, 0.0], 103.0),
    Camera::look_at([127.0, 400.0, 35.0], [127.0, 0.0, 34.99], 65.0),
    Camera::look_at([208.0, 76.0, 10.0], [208.0, 0.0, 9.99], 65.0),
];
/// Forty-metre view of the same shore margin as the sand camera.
pub const BEACH_OVERHEAD: Camera = Camera::look_at([-106.0, 40.0, 22.0], [-101.0, 0.0, 12.0], 65.0);

/// Build the island with its cooked ground and the mapped water contribution.
/// # Errors
/// Returns invalid controls, content or water declarations.
/// # Panics
/// Panics if the scene's map lock is poisoned or its bounded stage control fails.
pub fn build(
    wind: Wind,
    draw_water: bool,
) -> Result<(SkyGame, SharedReport, SharedMap), Box<dyn Error>> {
    build_foam(wind, draw_water, false).map(|(game, report, map, _)| (game, report, map))
}

/// Diagnostics attached to the actual shoreline contribution.
pub struct FoamProbes {
    /// The rendered field's last submitted material banks.
    pub foam: thrum_water::foam::Tap,
    /// The public water query for shoreline columns.
    pub query: thrum_water::query::QueryMailbox,
}

/// Island game, its submitted report, map, and optional whitewater probes.
pub type FoamScene = (SkyGame, SharedReport, SharedMap, Option<FoamProbes>);

/// Build the island with optional transported whitewater.
/// # Errors
/// Returns invalid controls, content or water declarations.
/// # Panics
/// Panics if a scene lock is poisoned or a bounded control fails.
pub fn build_foam(wind: Wind, draw_water: bool, foam: bool) -> Result<FoamScene, Box<dyn Error>> {
    let map = Arc::new(Mutex::new(map(wind)?));
    let spectrum = Spectrum::build(&wind.forcing()).map_err(|e| format!("{e:?}"))?;
    let mut profile = crate::profile(&spectrum);
    if foam {
        profile.query = Some(thrum_water::query::QueryLimits {
            points: 512,
            max_age: 8,
        });
    }
    let mut water = Water::mapped(Arc::clone(&map), spectrum, profile)?;
    let probes = if foam {
        water.enable_foam(wind.velocity())?;
        Some(FoamProbes {
            foam: water.tap_foam(),
            query: water.query_mailbox().expect("enabled shore query"),
        })
    } else {
        None
    };
    let report = water.report();
    let display_map = Arc::clone(&map);
    let mut controls = Controls::new();
    let camera = controls
        .slider(
            "camera.island",
            if foam {
                "Beach / Lake / Pond / Hill / Map / Sill / Aerial"
            } else {
                "Beach / Lake / Pond / Hill / Map / Sill"
            },
            0.0..=if foam { 6.0 } else { 5.0 },
        )
        .step(1.0)
        .default(3.0)
        .add()?;
    let stage = controls
        .slider("lake.stage", "Lake stage", 0.0..=1.0)
        .step(0.01)
        .default(0.0)
        .add()?;
    let sun = controls
        .slider("sun.elevation", "Sun elevation", 5.0..=80.0)
        .default(60.0)
        .add()?;
    let game = SkyGame::new(controls, CAMERAS[3], move |c, sky| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded discrete camera control"
        )]
        let index = c.get(camera).round() as usize;
        sky.camera = CAMERAS.get(index).copied().unwrap_or(BEACH_OVERHEAD);
        sky.sun_elevation_degrees = c.get(sun);
        sky.sun_azimuth_degrees = 228.0;
        sky.exposure_compensation_ev = 0.3;
        sky.temporal_inputs = true;
        display_map
            .lock()
            .expect("island map")
            .set_stage(1, f64::from(c.get(stage)))
            .expect("bounded stage control");
    })
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(Duration::from_secs_f64(1.0 / 60.0))?;
    let (game, lease) = game.with_meshes_and_texture_lease(
        vec![MeshPlacement::new("meshes/island", "textures/white")],
        Some(("water island", include_str!("../island.wgsl"))),
        if foam { &["textures/foam-rank"] } else { &[] },
    )?;
    if foam {
        water.lend_foam_pattern(lease);
    }
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok((game, report, map, probes))
}
