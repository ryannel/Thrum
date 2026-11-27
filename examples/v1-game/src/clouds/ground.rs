//! Valley-owned ground condensate and the surface sampled by its terrain authoring tool.
use std::sync::Arc;
use thrum_cloud_model::{Affinity, Error, Fog, Ground, GroundProfile, Surface, SurfacePoint};
use thrum_clouds::GroundConfig;

// One authored valley day at 1×. The cooling model receives solar seconds
// within that day, while cloud advection still uses actual simulation seconds.
pub(crate) const SOLAR_DAY_SECONDS: f64 = 300.0;

#[expect(
    clippy::cast_precision_loss,
    reason = "the game uses f64 seconds for its 60 Hz presentation clock"
)]
pub(crate) fn sun(time: f32, tick: u64, alpha: f32) -> crate::environment::SunSample {
    let offset = (time - 0.5).rem_euclid(1.0);
    let offset = if offset >= 1.0 { 0.0 } else { offset };
    let mut sun = crate::environment::DayCycle::sample(0, 0.0, offset);
    if tick != 0 {
        sun.simulation_time_seconds = ((tick - 1) as f64 + f64::from(alpha.clamp(0.0, 1.0))) / 60.0;
    }
    sun
}

/// Authored control settings for the valley's mist and fog bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundWeather {
    /// Amount zero, keeping the Ground resources declared.
    Clear,
    /// Valley mist at half amount.
    HalfMist,
    /// Valley mist at full amount.
    Mist,
    /// Fog with 50 m visual range in the bank's core.
    Fog,
    /// The same fog bank with its edge crossing the lookout's receivers.
    FogEdge,
    /// The same fog bank displaced beyond the lookout's receivers.
    FogOutside,
    /// Full mist and a thinner bank in the same valley.
    MistAndBank,
    /// A valley-wide dawn mist with a separate bank at the far bend.
    FarMist,
    /// Radiative mist driven by the game's solar day.
    DawnMist,
    /// An advected fog bank under moist air.
    SeaFog,
    /// Cloud below the valley's summits, with dawn mist beneath it.
    LowDeck,
    /// A bank on the valley shelf at an authored distance from the inspection pose.
    HandoverBank(u16),
}

impl GroundWeather {
    fn amounts(self) -> [f64; 2] {
        match self {
            Self::Clear => [0.0, 0.0],
            Self::HalfMist => [0.5, 0.0],
            Self::Mist | Self::DawnMist => [1.0, 0.0],
            Self::Fog | Self::FogEdge | Self::FogOutside | Self::HandoverBank(_) => [0.0, 1.0],
            Self::MistAndBank => [1.0, 0.25],
            Self::SeaFog => [0.0, 0.75],
            Self::FarMist => [1.0, 0.35],
            Self::LowDeck => [0.0, 0.35],
        }
    }
}

/// Targets shared by the game controls and the weather tick, in engine units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundTargets {
    /// Amounts of valley mist and the advected bank.
    pub amounts: [f64; 2],
    /// Moisture available to form radiative mist.
    pub moisture: f64,
    /// Clear-air visual range in metres.
    pub visual_range: f64,
    /// Spectral exponent of the clear-air aerosol.
    pub angstrom_exponent: f64,
    /// Enable the cooling and warming cycle for the mist row.
    pub radiative_day: bool,
}

impl GroundWeather {
    pub(crate) fn targets(self) -> GroundTargets {
        let (moisture, visual_range, radiative_day) = match self {
            Self::DawnMist => (1.0, 60_000.0, true),
            Self::SeaFog => (0.9, 35_000.0, false),
            _ => (0.0, 244_000.0, false),
        };
        GroundTargets {
            amounts: self.amounts(),
            moisture,
            visual_range,
            angstrom_exponent: 0.0,
            radiative_day,
        }
    }
}

impl GroundTargets {
    pub(crate) fn apply(self, state: &mut thrum_cloud_model::GroundState) -> Result<(), Error> {
        if !self.valid() {
            return Err(Error::Invalid);
        }
        state.set_targets(&self.amounts, self.moisture, self.visual_range)?;
        state.set_angstrom_target(self.angstrom_exponent)?;
        state.set_radiative_day(self.radiative_day);
        Ok(())
    }

