//! The V1 game's browser page bindings over the engine's web shell (`thrum_app::web`).
//!
//! The shell owns the canvas surface, the device, loss and replacement, and the frame step; this
//! facade keeps the game's controls, cooked valley, material reloads, capture slot and report.

mod environment;
mod submitted_capture;

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use thrum_app::web::SAMPLES;
use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::{CookedSource, CookedStats, RemoteFeed};
#[cfg(feature = "dev-material")]
use thrum_asset_gpu::{MaterialHook, MaterialOutcome, MaterialReload};
use thrum_controls::Value;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{
    FrameSample,
    controls::{ControlEdits, GameControls},
    host::web::{self as game, Shell},
};

/// The cooked valley's dev-server client and the feed of generations it has fetched.
#[wasm_bindgen]
pub struct CookedValley {
    /// Taken out while a poll runs, so overlapping polls are refused.
    client: Rc<RefCell<Option<DevClient<BrowserFetch>>>>,
    feed: RemoteFeed,
}

/// Fetch the current cooked publication from a dev server origin such as
/// `http://127.0.0.1:7878`. Every product is verified against its content address.
///
/// # Errors
/// Returns a malformed origin, a transport failure, an empty store or a product mismatch.
#[wasm_bindgen]
pub async fn fetch_cooked_valley(origin: String) -> Result<CookedValley, JsValue> {
    let mut client = DevClient::new(BrowserFetch, &origin).map_err(js_error)?;
    let generation = client
        .poll()
        .await
        .map_err(js_error)?
        .ok_or_else(|| JsValue::from_str("the dev server has no publication yet"))?;
    Ok(CookedValley {
        client: Rc::new(RefCell::new(Some(client))),
        feed: RemoteFeed::new(generation),
    })
}

#[wasm_bindgen]
impl CookedValley {
    /// Long-poll the dev server once. Resolves `true` when a newer publication was fetched
    /// (the attached valley stages it at its next frame) and `false` when the poll timed out.
    /// Call again after it settles; overlapping calls are refused.
    pub fn next_publication(&self) -> js_sys::Promise {
        let client = Rc::clone(&self.client);
        let feed = self.feed.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            let mut polling = client
                .borrow_mut()
                .take()
                .ok_or_else(|| JsValue::from_str("a cooked publication poll is already running"))?;
            let fetched = polling.poll().await;
            *client.borrow_mut() = Some(polling);
            let fetched = fetched.map_err(js_error)?;
            let changed = fetched.is_some();
            if let Some(generation) = fetched {
                feed.publish(generation);
            }
            Ok(JsValue::from_bool(changed))
        })
    }
}

/// One browser canvas running the V1 game through the engine's web shell.
#[wasm_bindgen]
pub struct BrowserGame {
    temporal_inputs: bool,
    shell: Shell,
    cooked: Option<Arc<Mutex<CookedStats>>>,
    /// Facts from the first adapter-backed record, for fixed-view qualification.
    initial_downlevel: String,
    /// Explicit material reloads of the attached cooked valley (development builds only).
    #[cfg(feature = "dev-material")]
    material: Option<MaterialReload>,
    /// The material revision last reported, so an idle page builds no report.
    #[cfg(feature = "dev-material")]
    material_reported: u64,
    /// Typed demo controls shared with the page and the macOS panel.
    demo: GameControls,
    /// Model revision the page last synced to.
    page_seen: u64,
    /// The page's outstanding capture, finished from a later animation frame.
    capture: Option<thrum_render::runtime::PendingCapture>,
}

