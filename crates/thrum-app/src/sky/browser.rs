//! Kit-owned bindings. A scene registers its ordinary builder through `run`.
use std::{cell::RefCell, error::Error, rc::Rc, time::Duration};

use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::RemoteFeed;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::{SkyGame, SkyScene, StillWorld};
use crate::{
    Config, InlineWorld,
    web::{self, ControlBinding, WebShell},
};
use thrum_render::capabilities::FIRST_DEVICE_GENERATION;

type Builder = Box<dyn FnOnce() -> Result<SkyGame, Box<dyn Error>>>;
thread_local! {
    static BUILDER: RefCell<Option<(Config, Builder)>> = RefCell::new(None);
    static FEED: RefCell<Option<RemoteFeed>> = const { RefCell::new(None) };
}

/// Register the scene's builder. The page fetches assets before invoking it.
pub fn run(config: Config, build: impl FnOnce() -> Result<SkyGame, Box<dyn Error>> + 'static) {
    BUILDER.with(|slot| *slot.borrow_mut() = Some((config, Box::new(build))));
}

pub(super) fn feed() -> Result<RemoteFeed, String> {
    FEED.with(|slot| {
        slot.borrow().clone().ok_or_else(|| {
            "asset store has no fetched publication; start with ./dev starter --web".into()
        })
    })
}

impl InlineWorld for StillWorld {
    fn pause(&mut self) -> Result<(), String> {
        StillWorld::pause(self);
        Ok(())
    }
    fn resume(&mut self, now: Duration) -> Result<(), String> {
        StillWorld::resume(self, now);
        Ok(())
    }
}

/// One kit scene, sharing the native controls and sky model.
#[wasm_bindgen]
pub struct BrowserScene {
    shell: WebShell<StillWorld, SkyScene>,
    game: SkyGame,
    binding: ControlBinding,
    kept: Option<(u64, String)>,
    config: Config,
    transport_error: Rc<RefCell<Option<String>>>,
}

/// Fetch verified cooked products, then prepare the registered scene on the web shell.
#[wasm_bindgen]
/// # Errors
/// Reports asset, input, clock or graphics failures with their cause.
pub async fn create_scene(
    canvas: HtmlCanvasElement,
    origin: String,
) -> Result<BrowserScene, JsValue> {
    let mut client = DevClient::new(BrowserFetch, &origin).map_err(js)?;
    let generation = client
        .poll()
        .await
        .map_err(|error| js(asset_error(&origin, error)))?
        .ok_or_else(|| js("asset store has no publication; cook the scene's assets and restart ./dev starter --web"))?;
    let feed = RemoteFeed::new(generation);
    FEED.with(|slot| *slot.borrow_mut() = Some(feed.clone()));
    let (config, build) = BUILDER
        .with(|slot| slot.borrow_mut().take())
        .ok_or_else(|| js("scene builder was not registered; rebuild with ./dev starter --web"))?;
    let mut game = build().map_err(js)?;
    let shell = web::create_game(canvas, FIRST_DEVICE_GENERATION, &mut game)
        .await
        .map_err(js)?;
    let transport_error = Rc::new(RefCell::new(None));
    let polling_error = Rc::clone(&transport_error);
    // The client lives in this detached loop, with exactly one outstanding long poll.
    wasm_bindgen_futures::spawn_local(async move {
        loop {
            match client.poll().await {
                Ok(Some(generation)) => feed.publish(generation),
                Ok(None) => {}
                Err(error) => {
                    *polling_error.borrow_mut() = Some(format!(
                        "{}; check the store URL and restart the scene command",
                        asset_error(&origin, error)
                    ));
                    break;
                }
            }
        }
    });
    Ok(BrowserScene {
        shell,
        game,
        binding: ControlBinding::default(),
        kept: None,
        config,
        transport_error,
    })
}

#[wasm_bindgen]
impl BrowserScene {
    /// Start replacement of a lost graphics device, preserving the scene and clock.
    /// # Errors
    /// Refuses a healthy device or an already pending replacement.
    pub fn recover_graphics(&mut self) -> Result<(), JsValue> {
        self.shell.request_graphics_recovery().map_err(js)
    }

    /// Device generation and the latest submitted clock on that device.
    /// The clock fields are zero until the device submits its first frame.
    pub fn frame_info(&self) -> String {
        let status = self.shell.status();
        let clock = self.shell.host().last_submitted_clock();
        format!(
            "generation={} recovered={} presented={} tick={} alpha={} submission={}",
            status.generation,
            status.recovered,
            status.presented,
            clock.map_or(0, |frame| frame.tick),
            clock.map_or(0.0, |frame| frame.presentation_alpha),
            clock.map_or(0, |frame| frame.submission_order),
        )
    }
    /// Number of submitted images omitted by the bounded observer.
    pub fn capture_omissions(&self) -> u32 {
        u32::try_from(self.shell.host().submitted_capture_omissions()).unwrap_or(u32::MAX)
    }
    /// Drain GPU completion callbacks without advancing the scene clock.
    /// # Errors
    /// Reports a device polling fault.
    pub fn frame_ready(&mut self) -> Result<bool, JsValue> {
        self.shell.poll_completed().map_err(js)
    }
    /// The scene title declared by its creator.
    pub fn title(&self) -> String {
        self.config.window_title.to_owned()
    }

