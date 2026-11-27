//! The water scene's authored weather, surface field and fog table.
use std::{error::Error, sync::Arc};
use thrum_app::sky::{Camera, ContentStatus, SkyGame};
use thrum_cloud_model::{
    Affinity, Fog, Ground, GroundProfile, GroundState, Parameters, Profile, State, Surface,
    SurfacePoint, Type,
};
use thrum_clouds::{
    GroundConfig, Shade, ShadeConfig, SkyProfile, SunLevel, SunProfile, WeatherInput,
    WeatherProfile,
};
use thrum_render::atmosphere::Aerosol;
use thrum_water_model::Map;

use crate::{SharedMap, SharedReport, island::Wind};

/// Authored cloud preset and the scene that supplies its surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    /// Fair-weather cumulus over the open sea.
    Fair,
    /// Broken cloud over the open sea.
    Broken,
    /// A bank approaching the island from offshore.
    SeaFog,
    /// Steam mist over the island lake at dawn.
    DawnMist,
}

/// View of the bank from the deck or the shore.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BankView {
    /// Deck camera two kilometres from the leading shoulder.
    #[default]
    Offshore,
    /// Deck camera just outside the leading shoulder.
    Front,
    /// Deck camera inside the bank.
    Inside,
    /// View of the approaching bank and beach.
    Shore,
}

/// Opt-in weather and its authored inspection settings.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent authored inspection switches"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Weather preset.
    pub weather: Weather,
    /// Select the 10° Sun.
    pub low_sun: bool,
    /// Use a 40 m open-sea or inside-the-bank camera.
    pub high: bool,
    /// Use 8× weather time.
    pub fast: bool,
    /// Use the sea scene with the moving craft in view.
    pub craft_view: bool,
    /// Fog bank camera.
    pub bank_view: BankView,
    /// View the lake an hour after sunrise.
    pub after_sunrise: bool,
}

/// A prepared water scene and its optional weather publication and ground model.
pub struct Scene {
    /// The ordinary running sky kit scene.
    pub game: SkyGame,
    /// Water pass report.
    pub report: SharedReport,
    /// Read-only handles for the scene's submitted cloud products.
    pub weather: Option<WeatherInput>,
    /// The ground configuration shared by the CPU and GPU.
    pub ground: Option<GroundConfig>,
}

impl Selection {
    /// Read the default-off selection from the native host's environment.
    pub fn from_variables(variable: impl Fn(&str) -> Option<String>) -> Option<Self> {
        Self::parse(&|(_, name)| variable(name))
    }

    /// Read the default-off selection from the browser's page query.
    pub fn from_query(query: &str) -> Option<Self> {
        Self::parse(&|(key, _)| crate::settings::query_value(query, key))
    }

    fn parse(get: &impl Fn((&str, &str)) -> Option<String>) -> Option<Self> {
        let weather = match get(("clouds", "THRUM_WATER_CLOUDS")).as_deref()? {
            "fair" => Weather::Fair,
            "broken" => Weather::Broken,
            "sea-fog" => Weather::SeaFog,
            "dawn-mist" => Weather::DawnMist,
            _ => return None,
        };
        let bank_view = match get(("bank-view", "THRUM_WATER_BANK_VIEW")).as_deref() {
            Some("front") => BankView::Front,
            Some("inside") => BankView::Inside,
            Some("shore") => BankView::Shore,
            _ => BankView::Offshore,
        };
        Some(Self {
            weather,
            low_sun: get(("cloud-sun", "THRUM_WATER_CLOUD_SUN")).as_deref() == Some("low"),
            high: get(("cloud-height", "THRUM_WATER_CLOUD_HEIGHT")).as_deref() == Some("40"),
            fast: get(("weather-speed", "THRUM_WATER_WEATHER_SPEED")).as_deref() == Some("8"),
            craft_view: get(("cloud-view", "THRUM_WATER_CLOUD_VIEW")).as_deref() == Some("craft"),
            bank_view,
            after_sunrise: get(("mist-time", "THRUM_WATER_MIST_TIME")).as_deref() == Some("after"),
        })
    }

