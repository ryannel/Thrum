//! Body-following interaction waves. The GPU owns height and potential, and the
//! renderer and body query sample the same committed endpoint buffers.
mod gpu;
mod regions;
mod static_dent;

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use thrum_water_model::Impulse;

const GRID_WIDTH: u32 = 256;
const SIDE: usize = GRID_WIDTH as usize;
const CELLS: usize = SIDE * SIDE;
const GRID_CELLS: u32 = GRID_WIDTH * GRID_WIDTH;
const CELL_METRES: [f32; 2] = [0.25, 2.0];
const GRAVITY: f32 = 9.81;
const DENSITY: f32 = 1025.0;
const MAX_TICKS: u64 = 8;

/// Registered budget for interaction wave sources and regional solves.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Number of body reactions admitted per tick.
    pub bodies: u32,
    /// Number of impulses admitted per tick.
    pub impulses: u32,
    /// Number of region and depth pairs solved by each level.
    pub solves: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            bodies: 16,
            impulses: 32,
            solves: 16,
        }
    }
}

/// A Gaussian pressure footprint, with its scales along the hull axes.
#[derive(Clone, Copy, Debug)]
pub struct BodyPressure {
    /// Stable body ID, shared with the query.
    pub id: u32,
    /// World x/z, metres.
    pub place: [f64; 2],
    /// Forward-axis angle from world x toward world z, radians.
    pub heading: f64,
    /// Longitudinal and lateral standard deviations, metres.
    pub footprint: [f64; 2],
    /// Vertical reaction returned by the game's water force, newtons.
    pub force: f64,
}

#[derive(Default)]
struct Page {
    follow: Option<u32>,
    bodies: Vec<BodyPressure>,
    refused_bodies: u64,
}

#[derive(Default)]
struct ImpulsePage {
    impulses: Vec<Impulse>,
    impulse_ids: BTreeSet<u64>,
    refused_impulses: u64,
}

#[derive(Default)]
struct Pages {
    bodies: BTreeMap<u64, Page>,
    impulses: BTreeMap<u64, ImpulsePage>,
    retired_through: Option<u64>,
}

impl Pages {
    fn retire_impulses(&mut self, tick: u64, reseek: bool) {
        if reseek {
            self.impulses.clear();
        } else {
            while self
                .impulses
                .first_key_value()
                .is_some_and(|(&first, _)| first <= tick)
            {
                self.impulses.pop_first();
            }
        }
        self.retired_through = Some(tick);
    }
}

/// Tick pages of forces and impulses. Posting the same tick replaces its page
/// of body reactions and keeps one impulse per stable ID.
#[derive(Clone)]
pub struct Mailbox {
    limits: Limits,
    pages: Arc<Mutex<Pages>>,
}

impl Mailbox {
    pub(super) fn new(limits: Limits) -> Self {
        Self {
            limits,
            pages: Arc::default(),
        }
    }

    /// Post the reactions for this tick and optionally name the body the windows follow.
    /// # Panics
    /// Panics if another thread poisoned the mailbox.
    pub fn post_bodies(&self, tick: u64, follow: Option<u32>, mut bodies: Vec<BodyPressure>) {
        bodies.retain(|body| {
            body.place.iter().all(|v| v.is_finite())
                && body.heading.is_finite()
                && body.force.is_finite()
                && body.footprint.iter().all(|v| v.is_finite() && *v > 0.0)
        });
        bodies.sort_by_key(|body| body.id);
        bodies.dedup_by_key(|body| body.id);
        let mut pages = self.pages.lock().expect("water interaction mailbox");
        let page = pages.bodies.entry(tick).or_default();
        page.follow = follow;
        page.refused_bodies = bodies.len().saturating_sub(self.limits.bodies as usize) as u64;
        bodies.truncate(self.limits.bodies as usize);
        page.bodies = bodies;
        while pages.bodies.len() > 32 {
            pages.bodies.pop_first();
        }
    }

    /// Post a vertical impulse for its named tick. Pending impulses remain
    /// queued until submission reaches that tick. Retired ticks ignore late posts.
    /// # Panics
    /// Panics if another thread poisoned the mailbox.
    pub fn post_impulse(&self, impulse: Impulse) {
        if !impulse.place.iter().all(|v| v.is_finite())
            || !impulse.vertical.is_finite()
            || !impulse.radius.is_finite()
            || impulse.radius <= 0.0
        {
            return;
        }
        let mut pages = self.pages.lock().expect("water interaction mailbox");
        if pages
            .retired_through
            .is_some_and(|tick| impulse.tick <= tick)
        {
            return;
        }
        let page = pages.impulses.entry(impulse.tick).or_default();
        if page.impulse_ids.insert(impulse.id) {
            let index = page.impulses.partition_point(|entry| entry.id < impulse.id);
            page.impulses.insert(index, impulse);
            if page.impulses.len() > self.limits.impulses as usize {
                page.impulses.pop();
                page.refused_impulses += 1;
            }
        }
    }
}

/// Submitted interaction state and cumulative omissions for diagnostics.
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    /// Previous and current endpoint ticks.
    pub endpoints: Option<[u64; 2]>,
    /// Number of submitted full resets.
    pub resets: u64,
    /// Number of omitted ticks due to the catch-up bound.
    pub dropped_ticks: u64,
    /// Reactions refused by the body capacity.
    pub refused_bodies: u64,
    /// Impulses refused by the impulse capacity.
    pub refused_impulses: u64,
    /// Sources whose region did not fit the solve budget, by level.
    pub refused_regions: [u64; 2],
    /// Number of wet regions in each window at the latest step.
    pub regions: [u32; 2],
    /// Number of active depth solves in each window at the latest step.
    pub solves: [u32; 2],
    /// Latest CPU time spent sampling and labelling the windows, milliseconds.
    pub labelling_ms: f64,
}

/// Submitted world-space placement and packing for one interaction endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Endpoint {
    /// Source tick stamped on this bank.
    pub tick: u64,
    /// World x/z of cell zero, for the near and far levels.
    pub origins: [[f32; 2]; 2],
    /// Offset in pairs of height and potential, for the near and far levels.
    pub offsets: [usize; 2],
}

/// Opt-in diagnostic access to the two committed endpoints.
#[derive(Clone)]
pub struct Witness {
    /// Previous and current submitted banks.
    pub endpoints: [Endpoint; 2],
    /// Packed row-major `[height, potential]` pairs in GPU memory.
    pub buffer: wgpu::Buffer,
}

impl Witness {
    /// Width and height of each level in cells.
    pub const SIDE: usize = SIDE;
    /// Metres per cell in each level.
    pub const CELL_METRES: [f32; 2] = CELL_METRES;
}

/// Optional witness receiver, updated only when the interaction state commits.
#[derive(Clone, Default)]
pub struct WitnessTap(pub(super) Arc<Mutex<Option<Witness>>>);

impl WitnessTap {
    /// Return the metadata and buffer of the latest submitted state.
    /// Copy it before the next frame is submitted to keep a snapshot.
    /// # Panics
    /// Panics if the witness lock was poisoned.
    pub fn latest(&self) -> Option<Witness> {
        self.0.lock().expect("interaction witness").clone()
    }
}

pub(crate) use gpu::{Interaction, SampleSettings};