    pub(crate) fn valid(self) -> bool {
        self.amounts
            .into_iter()
            .chain([self.moisture])
            .all(|value| (0.0..=1.0).contains(&value))
            && (3000.0..=300_000.0).contains(&self.visual_range)
            && (0.0..=2.0).contains(&self.angstrom_exponent)
    }

    pub(crate) fn state(self) -> Result<thrum_cloud_model::GroundState, Error> {
        let mut state = thrum_cloud_model::GroundState::new(
            self.amounts.to_vec(),
            self.moisture,
            self.visual_range,
        )?;
        self.apply(&mut state)?;
        Ok(state)
    }
}

pub(crate) fn valley(preset: GroundWeather) -> Result<GroundConfig, Error> {
    let bank_centre_x = match preset {
        GroundWeather::FogEdge => 49.0,
        GroundWeather::FogOutside => 72.0,
        _ => 0.0,
    };
    let (bank_centre, bank_radius, bank_depth, bank_softness, bank_noise) = match preset {
        GroundWeather::FarMist
        | GroundWeather::DawnMist
        | GroundWeather::SeaFog
        | GroundWeather::LowDeck => ([120.0, -2350.0], [1050.0, 340.0], 88.0, 39.0, 0.15),
        // The fixed eye is at z = 810. The near face starts one radius
        // north of the centre and the camera sees both a shoulder and its flank.
        GroundWeather::HandoverBank(distance) => (
            [
                -510.0 + 0.14 * (f64::from(distance) + 150.0),
                660.0 - f64::from(distance),
            ],
            [118.0, 150.0],
            60.0,
            16.0,
            0.0,
        ),
        _ => ([bank_centre_x, 8.0], [60.0, 47.0], 32.0, 11.0, 0.08),
    };
    let fog = Ground::new(
        GroundProfile {
            shore_margin: 40.0,
            calm: [2.0, 8.0],
            twilight: [-6.0, 10.0],
            cooling_seconds: 10_800.0,
            warming_seconds: 3600.0,
        },
        vec![
            Fog {
                liquid_water: match preset {
                    GroundWeather::DawnMist => 0.000_018,
                    GroundWeather::FarMist | GroundWeather::SeaFog => 0.000_012,
                    _ => 0.000_054,
                },
                droplet_radius: 10e-6,
                depth: if matches!(
                    preset,
                    GroundWeather::FarMist | GroundWeather::DawnMist | GroundWeather::SeaFog
                ) {
                    37.0
                } else {
                    36.0
                },
                depression: 12.0,
                softness: 18.0,
                pooling: 1.0,
                drifting: false,
                affinity: Affinity::Land,
                centre: [0.0; 2],
                radius: [1.0; 2],
                noise: 0.18,
                noise_scale: 85.0,
                radiative: true,
            },
            Fog {
                liquid_water: (3.912 / 50.0 - 3.912 / 100_000.0) * (2.0 * 1000.0 * 10e-6 / 3.0),
                droplet_radius: 10e-6,
                depth: bank_depth,
                depression: 0.0,
                softness: bank_softness,
                pooling: 0.0,
                drifting: true,
                affinity: Affinity::Land,
                // The lookout bank thickens across the level shelf, giving
                // the scene a shallow edge before the polished receiver.
                centre: bank_centre,
                radius: bank_radius,
                noise: bank_noise,
                noise_scale: 30.0,
                radiative: false,
            },
        ],
    )?;
    let grid = include_bytes!("../../assets/showcase/surface-field.bin");
    let read_float = |i| {
        f64::from(f32::from_le_bytes(
            grid[i..i + 4].try_into().expect("surface header"),
        ))
    };
    let read_size =
        |i| u32::from_le_bytes(grid[i..i + 4].try_into().expect("surface header")) as usize;
    let nodes: Vec<_> = grid[20..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&bits| SurfacePoint {
            ground: f64::from(f32::from_le_bytes(bits)),
            water: None,
        })
        .collect();
    let surface = Surface::new(
        [read_float(0), read_float(4)],
        [read_size(12), read_size(16)],
        read_float(8),
        3000.0,
        &nodes,
    )?;
    Ok(GroundConfig {
        surface: Arc::new(surface),
        fog: Arc::new(fog),
    })
}
