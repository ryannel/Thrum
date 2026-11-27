//! Crowd replay with a status display, shared by the window, page and UIKit app.

use std::{cell::Cell, sync::Arc, time::Duration};

use crowd_2d_view::{CrowdScene, CrowdWorld, prepare_crowd_scene, proof::ORDER_BAND};
use thrum_app::{Edits, Game, GameError, Gpu, KeyCode, KeyResponse, OutputMetrics, Prepared};
use thrum_asset_gpu::CookedSource;
use thrum_controls::Controls;
use thrum_core::sim::PublicationPage;
use thrum_render::{
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameError, FrameHost, SceneFrame, SceneSource},
    sprite::{SpriteFrame, SpriteSink},
    synthetic::FrameLighting,
};
use thrum_ui::{Event, InterfaceConfig, PaintList, Viewport};

mod display;
mod input;
#[cfg(target_os = "ios")]
mod mobile;
pub use input::Input;
#[cfg(target_os = "ios")]
pub use mobile::MobileCrowd;

use display::Display;

/// Number of bodies at the start of the reference Crowd replay.
pub const BODIES: u32 = 2000;
/// Last tick of the retained Crowd replay at this body count.
pub const END_TICK: u64 = 3600;

// Fit the 120-unit arena radius, a 32-unit pan and the sprite edges.
const VIEW_SPAN: std::num::NonZeroU16 =
    std::num::NonZeroU16::new(312).expect("the Crowd view span is positive");

/// Crowd rules and sprite extraction with an independent interface stage.
pub struct StatusScene {
    crowd: CrowdScene,
    display: Display,
    pan: [f64; 2],
    digest: Cell<Option<u64>>,
}

impl SceneSource<Arc<PublicationPage>> for StatusScene {
    fn extract(&self, rows: &mut RenderExtractor, page: &Arc<PublicationPage>) -> ExtractReport {
        self.crowd.extract(rows, page)
    }

    fn frame(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        self.crowd.frame(source, alpha, extent, lighting)
    }

    fn extract_sprites(&self, sink: &mut SpriteSink<'_>, page: &Arc<PublicationPage>) {
        self.crowd.extract_sprites(sink, page);
    }

    fn sprites(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
    ) -> Result<SpriteFrame<'_>, FrameError> {
        self.crowd.sprites(source, alpha, extent)
    }

    fn interface_config(&self) -> Option<InterfaceConfig> {
        Some(self.display.config())
    }

    fn paint_lists(&mut self, extent: [u32; 2]) -> [Option<PaintList>; 2] {
        self.display.paint(extent, self.crowd.population())
    }
}

impl StatusScene {
    fn set_pan(&mut self, pan: [f64; 2]) {
        self.crowd.set_pan(pan);
        self.pan = pan;
    }
}

/// Frame host used by all three platforms.
pub type Host = FrameHost<CrowdWorld, StatusScene>;

/// Prepare the same 2D scene on a shell's selected device.
///
/// # Errors
/// Returns a font store or renderer preparation refusal.
pub async fn prepare(
    gpu: Gpu<'_>,
    source: CookedSource,
    end: u64,
    exact: bool,
) -> Result<Host, GameError> {
    let (display, assets) = Display::new(source, gpu.surface.extent)?;
    let scene = StatusScene {
        crowd: CrowdScene::new(false).with_span(VIEW_SPAN),
        display,
        pan: [0.0; 2],
        digest: Cell::default(),
    };
    let world = CrowdWorld::new(BODIES, exact).through(end);
    prepare_crowd_scene(
        gpu,
        world,
        scene,
        BODIES,
        ORDER_BAND,
        Some(Box::new(assets)),
    )
    .await
}

/// A committed Crowd publication and the layout sampled for it.
#[derive(Clone, Copy)]
pub struct Readback {
    /// Tick of the world publication used by the scene.
    pub tick: u64,
    /// Live body count read from the publication.
    pub live: usize,
    /// World sprite rows held by the renderer.
    pub rows: usize,
    /// Camera offset in the frame.
    pub pan: [f64; 2],
    /// Viewport and displayed card box in interface points.
    pub shown: Option<(Viewport, [[f64; 2]; 2])>,
    /// Authority digest at the last retained tick.
    pub digest: Option<u64>,
}

impl Readback {
    /// Record the game and layout values in a portable report.
    #[must_use]
    pub fn record(&self) -> serde_json::Value {
        serde_json::json!({
            "tick": self.tick, "live": self.live, "rows": self.rows, "pan": self.pan,
            "drawn": self.shown.map(|(viewport, bounds)| serde_json::json!({
                "extent": viewport.extent, "scale": viewport.scale,
                "insets": viewport.insets, "bounds": bounds,
            })),
            "digest": self.digest.map(|digest| format!("{digest:016x}")),
        })
    }
}

