//! The V1 game's cloud look and weather defaults; the engine supplies mechanisms.
use thrum_cloud_model::{Parameters, Profile, State, Type};
use thrum_clouds::{ShadeConfig, SunLevel, SunProfile, WeatherProfile};

/// Checks for the weather map produced at the game's authored scale.
#[cfg(feature = "proving")]
pub mod qualification;

mod library;
pub use library::PlacedSky;
pub(crate) use library::PlacementDemo;

pub(crate) const GROUND_SIZE: u32 = 1024;

/// The four weather presets authored for this game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherPreset {
    /// No procedural cloud.
    Clear,
    /// Scattered heaped cloud.
    Fair,
    /// A broken deck with heaped banks.
    Broken,
    /// A closed layered deck.
    Overcast,
}

impl WeatherPreset {
    #[cfg(any(not(target_arch = "wasm32"), feature = "proving"))]
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "clear" => Some(Self::Clear),
            "fair" => Some(Self::Fair),
            "broken" => Some(Self::Broken),
            "overcast" => Some(Self::Overcast),
            _ => None,
        }
    }

    /// Initial controls for this sky, with the game's usual wind and time scale.
    pub fn parameters(self) -> Parameters {
        let (coverage, kind) = match self {
            Self::Clear => (0.0, 1.0),
            Self::Fair => (0.50, 1.0),
            Self::Broken => (0.712, 0.42),
            Self::Overcast => (1.0, 0.0),
        };
        Parameters {
            coverage,
            kind,
            wind: [10.0, 0.0],
            time_scale: 1.0,
        }
    }
}

/// An optional, deterministic weather sequence in the V1 scene.
#[cfg(feature = "proving")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeatherSequence {
    /// Keep the selected initial parameters.
    #[default]
    Held,
    /// Ease through clear, fair, broken, overcast and clear over four minutes.
    Presets,
    /// Turn the selected wind 90 degrees after ten seconds.
    WindTurn,
}

#[cfg(feature = "proving")]
impl WeatherSequence {
    pub(crate) fn target(self, initial: Parameters, seconds: f64) -> Parameters {
        match self {
            Self::Held => initial,
            Self::WindTurn => Parameters {
                wind: if seconds < 10.0 {
                    initial.wind
                } else {
                    [-initial.wind[1], initial.wind[0]]
                },
                ..initial
            },
            Self::Presets => {
                let (from, to, start) = match seconds {
                    t if t < 60.0 => (WeatherPreset::Clear, WeatherPreset::Fair, 0.0),
                    t if t < 120.0 => (WeatherPreset::Fair, WeatherPreset::Broken, 60.0),
                    t if t < 180.0 => (WeatherPreset::Broken, WeatherPreset::Overcast, 120.0),
                    _ => (WeatherPreset::Overcast, WeatherPreset::Clear, 180.0),
                };
                let elapsed = seconds - start;
                let ease = |fraction: f64| {
                    let t = fraction.clamp(0.0, 1.0);
                    t * t * (3.0 - 2.0 * t)
                };
                // Let the deck start forming before losing the heaped banks.
                let coverage_duration = if from == WeatherPreset::Fair {
                    48.0
                } else {
                    60.0
                };
                let coverage_blend = ease(elapsed / coverage_duration);
                // Return to the fair type only after the layer has cleared.
                // Bringing up its cumulus sooner can grow new clouds in the
                // last thin patches of the dissolving deck.
                let kind_blend = if to == WeatherPreset::Clear {
                    ease((elapsed - 60.0) / 10.0)
                } else {
                    ease(elapsed / 60.0)
                };
                let a = from.parameters();
                let b = to.parameters();
                let coverage = if to == WeatherPreset::Clear {
                    // Spend most of the minute opening holes in the deck.
                    // A small coverage change here thins a broad area at once.
                    let thinning = 0.20 * ease(elapsed / 8.0);
                    let opening = 0.26 * ease((elapsed - 8.0) / 43.0);
                    let clearing = 0.54 * ease((elapsed - 51.0) / 9.0);
                    1.0 - (thinning + opening + clearing)
                } else {
                    (b.coverage - a.coverage).mul_add(coverage_blend, a.coverage)
                };
                Parameters {
                    coverage,
                    kind: (b.kind - a.kind).mul_add(kind_blend, a.kind),
                    ..initial
                }
            }
        }
    }
}

