use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crowd_2d_view::CrowdWorld;
use thrum_app::{
    GameError, Gpu, InlineWorld, KeyCode, OutputMetrics,
    ios::{MobileGame, MobilePrepared, SceneRequest},
};
use thrum_ui::{Event, Viewport};

use crate::{END_TICK, Host, Input, Readback, StatusScene, prepare, readback};

/// The Crowd game wired to the shared UIKit shell.
pub struct MobileCrowd;

/// Game input and the presented frame witnessed by the iOS route.
pub struct MobileInput {
    input: Input,
    pan: Arc<Mutex<[f64; 2]>>,
    drawn: Option<Readback>,
    rows_checked: u64,
    row_mismatches: u64,
}

impl MobileGame for MobileCrowd {
    type World = CrowdWorld;
    type Scene = StatusScene;
    type Presentation = Arc<Mutex<[f64; 2]>>;
    type Controls = ();
    type Input = MobileInput;
    type Readback = Readback;

    async fn prepare(
        gpu: Gpu<'_>,
        _: Instant,
        _: fn(),
        _: Option<SceneRequest>,
        qualification: bool,
    ) -> Result<MobilePrepared<Self>, GameError> {
        let store = std::env::var_os("THRUM_ASSET_STORE").map_or_else(
            || {
                std::env::home_dir()
                    .unwrap_or_default()
                    .join("Documents/crowd-store")
            },
            Into::into,
        );
        let pan = Arc::new(Mutex::new([0.0; 2]));
        // The function proof advances by committed ticks. Its first checkpoint
        // keeps the replay available for rotation, and the second for backgrounding.
        let first_stop = if qualification { 240 } else { END_TICK };
        let host = prepare(gpu, store.into(), first_stop, qualification).await?;
        Ok(MobilePrepared {
            host,
            scene: Arc::clone(&pan),
            controls: (),
            input: MobileInput {
                input: Input::default(),
                pan,
                drawn: None,
                rows_checked: 0,
                row_mismatches: 0,
            },
            output_metrics: true,
        })
    }

    fn output(host: &mut Host, output: OutputMetrics) {
        if output.extent[0] > output.extent[1] {
            host.world_mut().extend_through(1200);
        }
        host.continuous_source_mut().display.output(output);
    }

    fn frame(
        host: &mut Host,
        pan: &mut Self::Presentation,
        _: u64,
        _: Duration,
    ) -> Result<(), GameError> {
        let latest = pan.try_lock().ok().map(|pan| *pan);
        if let Some(latest) = latest {
            host.continuous_source_mut().set_pan(latest);
        }
        Ok(())
    }

    fn resume_inline(world: &mut CrowdWorld, now: Duration) -> Result<(), GameError> {
        world.extend_through(END_TICK);
        world.pause()?;
        world.resume(now)
    }

    fn wants_input(_: &MobileInput) -> bool {
        true
    }

    fn event(input: &mut MobileInput, event: Event<KeyCode>, now: Duration) {
        let scale = input
            .drawn
            .and_then(|drawn| drawn.shown)
            .map_or(1, |(viewport, _)| viewport.scale);
        input.input.event(event, now, scale);
        if let Ok(mut pan) = input.pan.lock() {
            *pan = input.input.pan();
        }
    }

    fn readback(host: &mut Host) -> Option<Readback> {
        Some(readback(host))
    }

    fn viewport(readback: &Readback) -> Option<Viewport> {
        readback.shown.map(|(viewport, _)| viewport)
    }

    fn submitted(input: &mut MobileInput, readback: &Readback, _: u64, _: Duration) -> bool {
        input.rows_checked += 1;
        input.row_mismatches += u64::from(readback.rows != readback.live);
        let changed = input.drawn.is_none_or(|drawn| {
            drawn.shown != readback.shown
                || drawn.pan.map(f64::to_bits) != readback.pan.map(f64::to_bits)
                || drawn.digest != readback.digest
        });
        input.drawn = Some(*readback);
        changed
    }

    fn witness(input: &MobileInput) -> String {
        let mut report = input
            .drawn
            .map_or_else(|| serde_json::json!({}), |drawn| drawn.record());
        report["presses"] = serde_json::json!(input.input.presses());
        report["rows_checked"] = serde_json::json!(input.rows_checked);
        report["row_mismatches"] = serde_json::json!(input.row_mismatches);
        format!("crowd_status={report}\n")
    }
}