/// Prepare the existing V1 simulation and renderer for a WebGPU canvas.
/// `valley_stars` selects the star catalogue before attaching the cooked valley.
/// `diagnostic_clouds` opts into the labelled analytic cloud-shade stand-in.
///
/// # Errors
/// Returns a JavaScript error when no compatible adapter, surface format or GPU resource exists.
#[wasm_bindgen]
pub async fn create_browser_game(
    canvas: HtmlCanvasElement,
    valley_stars: Option<bool>,
    diagnostic_clouds: Option<bool>,
    temporal_inputs: Option<bool>,
    culling: Option<String>,
    meshes: Option<bool>,
) -> Result<BrowserGame, JsValue> {
    create_game(
        canvas,
        crate::GameSetup {
            display_overlay: if temporal_inputs.unwrap_or(false) {
                thrum_render::synthetic::DisplayOverlay::TemporalInputs
            } else {
                thrum_render::synthetic::DisplayOverlay::Off
            },
            diagnostic_clouds: diagnostic_clouds.unwrap_or(false),
            valley_stars: valley_stars.unwrap_or(false),
            field_meshes: meshes.unwrap_or(false),
            camera_culling: valley_culling(culling.as_deref(), valley_stars.unwrap_or(false))?,
            ..crate::GameSetup::default()
        },
    )
    .await
}

/// Prepare the dense flythrough field with explicit culling or the measured `"default"`.
///
/// # Errors
/// Returns an unknown culling mode, or the same failures as [`create_browser_game`].
#[wasm_bindgen]
pub async fn create_field_game(
    canvas: HtmlCanvasElement,
    culling: String,
    field_meshes: bool,
) -> Result<BrowserGame, JsValue> {
    let camera_culling = match culling.as_str() {
        "default" => crate::GameSetup::field_default_culling(field_meshes),
        "direct" => thrum_render::synthetic::CullMode::Direct,
        "frustum" => thrum_render::synthetic::CullMode::Frustum,
        "occlusion" => thrum_render::synthetic::CullMode::Occlusion,
        _ => {
            return Err(JsValue::from_str(
                "culling must be default, direct, frustum or occlusion",
            ));
        }
    };
    create_game(
        canvas,
        crate::GameSetup {
            display_overlay: thrum_render::synthetic::DisplayOverlay::Off,
            content: crate::GameContent::Field,
            diagnostic_clouds: false,
            camera_culling,
            valley_stars: false,
            route_overlay: crate::RouteOverlay::Off,
            field_meshes,
        },
    )
    .await
}

async fn create_game(
    canvas: HtmlCanvasElement,
    setup: crate::GameSetup,
) -> Result<BrowserGame, JsValue> {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&info.to_string()));
    }));
    let mut initial_downlevel = String::new();
    let shell = thrum_app::web::create(canvas, game::DEVICE_GENERATION, |gpu| {
        initial_downlevel = format!("{:?}", gpu.capabilities.downlevel());
        game::prepare(gpu, setup)
    })
    .await
    .map_err(js_error)?;
    let mut demo = GameControls::new().map_err(js_error)?;
    demo.observe_camera(game::camera_orientation(&shell));
    demo.observe_time(game::time_of_day(&shell));
    Ok(BrowserGame {
        temporal_inputs: setup.display_overlay
            == thrum_render::synthetic::DisplayOverlay::TemporalInputs,
        shell,
        cooked: None,
        initial_downlevel,
        #[cfg(feature = "dev-material")]
        material: None,
        #[cfg(feature = "dev-material")]
        material_reported: 0,
        demo,
        page_seen: 0,
        capture: None,
    })
}

