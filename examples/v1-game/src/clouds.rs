//! The V1 game's cloud look and weather defaults; the engine supplies mechanisms.
use thrum_cloud_model::{Parameters, Profile, State, Type};
use thrum_clouds::{ShadeConfig, SunLevel, SunProfile, WeatherProfile};

pub(crate) const GROUND_SIZE: u32 = 1024;

/// Cloud capabilities selected by the V1 game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CloudSelection {
    /// Keep the original world, publication and render graph.
    #[default]
    Off,
    /// Weather and ground shade, with no visible cloud layer.
    Shade,
}

/// Game-authored Weather and Shade configuration, with no visible cloud layer.
pub fn shade_config() -> ShadeConfig {
    ShadeConfig {
        seed: 7,
        model: Profile {
            pitch: 3000.0,
            period: 1800.0,
            detail_mean: 0.5,
            erosion: 0.4,
        },
        types: vec![
            Type {
                base: 1600.0,
                depth: [300.0, 600.0],
                liquid_water: 0.00025,
                droplet_radius: 0.00001,
                deck: 1.0,
                profile: [0.2, 0.2],
            },
            Type {
                base: 1400.0,
                depth: [700.0, 1800.0],
                liquid_water: 0.0004,
                droplet_radius: 0.000_012,
                deck: 0.0,
                profile: [0.1, 0.3],
            },
        ],
        weather: WeatherProfile {
            size: 256,
            texels: vec![20.0, 80.0, 320.0, 3200.0],
            maximum_age: 0.5,
        },
        sun: SunProfile {
            size: 256,
            levels: [
                SunLevel {
                    texel: 80.0,
                    slices: 48,
                    maximum_age: 8.0 / 120.0,
                },
                SunLevel {
                    texel: 320.0,
                    slices: 32,
                    maximum_age: 32.0 / 120.0,
                },
                SunLevel {
                    texel: 1600.0,
                    slices: 16,
                    maximum_age: 128.0 / 120.0,
                },
            ],
            ground_size: GROUND_SIZE,
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
    }
}

/// Initial fair weather. Travel and weather time start at the world's tick zero.
/// # Panics
/// Only if the game's static defaults violate the model's invariants.
pub fn initial_state() -> State {
    State::new(
        Parameters {
            coverage: 0.45,
            kind: 1.0,
            wind: [10.0, 0.0],
            time_scale: 1.0,
        },
        5.0,
        0,
    )
    .expect("the game's weather defaults are finite and in range")
}

/// Checked game launch controls for the selected weather state.
/// `THRUM_CLOUD_WIND` sets X wind in m/s and `THRUM_CLOUD_TIME_SCALE` sets weather speed.
/// # Errors
/// Rejects malformed, nonfinite or out-of-domain launch values.
pub(crate) fn selected_state() -> Result<State, String> {
    let initial = initial_state();
    let number = |name: &str, fallback: f64| -> Result<f64, String> {
        match std::env::var(name) {
            Ok(value) => value
                .parse()
                .map_err(|_| format!("{name} must be a number")),
            Err(std::env::VarError::NotPresent) => Ok(fallback),
            Err(error) => Err(error.to_string()),
        }
    };
    State::new(
        Parameters {
            wind: [number("THRUM_CLOUD_WIND", 10.0)?, 0.0],
            time_scale: number("THRUM_CLOUD_TIME_SCALE", 1.0)?,
            ..initial.parameters()
        },
        5.0,
        0,
    )
    .map_err(|e| e.to_string())
}
