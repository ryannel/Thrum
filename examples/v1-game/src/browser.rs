//! Browser WebGPU surface adapter for the same fixed-step V1 frame host.

use std::time::Duration;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::{CookedSource, CookedStats, RemoteFeed};
#[cfg(feature = "dev-material")]
use thrum_asset_gpu::{MaterialHook, MaterialOutcome, MaterialReload};
use thrum_render::present::SurfaceProfile;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{
    FrameConfig, FrameHost, FrameSample, SurfaceOutput,
    controls::{ControlEdits, GameControls},
};
use thrum_controls::Value;

mod recovery;

const FRAMES: usize = 600;

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

/// One browser canvas, WebGPU surface and serial V1 game host.
#[wasm_bindgen]
pub struct BrowserGame {
    canvas: HtmlCanvasElement,
    adapter_name: String,
    surface: Rc<wgpu::Surface<'static>>,
    recovery: recovery::Recovery,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    graphics: Arc<GraphicsFault>,
    queue: wgpu::Queue,
    host: FrameHost,
    cooked: Option<Arc<Mutex<CookedStats>>>,
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
    first_frame_ms: Option<f64>,
    last_frame: Duration,
    visible: bool,
    user_paused: bool,
    callbacks: u64,
    presented: u64,
    unavailable: u64,
    overload_events: u64,
    refused_ticks: u64,
    timestamp_clamps: u64,
    needs_reconfigure: bool,
    samples: Vec<FrameSample>,
    /// The page's outstanding capture, finished from a later animation frame.
    capture: Option<thrum_render::runtime::PendingCapture>,
}

/// Prepare the existing V1 simulation and renderer for a WebGPU canvas.
/// `valley_stars` selects the star catalogue before attaching the cooked valley.
///
/// # Errors
/// Returns a JavaScript error when no compatible adapter, surface format or GPU resource exists.
#[wasm_bindgen]
pub async fn create_browser_game(
    canvas: HtmlCanvasElement,
    valley_stars: Option<bool>,
) -> Result<BrowserGame, JsValue> {
    create_game(
        canvas,
        crate::GameSetup {
            valley_stars: valley_stars.unwrap_or(false),
            field_meshes: false,
            ..crate::GameSetup::default()
        },
    )
    .await
}

