//! The island's terrain, water declarations and inspection cameras.
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_sea_model::{Component, Forcing, SeaError, Spectrum};
use thrum_water::Water;
use thrum_water_model::{
    Bed, Body, BodyKind, BreakerSection, Map, Profile, SectionPoint, Stage, SurfProfile,
};

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

    fn surf_forcing(self) -> Forcing {
        let mut forcing = self.forcing();
        if let Some(swell) = forcing.swells.first_mut() {
            let period: f64 = if self == Self::Trade { 6.5 } else { 10.0 };
            swell.wavelength = 9.81 * period.powi(2) / std::f64::consts::TAU;
            swell.direction = std::f64::consts::FRAC_PI_4;
            if matches!(self, Self::Trade) {
                swell.significant_height = 2.0;
                // A local trade wind over a limited fetch leaves room for the
                // incoming swell to organize the exposed beach.
                forcing.fetch = 18_000.0;
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

const SIZE: [usize; 2] = [289, 181];
// The first two glTF buffer views emitted by make_island.py.
const COARSE_MESH: [usize; 2] = [330, 252];
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
    island_map(&wind.forcing(), None)
}

/// The map and spectrum used by the island's surf scene.
/// # Errors
/// Returns invalid terrain, tide, surf or wave declarations.
pub fn surf_inputs(wind: Wind) -> Result<(Map, Spectrum), Box<dyn Error>> {
    let forcing = wind.surf_forcing();
    let map = island_map(&forcing, Some(true))?;
    let spectrum = surf_spectrum(wind, &forcing)?;
    Ok((map, spectrum))
}

fn surf_spectrum(wind: Wind, forcing: &Forcing) -> Result<Spectrum, SeaError> {
    if !matches!(wind, Wind::Trade) {
        return Spectrum::build(forcing);
    }

    // A repeatable swell set on the exposed beach. The sidebands vary the
    // height from one crest to the next while the wind sea keeps its detail.
    let swell = forcing.swells[0];
    let mut wind_sea = forcing.clone();
    wind_sea.swells.clear();
    let mut spectrum = Spectrum::build(&wind_sea)?;
    let (z, x) = swell.direction.sin_cos();
    let weights = [(0.88_f64, 0.3_f64, 0.9), (1.0, 0.9, 0.0), (1.13, 0.3, -0.9)];
    let norm = weights
        .iter()
        .map(|(_, weight, _)| weight.powi(2))
        .sum::<f64>()
        .sqrt();
    let amplitude = swell.significant_height / (2.0 * 2.0_f64.sqrt() * norm);
    for (slot, (period_ratio, weight, phase)) in spectrum.long.iter_mut().zip(weights) {
        // These lowest wavenumber slots have zero energy in the wind sea.
        debug_assert!(slot.amplitude < f64::EPSILON);
        let k = std::f64::consts::TAU / (swell.wavelength * period_ratio.powi(2));
        let mut component = Component {
            wave_vector: [k * x, k * z],
            amplitude: amplitude * weight,
            phase: 0.0,
        };
        component.phase = phase - component.anchored_phase([-173.0, 0.0], 4.0);
        *slot = component;
    }
    Ok(spectrum)
}

/// The island's shared draw and query parameters for its surf forcing.
pub fn surf_water_profile(spectrum: &Spectrum) -> thrum_water::Profile {
    let mut profile = crate::profile(spectrum);
    // Refraction raises the crest's local wavenumber. Leave room for the
    // section's forward throw without compounding it with deep-sea chop.
    profile.waves.choppiness = profile.waves.choppiness.min(1.0);
    profile
}

fn island_map(forcing: &Forcing, surf: Option<bool>) -> Result<Map, Box<dyn Error>> {
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
    let bed = Bed::from_triangles([-320.0, -225.0], 2.5, SIZE, -12.0, &triangles)
        .map_err(|e| format!("{e:?}"))?;
    let tide = if surf.is_some() { 0.5 } else { 0.0 };
    let mut map = Map::prepare(
        bed,
        vec![
            Body {
                name: "Sea".into(),
                kind: BodyKind::Sea,
                level: 0.0,
                stage: Stage {
                    min: -tide,
                    max: tide,
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
            // The exposed surf beach has a longer swell than the earlier
            // shore views. Use a thinner swash on both arms of the comparison.
            swash_ratio: if surf.is_some() { 0.45 } else { 0.7 },
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    map.set_forcing(forcing).map_err(|e| format!("{e:?}"))?;
    if surf == Some(true) {
        map.enable_surf(SurfProfile {
            section: island_breaker_section()?,
            ..SurfProfile::default()
        })
        .map_err(|e| format!("{e:?}"))?;
    }
    Ok(map)
}

/// The long island swell steepens on its shoreward face before it throws.
/// Ramp the skew in before the throw so the pitching rows share one height
/// curve and blend as a single monotone shear.
fn island_breaker_section() -> Result<BreakerSection, Box<dyn Error>> {
    const BORE: f64 = 2.0;
    const SAMPLES: u32 = 256;
    let indices = vec![
        0.0, 0.2, 0.4, 0.65, 0.75, 0.8, 0.85, 1.0, 1.15, 1.3, 1.5, 1.75, BORE, 8.0,
    ];
    let smooth = |u: f64| {
        let t = u.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let rows = indices
        .iter()
        .map(|&index| {
            let skew = smooth(index / 0.65);
            // Leave a shared row between the pre-break lift and the throw.
            // Interpolated pitching rows must have one throw at each height.
            let crest_lift = 0.28 * smooth(index / 0.48) * smooth((0.8 - index) / 0.22);
            let throw = 6.0 * smooth((index - 0.8) / 0.4) * smooth((BORE - index) / 0.85);
            (0..SAMPLES)
                .map(|i| {
                    let phase = std::f64::consts::TAU * f64::from(i) / f64::from(SAMPLES);
                    let angle = phase + 0.65 * skew * phase.sin();
                    let wave = (angle + 0.42 * skew * angle.sin().powi(2)).cos();
                    let height = wave + crest_lift * wave.max(0.0).powi(2);
                    SectionPoint {
                        height,
                        throw: throw * (1.0 + wave) * 0.5,
                    }
                })
                .collect()
        })
        .collect();
    BreakerSection::new(indices, rows, BORE).map_err(|e| format!("{e:?}").into())
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

/// Sand, overhead, offshore, lee, whole-island and specialised surf inspection views.
pub const SURF_VIEWS: [Camera; 10] = [
    CAMERAS[0],
    Camera::look_at([-126.0, 40.0, -17.0], [-142.0, 0.0, -54.0], 66.0),
    Camera::look_at([-1126.0, 180.0, -54.0], [-126.0, 0.0, -54.0], 3.8),
    Camera::look_at([253.0, 40.0, 171.0], [237.0, 0.0, 154.0], 59.0),
    Camera::look_at([-101.0, 40.0, -94.0], [-127.0, 0.0, -98.0], 79.0),
    Camera::look_at([360.0, 300.0, 350.0], [125.0, 0.0, 105.0], 72.0),
    Camera::look_at([-101.0, 2.0, -46.0], [-147.0, -0.8, -64.0], 65.0),
    // The 2 m coarse cells put the near window edge at z = -56 m here.
    Camera::look_at([-165.0, 90.0, 200.0], [-128.0, 0.0, -56.0], 20.0),
    // Look along the beach so perspective preserves the relative cross-shore gaps.
    Camera::look_at([-188.0, 105.0, 93.0], [-188.0, 0.0, -8.0], 60.0),
    // Look across the gale set so cross-shore distance and crest relief are visible together.
    Camera::look_at([-175.0, 8.0, -135.0], [-175.0, 0.0, -54.0], 48.0),
];

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
    build_island(wind, draw_water, foam, None)
}

/// Use the island's surf profile and tide and beach inspection controls.
/// # Errors
/// Returns invalid controls, content or water declarations.
pub fn build_surf(wind: Wind, draw_water: bool) -> Result<FoamScene, Box<dyn Error>> {
    build_surf_paired(wind, draw_water, true)
}

/// Build a same-view, same-forcing arm with or without the surf capability.
/// # Errors
/// Returns invalid controls, content or water declarations.
pub fn build_surf_paired(
    wind: Wind,
    draw_water: bool,
    surf: bool,
) -> Result<FoamScene, Box<dyn Error>> {
    build_island(wind, draw_water, true, Some(surf))
}

fn build_island(
    wind: Wind,
    draw_water: bool,
    foam: bool,
    surf: Option<bool>,
) -> Result<FoamScene, Box<dyn Error>> {
    let surf_route = surf.is_some();
    let forcing = if surf_route {
        wind.surf_forcing()
    } else {
        wind.forcing()
    };
    let map = island_map(&forcing, surf)?;
    let spectrum = if surf_route {
        surf_spectrum(wind, &forcing)
    } else {
        Spectrum::build(&forcing)
    }?;
    let map = Arc::new(Mutex::new(map));
    let mut profile = if surf_route {
        surf_water_profile(&spectrum)
    } else {
        crate::profile(&spectrum)
    };
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
            if surf_route {
                "Sand/Beach/Sea/Lee/Point/Map/Out/Foam/Approach/Set"
            } else if foam {
                "Beach / Lake / Pond / Hill / Map / Sill / Aerial"
            } else {
                "Beach / Lake / Pond / Hill / Map / Sill"
            },
            0.0..=if surf_route {
                9.0
            } else if foam {
                6.0
            } else {
                5.0
            },
        )
        .step(1.0)
        .default(if surf_route { 1.0 } else { 3.0 })
        .add()?;
    let stage = controls
        .slider("lake.stage", "Lake stage", 0.0..=1.0)
        .step(0.01)
        .default(0.0)
        .add()?;
    let sun = controls
        .slider("sun.elevation", "Sun elevation", 5.0..=80.0)
        .default(if surf_route && wind == Wind::Trade {
            38.0
        } else {
            60.0
        })
        .add()?;
    let tide = surf_route
        .then(|| {
            controls
                .slider("sea.stage", "Tide (m)", -0.5..=0.5)
                .step(0.01)
                .default(0.0)
                .add()
        })
        .transpose()?;
    let game = SkyGame::new(controls, CAMERAS[3], move |c, sky| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded discrete camera control"
        )]
        let index = c.get(camera).round() as usize;
        sky.camera = if surf_route {
            SURF_VIEWS[index]
        } else {
            CAMERAS.get(index).copied().unwrap_or(BEACH_OVERHEAD)
        };
        sky.sun_elevation_degrees = c.get(sun);
        sky.sun_azimuth_degrees = if surf_route && wind == Wind::Trade {
            224.0
        } else {
            228.0
        };
        sky.exposure_compensation_ev = 0.3;
        sky.temporal_inputs = true;
        let mut map = display_map.lock().expect("island map");
        map.set_stage(1, f64::from(c.get(stage)))
            .expect("bounded stage control");
        if let Some(tide) = tide {
            map.set_stage(0, f64::from(c.get(tide)))
                .expect("bounded tide control");
        }
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
