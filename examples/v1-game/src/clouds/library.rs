use std::sync::{Arc, Mutex};
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedSource, CookedStats, LeaseRequest};
use thrum_cloud_model::{
    Directed, Footprint, Parameters, PlacedLibrary, PlacedSlab, ShapeSource, State,
};
use thrum_clouds::{Shade, ShadeConfig, ShapesConfig};

pub(crate) const NAMES: [&str; 2] = ["cloud-library", "cloud-growth"];
const SHAPES: usize = 12;
const KEYS: usize = 11;
const SIDE: usize = 128;
const WORDS: usize = 5 + 3 * SIDE * SIDE;

const TOWER: u64 = 1;
const FAIR: u64 = 2;
const DISTANT: u64 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlacementDemo {
    Tower,
    Mixed,
}

impl PlacementDemo {
    pub(crate) fn step(self, state: &mut State) -> Result<(), thrum_cloud_model::Error> {
        let time = state.time();
        let smooth = |unit: f64| {
            let t = unit.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let update = |state: &mut State, id, forming: f64, thinning, growth| {
            if state.shape_source(id).is_some() {
                state.direct(id, forming.max(1e-6) * thinning, growth)?;
            }
            Ok(())
        };
        let growth = ((time - 12.0) / 120.0).clamp(0.0, 1.0);
        let tower = match self {
            Self::Tower => smooth(time / 18.0),
            Self::Mixed => smooth(time / 36.0).powi(2),
        };
        update(
            state,
            TOWER,
            tower,
            1.0 - smooth((time - 174.0) / 60.0),
            growth,
        )?;
        if self == Self::Mixed {
            // At 60x, a delayed frame can span several weather seconds.
            // Keep initial optical depths small even for the far bank's thickest voxels.
            update(
                state,
                FAIR,
                smooth(time / 24.0).powi(2),
                1.0 - smooth((time - 25.0) / 65.0),
                0.0,
            )?;
            update(
                state,
                DISTANT,
                smooth(time / 42.0).powi(2),
                1.0 - smooth((time - 205.0) / 95.0),
                0.0,
            )?;
        }
        Ok(())
    }
}

/// The game's placed sky, including the cooked publication and authoritative placement controls.
#[derive(Clone, Debug)]
pub struct PlacedSky {
    source: CookedSource,
    library: Arc<PlacedLibrary>,
    directed: bool,
    weather_share: Option<f64>,
    initial: Option<State>,
    capacity: usize,
    profile: Option<ShadeConfig>,
    stats: Arc<Mutex<Option<Arc<Mutex<CookedStats>>>>>,
}

impl PlacedSky {
    /// Use the example's accepted library and a prepared store or remote publication.
    ///
    /// # Errors
    /// Reports an invalid embedded footprint record.
    pub fn new(source: CookedSource, directed: bool) -> Result<Self, thrum_cloud_model::Error> {
        let bytes = include_bytes!("../../assets/clouds/footprints.bin");
        Ok(Self {
            source,
            library: Arc::new(read_footprints(bytes)?),
            directed,
            weather_share: None,
            initial: None,
            capacity: 8,
            profile: None,
            stats: Arc::default(),
        })
    }

    /// Use a game-authored initial weather state whose library is already attached.
    ///
    /// # Errors
    /// Rejects a state past tick zero or without a placed library.
    pub fn from_state(
        source: CookedSource,
        state: State,
        capacity: usize,
    ) -> Result<Self, thrum_cloud_model::Error> {
        let library = state.library().ok_or(thrum_cloud_model::Error::Missing)?;
        if state.tick() != 0 || capacity > 16 {
            return Err(thrum_cloud_model::Error::Invalid);
        }
        Ok(Self {
            source,
            library: Arc::new(library.clone()),
            directed: true,
            weather_share: None,
            initial: Some(state),
            capacity,
            profile: None,
            stats: Arc::default(),
        })
    }

    /// Use game-authored weather and shadow resolutions for this placement scene.
    #[must_use]
    pub fn with_profile(mut self, profile: ShadeConfig) -> Self {
        self.profile = Some(profile);
        self
    }

    pub(crate) fn profile(&self) -> ShadeConfig {
        self.profile.clone().unwrap_or_else(super::shade_config)
    }