    pub(crate) fn sun(self) -> f32 {
        match self.weather {
            Weather::DawnMist if self.after_sunrise => 12.0,
            Weather::DawnMist => 1.0,
            _ if self.low_sun => 10.0,
            _ => 60.0,
        }
    }

    pub(crate) fn height(self) -> f32 {
        if self.high { 40.0 } else { 2.0 }
    }

    pub(crate) fn open_sea_offset(self) -> [f32; 3] {
        // Frame the edge of the GPU Sun-path footprint where it crosses the
        // swell in the direction of the Sun. Both heights share the same view.
        match (self.weather, self.low_sun) {
            (Weather::Fair, false) => [-308.0, 0.0, -32.0],
            (Weather::Fair, true) => [-4348.0, 0.0, -232.0],
            (Weather::Broken, true) => [252.0, 0.0, 8.0],
            _ => [0.0; 3],
        }
    }

    /// Build the scene used by both shells. The control arm uses the same scene settings
    /// with no weather state, provider, or cloud contribution.
    /// # Errors
    /// Propagates invalid scene assets, terrain, weather or controls.
    pub fn build(self, draw_craft: bool, enabled: bool) -> Result<Scene, Box<dyn Error>> {
        let (game, report, map) = match self.weather {
            Weather::Fair | Weather::Broken if self.craft_view => {
                let scene =
                    crate::floating::build(Wind::Trade, false, 1.0, std::f32::consts::PI, false)?;
                (scene.game, scene.report, None)
            }
            Weather::Fair | Weather::Broken => {
                let (game, report) =
                    crate::build_open_sea(draw_craft, true, Some(Wind::Trade), Some(self))?;
                (game, report, None)
            }
            Weather::SeaFog | Weather::DawnMist => {
                let (game, report, map) =
                    crate::island::build_cloud_view(Wind::Trade, self, draw_craft)?;
                (game, report, Some(map))
            }
        };
        let game = game.with_temporal_sampling(true);
        if enabled {
            let (game, weather, ground) = self.install(game, map.as_ref())?;
            Ok(Scene {
                game,
                report,
                weather: Some(weather),
                ground,
            })
        } else {
            Ok(Scene {
                game,
                report,
                weather: None,
                ground: None,
            })
        }
    }

    pub(crate) fn camera(self) -> Camera {
        match self.weather {
            Weather::DawnMist => Camera::look_at([171.0, 14.0, 34.0], [188.0, 13.0, 12.0], 57.0),
            _ => match self.bank_view {
                BankView::Offshore => {
                    Camera::look_at([-4090.0, 2.0, 150.0], [-1000.0, 22.0, 22.0], 55.0)
                }
                BankView::Front => {
                    Camera::look_at([-2040.0, 2.0, 126.0], [-1150.0, 25.0, 22.0], 58.0)
                }
                BankView::Inside => {
                    Camera::look_at([-1320.0, self.height(), 100.0], [-110.0, 8.0, 10.0], 60.0)
                }
                BankView::Shore => {
                    Camera::look_at([-270.0, 17.0, 145.0], [-216.0, 3.0, 26.0], 72.0)
                }
            },
        }
    }

    fn install(
        self,
        game: SkyGame,
        map: Option<&SharedMap>,
    ) -> Result<(SkyGame, WeatherInput, Option<GroundConfig>), Box<dyn Error>> {
        let ground = map
            .map(|map| {
                let map = map.lock().map_err(|_| "water map")?;
                Ok::<_, Box<dyn Error>>(self.ground(&map)?)
            })
            .transpose()?;
        let mut state = self.state(ground.as_ref())?;
        let mut shade = Shade::new(profile())?.with_sky_following_extent(SkyProfile {
            extent: [64, 36],
            step: 15.0,
            light_samples: 3,
            refresh_stride: 1,
        })?;
        if let Some(ground) = &ground {
            shade = shade.with_ground(ground.clone());
        }
        let provider = shade.provider()?;
        let input = shade.input();
        let publication = input.publisher();
        let aerosol = ground.as_ref().map(|_| Aerosol::default());
        let radiative = ground
            .as_ref()
            .filter(|_| self.weather == Weather::DawnMist)
            .map(|ground| Arc::clone(&ground.fog));
        let sun = f64::from(self.sun());
        let game = game
            .with_sky_overlay(aerosol)
            .with_sun_optical_depth(provider)
            .and_fixed_update(move |tick, step| {
                if tick != 0 {
                    if let Some(fog) = &radiative {
                        state.step_day(tick, step.as_secs_f64(), sun, fog)
                    } else {
                        state.step(tick, step.as_secs_f64())
                    }
                    .expect("fixed weather tick");
                }
                publication
                    .publish(Arc::new(state.clone()))
                    .expect("weather publication");
            })
            .with_contribution(Box::new(shade), || ContentStatus::Drawn(1));
        Ok((game, input, ground))
    }