/// Material hook reloads, a development control compiled only with the `dev-material` feature
/// (`examples/v1-game/web/build.sh --dev`), so the default page cannot start material work from
/// its URL.
#[cfg(feature = "dev-material")]
#[wasm_bindgen]
impl BrowserGame {
    /// Replace the cooked valley's material hook with the WGSL `source`, named `name` in its
    /// diagnostics, and return the request's serial. The previous material keeps drawing until
    /// the replacement has compiled and validated; [`Self::material_report`] says what became of
    /// it. The shader module and pipeline are created in this call through WebGPU's synchronous
    /// methods (wgpu 30.0.1 has no `createRenderPipelineAsync`), so a cold compile stalls the
    /// page for hundreds of milliseconds. This is a development control, not part of play.
    /// # Errors
    /// Returns an error when no cooked valley is attached, or `graphics device is lost` while
    /// the device is lost (compiling would call it).
    pub fn reload_material(&mut self, name: String, source: String) -> Result<f64, JsValue> {
        if self.shell.graphics_lost() {
            // Compiling would call the lost device; the accepted hook is restored on the
            // replacement, and a fresh reload is accepted once it is published.
            return Err(js_error("graphics device is lost"));
        }
        let material = self.material.as_ref().ok_or_else(|| {
            JsValue::from_str("material reload needs the cooked valley (?assets)")
        })?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "request serials stay far below 2^53"
        )]
        Ok(material.request(MaterialHook { name, source }) as f64)
    }

    /// Material reload outcomes since the last call, the accepted hook and whether it is being
    /// restored for a replaced device (`restoring`), as JSON; `None` without a cooked valley or
    /// when nothing changed since the last report. Each outcome has `serial` (null for
    /// `dropped`, which answers no request), `kind` (`promoted`, `rejected`, `superseded`,
    /// `retired` or `dropped`) and `text`; a promotion adds `timingMs`, and a rejection or a
    /// dropped restore `diagnostics` (`file`, `line`, `column`, `message`).
    pub fn material_report(&mut self) -> Option<String> {
        let material = self.material.as_ref()?;
        let revision = material.revision();
        if revision == self.material_reported {
            return None;
        }
        self.material_reported = revision;
        let outcomes: Vec<_> = material
            .take_outcomes()
            .iter()
            .map(|outcome| {
                let mut value = serde_json::json!({
                    "serial": outcome.serial(),
                    "text": outcome.to_string(),
                });
                match outcome {
                    MaterialOutcome::Promoted { name, timing, .. } => {
                        value["kind"] = "promoted".into();
                        value["name"] = name.as_str().into();
                        value["timingMs"] = serde_json::json!({
                            "call": timing.call.as_secs_f64() * 1e3,
                            "prepare": timing.prepare.as_secs_f64() * 1e3,
                            "eligible": timing.eligible.as_secs_f64() * 1e3,
                            "drawn": timing.drawn.as_secs_f64() * 1e3,
                        });
                    }
                    MaterialOutcome::Rejected { diagnostics, .. }
                    | MaterialOutcome::Dropped { diagnostics, .. } => {
                        value["kind"] = if matches!(outcome, MaterialOutcome::Dropped { .. }) {
                            "dropped"
                        } else {
                            "rejected"
                        }
                        .into();
                        value["diagnostics"] = diagnostics
                            .iter()
                            .map(|diagnostic| {
                                serde_json::json!({
                                    "file": diagnostic.file,
                                    "line": diagnostic.line,
                                    "column": diagnostic.column,
                                    "message": diagnostic.message,
                                })
                            })
                            .collect();
                    }
                    MaterialOutcome::Superseded { .. } => value["kind"] = "superseded".into(),
                    MaterialOutcome::Retired { .. } => value["kind"] = "retired".into(),
                }
                value
            })
            .collect();
        Some(
            serde_json::json!({
                "accepted": material.accepted(),
                "restoring": material.restoring(),
                "outcomes": outcomes,
            })
            .to_string(),
        )
    }
}

/// Prepare the game for the moving proof route: the canvas takes the route's render
/// extent, and the game is built with the route's overlay and the valley's stars.
/// The page attaches the cooked valley and installs the route with
/// [`BrowserGame::install_route`]; nothing here admits the overlay without its route.
/// `diagnostic_clouds` selects the same opt-in shade stand-in as the ordinary page.
///
/// # Errors
/// Refuses a file that is not a route, or the same failures as [`create_browser_game`].
#[wasm_bindgen]
pub async fn create_route_game(
    canvas: HtmlCanvasElement,
    route: String,
    diagnostic_clouds: Option<bool>,
    culling: Option<String>,
    meshes: Option<bool>,
) -> Result<BrowserGame, JsValue> {
    let (extent, overlay) = crate::CameraRoute::declared(&route).map_err(js_error)?;
    canvas.set_width(extent[0]);
    canvas.set_height(extent[1]);
    create_game(
        canvas,
        crate::GameSetup {
            valley_stars: true,
            route_overlay: overlay,
            diagnostic_clouds: diagnostic_clouds.unwrap_or(false),
            camera_culling: valley_culling(culling.as_deref(), true)?,
            field_meshes: meshes.unwrap_or(false),
            ..crate::GameSetup::default()
        },
    )
    .await
}