    /// Current placed-stack residency observations, once the scene is prepared.
    pub fn residency(&self) -> Option<Arc<Mutex<CookedStats>>> {
        self.stats.lock().ok()?.clone()
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn with_weather_placement(mut self, share: f64) -> Self {
        self.weather_share = Some(share);
        self
    }

    pub(crate) fn demo(&self) -> Option<PlacementDemo> {
        if self.initial.is_some() || self.weather_share.is_some() {
            return None;
        }
        Some(if self.directed {
            PlacementDemo::Tower
        } else {
            PlacementDemo::Mixed
        })
    }

    pub(crate) fn state(
        &self,
        mut parameters: Parameters,
    ) -> Result<State, thrum_cloud_model::Error> {
        if let Some(state) = &self.initial {
            return Ok(state.clone());
        }
        if self.directed {
            parameters.coverage = 0.0;
        }
        let mut state = State::new(parameters, 5.0, 16)?;
        if let Some(share) = self.weather_share {
            state.set_hashed_library(
                Arc::clone(&self.library),
                thrum_cloud_model::HashedShapes {
                    block_sites: 3,
                    shares: vec![0.0, share],
                },
            )?;
            return Ok(state);
        }
        state.set_library(Arc::clone(&self.library))?;
        let placements = if self.directed {
            &[(TOWER, [-6_900.0, -2_600.0], ShapeSource::Growth)]
        } else {
            &[
                (TOWER, [-6_900.0, -4_650.0], ShapeSource::Growth),
                (FAIR, [-4_610.0, -6_700.0], ShapeSource::Library(4)),
                (DISTANT, [-10_600.0, -15_800.0], ShapeSource::Library(8)),
            ][..]
        };
        for &(id, centre, source) in placements {
            let slab = self
                .library
                .geometry(source)
                .ok_or(thrum_cloud_model::Error::Missing)?;
            state.add_from_library(
                Directed {
                    id,
                    centre,
                    yaw: slab.heading,
                    scale: 1.0,
                    footprint: Arc::clone(&slab.footprint),
                    envelope: 0.0,
                    target: 1.0,
                    growth: 0.0,
                    growth_target: 0.0,
                },
                source,
            )?;
        }
        Ok(state)
    }

    pub(crate) fn shade(&self, shade: Shade) -> Result<(CookedAssets, Shade), String> {
        let mut config = CookedConfig::new(self.source.clone(), Vec::new());
        config.leased = NAMES
            .map(|name| LeaseRequest {
                name: name.to_owned(),
                dimension: wgpu::TextureDimension::D3,
            })
            .into();
        let assets = CookedAssets::new(config).map_err(|error| error.to_string())?;
        let shade = shade
            .with_shapes(ShapesConfig {
                library: Arc::clone(&self.library),
                lease: assets.lease(),
                range: [8_000.0, 19_000.0],
                capacity: self.capacity,
            })
            .map_err(|error| error.to_string())?;
        *self
            .stats
            .lock()
            .map_err(|_| "placed residency report unavailable")? = Some(assets.stats());
        Ok((assets, shade))
    }
}

fn read_footprints(bytes: &[u8]) -> Result<PlacedLibrary, thrum_cloud_model::Error> {
    use thrum_cloud_model::Error;
    let (records, rest) = bytes.as_chunks::<{ WORDS * 4 }>();
    if records.len() != SHAPES + KEYS || !rest.is_empty() {
        return Err(Error::Invalid);
    }
    let mut slabs = records
        .iter()
        .map(read_slab)
        .collect::<Result<Vec<_>, _>>()?;
    let growth = slabs.split_off(SHAPES);
    PlacedLibrary::new(slabs, growth)
}

fn read_slab(bytes: &[u8; WORDS * 4]) -> Result<PlacedSlab, thrum_cloud_model::Error> {
    let words = bytes.as_chunks::<4>().0;
    let get = |index: usize| f64::from(f32::from_le_bytes(words[index]));
    let extent = get(0);
    let columns = (0..SIDE * SIDE)
        .map(|index| std::array::from_fn(|i| get(5 + 3 * index + i)))
        .collect();
    let footprint = Footprint::from_columns([SIDE; 2], extent / 128.0, columns)?;
    Ok(PlacedSlab {
        extent,
        floor: -get(1) - extent * 0.5,
        heading: get(4).atan2(get(2)),
        footprint: Arc::new(footprint),
    })
}
