//! The opt-in valley shower. The world owns the rain state and each published
//! endpoint lends its projection to the forward material and the water map.
mod ground;
#[cfg(feature = "proving")]
mod witness;
#[cfg(feature = "proving")]
pub use witness::{RainWitness, rain_witness_points};

use std::sync::{Arc, Mutex};

use thrum_rain_model::{
    GroundClass, GroundStores, Profile, Rain, RainState, RingProfile, TickReport, Tree,
};
use thrum_water::{
    Water,
    wet_ground::{self, HollowWetness, Reaches, WetGround},
};
use thrum_water_model::{
    Body, BodyKind, Exposure, HollowOptics, Map, RainBands, RainBandsProfile, RateHistory, Stage,
};

/// Weather at game launch for the opt-in valley patch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RainSelection {
    /// Use the ordinary exit scene.
    #[default]
    Off,
    /// Start dry, then run a shower and dry again on the game clock.
    Shower,
    /// Hold the end of the shower, with rain standing in the hollows.
    Soaked,
    /// Start partway through drying.
    HalfDry,
}

impl RainSelection {
    pub(crate) fn named(name: &str) -> Result<Self, &'static str> {
        match name {
            "off" => Ok(Self::Off),
            "shower" => Ok(Self::Shower),
            "soaked" => Ok(Self::Soaked),
            "half-dry" => Ok(Self::HalfDry),
            _ => Err("rain must be off, shower, soaked or half-dry"),
        }
    }

    fn start(self) -> u64 {
        match self {
            Self::Off | Self::Shower => 0,
            Self::Soaked => SHOWER_END - 1,
            Self::HalfDry => SHOWER_END + 155,
        }
    }
}

const SHOWER_START: u64 = 60;
const SHOWER_END: u64 = 420;
const PATCH_DATUM: f32 = 240.0;
const STEP: f64 = 1.0 / 60.0;
const RAIN: f64 = 60.0 / 3600.0;
const DRYING: f64 = 60.0 / 3600.0;
const SHOWER_RATE: f64 = 300.0;
const DRYING_RATE: f64 = 600.0;

fn model_profile() -> Profile {
    Profile {
        least_depth: 0.008,
        canopy_cell: 0.25,
        edge_width: 0.35,
        ..Profile::default()
    }
}

const CLASS: GroundClass = GroundClass {
    soak: 13.0 / 3600.0,
    percolation: 1.6 / 3600.0,
    pores: 0.003,
    relief: 0.00065,
    share: 0.66,
    porosity: 0.88,
    steepest: 0.46,
    film_roughness: 0.58,
};

#[expect(clippy::cast_possible_truncation, reason = "wetness field uses f32")]
fn wet_class() -> wet_ground::GroundClass {
    wet_ground::GroundClass {
        relief: CLASS.relief as f32,
        steepest: CLASS.steepest as f32,
        share: CLASS.share as f32,
        porosity: CLASS.porosity as f32,
        film_roughness: CLASS.film_roughness as f32,
        pattern: wet_ground::ReliefPattern::default(),
    }
}

#[derive(Clone)]
pub(crate) struct RainPage {
    tick: u64,
    pub(crate) record: Rain,
    history: RateHistory,
    reaches: Reaches,
    hollows: Arc<[HollowWetness]>,
    levels: Arc<[f64]>,
}

pub(crate) struct ValleyRain {
    tree: Tree,
    state: RainState,
    stores: GroundStores,
    report: TickReport,
    start: u64,
    page: RainPage,
}

pub(crate) struct RainSurface {
    ground: WetGround,
    map: Arc<Mutex<Map>>,
    last_tick: Option<u64>,
    open_store: [f32; 2],
    report: Option<Arc<Mutex<thrum_water::WaterReport>>>,
}

impl ValleyRain {
    pub(crate) fn prepare(selection: RainSelection) -> Result<(Self, RainSurface), String> {
        let canopy = ground::canopy().map_err(|error| error.to_string())?;
        let world = Self::prepare_model(selection, &canopy)?;
        let mut map = world.water_map()?;
        // At 60 mm/h, roughly 40 drops per square metre per second are
        // at least 4 mm across. Resolve those rings and leave the smaller
        // drops in the same model's slope variance.
        let schedule = RainBandsProfile {
            count: 40.0,
            ..RainBandsProfile::default()
        };
        let rings = RingProfile {
            count: schedule.count,
            reference_rate: 60.0,
            classes: vec![[4.0, 4.5], [4.5, 5.2], [5.2, 6.5]],
            ..RingProfile::default()
        };
        let bands = RainBands::new(
            schedule,
            Arc::new(rings.table()),
            world.page.history.clone(),
            Arc::<thrum_rain_model::Canopy>::clone(&canopy),
        )
        .map_err(|error| format!("{error:?}"))?;
        map.set_rain(Some(bands));
        let mut surface = RainSurface {
            ground: WetGround::new_forward_at_datum(
                canopy.canopy_grid(),
                ground::wetness_grid(&world.tree),
                &[wet_class()],
                world.tree.hollow_count(),
                PATCH_DATUM,
            )
            .ok_or("valley wetness field exceeds its render projection")?,
            map: Arc::new(Mutex::new(map)),
            last_tick: None,
            open_store: [0.0; 2],
            report: None,
        };
        surface.publish(&world.page);
        Ok((world, surface))
    }

