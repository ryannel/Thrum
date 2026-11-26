//! Shared placement for the library inspections and the paired cost scene.
use super::{
    Directed, Parameters, PlacedLibrary, PlacedSky, PlacementDemo, ShapeSource, State, TOWER,
};
use std::sync::Arc;
use thrum_cloud_model::{Error, HashedShapes};

#[derive(Clone, Copy, Debug)]
pub(super) enum Placement {
    Shape { index: u32, side_lit: bool },
    Growth,
    Cost,
    Mixed,
    FullShare,
    NoShare,
}

impl PlacedSky {
    /// Configure the accepted library's proving scene. The gallery is shared by Metal and Chrome.
    ///
    /// # Errors
    /// Rejects unknown placement names and out-of-range library indices.
    pub fn with_capture_placement(mut self, name: &str) -> Result<Self, Error> {
        self.capture = Some(match name {
            "library-growth" => Placement::Growth,
            "library-cost" => Placement::Cost,
            "library-mixed" => Placement::Mixed,
            "library-only" => Placement::FullShare,
            "library-procedural" => Placement::NoShare,
            _ => {
                let side_lit = name.ends_with("-side-lit");
                let index = name
                    .strip_suffix("-side-lit")
                    .unwrap_or(name)
                    .strip_prefix("library-")
                    .and_then(|value| value.parse::<u32>().ok())
                    .filter(|&index| self.library.geometry(ShapeSource::Library(index)).is_some())
                    .ok_or(Error::Invalid)?;
                Placement::Shape { index, side_lit }
            }
        });
        if matches!(self.capture, Some(Placement::FullShare)) {
            let mut profile = super::super::shade_config();
            // The fair library uses one authored cell per three-site block.
            // Bring their spacing in for the fair preset without moving the weather windows.
            profile.seed = 374;
            profile.model.pitch = 2300.0;
            // Keep the denser lattice inside the Shapes table at every held tick.
            profile.weather.texels[3] = 900.0;
            self.profile = Some(profile);
        }
        self.capacity = 16;
        Ok(self)
    }
}

impl Placement {
    pub(super) fn demo(self) -> Option<PlacementDemo> {
        matches!(self, Self::Growth).then_some(PlacementDemo::Growth)
    }

    pub(super) fn state(
        self,
        library: &Arc<PlacedLibrary>,
        mut parameters: Parameters,
    ) -> Result<State, Error> {
        if matches!(self, Self::FullShare | Self::NoShare) {
            let mut state = State::new(parameters, 5.0, 16)?;
            let share = if matches!(self, Self::FullShare) {
                1.0
            } else {
                0.0
            };
            state.set_hashed_library(
                Arc::clone(library),
                HashedShapes {
                    block_sites: 3,
                    shares: vec![0.0, share],
                },
            )?;
            return Ok(state);
        }

        if !matches!(self, Self::Mixed) {
            parameters.coverage = 0.0;
        }
        let mut state = State::new(parameters, 5.0, 16)?;
        state.set_library(Arc::clone(library))?;
        let offset = match self {
            Self::Shape { side_lit: true, .. } => 55.0_f64,
            _ => 0.0,
        };
        let mut place = |id, source, azimuth: f64, distance, scale| {
            let slab = library.geometry(source).ok_or(Error::Missing)?;
            let (x, z) = (azimuth + offset).to_radians().sin_cos();
            state.add_from_library(
                Directed {
                    id,
                    centre: [distance * x, 80.0 - distance * z],
                    yaw: slab.heading + offset.to_radians(),
                    scale,
                    footprint: Arc::clone(&slab.footprint),
                    envelope: 0.0,
                    target: 1.0,
                    growth: 0.0,
                    growth_target: 0.0,
                },
                source,
            )
        };
        match self {
            Self::Shape { index, .. } => {
                place(4, ShapeSource::Library(index), -35.0, 5100.0, 1.0)?;
                place(
                    5,
                    ShapeSource::Library((index + 4) % 6),
                    -57.0,
                    6700.0,
                    0.85,
                )?;
                place(6, ShapeSource::Library((index + 3) % 6), -13.0, 7500.0, 0.9)?;
            }
            Self::Growth => {
                place(TOWER, ShapeSource::Growth, -35.0, 9900.0, 1.0)?;
                place(5, ShapeSource::Library(0), -50.0, 8300.0, 1.0)?;
                place(6, ShapeSource::Library(4), -19.0, 9100.0, 1.0)?;
            }
            Self::Mixed => {
                // The lower, nearer fair cell crosses the procedural bank in
                // projection. A second facing shows relief from the low Sun.
                place(4, ShapeSource::Library(4), -34.5, 8150.0, 1.0)?;
                place(5, ShapeSource::Library(6), -13.0, 7100.0, 1.0)?;
            }
            Self::Cost => {
                // Stagger distances so the 13 footprints fit the existing low-Sun
                // camera frustum without overlapping in the authoritative state.
                place(TOWER, ShapeSource::Growth, -79.844, 6550.0, 0.29)?;
                for index in 0..12_u32 {
                    let order = if index < 6 { index } else { index + 1 };
                    let angle = (f64::from(order) - 6.0).mul_add(4.4, -79.844);
                    let (distance, scale) = if order % 2 == 0 {
                        (6550.0, 0.29)
                    } else {
                        (9250.0, 0.40)
                    };
                    place(
                        10 + u64::from(index),
                        ShapeSource::Library(index),
                        angle,
                        distance,
                        scale,
                    )?;
                }
            }
            Self::FullShare | Self::NoShare => unreachable!("hashed placement returns above"),
        }
        Ok(state)
    }
}
