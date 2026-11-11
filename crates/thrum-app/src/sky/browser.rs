//! Kit-owned bindings. A scene registers its ordinary builder through `run`.
use std::{cell::RefCell, error::Error, rc::Rc, time::Duration};

use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::RemoteFeed;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::{FALLBACK_EXPOSURE, SkyGame, SkyScene, StillWorld, ground_scene};
use crate::{
    Config,
    web::{self, WebShell, WebWorld},
};
use thrum_render::runtime::FrameHost;

type Builder = Box<dyn FnOnce() -> Result<SkyGame, Box<dyn Error>>>;
thread_local! {
    static BUILDER: RefCell<Option<Builder>> = RefCell::new(None);
    static FEED: RefCell<Option<RemoteFeed>> = const { RefCell::new(None) };
}

/// Register the scene's builder. The page fetches assets before invoking it.
pub fn run(_config: Config, build: impl FnOnce() -> Result<SkyGame, Box<dyn Error>> + 'static) {
    BUILDER.with(|slot| *slot.borrow_mut() = Some(Box::new(build)));
}

pub(super) fn feed() -> Result<RemoteFeed, String> {
    FEED.with(|slot| {
        slot.borrow().clone().ok_or_else(|| {
            "asset store has no fetched publication; start with ./dev starter --web".into()
        })
    })
}

impl WebWorld for StillWorld {
    fn pause(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn resume_after_drain(&mut self, _now: Duration) -> Result<(), String> {
        Ok(())
    }
}

/// One kit scene, sharing the native controls and sky model.
#[wasm_bindgen]
pub struct BrowserScene {
    shell: WebShell<StillWorld, SkyScene>,
    game: SkyGame,
    seen: u64,
    kept: String,
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
    let build = BUILDER
        .with(|slot| slot.borrow_mut().take())
        .ok_or_else(|| js("scene builder was not registered; rebuild with ./dev starter --web"))?;
    let mut game = build().map_err(js)?;
    let _ = game.update();
    let scene = SkyScene {
        sky: game.sky,
        ground: game.ground.is_some(),
    };
    let ground = ground_scene(game.ground).map_err(js)?;
    let contribution = game.contribution.take();
    let shell = web::create(canvas, 1, |gpu| async move {
        let mut host = FrameHost::new(
            &gpu.capabilities,
            &gpu.queue,
            gpu.frame_config,
            StillWorld,
            scene,
            ground,
            FALLBACK_EXPOSURE,
        )
        .await
        .map_err(|error| error.to_string())?;
        if let Some(contribution) = contribution {
            host.set_scene_contribution(contribution)?;
        }
        Ok(host)
    })
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
                    *polling_error.borrow_mut() = Some(asset_error(&origin, error));
                    break;
                }
            }
        }
    });
    Ok(BrowserScene {
        shell,
        game,
        seen: 0,
        kept: String::new(),
        transport_error,
    })
}

#[wasm_bindgen]
impl BrowserScene {
    /// Draw one callback through the shared shell.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn frame(&mut self, now: f64) -> Result<(), JsValue> {
        if let Some(sky) = self.game.update().0 {
            self.shell.source_mut().sky = sky;
        }
        self.game.poll_reload();
        self.shell.frame(now).map_err(js)
    }
    /// Preserve the shell's page clock policy.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn set_visible(&mut self, visible: bool, now: f64) -> Result<(), JsValue> {
        self.shell.set_visible(visible, now).map_err(js)
    }
    /// Model description used by the DOM view.
    pub fn controls_json(&self) -> String {
        self.game.controls.describe_json()
    }
    /// Only changes since the DOM's last refresh.
    pub fn control_changes(&mut self) -> Option<String> {
        let revision = self.game.controls.revision();
        if revision == self.seen {
            return None;
        }
        let json = self.game.controls.changes_json(self.seen);
        self.seen = revision;
        Some(json)
    }
    /// Apply a widget edit through the model setters used by the Mac panel.
    /// # Errors
    /// Reports an unknown key, incompatible value or unsupported browser argument.
    pub fn edit_control(&mut self, key: &str, value: &JsValue) -> Result<bool, JsValue> {
        let model = &mut self.game.controls;
        let id = model.lookup(key).map_err(js)?;
        if let Some(number) = value.as_f64() {
            model.edit_display(id, number).map_err(js)
        } else if let Some(on) = value.as_bool() {
            model
                .edit_value(id, thrum_controls::Value::Flag(on))
                .map_err(js)
        } else if let Some(text) = value.as_string() {
            model
                .edit_value(id, thrum_controls::Value::Text(text))
                .map_err(js)
        } else {
            Err(js(
                "browser control input needs a number, boolean or string",
            ))
        }
    }

    /// Apply display values through the same checked parameter path as the native panel.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn apply_params(&mut self, text: &str) -> Result<String, JsValue> {
        let report = self.game.controls.apply_file(text).map_err(js)?;
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
            return format!(
                "Graphics device unavailable: {fault}; reload this page to create a new device"
            );
        }
        let content = self.game.poll_status();
        if let Some(error) = self.transport_error.borrow_mut().take() {
            self.kept = error;
        }
        let kept = self.game.poll_kept();
        if !kept.is_empty() {
            self.kept = kept.into_iter().map(|(generation, error)| format!("Generation {generation} stays drawn. {error}; fix the named asset and save it again")).collect::<Vec<_>>().join("\n");
        }
        let content = match content {
            Some(super::ContentStatus::Drawn(generation)) => {
                format!("Cooked generation {generation} drawn")
            }
            Some(super::ContentStatus::Failed(error)) => {
                format!("Asset load failed: {error}; fix the named asset and save again")
            }
            _ => "Cooked assets loading; keep this page open".into(),
        };
        format!("{content}. {}", self.kept)
    }
    /// Enable the qualified submitted-output tap for a fixed capture.
    /// # Errors
    /// Reports asset, input, clock or graphics failures with their cause.
    pub fn begin_capture(&mut self) -> Result<(), JsValue> {
        self.shell.begin_submitted_capture(1).map_err(js)
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
    format!("asset store {origin}: {cause}; check the store URL and restart the scene command")
}