    fn prepare_model(
        selection: RainSelection,
        canopy: &thrum_rain_model::Canopy,
    ) -> Result<Self, String> {
        let profile = model_profile();
        let bed = ground::bed()?;
        let tree = Tree::prepare(&bed, canopy, &profile).map_err(|error| error.to_string())?;
        let mut world = Self {
            state: RainState::new(&tree),
            stores: GroundStores::new(&[CLASS]).map_err(|error| error.to_string())?,
            report: TickReport::default(),
            start: selection.start(),
            page: RainPage {
                tick: 0,
                record: Rain {
                    flux: 0.0,
                    drying: 0.0,
                },
                history: RateHistory::dry(STEP, -1, 62),
                reaches: [[0.0; 2]; 3],
                levels: Vec::new().into(),
                hollows: Vec::new().into(),
            },
            tree,
        };
        // Reproduce the same fixed-step trajectory when opening a checkpoint.
        for age in SHOWER_START..=world.start {
            world.advance_stores(age);
        }
        for tick in -62_i64..0 {
            let age = world.start.saturating_sub(tick.unsigned_abs() - 1);
            world.page.history.last = tick - 1;
            world.page.history.commit(weather(age).0.flux);
        }
        world.page.record = weather(world.start).0;
        world.project();
        Ok(world)
    }

    fn water_map(&self) -> Result<Map, String> {
        let hollows = self
            .tree
            .water_hollows(&self.page.levels)
            .and_then(|hollows| {
                hollows.with_optical_types(vec![
                    Some(HollowOptics {
                        extinction: 23.0,
                        scatter: [0.10, 0.072, 0.046],
                        bed_tint: [0.72, 0.75, 0.78],
                    });
                    self.tree.hollow_count()
                ])
            })
            .map_err(|error| format!("{error:?}"))?;
        let body = Body {
            name: "Valley hollows".into(),
            kind: BodyKind::Hollows(hollows),
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1.0,
            gains: [0.0; 4],
        };
        Map::prepare_with(
            ground::map_bed(&self.tree)?,
            vec![body],
            thrum_water_model::Profile {
                margin_height: 0.0,
                swash_ratio: 0.0,
            },
            model_profile().map_limits(),
        )
        .map_err(|error| format!("{error:?}"))
    }

    fn advance_stores(&mut self, age: u64) {
        let (rain, rate) = weather(age);
        if rate == 0.0 {
            return;
        }
        let dt = STEP * rate;
        self.state
            .step_with(&self.tree, rain, &self.stores, 0, dt, 1, &mut self.report)
            .expect("the authored shower has valid inputs");
        self.stores
            .step(rain, dt)
            .expect("the authored rain advances valid stores");
    }

    pub(crate) fn step(&mut self, tick: u64) {
        let hold_soaked = self.start == SHOWER_END - 1;
        let age = if hold_soaked {
            self.start
        } else {
            self.start.saturating_add(tick)
        };
        self.page.tick = tick;
        self.page.record = weather(age).0;
        self.page.history.commit(self.page.record.flux);
        if !hold_soaked {
            self.advance_stores(age);
            self.project();
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPU projection of f64 model"
    )]
    fn project(&mut self) {
        self.page.levels = self.state.levels(&self.tree).into();
        self.page.hollows = self
            .page
            .levels
            .iter()
            .enumerate()
            .map(|(i, &level)| HollowWetness {
                level: (level - f64::from(PATCH_DATUM)) as f32,
                mark: (self.state.marks()[i] - f64::from(PATCH_DATUM)) as f32,
                margin: self.state.bands()[i] as f32,
                held_fill: self.state.held_fill(&self.tree, &CLASS, i) as f32,
            })
            .collect();
        self.page.reaches = std::array::from_fn(|reach| {
            let store = self.stores.store(0, reach);
            [store.surface as f32, store.fill as f32]
        });
    }

    pub(crate) fn page(&self) -> RainPage {
        self.page.clone()
    }
}