    /// Draw one callback through the shared shell.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn frame(&mut self, now: f64) -> Result<(), JsValue> {
        self.shell.update_profiler(&mut self.game.controls, false);
        self.shell.frame_game(&mut self.game, now).map_err(js)
    }
    /// Preserve the shell's page clock policy.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn set_visible(&mut self, visible: bool, now: f64) -> Result<(), JsValue> {
        self.shell.set_visible(visible, now).map_err(js)?;
        self.shell.update_profiler(&mut self.game.controls, false);
        Ok(())
    }
    /// Model description used by the DOM view.
    pub fn controls_json(&self) -> String {
        self.game.controls.describe_json()
    }
    /// Only changes since the DOM's last refresh.
    pub fn control_changes(&mut self) -> Option<String> {
        self.binding.changes(&self.game.controls)
    }
    /// Apply a widget edit through the model setters used by the Mac panel.
    /// # Errors
    /// Reports an unknown key, incompatible value or unsupported browser argument.
    pub fn edit_control(&mut self, key: &str, value: &JsValue) -> Result<bool, JsValue> {
        let changed = ControlBinding::edit(&mut self.game.controls, key, value)?;
        self.shell.update_profiler(&mut self.game.controls, false);
        Ok(changed)
    }

    /// Apply display values through the same checked parameter path as the native panel.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn apply_params(&mut self, text: &str) -> Result<String, JsValue> {
        let report = ControlBinding::parameters(&mut self.game.controls, text)?;
        Ok(report
            .problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"))
    }
    /// Current content generation and every fresh refusal, for the page's live status.
    pub fn report(&mut self) -> String {
        if let Some(fault) = self.shell.graphics_fault() {
            if self.shell.recovering() {
                return "Replacing graphics device; the scene and clock are retained".into();
            }
            return format!(
                "Graphics device unavailable: {fault}; use Recover graphics to create a new device"
            );
        }
        let content = self.game.poll_status();
        let kept = self.game.poll_kept();
        if let Some((generation, _)) = kept.last() {
            let message = kept.iter().map(|(generation, error)| {
                format!("Generation {generation} stays drawn. {error}; fix the named asset and save it again")
            }).collect::<Vec<_>>().join("\n");
            self.kept = Some((*generation, message));
        }
        if let Some(super::ContentStatus::Drawn(current)) = &content
            && self.kept.as_ref().is_some_and(|(held, _)| current > held)
        {
            self.kept = None;
        }
        let content = match content {
            Some(super::ContentStatus::Drawn(generation)) => {
                format!("Cooked generation {generation} drawn")
            }
            Some(super::ContentStatus::Failed(error)) => {
                format!("Asset load failed: {error}")
            }
            _ => "Cooked assets loading; keep this page open".into(),
        };
        let refusal = self
            .kept
            .as_ref()
            .map_or("", |(_, message)| message.as_str());
        let transport = self.transport_error.borrow();
        format!(
            "{content}. {refusal} {}",
            transport.as_deref().unwrap_or_default()
        )
    }
    /// Enable the qualified submitted-output tap for a fixed capture.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn begin_capture(&mut self) -> Result<(), JsValue> {
        self.shell
            .begin_submitted_capture_with_limits(
                1,
                thrum_render::runtime::CaptureLimits {
                    byte_allowance: 256 * 1024 * 1024,
                    ..Default::default()
                },
            )
            .map_err(js)
    }
    /// One submitted image's pixels, with its source identity recorded separately.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn capture(&mut self) -> Result<JsValue, JsValue> {
        let Some(image) = self.shell.next_submitted_image().map_err(js)? else {
            return Ok(JsValue::NULL);
        };
        let value = js_sys::Object::new();
        js_sys::Reflect::set(
            &value,
            &js("frame"),
            &js(format!(
                "tick={} alpha={} submission={} observation={:.9}",
                image.sample.tick,
                image.sample.presentation_alpha,
                image.sample.submission_order,
                image.sample.observation.as_secs_f64()
            )),
        )?;
        js_sys::Reflect::set(
            &value,
            &js("rgba"),
            &js_sys::Uint8Array::from(image.rgba.as_slice()),
        )?;
        js_sys::Reflect::set(
            &value,
            &js("identity"),
            &js(format!(
                "session={} submission={} presented={} source={:?} history={} device={} elapsed={:?}",
                image.session,
                image.sample.submission_order,
                image.sample.surface_presented,
                image.source,
                image.history_epoch,
                image.device_generation,
                image.submitted_elapsed
            )),
        )?;
        self.shell.recycle_submitted_image(image);
        Ok(value.into())
    }
    /// Stop readback work.
    pub fn end_capture(&mut self) {
        self.shell.end_submitted_capture();
    }
}
fn js(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn asset_error(origin: &str, error: thrum_asset::AssetError) -> String {
    let cause = match error {
        thrum_asset::AssetError::Transport(_) => "fetch request did not complete".to_owned(),
        error => error.to_string(),
    };
    format!("asset store {origin}: {cause}")
}