/// Prepare the dense flythrough field with `"direct"`, `"frustum"`, or `"occlusion"` culling.
///
/// # Errors
/// Returns an unknown culling mode, or the same failures as [`create_browser_game`].
#[wasm_bindgen]
pub async fn create_field_game(
    canvas: HtmlCanvasElement,
    culling: String,
) -> Result<BrowserGame, JsValue> {
    let camera_culling = match culling.as_str() {
        "direct" => thrum_render::synthetic::CullMode::Direct,
        "frustum" => thrum_render::synthetic::CullMode::Frustum,
        "occlusion" => thrum_render::synthetic::CullMode::Occlusion,
        _ => {
            return Err(JsValue::from_str(
                "culling must be direct, frustum or occlusion",
            ));
        }
    };
    create_game(
        canvas,
        crate::GameSetup {
            content: crate::GameContent::Field,
            camera_culling,
            valley_stars: false,
            field_meshes: false,
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
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::BROWSER_WEBGPU;
    let instance = wgpu::Instance::new(descriptor);
    let surface = Rc::new(
        instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?,
    );
    let gpu = recovery::request_device(
        &instance,
        &surface,
        [canvas.width().max(1), canvas.height().max(1)],
        FrameHost::DEVICE_GENERATION,
    )
    .await
    .map_err(js_error)?;
    surface.configure(&gpu.device, &gpu.config);
    let host = FrameHost::new_with_setup(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        FrameConfig {
            render_extent: [gpu.config.width, gpu.config.height],
            surface: Some(SurfaceProfile {
                format: gpu.config.format,
                extent: [gpu.config.width, gpu.config.height],
            }),
        },
        setup,
    )
    .await
    .map_err(js_error)?;
    let recovery = recovery::Recovery::new(instance, Rc::clone(&surface));
    let mut demo = GameControls::new().map_err(js_error)?;
    demo.observe_camera(host.camera_orientation());
    demo.observe_time(host.time_of_day());
    Ok(BrowserGame {
        canvas,
        adapter_name: gpu.adapter_name,
        surface,
        recovery,
        config: gpu.config,
        device: gpu.device,
        graphics: gpu.fault,
        queue: gpu.queue,
        host,
        cooked: None,
        #[cfg(feature = "dev-material")]
        material: None,
        #[cfg(feature = "dev-material")]
        material_reported: 0,
        demo,
        page_seen: 0,
        first_frame_ms: None,
        last_frame: Duration::ZERO,
        visible: true,
        user_paused: false,
        callbacks: 0,
        presented: 0,
        unavailable: 0,
        overload_events: 0,
        refused_ticks: 0,
        timestamp_clamps: 0,
        needs_reconfigure: false,
        samples: Vec::with_capacity(FRAMES),
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
    /// Returns an error when no cooked valley is attached.
    pub fn reload_material(&mut self, name: String, source: String) -> Result<f64, JsValue> {
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

#[wasm_bindgen]
impl BrowserGame {
    /// Draw the authored valley from fetched cooked products, with its authored camera.
    /// The products survive device replacement: a new device stages the newest fetched
    /// generation again. Publications fetched later by [`CookedValley::next_publication`] swap
    /// in as they upload.
    /// # Errors
    /// Returns an invalid embedded recipe or the scene's refusal of the contribution.
    pub fn attach_cooked_valley(&mut self, valley: &CookedValley) -> Result<(), JsValue> {
        let (camera, assets) =
            crate::showcase::prepare_showcase(CookedSource::Remote(valley.feed.clone()))
                .map_err(js_error)?;
        let stats = assets.stats();
        #[cfg(feature = "dev-material")]
        let material = assets.material();
        self.host
            .set_scene_contribution(Box::new(assets))
            .map_err(js_error)?;
        self.host.set_scene_camera(camera);
        self.demo.observe_camera(self.host.camera_orientation());
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
        self.host.set_time_of_day(fraction).map_err(js_error)?;
        self.demo.observe_time(self.host.time_of_day());
        Ok(())
    }

    /// Observe the solar time of the latest committed world endpoint.
    pub fn time_of_day(&self) -> f32 {
        self.host.time_of_day()
    }

    /// Look up/down within the scene's supported camera range.
    /// # Errors
    /// Refuses a nonfinite pitch or one outside plus/minus 0.6 radians.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), JsValue> {
        self.host.set_camera_pitch(radians).map_err(js_error)?;
        self.demo.observe_camera(self.host.camera_orientation());
        Ok(())
    }

    /// Turn the camera within plus/minus pi radians.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside that range.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), JsValue> {
        self.host.set_camera_yaw(radians).map_err(js_error)?;
        self.demo.observe_camera(self.host.camera_orientation());
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
        let now = self.time_from_browser(timestamp_ms)?;
        self.demo.observe_time(self.host.time_of_day());
        if !self.visible {
            return Ok(());
        }
        self.callbacks += 1;
        self.last_frame = now;
        let extent = [self.canvas.width(), self.canvas.height()];
        if let Some((gpu, reconfigure)) = self.recovery.poll(&mut self.host, extent) {
            self.adapter_name = gpu.adapter_name;
            self.config = gpu.config;
            self.device = gpu.device;
            self.queue = gpu.queue;
            self.graphics = gpu.fault;
            self.needs_reconfigure = reconfigure;
            // Timings belong to one device generation, while world/platform
            // counters retain their identity across graphics reconstruction.
            self.samples.clear();
        }
        if self.graphics.is_lost() {
            self.recovery.awaiting_first_present = false;
            self.unavailable += 1;
            // No polling, completion collection, resize or GPU encoding after
            // loss. Authority and controls continue on the same resident world.
            let report = self.host.advance_without_rendering(now).map_err(js_error)?;
            if let Some(refused_ticks) = report.overload {
                self.recover_overload(refused_ticks, now)?;
            }
            return Ok(());
        }
        let mut resize = extent != [self.config.width, self.config.height];
        if (resize || self.needs_reconfigure)
            && extent[0] > 0
            && extent[1] > 0
            && self.host.pending_frames() == 0
        {
            self.config.width = extent[0];
            self.config.height = extent[1];
            self.surface.configure(&self.device, &self.config);
            self.host.resize_surface(extent).map_err(js_error)?;
            self.needs_reconfigure = false;
            resize = false;
        }
        let advanced = self.host.advance(now).map_err(js_error)?;
        let can_acquire = advanced.can_acquire_serial()
            && extent[0] > 0
            && extent[1] > 0
            && !resize
            && !self.needs_reconfigure;
        let report = if can_acquire {
            let encoded = advanced.encode_scene().map_err(js_error)?;
            let frame = if encoded.can_acquire_surface() {
                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
                    wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                        self.needs_reconfigure = true;
                        Some(frame)
                    }
                    wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                        self.needs_reconfigure = true;
                        self.unavailable += 1;
                        None
                    }
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => {
                        self.unavailable += 1;
                        None
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        return Err(js_error("canvas surface validation fault"));
                    }
                }
            } else {
                None
            };
            if let Some(frame) = frame {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let report = encoded
                    .submit(Some(&SurfaceOutput {
                        view: &view,
                        extent: [self.config.width, self.config.height],
                    }))
                    .map_err(js_error)?;
                if report.surface_presented {
                    self.queue.present(frame);
                    self.presented += 1;
                    self.recovery.awaiting_first_present = false;
                }
                report
            } else {
                encoded.skip()
            }
        } else {
            advanced.submit(None).map_err(js_error)?
        };
        if let Some(refused_ticks) = report.overload {
            self.recover_overload(refused_ticks, now)?;
        }
        while let Some(sample) = self.host.poll_completed().map_err(js_error)? {
            if self.samples.len() < FRAMES {
                self.samples.push(sample);
            }
        }
        Ok(())
    }

    /// Sample normalized player movement for the next authoritative world tick.
    ///
    /// # Errors
    /// Returns a JavaScript error for a nonfinite or out-of-range axis.
    pub fn set_controls(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
        self.host.set_controls([x, y, 0.0]).map_err(js_error)
    }

    /// Committed position of the one local player in this V1 fixture.
    #[must_use]
    pub fn controlled_position(&self) -> String {
        self.host.controlled_position().map_or_else(
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
        let now = self.time_from_browser(timestamp_ms)?;
        self.set_play_state(visible, self.user_paused, now)
    }

    /// Stop admitting world time while retaining camera and sky controls.
    /// Already accepted updates drain through ordinary frame callbacks.
    /// Tab visibility cannot override this explicit local pause preference.
    /// # Errors
    /// Returns a timestamp, retained-update or clock restart failure.
    pub fn set_paused(&mut self, paused: bool, timestamp_ms: f64) -> Result<(), JsValue> {
        let now = self.time_from_browser(timestamp_ms)?;
        self.set_play_state(self.visible, paused, now)?;
        self.demo.observe_paused(self.user_paused);
        Ok(())
    }

    /// Whether the local user requested pause, independent of tab visibility.
    pub fn paused(&self) -> bool {
        self.user_paused
    }

    /// Persistent device-loss diagnostic. Rendering stops while the world and
    /// local controls remain usable until a complete replacement is published.
    pub fn graphics_fault(&self) -> Option<String> {
        self.graphics.message()
    }

    /// Start detached graphics reconstruction without borrowing the running world.
    /// # Errors
    /// Refuses a healthy device, duplicate attempt or invalid retained inputs.
    pub fn request_graphics_recovery(&mut self) -> Result<(), JsValue> {
        if !self.graphics.is_lost() {
            return Err(js_error("graphics device is still available"));
        }
        self.recovery
            .request(&self.host, [self.config.width, self.config.height])
            .map_err(js_error)
    }

    /// Whether an explicit attempt is preparing or awaiting its first present call.
    pub fn recovering(&self) -> bool {
        self.recovery.is_pending() || self.recovery.awaiting_first_present
    }

    /// Last failed reconstruction attempt; a fresh explicit request clears it.
    pub fn graphics_recovery_error(&self) -> Option<String> {
        self.recovery.error.clone()
    }

    /// Submit a capture of the committed endpoint for the page's oracle and
    /// return `tick=<n>;extent=<w>x<h>;generation=<g>`. One capture at a time.
    /// Like [`Self::frame`], it does no GPU work once the device is lost.
    /// # Errors
    /// Refuses a lost device or a second outstanding capture, or returns a
    /// recording failure.
    pub fn begin_capture(&mut self) -> Result<String, JsValue> {
        if self.graphics.is_lost() {
            return Err(js_error("graphics device is lost"));
        }
        if self.capture.is_some() {
            return Err(JsValue::from_str("a capture is already outstanding"));
        }
        let pending = self
            .host
            .begin_capture()
            .map_err(|error| JsValue::from_str(&error))?;
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
        if self.graphics.is_lost() && self.capture.take().is_some() {
            return Err(js_error(
                "graphics device was lost before the capture finished",
            ));
        }
        let Some(pending) = self.capture.as_mut() else {
            return Ok(None);
        };
        match self.host.try_finish_capture(pending) {
            Ok(None) => Ok(None),
            Ok(Some(pixels)) => {
                self.capture = None;
                Ok(Some(pixels))
            }
            Err(error) => {
                self.capture = None;
                Err(JsValue::from_str(&error))
            }
        }
    }

    /// Number of surface presentation calls, not physical displayed frames.
    #[must_use]
    pub fn presented(&self) -> u64 {
        self.presented
    }

    /// True after 600 GPU-complete frame samples have been collected.
    #[must_use]
    pub fn complete(&self) -> bool {
        self.samples.len() == FRAMES
    }

    /// Final callback, world and GPU report after 600 completed frames.
    #[must_use]
    pub fn report(&self) -> String {
        let timed = self
            .samples
            .iter()
            .filter(|sample| sample.gpu_pass_ns.is_some() && sample.gpu_present_ns.is_some())
            .count();
        let tick = percentile(&self.samples, |sample| sample.tick_cpu.as_secs_f64() * 1e6);
        let encode = percentile(&self.samples, |sample| {
            sample.encode_cpu.as_secs_f64() * 1e6
        });
        let gpu = if timed == self.samples.len() && timed > 0 {
            let near = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[0]) / 1e6
            });
            let far = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[1]) / 1e6
            });
            let cluster = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[2]) / 1e6
            });
            let forward = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[3]) / 1e6
            });
            let history = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[4]) / 1e6
            });
            let display = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[5]) / 1e6
            });
            let present = percentile(&self.samples, |sample| {
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
        let mut drawn: Vec<u32> = self
            .samples
            .iter()
            .filter_map(|sample| sample.camera_culling.map(|counts| counts.drawn))
            .collect();
        drawn.sort_unstable();
        let mut prepass: Vec<u32> = self
            .samples
            .iter()
            .filter_map(|sample| sample.camera_culling.map(|counts| counts.prepass))
            .collect();
        prepass.sort_unstable();
        let culling = match self
            .samples
            .iter()
            .rev()
            .find_map(|sample| sample.camera_culling)
        {
            Some(last) => format!(
                "; culling_counted={}/{} culling_drawn_p50={} culling_drawn_p95={} culling_prepass_p50={} culling_active={}",
                drawn.len(),
                self.samples.len(),
                drawn[drawn.len() / 2],
                drawn[drawn.len() * 95 / 100],
                prepass[prepass.len() / 2],
                last.active
            ),
            None => String::new(),
        };
        format!(
            "adapter={} format={:?} extent={}x{}; callbacks={} presented={} samples={} ticks={} elapsed={:.3}s unavailable={} overload_events={} refused_ticks={} timestamp_clamps={} shed_views={} report_drops={} generation={} recovery_attempts={} recovered={} GPU_timed={} latest_visual_age={}; tick CPU p50/p95={:.1}/{:.1} us; encode CPU p50/p95={:.1}/{:.1} us; {gpu}; culling={:?}{culling}",
            self.adapter_name,
            self.config.format,
            self.config.width,
            self.config.height,
            self.callbacks,
            self.presented,
            self.samples.len(),
            self.host.tick(),
            self.last_frame.as_secs_f64(),
            self.unavailable,
            self.overload_events,
            self.refused_ticks,
            self.timestamp_clamps,
            self.host.dropped_views(),
            self.host.dropped_reports(),
            self.recovery.generation,
            self.recovery.attempts,
            self.recovery.recovered,
            timed,
            self.samples
                .last()
                .map_or(0, |sample| sample.visual_age_ticks),
            tick.0,
            tick.1,
            encode.0,
            encode.1,
            self.host.camera_culling(),
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
            self.host.set_time_of_day(time).map_err(js_error)?;
        }
        if let Some(pitch) = edits.camera_pitch {
            self.host.set_camera_pitch(pitch).map_err(js_error)?;
        }
        if let Some(yaw) = edits.camera_yaw {
            self.host.set_camera_yaw(yaw).map_err(js_error)?;
        }
        if let Some(overlay) = edits.display_overlay() {
            self.host.set_display_overlay(overlay);
        }
        if let Some(paused) = edits.paused {
            self.set_paused(paused, timestamp_ms)?;
        }
        Ok(())
    }

    fn set_play_state(
        &mut self,
        visible: bool,
        user_paused: bool,
        now: Duration,
    ) -> Result<(), JsValue> {
        let was_running = self.visible && !self.user_paused;
        let will_run = visible && !user_paused;
        if will_run && !was_running {
            self.host.resume_after_drain(now).map_err(js_error)?;
        } else if !will_run && was_running {
            self.host.pause().map_err(js_error)?;
        }
        self.visible = visible;
        self.user_paused = user_paused;
        self.last_frame = now;
        Ok(())
    }

    fn recover_overload(&mut self, refused_ticks: u64, now: Duration) -> Result<(), JsValue> {
        self.overload_events += 1;
        self.refused_ticks = self.refused_ticks.saturating_add(refused_ticks);
        self.host.pause().map_err(js_error)?;
        self.host.resume_after_drain(now).map_err(js_error)
    }

    fn time_from_browser(&mut self, timestamp_ms: f64) -> Result<Duration, JsValue> {
        if !timestamp_ms.is_finite() || timestamp_ms < 0.0 {
            return Err(js_error(
                "animation timestamp must be finite and nonnegative",
            ));
        }
        let first = *self.first_frame_ms.get_or_insert(timestamp_ms);
        let now = Duration::try_from_secs_f64((timestamp_ms - first).max(0.0) / 1_000.0)
            .map_err(|_| js_error("animation timestamp outside supported range"))?;
        if now < self.last_frame {
            // RAF reports the frame-start time; a visibility event samples
            // performance.now() later in that same frame.
            self.timestamp_clamps += 1;
            return Ok(self.last_frame);
        }
        Ok(now)
    }
}

// The callback owns no world/renderer reference. The hot path observes one
// atomic; the cold diagnostic allocates only when loss is reported/read.
struct GraphicsFault {
    lost: AtomicBool,
    first: Mutex<Option<String>>,
}
impl GraphicsFault {
    fn register(device: &wgpu::Device) -> Arc<Self> {
        let fault = Arc::new(Self {
            lost: AtomicBool::new(false),
            first: Mutex::new(None),
        });
        let signal = Arc::clone(&fault);
        device.set_device_lost_callback(move |reason, message| {
            if let Ok(mut first) = signal.first.lock()
                && first.is_none()
            {
                *first = Some(format!("{reason:?}: {message}"));
            }
            signal.lost.store(true, Ordering::Release);
        });
        fault
    }
    fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
    fn message(&self) -> Option<String> {
        if !self.is_lost() {
            return None;
        }
        Some(
            self.first
                .lock()
                .ok()
                .and_then(|first| first.clone())
                .unwrap_or_else(|| "Graphics device lost".to_owned()),
        )
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