fn valley_culling(
    request: Option<&str>,
    valley: bool,
) -> Result<thrum_render::synthetic::CullMode, JsValue> {
    use thrum_render::synthetic::CullMode;
    match request.unwrap_or("default") {
        "default" => Ok(if valley {
            crate::GameSetup::valley_default_culling()
        } else {
            CullMode::Direct
        }),
        "direct" => Ok(CullMode::Direct),
        "frustum" => Ok(CullMode::Frustum),
        "occlusion" => Ok(CullMode::Occlusion),
        _ => Err(JsValue::from_str(
            "culling must be default, direct, frustum or occlusion",
        )),
    }
}

#[wasm_bindgen]
impl BrowserGame {
    /// Install the moving route (the route file's JSON) once the cooked valley of the
    /// route's generation has drawn, when the committed tick is the one before its origin.
    /// Camera, time and control edits are refused from then on.
    /// # Errors
    /// Returns the route's installation refusals.
    pub fn install_route(&mut self, route: &str) -> Result<(), JsValue> {
        let drawn = self
            .cooked
            .as_ref()
            .and_then(|stats| stats.lock().ok().and_then(|stats| stats.drawn));
        game::install_route(&mut self.shell, route, drawn).map_err(js_error)
    }

    /// Collect GPU completions without admitting time and report whether a frame
    /// slot is free, so the route page drains callbacks before a step only where needed.
    /// # Errors
    /// Returns a device polling fault.
    pub fn route_slot_ready(&mut self) -> Result<bool, JsValue> {
        game::route_slot_ready(&mut self.shell).map_err(js_error)
    }

