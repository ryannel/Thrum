//! Public sprite scene for the retained Crowd replay.
#[cfg(target_arch = "wasm32")]
mod browser;
pub mod proof;
mod scene;
mod world;

use std::{sync::Arc, time::Duration};

use thrum_app::{Edits, Game, GameError, Gpu, Prepared};
use thrum_controls::Controls;
use thrum_core::sim::PublicationPage;
use thrum_render::runtime::{FrameConfig, FrameHost, FrameWorld, SpriteSceneRequests};

pub use scene::CrowdScene;
pub use world::CrowdWorld;

/// Prepare the ordinary Crowd renderer for any world that publishes Crowd tick pages.
///
/// # Errors
/// Returns a renderer preparation failure.
pub async fn prepare_crowd<W: FrameWorld<Page = Arc<PublicationPage>>>(
    gpu: Gpu<'_>,
    world: W,
    count: u32,
    close: bool,
    band: f64,
) -> Result<FrameHost<W, CrowdScene>, GameError> {
    let config = FrameConfig {
        render_extent: gpu.surface.extent,
        surface: Some(gpu.surface),
    };
    Ok(FrameHost::new_2d(
        gpu.capabilities,
        gpu.queue,
        config,
        world,
        CrowdScene::new(close),
        scene::configuration(count, band),
        SpriteSceneRequests::default(),
    )
    .await?)
}

/// Replay renderer using the same rule package on every target.
pub struct CrowdView {
    controls: Controls,
    count: u32,
    close: bool,
    exact_steps: bool,
    timed: bool,
    band: f64,
}

impl CrowdView {
    /// A replay at the given body count, optionally following the first player.
    ///
    /// # Errors
    /// Counts outside the rule package's supported range are refused.
    pub fn new(count: u32, close: bool) -> Result<Self, GameError> {
        if !(4..=10_000).contains(&count) {
            return Err(GameError::InvalidScene(
                "Crowd requires 4 to 10,000 bodies".into(),
            ));
        }
        Ok(Self {
            controls: Controls::new(),
            count,
            close,
            exact_steps: false,
            timed: false,
            band: scene::ORDER_BAND,
        })
    }

    /// Drive a whole committed tick per callback for endpoint and cost records.
    #[must_use]
    pub fn exact_steps(mut self) -> Self {
        self.exact_steps = true;
        self
    }

    /// Request the renderer's timestamp queries for a cost record.
    #[must_use]
    pub fn with_timing(mut self, enabled: bool) -> Self {
        self.timed = enabled;
        self
    }

    /// Select the ordering band for the band comparison.
    #[must_use]
    pub fn with_band(mut self, band: f64) -> Self {
        self.band = band;
        self
    }
}

/// This replay has no presentation controls to transfer between owners.
#[derive(Clone, Copy, Default)]
pub struct NoEdits;

impl Edits for NoEdits {
    fn merge(self, _newer: Self) -> Self {
        self
    }

    fn is_empty(&self) -> bool {
        true
    }
}

impl Game for CrowdView {
    type World = CrowdWorld;
    type Scene = CrowdScene;
    type Edits = NoEdits;
    type Readback = ();

    async fn prepare<'a>(
        &'a mut self,
        gpu: Gpu<'a>,
        _world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<CrowdWorld, CrowdScene>, GameError> {
        let world = CrowdWorld::new(self.count, self.exact_steps);
        let mut host = prepare_crowd(gpu, world, self.count, self.close, self.band).await?;
        host.set_timing_requested(self.timed);
        Ok(Prepared::inline(host))
    }

    fn supports_edr(&self) -> bool {
        true
    }

    fn controls(&self) -> &Controls {
        &self.controls
    }

    fn controls_mut(&mut self) -> &mut Controls {
        &mut self.controls
    }

    fn take_edits(&mut self) -> (NoEdits, Option<bool>) {
        (NoEdits, None)
    }

    fn replay(&self) -> NoEdits {
        NoEdits
    }

    fn apply(_: &mut FrameHost<CrowdWorld, CrowdScene>, _: NoEdits) -> Result<(), GameError> {
        Ok(())
    }

    fn readback(_: &FrameHost<CrowdWorld, CrowdScene>) {}

    fn observe(&mut self, (): ()) {}
}

impl thrum_app::InlineWorld for CrowdWorld {
    fn pause(&mut self) -> Result<(), GameError> {
        self.pause_clock();
        Ok(())
    }

    fn resume(&mut self, now: Duration) -> Result<(), GameError> {
        self.resume_clock(now)
            .map_err(|error| GameError::Rejected(error.to_string()))
    }
}
