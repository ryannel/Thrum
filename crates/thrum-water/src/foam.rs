//! Transported whitewater. The scene supplies the wind and hull work; the
//! water map and wave field supply the other births and the transport support.
mod gpu;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(crate) use gpu::Foam;

/// Bounded whitewater birth rate per watt per square metre of chart loss.
pub const CHART_LOSS_BIRTH_RATE: f32 = 0.004;

#[derive(Clone)]
pub(crate) struct Whitewater {
    pub(super) input: Input,
    pub(super) wind: [f32; 2],
    pub(super) jacobian_variance: [f32; 4],
    pub(super) long_bounds: [f32; 4],
}

/// A hull in contact with the water at a committed tick.
#[derive(Clone, Copy, Debug)]
pub struct Hull {
    /// World x/z in metres.
    pub place: [f32; 2],
    /// Unit forward vector in world x/z.
    pub forward: [f32; 2],
    /// Half-length and half-beam in metres.
    pub extent: [f32; 2],
    /// World x/z velocity in metres per second.
    pub velocity: [f32; 2],
    /// Dissipated horizontal drag power in watts.
    pub drag_power: f32,
}

/// The latest authored hull input. A missing page supplies no births.
#[derive(Clone, Default)]
pub struct Input(Arc<Mutex<(u64, Vec<Hull>)>>, Arc<AtomicBool>);
impl Input {
    /// Switch births from chart loss off or on. Existing foam continues to age and travel.
    pub fn set_chart_loss_births(&self, enabled: bool) {
        self.1.store(!enabled, Ordering::Relaxed);
    }

    pub(crate) fn chart_loss_births(&self) -> bool {
        !self.1.load(Ordering::Relaxed)
    }

    /// Replace the hull input for a tick.
    /// # Panics
    /// Panics if another thread poisoned the hull input lock.
    pub fn post(&self, tick: u64, hulls: impl IntoIterator<Item = Hull>) {
        let mut page = self.0.lock().expect("foam input");
        page.0 = tick;
        page.1.clear();
        page.1.extend(
            hulls
                .into_iter()
                .filter(|body| {
                    body.place
                        .into_iter()
                        .chain(body.velocity)
                        .chain(body.forward)
                        .all(f32::is_finite)
                        && body.extent.into_iter().all(|v| v.is_finite() && v > 0.0)
                        && body.drag_power.is_finite()
                        && body.drag_power >= 0.0
                })
                .take(gpu::HULL_LIMIT),
        );
    }
}

/// Diagnostic facts about the last submitted foam state.
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    /// Previous and current endpoint ticks.
    pub ticks: [u64; 2],
    /// Cumulative ticks omitted beyond the catch-up bound.
    pub dropped_ticks: u64,
    /// Number of full resets.
    pub resets: u64,
}

/// Submitted endpoint buffer and world placement for numerical diagnostics.
#[derive(Clone, Debug)]
pub struct Witness {
    /// Two banks of two row-major planes of RGBA f32 stocks.
    pub buffer: wgpu::Buffer,
    /// Source parcels for the latest step in the previous endpoint's windows.
    /// The second four-lane word holds shift x/z, diffusion and shore birth rate.
    /// Use only when the endpoint ticks differ.
    pub parcels: wgpu::Buffer,
    /// Number of cells along each side.
    pub side: u32,
    /// Cell spacing in metres, finest first.
    pub spacing: [f32; 2],
    /// World-space origin of each plane in endpoint and level order.
    pub origins: [[[f32; 2]; 2]; 2],
    /// Buffer bank holding each endpoint.
    pub banks: [usize; 2],
    /// Submitted clock.
    pub report: Report,
}

/// Opt-in tap for submitted foam diagnostics.
#[derive(Clone, Default)]
pub struct Tap(pub(super) Arc<Mutex<Option<Witness>>>);
impl Tap {
    /// Last submitted resource, if any.
    /// # Panics
    /// Panics if another thread poisoned the diagnostic lock.
    pub fn latest(&self) -> Option<Witness> {
        self.0.lock().expect("foam witness").clone()
    }
}