    /// Latest committed authority tick.
    #[must_use]
    pub fn committed_tick(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "route runs stay far below 2^53 ticks"
        )]
        let tick = game::committed_tick(&self.shell) as f64;
        tick
    }

    /// The next route step's page time in milliseconds from the clock origin:
    /// `(tick + 1 + phi)·P`. Route mode starts the clock with `set_visible(true, 0)`.
    #[must_use]
    pub fn route_step_ms(&self, phi: f64) -> f64 {
        game::route_step_ms(&self.shell, phi)
    }

    /// Route identity as JSON: counters, solar hours and retained samples from `from_tick`.
    #[must_use]
    pub fn route_report(&self, from_tick: f64) -> String {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the page passes a committed tick it read from this game"
        )]
        let from = from_tick.max(0.0) as u64;
        game::route_report(&self.shell, from).to_string()
    }

    /// Draw the authored valley from fetched cooked products, with its authored camera.
    /// The products survive device replacement: a new device stages the newest fetched
    /// generation again. Publications fetched later by [`CookedValley::next_publication`] swap
    /// in as they upload. While the device is lost the contribution is held, with no GPU call,
    /// and attaches right after the replacement device is published; a refusal then is
    /// reported in [`Self::cooked_status`] errors. A later attach replaces a held one.
    /// # Errors
    /// Returns an invalid embedded recipe or the scene's refusal of the contribution.
    pub fn attach_cooked_valley(&mut self, valley: &CookedValley) -> Result<(), JsValue> {
        let source = CookedSource::Remote(valley.feed.clone());
        let (camera, assets) = if self.temporal_inputs {
            crate::showcase::prepare_temporal_showcase(source)
        } else {
            crate::showcase::prepare_showcase(source)
        }
        .map_err(js_error)?;
        let stats = assets.stats();
        #[cfg(feature = "dev-material")]
        let material = assets.material();
        self.shell
            .set_scene_contribution(Box::new(assets))
            .map_err(js_error)?;
        game::set_scene_camera(&mut self.shell, camera).map_err(js_error)?;
        self.demo
            .observe_camera(game::camera_orientation(&self.shell));
        self.cooked = Some(stats);
        #[cfg(feature = "dev-material")]
        {
            self.material = Some(material);
            self.material_reported = 0;
        }
        Ok(())
    }

    /// Cooked generation drawn, swaps and loader errors, or `None` without cooked content.
    pub fn cooked_status(&self) -> Option<String> {
        let stats = self.cooked.as_ref()?.lock().ok()?;
        Some(
            serde_json::json!({
                "drawn": stats.drawn,
                "drawPath": format!("{:?}", stats.draw_path),
                "initialDownlevel": self.initial_downlevel,
                "visibleMeshlets": stats.visible_meshlets,
                "swaps": stats.swaps.len(),
                "uploadedBytes": stats.swaps.last().map_or(0, |swap| swap.uploaded_bytes),
                "submittedFrames": stats.submitted_frames,
                "errors": stats.errors,
            })
            .to_string(),
        )
    }

    /// Select the solar time without changing the authoritative world clock.
    /// Zero/one is midnight, one quarter sunrise and three quarters sunset.
    /// # Errors
    /// Refuses a nonfinite time or one outside the normalized day.
    pub fn set_time_of_day(&mut self, fraction: f32) -> Result<(), JsValue> {
        game::set_time_of_day(&mut self.shell, fraction).map_err(js_error)?;
        self.demo.observe_time(game::time_of_day(&self.shell));
        Ok(())
    }

    /// Observe the solar time of the latest committed world endpoint.
    pub fn time_of_day(&self) -> f32 {
        game::time_of_day(&self.shell)
    }

    /// Look up/down within the scene's supported camera range.
    /// # Errors
    /// Refuses a nonfinite pitch or one outside plus/minus 0.6 radians.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), JsValue> {
        game::set_camera_pitch(&mut self.shell, radians).map_err(js_error)?;
        self.demo
            .observe_camera(game::camera_orientation(&self.shell));
        Ok(())
    }

    /// Turn the camera within plus/minus pi radians.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside that range.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), JsValue> {
        game::set_camera_yaw(&mut self.shell, radians).map_err(js_error)?;
        self.demo
            .observe_camera(game::camera_orientation(&self.shell));
        Ok(())
    }

    /// Every demo control as JSON for `thrum-controls.js` to build the page's widgets:
    /// keys, labels, kinds, ranges and values in display units. The page is then in sync.
    pub fn controls_json(&mut self) -> String {
        self.page_seen = self.demo.model().revision();
        self.demo.model().describe_json()
    }

    /// Controls changed since the page last synced, as JSON, or `undefined` when none did.
    /// The page calls this after edits and each frame; the running clock changes solar time
    /// once per slider stop.
    pub fn control_changes(&mut self) -> Option<String> {
        let model = self.demo.model();
        if model.revision() == self.page_seen {
            return None;
        }
        let changes = model.changes_json(self.page_seen);
        self.page_seen = model.revision();
        Some(changes)
    }

    /// Apply a person's edit from the page and route it through the game's checked setters
    /// at once. Sliders take a display value (hours, degrees) and snap to their stops;
    /// toggles take a boolean; text fields a string. `world.paused` takes the same path as
    /// [`Self::set_paused`]. Returns whether the value changed.
    /// # Errors
    /// Names an unknown key (with the nearest declared one), a value of the wrong type,
    /// or a setter or lifecycle refusal.
    pub fn edit_control(
        &mut self,
        key: &str,
        value: &JsValue,
        timestamp_ms: f64,
    ) -> Result<bool, JsValue> {
        let id = self.demo.model().lookup(key).map_err(js_error)?;
        let model = self.demo.model_mut();
        let changed = if let Some(on) = value.as_bool() {
            model.edit_value(id, Value::Flag(on)).map_err(js_error)?
        } else if let Some(number) = value.as_f64() {
            model.edit_display(id, number).map_err(js_error)?
        } else if let Some(text) = value.as_string() {
            model.edit_value(id, Value::Text(text)).map_err(js_error)?
        } else {
            return Err(js_error(format!(
                "control \"{key}\" needs a number, boolean or string"
            )));
        };
        let edits = self.demo.take_edits();
        self.apply_control_edits(edits, timestamp_ms)?;
        Ok(changed)
    }

    /// Apply a parameter file: a JSON object of control keys to display values. Valid
    /// entries apply at once; the result lists how many changed and explains each refusal.
    /// # Errors
    /// Refuses text that is not a JSON object, or a setter or lifecycle refusal.
    pub fn apply_parameters(&mut self, json: &str, timestamp_ms: f64) -> Result<String, JsValue> {
        let report = self.demo.model_mut().apply_file(json).map_err(js_error)?;
        let edits = self.demo.take_edits();
        self.apply_control_edits(edits, timestamp_ms)?;
        let problems: Vec<String> = report.problems.iter().map(ToString::to_string).collect();
        Ok(format!(
            "{{\"changed\":{},\"unchanged\":{},\"problems\":{}}}",
            report.changed,
            report.unchanged,
            serde_json::to_string(&problems).map_err(js_error)?
        ))
    }

    /// Advance the fixed clock and encode the scene before acquiring a drawable.
    ///
    /// # Errors
    /// Returns a JavaScript error for an authoritative tick or GPU host fault.
    pub fn frame(&mut self, timestamp_ms: f64) -> Result<(), JsValue> {
        self.demo.observe_time(game::time_of_day(&self.shell));
        let result = self.shell.frame(timestamp_ms).map_err(js_error);
        if let Some(error) = self.shell.take_attach_error()
            && let Some(Ok(mut stats)) = self.cooked.as_ref().map(|stats| stats.lock())
        {
            stats.errors.push(format!(
                "cooked valley attach on the replacement device: {error}"
            ));
        }
        result
    }

    /// Sample normalized player movement for the next authoritative world tick.
    ///
    /// # Errors
    /// Returns a JavaScript error for a nonfinite or out-of-range axis.
    pub fn set_controls(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
        game::set_controls(&mut self.shell, [x, y, 0.0]).map_err(js_error)
    }

    /// Committed position of the one local player in this V1 fixture.
    #[must_use]
    pub fn controlled_position(&self) -> String {
        game::controlled_position(&self.shell).map_or_else(
            || "player unavailable".to_owned(),
            |position| format!("player x={:.3} y={:.3}", position[0], position[1]),
        )
    }

    /// Pause for a hidden tab and restart the clock when it becomes visible.
    /// Queued accepted ticks drain before resume; hidden wall time is not admitted.
    ///
    /// # Errors
    /// Returns a JavaScript error if a retained tick or clock restart fails.
    pub fn set_visible(&mut self, visible: bool, timestamp_ms: f64) -> Result<(), JsValue> {
        self.shell
            .set_visible(visible, timestamp_ms)
            .map_err(js_error)
    }

    /// Stop admitting world time while retaining camera and sky controls.
    /// Already accepted updates drain through ordinary frame callbacks.
    /// Tab visibility cannot override this explicit local pause preference.
    /// # Errors
    /// Returns a timestamp, retained-update or clock restart failure.
    pub fn set_paused(&mut self, paused: bool, timestamp_ms: f64) -> Result<(), JsValue> {
        self.shell
            .set_paused(paused, timestamp_ms)
            .map_err(js_error)?;
        self.demo.observe_paused(self.shell.paused());
        Ok(())
    }

    /// Whether the local user requested pause, independent of tab visibility.
    pub fn paused(&self) -> bool {
        self.shell.paused()
    }

    /// Persistent device-loss diagnostic. Rendering stops while the world and
    /// local controls remain usable until a complete replacement is published.
    pub fn graphics_fault(&self) -> Option<String> {
        self.shell.graphics_fault()
    }

    /// Start detached graphics reconstruction without borrowing the running world.
    /// # Errors
    /// Refuses a healthy device, duplicate attempt or invalid retained inputs.
    pub fn request_graphics_recovery(&mut self) -> Result<(), JsValue> {
        self.shell.request_graphics_recovery().map_err(js_error)
    }

    /// Whether an explicit attempt is preparing or awaiting its first present call.
    pub fn recovering(&self) -> bool {
        self.shell.recovering()
    }

    /// Last failed reconstruction attempt; a fresh explicit request clears it.
    pub fn graphics_recovery_error(&self) -> Option<String> {
        self.shell.recovery_error().map(str::to_owned)
    }

    /// Submit a capture of the committed endpoint for the page's oracle and
    /// return `tick=<n>;extent=<w>x<h>;generation=<g>`. One capture at a time.
    /// Like [`Self::frame`], it does no GPU work once the device is lost.
    /// # Errors
    /// Refuses a lost device or a second outstanding capture, or returns a
    /// recording failure.
    pub fn begin_capture(&mut self) -> Result<String, JsValue> {
        if self.capture.is_some() && !self.shell.graphics_lost() {
            return Err(JsValue::from_str("a capture is already outstanding"));
        }
        let pending = self.shell.begin_capture().map_err(js_error)?;
        let [width, height] = pending.extent();
        let identity = format!(
            "tick={};extent={width}x{height};generation={}",
            pending.tick(),
            pending.generation()
        );
        self.capture = Some(pending);
        Ok(identity)
    }

    /// The outstanding capture's sRGB RGBA8 pixels once ready, else nothing.
    /// A failed, lost or retired capture is dropped and reported. The loss
    /// signal is checked before any poll, so a readback that completed before
    /// loss is never returned after it.
    /// # Errors
    /// Returns a lost device, retired device generation or readback failure.
    pub fn try_finish_capture(&mut self) -> Result<Option<Vec<u8>>, JsValue> {
        let Some(pending) = self.capture.as_mut() else {
            return Ok(None);
        };
        match self.shell.try_finish_capture(pending) {
            Ok(None) => Ok(None),
            Ok(Some(pixels)) => {
                self.capture = None;
                Ok(Some(pixels))
            }
            Err(error) => {
                self.capture = None;
                Err(js_error(error))
            }
        }
    }

    /// Number of surface presentation calls, not physical displayed frames.
    #[must_use]
    pub fn presented(&self) -> u64 {
        self.shell.presented()
    }

    /// True after 600 GPU-complete frame samples have been collected.
    #[must_use]
    pub fn complete(&self) -> bool {
        self.shell.samples().len() == SAMPLES
    }

    /// Final callback, world and GPU report after 600 completed frames.
    #[must_use]
    pub fn report(&self) -> String {
        let samples = self.shell.samples();
        let status = self.shell.status();
        let (shed_views, report_drops) = game::dropped(&self.shell);
        let timed = samples
            .iter()
            .filter(|sample| sample.gpu_pass_ns.is_some() && sample.gpu_present_ns.is_some())
            .count();
        let tick = percentile(samples, |sample| sample.tick_cpu.as_secs_f64() * 1e6);
        let encode = percentile(samples, |sample| sample.encode_cpu.as_secs_f64() * 1e6);
        let gpu = if timed == samples.len() && timed > 0 {
            let near = percentile(samples, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::shadow_near)
                    / 1e6
            });
            let far = percentile(samples, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::shadow_far)
                    / 1e6
            });
            let cluster = percentile(samples, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::clusters)
                    / 1e6
            });
            let forward = percentile(samples, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::forward)
                    / 1e6
            });
            let history = percentile(samples, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::history)
                    / 1e6
            });
            let display = percentile(samples, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::tone)
                    / 1e6
            });
            let present = percentile(samples, |sample| {
                sample.gpu_present_ns.unwrap_or_default() / 1e6
            });
            format!(
                "GPU near/far/cluster/forward/history/display/present p50-p95 ms={:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}",
                near.0,
                near.1,
                far.0,
                far.1,
                cluster.0,
                cluster.1,
                forward.0,
                forward.1,
                history.0,
                history.1,
                display.0,
                display.1,
                present.0,
                present.1
            )
        } else {
            "GPU timestamps unavailable or incomplete".to_owned()
        };
        // Camera culling's read-back draw counts, over the frames whose
        // count had settled when they retired; Direct reports none.
        let mut drawn: Vec<u32> = samples
            .iter()
            .filter_map(|sample| sample.camera_culling.map(|counts| counts.drawn))
            .collect();
        drawn.sort_unstable();
        let mut prepass: Vec<u32> = samples
            .iter()
            .filter_map(|sample| sample.camera_culling.map(|counts| counts.prepass))
            .collect();
        prepass.sort_unstable();
        let culling = match samples
            .iter()
            .rev()
            .find_map(|sample| sample.camera_culling)
        {
            Some(last) => format!(
                "; culling_counted={}/{} culling_drawn_p50={} culling_drawn_p95={} culling_prepass_p50={} culling_active={}",
                drawn.len(),
                samples.len(),
                drawn[drawn.len() / 2],
                drawn[drawn.len() * 95 / 100],
                prepass[prepass.len() / 2],
                last.active
            ),
            None => String::new(),
        };
        format!(
            "adapter={} format={:?} extent={}x{}; callbacks={} presented={} samples={} ticks={} elapsed={:.3}s unavailable={} overload_events={} refused_ticks={} timestamp_clamps={} shed_views={} report_drops={} generation={} recovery_attempts={} recovered={} GPU_timed={} latest_visual_age={}; tick CPU p50/p95={:.1}/{:.1} us; encode CPU p50/p95={:.1}/{:.1} us; {gpu}; culling={:?}{culling}",
            status.adapter_name,
            status.format,
            status.extent[0],
            status.extent[1],
            status.callbacks,
            status.presented,
            samples.len(),
            game::tick(&self.shell),
            status.last_frame.as_secs_f64(),
            status.unavailable,
            status.overload_events,
            status.refused_ticks,
            status.timestamp_clamps,
            shed_views,
            report_drops,
            status.generation,
            status.recovery_attempts,
            status.recovered,
            timed,
            samples.last().map_or(0, |sample| sample.visual_age_ticks),
            tick.0,
            tick.1,
            encode.0,
            encode.1,
            game::camera_culling(&self.shell).resolved,
        ) + &format!(
            "; culling_requested={:?} culling_reason={:?}",
            game::camera_culling(&self.shell).requested,
            game::camera_culling(&self.shell).reason
        )
    }
}

