//! Rain on the CPU, in f64, with no graphics device: the tree of hollows a bed is cut into,
//! the fill step that fills, spills, merges and parts puddles with its water account,
//! exposure under the shelters a game gives, the ground's stores and the wetness function,
//! the wet look's reference, and a raindrop's rings and the tables the rain bands read. Coordinates are metres, x/z on the ground and
//! y up; rain and drying are kilograms of water a square metre a second.
//!
//! The rain state is plain data a game keeps in its world and steps with the tick. The map
//! reads it through [`Tree::water_hollows`], a hollows body in `thrum-water-model`.
mod canopy;
mod error;
mod fill;
mod ground;
mod look;
mod rings;
mod state;
mod tree;

pub use canopy::{Canopy, Shelter};
pub use error::RainError;
pub use fill::{Account, LeafGain, Rain, RainState, TickReport};
pub use ground::{
    Flow, GroundClass, GroundStores, MAX_CLASSES, REACHES, Store, WetPoint, Wetness, band_fill,
    band_step, reach_weights, relief_share, shore_fill, sky_share, wetness, wetness_at,
};
pub use look::{FILM_REFLECTANCE, Material, porosity, wet_albedo, wet_look};
pub use rings::{Raindrop, Ring, RingProfile, Water, Wavenumbers, drops_over, marshall_palmer};
pub use tree::{Ground, Node, SmallHollow, Tree};

/// The physical profile of rain on a hollows body. Never a quality setting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Profile {
    /// A hollow shallower than this, metres from its pit to its sill, is filled.
    pub least_depth: f64,
    /// A hollow fewer cells across than this at its sill is reported.
    pub least_across: usize,
    /// Most hollows a body may hold.
    pub max_hollows: usize,
    /// Most cells a body may hold.
    pub max_cells: usize,
    /// The canopy grid's cell, metres.
    pub canopy_cell: f64,
    /// Width over which exposure passes from one shelter's share to the next, metres.
    pub edge_width: f64,
}

impl Default for Profile {
    fn default() -> Self {
        let map = thrum_water_model::HollowsProfile::default();
        Self {
            least_depth: 0.003,
            least_across: 4,
            max_hollows: map.max_hollows,
            max_cells: map.max_cells,
            canopy_cell: 0.1,
            edge_width: 0.3,
        }
    }
}

impl Profile {
    /// The limits the map counts a hollows body against.
    pub fn map_limits(&self) -> thrum_water_model::HollowsProfile {
        thrum_water_model::HollowsProfile {
            max_cells: self.max_cells,
            max_hollows: self.max_hollows,
        }
    }
}