    /// Seed the same weather state as the scene's fixed clock.
    /// # Errors
    /// Returns invalid model parameters.
    pub fn state(self, ground: Option<&GroundConfig>) -> Result<State, thrum_cloud_model::Error> {
        let (coverage, kind, wind) = match self.weather {
            Weather::Fair => (0.50, 1.0, [10.0, 1.0]),
            Weather::Broken => (0.726, 0.42, [10.0, 1.0]),
            Weather::SeaFog => (0.0, 1.0, [5.5, 0.0]),
            Weather::DawnMist => (0.0, 1.0, [0.3, 0.1]),
        };
        let mut state = State::new(
            Parameters {
                coverage,
                kind,
                wind,
                time_scale: if self.fast { 8.0 } else { 1.0 },
            },
            5.0,
            0,
        )?;
        if let Some(ground) = ground {
            let mist = self.weather == Weather::DawnMist;
            let mut controls = GroundState::new(vec![1.0], 1.0, 244_000.0)?;
            controls.set_radiative_day(mist);
            state.set_ground(controls);
            if mist {
                state.advance_ground_cooling(&ground.fog, 36_000.0, -10.0)?;
                if self.after_sunrise {
                    state.advance_ground_cooling(&ground.fog, 3600.0, 12.0)?;
                }
            }
        }
        Ok(state)
    }

    fn ground(self, map: &Map) -> Result<GroundConfig, thrum_cloud_model::Error> {
        let bed = map.bed();
        let (origin, spacing, size) = bed.lattice();
        let nodes = bed
            .heights()
            .iter()
            .enumerate()
            .map(|(index, &ground)| {
                let water = map.sample(bed.point(index)).map(|sample| sample.row.level);
                SurfacePoint { ground, water }
            })
            .collect::<Vec<_>>();
        let surface = Surface::new(origin, size, spacing, 3000.0, &nodes)?;
        let steam = self.weather == Weather::DawnMist;
        let row = Fog {
            liquid_water: if steam { 0.000_022 } else { 0.000_067 },
            droplet_radius: if steam { 8e-6 } else { 10e-6 },
            depth: if steam { 2.6 } else { 100.0 },
            softness: if steam { 2.2 } else { 48.0 },
            depression: 0.0,
            pooling: 0.0,
            drifting: !steam,
            affinity: Affinity::Water,
            centre: [-1110.0, 0.0],
            radius: [950.0, 1700.0],
            noise: if steam { 0.92 } else { 0.30 },
            noise_scale: if steam { 16.0 } else { 110.0 },
            radiative: steam,
        };
        let fog = Ground::new(
            GroundProfile {
                shore_margin: 18.0,
                calm: [2.0, 8.0],
                twilight: [-6.0, 10.0],
                cooling_seconds: 10_800.0,
                warming_seconds: 3600.0,
            },
            vec![row],
        )?;
        Ok(GroundConfig {
            surface: Arc::new(surface),
            fog: Arc::new(fog),
        })
    }
}

fn profile() -> ShadeConfig {
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
                liquid_water: 0.000_350,
                droplet_radius: 10e-6,
                deck: 1.0,
                profile: [0.65, 0.19],
            },
            Type {
                base: 1600.0,
                depth: [1550.0, 2900.0],
                liquid_water: 0.000_335,
                droplet_radius: 11e-6,
                deck: 0.5,
                profile: [0.45, 0.33],
            },
            Type {
                base: 1600.0,
                depth: [1450.0, 3800.0],
                liquid_water: 0.000_320,
                droplet_radius: 12e-6,
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
            ground_size: 1024,
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
    }
}