impl BrowserGame {
    fn apply_control_edits(
        &mut self,
        edits: ControlEdits,
        timestamp_ms: f64,
    ) -> Result<(), JsValue> {
        if let Some(time) = edits.time_of_day {
            game::set_time_of_day(&mut self.shell, time).map_err(js_error)?;
        }
        if let Some(pitch) = edits.camera_pitch {
            game::set_camera_pitch(&mut self.shell, pitch).map_err(js_error)?;
        }
        if let Some(yaw) = edits.camera_yaw {
            game::set_camera_yaw(&mut self.shell, yaw).map_err(js_error)?;
        }
        if let Some(overlay) = edits.display_overlay() {
            game::set_display_overlay(&mut self.shell, overlay);
        }
        if let Some(paused) = edits.paused {
            self.set_paused(paused, timestamp_ms)?;
        }
        Ok(())
    }
}

fn percentile(samples: &[FrameSample], value: impl Fn(&FrameSample) -> f64) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut values: Vec<_> = samples.iter().map(value).collect();
    values.sort_by(f64::total_cmp);
    (values[values.len() / 2], values[values.len() * 95 / 100])
}

fn js_error(value: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&value.to_string())
}

/// Run the external diagnostic's exact state script on Chrome's WebGPU device.
/// # Errors
/// Returns a target preparation, state mismatch or recovery failure.
#[wasm_bindgen]
pub async fn qualify_effect_state() -> Result<String, JsValue> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(js_error)?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(js_error)?;
    let caps = thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
    Box::pin(crate::effect::qualification::run(
        &device, &queue, &caps, &instance,
    ))
    .await
    .map_err(js_error)
}

/// Qualify cooked leases on the browser's real fetched feed and replacement device.
/// # Errors
/// Returns a fetch, GPU, texel or publication mismatch.
#[wasm_bindgen]
pub async fn qualify_lent_products(origin: String) -> Result<String, JsValue> {
    Box::pin(crate::lent::browser::run(origin))
        .await
        .map_err(js_error)
}
