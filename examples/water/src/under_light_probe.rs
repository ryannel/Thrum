//! A portable capture fixture for the shared refracted Sun path.

use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_sea_model::{Cascade, Spectrum};
use thrum_water::{Optics, Profile, Water};
use thrum_water_model::{Bed, Body, BodyKind, Map, Stage};

const LEVEL: f32 = 6.0;

#[derive(Clone, Copy)]
pub(crate) enum Probe {
    ClearBed,
    ShadowedBed,
    ClearWater,
    ShadowedWater,
    Meter,
    MeterUnit,
    MeterHalf,
    RippleWithNet,
    RippleWithoutNet,
}

impl Probe {
    pub(crate) fn named(name: &str) -> Option<Self> {
        Some(match name {
            "clear-bed" => Self::ClearBed,
            "shadowed-bed" => Self::ShadowedBed,
            "clear-water" => Self::ClearWater,
            "shadowed-water" => Self::ShadowedWater,
            "meter" => Self::Meter,
            "meter-unit" => Self::MeterUnit,
            "ripple-with-net" => Self::RippleWithNet,
            "ripple-without-net" => Self::RippleWithoutNet,
            "meter-half" => Self::MeterHalf,
            _ => return None,
        })
    }

    pub(crate) fn game(self) -> Result<SkyGame, Box<dyn Error>> {
        let shafts = matches!(self, Self::ClearWater | Self::ShadowedWater);
        let shadowed = matches!(self, Self::ShadowedBed | Self::ShadowedWater);
        let ripple = matches!(self, Self::RippleWithNet | Self::RippleWithoutNet);
        let level = if ripple { 1.1 } else { LEVEL };
        let map = Map::prepare(
            Bed::new([-28.0, -24.0], 56.0, [2, 2], vec![0.0; 4])
                .map_err(|error| format!("{error:?}"))?,
            vec![Body {
                kind: BodyKind::Sea,
                name: "Shadow basin".into(),
                level: f64::from(level),
                stage: Stage {
                    min: 0.0,
                    max: 0.0,
                    value: 0.0,
                },
                fetch: 100.0,
                gains: [1.0; 4],
            }],
            thrum_water_model::Profile {
                margin_height: 1.0,
                swash_ratio: 0.7,
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        let spectrum = Spectrum {
            long: Vec::new(),
            cascades: [
                Cascade::new(160.0, &[])?,
                Cascade::new(37.0, &[])?,
                Cascade::new(
                    9.0,
                    if ripple {
                        &[(5, 0, 0.005, 0.0), (0, 4, 0.003, 0.7)]
                    } else {
                        &[]
                    },
                )?,
            ],
            tail_slope_variance: 0.0,
        };
        let water = Water::new(
            Arc::new(Mutex::new(map)),
            0,
            spectrum,
            Profile {
                caustics: !matches!(self, Self::RippleWithoutNet),
                shafts,
                underwater: shafts,
                optics: shafts.then_some(Optics::default()),
                deep_scatter: [0.025, 0.075, 0.13],
                ..Profile::default()
            },
        )?;
        let mut controls = Controls::new();
        let station = controls
            .slider("camera.station", "Station", -20.0..=-2.0)
            .step(0.05)
            .default(-9.0)
            .add()?;
        let camera = move |x| {
            let eye = if ripple { 0.85 } else { 0.25 };
            Camera::look_at([x, eye, 0.0], [x, 0.0, 0.01], 38.0)
        };
        let game = SkyGame::new(controls, camera(-9.0), move |controls, sky| {
            sky.camera = camera(controls.get(station));
            sky.sun_elevation_degrees = if ripple { 75.0 } else { 20.0 };
            sky.sun_azimuth_degrees = 90.0;
        })
        .with_ground(Some([0.09; 3]));
        let game = if shadowed {
            // The authored roof is a thin closed box. Place its lower face 15 cm
            // above the level, centred on the origin horizontally.
            let scale = 1.4;
            let box_over_water = MeshPlacement::new("meshes/rain-yard-roof", "textures/white")
                .scale(scale)
                .at([
                    -152.35 * scale,
                    LEVEL + 0.15 - 20.32 * scale,
                    -68.025 * scale,
                ]);
            game.with_meshes(vec![box_over_water])?
        } else {
            game
        };
        let game = match self {
            Self::MeterUnit => game.with_meter_medium(|_, _, _| [1.0; 3]),
            Self::MeterHalf => game.with_meter_medium(|_, _, _| [0.5; 3]),
            _ => game,
        };
        Ok(game
            .with_sun_path(water.sun_path_provider())
            .with_contribution(Box::new(water), || ContentStatus::Drawn(1)))
    }
}