/// Cloud capabilities selected by the V1 game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CloudSelection {
    /// Keep the original world, publication and render graph.
    #[default]
    Off,
    /// Weather and ground shade, with no visible cloud layer.
    Shade,
    /// Weather, shade and the visible cloud layer.
    Sky,
}

impl CloudSelection {
    pub(crate) fn sky_profile(self, extent: [u32; 2]) -> Option<thrum_clouds::SkyProfile> {
        (self == Self::Sky).then_some(thrum_clouds::SkyProfile {
            extent,
            step: 15.0,
            light_samples: 3,
            refresh_stride: 1,
        })
    }
}

pub(crate) fn with_shade(
    contribution: Box<dyn thrum_render::contribution::SceneContribution>,
    input: Option<&thrum_clouds::WeatherInput>,
    sky: Option<thrum_clouds::SkyProfile>,
    shapes: Option<&PlacedSky>,
) -> Result<Box<dyn thrum_render::contribution::SceneContribution>, String> {
    let Some(input) = input else {
        return Ok(contribution);
    };
    let profile = shapes.map_or_else(shade_config, PlacedSky::profile);
    let shade = thrum_clouds::Shade::new(profile)
        .map_err(|error| error.to_string())?
        .with_input(input.clone());
    let shade = match sky {
        Some(profile) => shade.with_sky(profile).map_err(|error| error.to_string())?,
        None => shade,
    };
    let members = if let Some(shapes) = shapes {
        let (assets, shade) = shapes.shade(shade)?;
        vec![contribution, Box::new(assets), Box::new(shade)]
    } else {
        vec![contribution, Box::new(shade)]
    };
    Ok(Box::new(
        thrum_render::contribution::SceneContributions::new(members),
    ))
}

/// Game-authored Weather and Shade configuration, with no visible cloud layer.
pub fn shade_config() -> ShadeConfig {
    ShadeConfig {
        seed: 39,
        model: Profile {
            pitch: 4600.0,
            turret_pitch: 1300.0,
            turret_least: 0.22,
            turret_fade: 0.12,
            period: 1800.0,
            detail_mean: 0.5,
            erosion: 0.61,
        },
        types: vec![
            Type {
                base: 1600.0,
                depth: [465.0, 680.0],
                liquid_water: 0.00035,
                droplet_radius: 0.00001,
                deck: 1.0,
                // Let the billow field shape the underside over a deeper
                // condensation ramp. Keep the upper layer optically thick.
                profile: [0.65, 0.19],
            },
            Type {
                base: 1600.0,
                depth: [1450.0, 3800.0],
                liquid_water: 0.00032,
                droplet_radius: 0.000_012,
                deck: 0.0,
                profile: [0.25, 0.47],
            },
        ],
        weather: WeatherProfile {
            size: 256,
            texels: vec![20.0, 80.0, 320.0, 1600.0],
            maximum_age: 0.5,
        },
        sun: SunProfile {
            size: 256,
            // At a grazing angle, one height interval spans a long reach
            // along the Sun path. Finer nodes resolve the lighting relief
            // without increasing the height-independent ground texture size.
            levels: [
                SunLevel {
                    texel: 80.0,
                    slices: 96,
                    maximum_age: 8.0 / 120.0,
                },
                SunLevel {
                    texel: 320.0,
                    slices: 128,
                    maximum_age: 32.0 / 120.0,
                },
                SunLevel {
                    texel: 1600.0,
                    slices: 128,
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
    State::new(WeatherPreset::Fair.parameters(), 5.0, 0)
        .expect("the game's weather defaults are finite and in range")
}
