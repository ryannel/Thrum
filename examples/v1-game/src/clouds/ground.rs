//! Valley-owned ground condensate and the surface sampled by its terrain authoring tool.
use std::sync::Arc;
use thrum_cloud_model::{Affinity, Error, Fog, Ground, GroundProfile, Surface, SurfacePoint};
use thrum_clouds::GroundConfig;

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
}

impl GroundWeather {
    pub(crate) fn amounts(self) -> Vec<f64> {
        match self {
            Self::Clear => vec![0.0, 0.0],
            Self::HalfMist => vec![0.5, 0.0],
            Self::Mist => vec![1.0, 0.0],
            Self::Fog | Self::FogEdge | Self::FogOutside => vec![0.0, 1.0],
            Self::MistAndBank => vec![1.0, 0.25],
        }
    }
}

pub(crate) fn valley(preset: GroundWeather) -> Result<GroundConfig, Error> {
    let bank_centre_x = match preset {
        GroundWeather::FogEdge => 49.0,
        GroundWeather::FogOutside => 72.0,
        _ => 0.0,
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
                liquid_water: 0.000_054,
                droplet_radius: 10e-6,
                depth: 36.0,
                depression: 12.0,
                softness: 18.0,
                pooling: 1.0,
                drifting: false,
                affinity: Affinity::Land,
                centre: [0.0; 2],
                radius: [1.0; 2],
                noise: 0.18,
                noise_scale: 85.0,
                radiative: false,
            },
            Fog {
                liquid_water: (3.912 / 50.0 - 3.912 / 100_000.0) * (2.0 * 1000.0 * 10e-6 / 3.0),
                droplet_radius: 10e-6,
                depth: 32.0,
                depression: 0.0,
                softness: 11.0,
                pooling: 0.0,
                drifting: true,
                affinity: Affinity::Land,
                // The lookout bank thickens across the level shelf, giving
                // the scene a shallow edge before the polished receiver.
                centre: [bank_centre_x, 8.0],
                radius: [60.0, 47.0],
                noise: 0.08,
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