fn weather(age: u64) -> (Rain, f64) {
    match age {
        0..SHOWER_START => (
            Rain {
                flux: 0.0,
                drying: 0.0,
            },
            0.0,
        ),
        SHOWER_START..SHOWER_END => (
            Rain {
                flux: RAIN,
                drying: 0.0,
            },
            SHOWER_RATE,
        ),
        _ => (
            Rain {
                flux: 0.0,
                drying: DRYING,
            },
            DRYING_RATE,
        ),
    }
}

impl RainSurface {
    #[cfg(feature = "proving")]
    pub(crate) fn proof_products(&self) -> (thrum_render::wetness::WetnessProvider, Map) {
        (
            self.provider(),
            self.map.lock().expect("valley water map").clone(),
        )
    }

    pub(crate) fn provider(&self) -> thrum_render::wetness::WetnessProvider {
        self.ground.provider()
    }

    pub(crate) fn water(&mut self) -> Result<Water, String> {
        let spectrum = thrum_sea_model::Spectrum {
            long: Vec::new(),
            cascades: [160.0, 37.0, 9.0].map(|length| {
                thrum_sea_model::Cascade::new(length, &[]).expect("empty wind spectrum")
            }),
            tail_slope_variance: 0.0,
        };
        let water = Water::mapped(
            Arc::clone(&self.map),
            spectrum,
            thrum_water::Profile {
                optics: Some(thrum_water::Optics {
                    reflection_reach: 24.0,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .map_err(|error| error.to_string())?;
        self.report = Some(water.report());
        Ok(water)
    }

    #[cfg(feature = "proving")]
    pub(crate) fn observation(&self) -> serde_json::Value {
        let report = self
            .report
            .as_ref()
            .map(|report| report.lock().expect("valley water report"));
        let map = self.map.lock().expect("valley water map");
        let depths = [[-1.0, 4.8], [2.1, 6.35]]
            .map(|point| map.sample(point).map_or(0.0, |sample| sample.depth));
        serde_json::json!({
            "tick": self.last_tick,
            "wet_depths_m": depths,
            "open_surface_m": self.open_store[0],
            "open_pores_fill": self.open_store[1],
            "submitted_hollow_vertices": report.as_ref().map(|r| r.hollow_surface_vertices),
            "rain_drops": report.as_ref().and_then(|r| r.rain_drops),
            "resources": report.as_ref().map(|r| &r.resources),
        })
    }

    pub(crate) fn publish(&mut self, page: &RainPage) {
        if self.last_tick == Some(page.tick) {
            return;
        }
        let mut map = self.map.lock().expect("valley water map");
        map.set_levels(0, &page.levels)
            .expect("valley level table fits its map");
        map.set_rate_history(page.history.clone())
            .expect("valid shower history");
        assert!(
            self.ground
                .publish(&[wet_class()], &[page.reaches], &page.hollows)
        );
        self.open_store = page.reaches[0];
        self.last_tick = Some(page.tick);
    }
}

/// Why the opt-in valley's cooked scene or model could not be prepared.
#[derive(Debug)]
pub enum RainValleyError {
    /// The shared cooked valley recipe was refused.
    Valley(crate::showcase::CookedValleyError),
    /// The valley needs the terrain material for the patch.
    MissingTerrain,
    /// The cooked contribution was refused.
    Cooked(thrum_asset_gpu::PlacementError),
    /// The rain model could not prepare the authored ground or canopy.
    Model(String),
}

impl std::fmt::Display for RainValleyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valley(error) => error.fmt(f),
            Self::MissingTerrain => f.write_str("the valley patch needs the terrain's material"),
            Self::Cooked(error) => error.fmt(f),
            Self::Model(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for RainValleyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Valley(error) => Some(error),
            Self::Cooked(error) => Some(error),
            Self::MissingTerrain | Self::Model(_) => None,
        }
    }
}

/// Prepare the populated valley's cooked placements with its optional patch.
///
/// # Errors
/// Refuses the same invalid recipe or cooked contribution as the standard valley.
#[doc(hidden)]
pub fn prepare_rain_cooked_valley(
    source: thrum_asset_gpu::CookedSource,
    path: thrum_asset_gpu::CookedDrawPath,
) -> Result<(crate::SceneCamera, thrum_asset_gpu::CookedAssets), RainValleyError> {
    let (camera, mut config) =
        crate::showcase::valley_config(source).map_err(RainValleyError::Valley)?;
    let mut ground = config
        .meshes
        .iter()
        .find(|placement| placement.mesh == "terrain")
        .ok_or(RainValleyError::MissingTerrain)?
        .clone();
    "exit-rain-patch".clone_into(&mut ground.mesh);
    // The patch is a thin relief over the existing terrain caster. It
    // receives scene shadows without casting between the two ground layers.
    ground.casts_shadows = false;
    config.meshes.push(ground);
    config.draw_path = path;
    let assets = thrum_asset_gpu::CookedAssets::new(config).map_err(RainValleyError::Cooked)?;
    Ok((camera, assets))
}