/// Read the scene's actual publication, sprite order and submitted interface geometry.
#[must_use]
pub fn readback(host: &Host) -> Readback {
    let scene = host.source();
    let (tick, live) = scene.crowd.population();
    if tick == END_TICK && host.world().tick() == END_TICK && scene.digest.get().is_none() {
        scene.digest.set(Some(host.world().digest()));
    }
    Readback {
        tick,
        live,
        rows: host.sprite_world_order().map_or(0, <[u32]>::len),
        pan: scene.pan,
        shown: scene.display.shown(),
        digest: scene.digest.get(),
    }
}

/// Latest game input and output reports for the render owner.
#[derive(Clone, Copy, Default)]
pub struct Update {
    pan: Option<[f64; 2]>,
    output: Option<OutputMetrics>,
}

impl Edits for Update {
    fn merge(self, newer: Self) -> Self {
        Self {
            pan: newer.pan.or(self.pan),
            output: newer.output.or(self.output),
        }
    }

    fn is_empty(&self) -> bool {
        self.pan.is_none() && self.output.is_none()
    }
}

/// Crowd app state retained by the window or page shell.
pub struct CrowdStatus {
    controls: Controls,
    source: CookedSource,
    input: Input,
    drawn: Option<Readback>,
    output: Option<OutputMetrics>,
    #[cfg(target_os = "macos")]
    script: Option<thrum_app::InterfaceScript>,
}

impl CrowdStatus {
    /// Create the game using an already cooked font store.
    #[must_use]
    pub fn new(source: CookedSource) -> Self {
        Self {
            controls: Controls::new(),
            source,
            input: Input::default(),
            drawn: None,
            output: None,
            #[cfg(target_os = "macos")]
            script: None,
        }
    }

    /// Use the Mac window's translated-event script for a qualification run.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn with_script(mut self, script: Option<thrum_app::InterfaceScript>) -> Self {
        self.script = script;
        self
    }

    /// The interface last seen on a presented frame and the input it passed to the game.
    #[must_use]
    pub fn witness(&self) -> String {
        let mut record = self
            .drawn
            .map_or_else(|| serde_json::json!({}), |drawn| drawn.record());
        record["presses"] = serde_json::json!(self.input.presses());
        record.to_string()
    }
}

impl Game for CrowdStatus {
    #[cfg(target_os = "macos")]
    fn mac_scene_kind(&self) -> thrum_render::runtime::SceneKind {
        thrum_render::runtime::SceneKind::TwoD
    }

    type World = CrowdWorld;
    type Scene = StatusScene;
    type Edits = Update;
    type Readback = Readback;

    #[cfg(target_os = "macos")]
    fn inline_world(
        world: &mut CrowdWorld,
    ) -> Option<
        &mut dyn thrum_app::InlineWorld<
            Page = <CrowdWorld as thrum_render::runtime::FrameWorld>::Page,
        >,
    > {
        Some(world)
    }

    async fn prepare<'a>(
        &'a mut self,
        gpu: Gpu<'a>,
        _wake: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<CrowdWorld, StatusScene>, GameError> {
        Ok(Prepared::inline(
            prepare(gpu, self.source.clone(), END_TICK, false).await?,
        ))
    }

    fn controls(&self) -> &Controls {
        &self.controls
    }
    fn controls_mut(&mut self) -> &mut Controls {
        &mut self.controls
    }
    fn take_edits(&mut self) -> (Update, Option<bool>) {
        (Update::default(), None)
    }

    fn frame_edits(&mut self) -> Update {
        Update {
            pan: Some(self.input.pan()),
            output: None,
        }
    }

    fn replay(&self) -> Update {
        Update {
            pan: Some(self.input.pan()),
            output: self.output,
        }
    }

    fn apply(host: &mut Host, update: Update) -> Result<(), GameError> {
        let scene = host.continuous_source_mut();
        if let Some(pan) = update.pan {
            scene.set_pan(pan);
        }
        if let Some(output) = update.output {
            scene.display.output(output);
        }
        Ok(())
    }

    fn readback(host: &Host) -> Readback {
        readback(host)
    }
    fn observe(&mut self, _frame: Readback) {}

    fn wants_interface_output(&self) -> bool {
        true
    }
    fn wants_interface_input(&self) -> bool {
        true
    }

    fn interface_output(&mut self, output: OutputMetrics) -> Update {
        self.output = Some(output);
        Update {
            pan: None,
            output: Some(output),
        }
    }

    fn interface_input(
        &mut self,
        event: Event<KeyCode>,
        now: Duration,
    ) -> Result<KeyResponse, GameError> {
        let scale = self
            .drawn
            .and_then(|drawn| drawn.shown)
            .map_or(1, |(viewport, _)| viewport.scale);
        self.input.event(event, now, scale);
        Ok(KeyResponse::Input)
    }

    fn interface_submitted(&mut self, frame: &Readback, _: u64, _: Duration) {
        self.drawn = Some(*frame);
    }

    #[cfg(target_os = "macos")]
    fn interface_script(&self) -> Option<&thrum_app::InterfaceScript> {
        self.script.as_ref()
    }

    #[cfg(target_os = "macos")]
    fn interface_witness(&self) -> Option<String> {
        Some(self.witness())
    }
}
