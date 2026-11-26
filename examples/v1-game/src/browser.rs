//! The V1 game's browser page bindings over the engine's web shell (`thrum_app::web`).
//!
//! The shell owns the canvas surface, the device, loss and replacement, and the frame step; this
//! facade keeps the game's controls, cooked valley, material reloads, capture slot and report.

#[cfg(feature = "proving")]
mod cloud_observation;
#[cfg(feature = "proving")]
mod environment;
#[cfg(feature = "proving")]
mod governor;
#[cfg(feature = "proving")]
mod interface_menu;
#[cfg(feature = "proving")]
pub use governor::{GovernorBrowser, create_governor_game};
#[cfg(feature = "draw-probe")]
mod light_probe;
#[cfg(feature = "draw-probe")]
pub use light_probe::{BrowserLightProbe, create_light_list_probe};
#[cfg(feature = "proving")]
mod sprites;
#[cfg(feature = "proving")]
mod submitted_capture;

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

#[cfg(feature = "proving")]
use thrum_app::web::SAMPLES;
use thrum_app::web::{ControlTiming, PageGame, PageRuntime};
use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::{CookedSource, CookedStats, RemoteFeed};
#[cfg(feature = "dev-material")]
use thrum_asset_gpu::{MaterialHook, MaterialOutcome, MaterialReload};
#[cfg(feature = "proving")]
use thrum_render::capabilities::{DeviceProfile, DeviceRecipe, DeviceRestriction, OpenedDevice};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use thrum_render::runtime::FrameSample;

use crate::{GameReconstruction, host::web as game, hosted::HostedGame};

#[cfg(feature = "proving")]
use crate::host::haze_capture::HazeTake;

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
    content: crate::GameContent,
    #[cfg(feature = "proving")]
    temporal_inputs: bool,
    page: PageGame<HostedGame<game::PageGame>>,
    cloud_sky: Option<thrum_clouds::SkyProfile>,
    #[cfg(feature = "proving")]
    haze_take: Option<HazeTake>,
    #[cfg(feature = "proving")]
    cloud_observation: Option<Rc<RefCell<cloud_observation::PreviousSun>>>,
    cooked: Option<Arc<Mutex<CookedStats>>>,
    #[cfg(feature = "proving")]
    content_preview: Option<crate::content_preview::PreviewRecord>,
    /// Explicit material reloads of the attached cooked valley (development builds only).
    #[cfg(feature = "dev-material")]
    material: Option<MaterialReload>,
    /// The material revision last reported, so an idle page builds no report.
    #[cfg(feature = "dev-material")]
    material_reported: u64,
    /// The page's outstanding capture, finished from a later animation frame.
    #[cfg(feature = "proving")]
    capture: Option<thrum_render::runtime::PendingCapture>,
}

impl BrowserGame {
    fn page_runtime(&self) -> &dyn PageRuntime {
        &self.page
    }

    fn page_runtime_mut(&mut self) -> &mut dyn PageRuntime {
        &mut self.page
    }
}

thrum_app::export_page_methods!(BrowserGame);

#[cfg(feature = "proving")]
#[wasm_bindgen]
impl BrowserGame {
    /// Time the same layout and paint workloads as the native cost command.
    /// # Errors
    /// Returns an inactive status scene, an unavailable clock or a refused measured tree.
    pub fn status_layout_costs(&self) -> Result<String, JsValue> {
        if self.page.game.0.setup.interface_text != Some(crate::InterfaceText::Status) {
            return Err(js_error("status costs need the status display scene"));
        }
        let performance = web_sys::window()
            .and_then(|window| window.performance())
            .ok_or_else(|| js_error("performance clock is unavailable"))?;
        let rows = crate::GameScene::status_layout_costs(|| performance.now())
            .map_err(|error| js_error(error.to_string()))?;
        Ok(serde_json::json!(rows).to_string())
    }

    /// Report retirement separately from timing eligibility while the status display becomes resident.
    pub fn status_cost_report(&self) -> String {
        let host = self.page.shell.host();
        let counts = host
            .source()
            .status_stats()
            .map(|s| [s.layouts, s.lists, s.rows]);
        let latest_presented = self
            .page
            .shell
            .samples()
            .iter()
            .rfind(|frame| frame.surface_presented);
        let frame = latest_presented.and_then(|frame| {
            let interface = frame.interface?;
            let timings = frame.gpu_pass_ns.as_ref()?;
            Some(serde_json::json!({
                "stamp":interface.stamp.map(|stamp| stamp.scale),
                "upload_bytes":interface.uploaded_bytes,
                "cpu_ns":frame.render_cpu.as_nanos(),
                "gpu_ns":timings.timed_pass_span_nanoseconds(),
                "tick":frame.tick,
                "submission":frame.submission_order,
            }))
        });
        let retired = latest_presented.map(|frame| frame.submission_order);
        serde_json::json!({"counts":counts,"retired":retired,"frame":frame}).to_string()
    }

    /// Select the measured every-tick status workload via the page's controls.
    pub fn set_status_cost_workload(&mut self) {
        use thrum_app::Game as _;
        self.page.game.0.status_cost = true;
        self.page
            .shell
            .update_profiler(self.page.game.controls_mut(), true);
    }

    /// Publish the page's current canvas density and safe-area insets.
    pub fn set_interface_output(
        &mut self,
        density: f64,
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
    ) {
        self.page
            .shell
            .set_interface_output(density, [left, top, right, bottom]);
    }
}

/// Prepare the existing V1 simulation and renderer for a WebGPU canvas.
/// `valley_stars` selects the star catalogue before attaching the cooked valley.
/// `cooked_valley` selects the measured valley default independently of its stars.
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
    reconstruction: Option<bool>,
    cooked_valley: Option<bool>,
    exit: Option<bool>,
    clouds: Option<String>,
) -> Result<BrowserGame, JsValue> {
    let exit = exit.unwrap_or(false);
    #[cfg(not(feature = "proving"))]
    if diagnostic_clouds == Some(true) || temporal_inputs == Some(true) {
        return Err(js_error("diagnostic scenes require the proving build"));
    }
    create_game(
        canvas,
        crate::GameSetup {
            content: if exit {
                crate::GameContent::Exit
            } else {
                crate::GameContent::Grid
            },
            reconstruction: if reconstruction.unwrap_or(false) {
                GameReconstruction::Taau
            } else {
                GameReconstruction::Off
            },
            #[cfg(feature = "proving")]
            display_overlay: if temporal_inputs.unwrap_or(false) {
                thrum_render::synthetic::DisplayOverlay::TemporalInputs
            } else {
                thrum_render::synthetic::DisplayOverlay::Off
            },
            #[cfg(feature = "proving")]
            diagnostic_clouds: diagnostic_clouds.unwrap_or(false),
            clouds: cloud_selection(clouds.as_deref(), diagnostic_clouds.unwrap_or(false))?,
            valley_stars: valley_stars.unwrap_or(false),
            field_meshes: meshes.unwrap_or(false),
            cooked_mesh_rows: (exit || (meshes.unwrap_or(false) && cooked_valley.unwrap_or(false)))
                .into(),
            camera_culling: valley_culling(
                culling.as_deref(),
                exit || cooked_valley.unwrap_or(false),
            )?,
            ..crate::GameSetup::default()
        },
    )
    .await
}

/// Prepare the dense flythrough field with explicit culling or the measured `"default"`.
///
/// # Errors
/// Returns an unknown culling mode, or the same failures as [`create_browser_game`].
#[cfg(feature = "proving")]
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
            sprite_scale: None,
            display_overlay: thrum_render::synthetic::DisplayOverlay::Off,
            clouds: crate::CloudSelection::Off,
            cloud_weather: crate::clouds::initial_state().parameters(),
            reconstruction: GameReconstruction::Off,
            content: crate::GameContent::Field,
            exit_entities: crate::ExitEntities::Draw,
            cooked_mesh_rows: crate::host::MeshRowSource::Code,
            #[cfg(feature = "proving")]
            diagnostic_clouds: false,
            haze: None,
            camera_culling,
            valley_stars: false,
            route_overlay: crate::RouteOverlay::Off,
            field_meshes,
            ..crate::GameSetup::default()
        },
    )
    .await
}

/// Prepare the sprite test card on the same WebGPU canvas and game clock.
/// # Errors
/// Refuses an unsupported scale or 3D-only launch setting.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn create_sprite_game(
    canvas: HtmlCanvasElement,
    scale: u16,
    sprite_rows: Option<u32>,
    culling: Option<String>,
    reconstruction: Option<String>,
) -> Result<BrowserGame, JsValue> {
    use thrum_render::synthetic::Reconstruction;
    if let Some(culling) = culling {
        return Err(js_error(format!(
            "2D scene refuses camera culling mode {culling}"
        )));
    }
    let reconstruction = match reconstruction.as_deref() {
        None | Some("off") => Reconstruction::Off,
        Some("taau") => Reconstruction::Taau,
        Some("reference") => Reconstruction::Reference,
        Some(provider) => {
            return Err(js_error(format!(
                "2D scene refuses reconstructor {provider}"
            )));
        }
    };
    create_game(
        canvas,
        crate::GameSetup {
            sprite_scale: Some(scale),
            sprite_rows,
            reconstruction,
            ..crate::GameSetup::default()
        },
    )
    .await
}

/// Prepare the panel card over the moving sprite field in the ordinary WebGPU canvas.
///
/// # Errors
/// Refuses unsupported scales or paint row counts and reports WebGPU preparation failures.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn create_interface_game(
    canvas: HtmlCanvasElement,
    scale: u8,
    sprite_rows: u32,
    paint_rows: Option<u32>,
) -> Result<BrowserGame, JsValue> {
    if !(1..=3).contains(&scale) || paint_rows.is_some_and(|rows| !matches!(rows, 0 | 2000)) {
        return Err(js_error(
            "interface scale must be 1..3 and the measured row count 0 or 2000",
        ));
    }
    create_game(
        canvas,
        crate::GameSetup {
            sprite_scale: Some(2),
            sprite_rows: Some(sprite_rows),
            interface_scale: Some(scale),
            interface_rows: paint_rows,
            ..crate::GameSetup::default()
        },
    )
    .await
}

/// Prepare a cooked-font card over the ordinary browser sprite field.
///
/// # Errors
/// Refuses an unknown card, an unsupported scale or an unavailable graphics device.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn create_interface_text_game(
    canvas: HtmlCanvasElement,
    store: &CookedValley,
    scale: u8,
    card: &str,
    rows: u32,
) -> Result<BrowserGame, JsValue> {
    let mode = match card {
        "type" => crate::InterfaceText::Type,
        "counting" => crate::InterfaceText::Counting,
        "specimen" => crate::InterfaceText::Specimen,
        "status" => crate::InterfaceText::Status,
        "menu" => crate::InterfaceText::Menu,
        _ => {
            return Err(js_error(
                "interface text card must be type, counting, specimen, status or menu",
            ));
        }
    };
    create_game_with_content(
        canvas,
        crate::GameSetup {
            sprite_scale: Some(2),
            sprite_rows: Some(rows),
            interface_scale: (!matches!(
                mode,
                crate::InterfaceText::Status | crate::InterfaceText::Menu
            ))
            .then_some(scale),
            interface_text: Some(mode),
            ..crate::GameSetup::default()
        },
        None,
        Some(store.feed.clone()),
    )
    .await
}

/// Open the moving sprite field with the fetched sheet store.
///
/// # Errors
/// Refuses an invalid sprite field configuration or unavailable WebGPU scene.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn create_cooked_sprite_game(
    canvas: HtmlCanvasElement,
    store: &CookedValley,
    rows: u32,
) -> Result<BrowserGame, JsValue> {
    create_game_with_content(
        canvas,
        crate::GameSetup {
            sprite_scale: Some(2),
            sprite_rows: Some(rows),
            ..crate::GameSetup::default()
        },
        None,
        Some(store.feed.clone()),
    )
    .await
}

/// Open the example's directed sky with its two volume products served by the cooker.
///
/// # Errors
/// Reports a failed fetch, invalid content, or any browser preparation failure.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn create_placed_browser_game(
    canvas: HtmlCanvasElement,
    origin: String,
) -> Result<BrowserGame, JsValue> {
    let placed = load_placed_clouds(&origin, true).await?;
    let setup = crate::GameSetup {
        clouds: crate::CloudSelection::Sky,
        ..crate::GameSetup::default()
    };
    create_game_with_shapes(canvas, setup, Some(placed)).await
}

#[cfg(feature = "proving")]
async fn load_placed_clouds(origin: &str, directed: bool) -> Result<crate::PlacedSky, JsValue> {
    let mut client = DevClient::new(BrowserFetch, origin).map_err(js_error)?;
    client.prefer(
        thrum_asset::ProductKind::Volume,
        &[thrum_asset::Profile::Portable],
    );
    let generation = client
        .poll()
        .await
        .map_err(js_error)?
        .ok_or_else(|| js_error("no cloud publication"))?;
    let feed = RemoteFeed::new(generation);
    crate::PlacedSky::new(CookedSource::Remote(feed), directed).map_err(js_error)
}

/// Prepare an authored sprite sampling scene on a WebGPU canvas.
///
/// # Errors
/// Refuses an unknown probe or a graphics startup error.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn create_sprite_probe(
    canvas: HtmlCanvasElement,
    probe: &str,
) -> Result<BrowserGame, JsValue> {
    let sprite_probe = crate::sprite_probe::parse(probe).map_err(js_error)?;
    create_game(
        canvas,
        crate::GameSetup {
            sprite_scale: Some(2),
            sprite_probe: Some(sprite_probe),
            ..Default::default()
        },
    )
    .await
}

async fn create_game(
    canvas: HtmlCanvasElement,
    setup: crate::GameSetup,
) -> Result<BrowserGame, JsValue> {
    create_game_with_shapes(canvas, setup, None).await
}

async fn create_game_with_shapes(
    canvas: HtmlCanvasElement,
    setup: crate::GameSetup,
    shapes: Option<crate::PlacedSky>,
) -> Result<BrowserGame, JsValue> {
    create_game_with_content(
        canvas,
        setup,
        shapes,
        #[cfg(feature = "proving")]
        None,
    )
    .await
}

async fn create_game_with_content(
    canvas: HtmlCanvasElement,
    setup: crate::GameSetup,
    shapes: Option<crate::PlacedSky>,
    #[cfg(feature = "proving")] sprite_store: Option<RemoteFeed>,
) -> Result<BrowserGame, JsValue> {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&info.to_string()));
    }));
    let cloud_sky = setup.clouds.sky_profile([canvas.width(), canvas.height()]);
    let hosted = HostedGame(game::PageGame {
        demo: setup.game_controls().map_err(js_error)?,
        setup,
        shapes,
        #[cfg(feature = "proving")]
        sprite_store,
        #[cfg(feature = "proving")]
        status_cost: false,
        #[cfg(feature = "proving")]
        menu: (setup.interface_text == Some(crate::InterfaceText::Menu))
            .then(|| crate::interface_menu::Menu::new(false)),
        initial_downlevel: String::new(),
    });
    let page = PageGame::new(
        canvas,
        game::DEVICE_GENERATION,
        hosted,
        ControlTiming::Immediate,
    )
    .await
    .map_err(js_error)?;
    #[cfg(feature = "proving")]
    let cooked = page.shell.host().source().sprite_sheet_stats();
    Ok(BrowserGame {
        content: setup.content,
        #[cfg(feature = "proving")]
        temporal_inputs: setup.display_overlay
            == thrum_render::synthetic::DisplayOverlay::TemporalInputs,
        page,
        cloud_sky,
        #[cfg(feature = "proving")]
        haze_take: None,
        #[cfg(feature = "proving")]
        cloud_observation: None,
        #[cfg(feature = "proving")]
        cooked,
        #[cfg(not(feature = "proving"))]
        cooked: None,
        #[cfg(feature = "proving")]
        content_preview: None,
        #[cfg(feature = "dev-material")]
        material: None,
        #[cfg(feature = "dev-material")]
        material_reported: 0,
        #[cfg(feature = "proving")]
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
        if self.page.shell.graphics_lost() {
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
    reconstruction: Option<bool>,
    exit: Option<bool>,
    clouds: Option<String>,
    haze: Option<String>,
    capture_weather: Option<String>,
) -> Result<BrowserGame, JsValue> {
    let exit = exit.unwrap_or(false);
    #[cfg(not(feature = "proving"))]
    if diagnostic_clouds == Some(true) || haze.is_some() || capture_weather.is_some() {
        return Err(js_error("diagnostic scenes require the proving build"));
    }
    let (extent, overlay) = crate::CameraRoute::declared(&route).map_err(js_error)?;
    #[cfg(not(feature = "proving"))]
    let _ = overlay;
    #[cfg(not(feature = "draw-probe"))]
    let valley_stars = true;
    #[cfg(feature = "draw-probe")]
    let valley_stars = overlay != crate::RouteOverlay::StarsOff;
    canvas.set_width(extent[0]);
    canvas.set_height(extent[1]);
    #[cfg(feature = "proving")]
    let weather: CaptureWeather = capture_weather
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(js_error)?
        .unwrap_or_default();
    #[cfg(feature = "proving")]
    let cloud_weather = weather.parameters()?;
    #[cfg(feature = "proving")]
    let shapes = if let Some(origin) = &weather.placed_origin {
        let sky = load_placed_clouds(origin, false).await?;
        Some(if let Some(placement) = &weather.placement {
            sky.with_capture_placement(placement).map_err(js_error)?
        } else {
            sky
        })
    } else {
        if weather.placement.is_some() {
            return Err(js_error(
                "a capture placement requires a cooked cloud store",
            ));
        }
        None
    };
    #[cfg(not(feature = "proving"))]
    let shapes = None;
    create_game_with_shapes(
        canvas,
        crate::GameSetup {
            content: if exit {
                crate::GameContent::Exit
            } else {
                crate::GameContent::Grid
            },
            valley_stars,
            reconstruction: if reconstruction.unwrap_or(false) {
                GameReconstruction::Taau
            } else {
                GameReconstruction::Off
            },
            #[cfg(feature = "proving")]
            route_overlay: overlay,
            #[cfg(feature = "proving")]
            diagnostic_clouds: diagnostic_clouds.unwrap_or(false),
            clouds: cloud_selection(clouds.as_deref(), diagnostic_clouds.unwrap_or(false))?,
            #[cfg(feature = "proving")]
            cloud_weather,
            #[cfg(feature = "proving")]
            haze: crate::host::haze_selection(haze.as_deref()).map_err(js_error)?,
            #[cfg(feature = "proving")]
            test_bank: haze.as_deref() == Some(crate::host::scene_source::test_bank::SELECTION),
            camera_culling: valley_culling(culling.as_deref(), true)?,
            field_meshes: meshes.unwrap_or(false),
            cooked_mesh_rows: (exit || meshes.unwrap_or(false)).into(),
            ..crate::GameSetup::default()
        },
        shapes,
    )
    .await
}

#[cfg(feature = "proving")]
#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureWeather {
    preset: Option<String>,
    rate: Option<f64>,
    wind: Option<f64>,
    placed_origin: Option<String>,
    placement: Option<String>,
}

#[cfg(feature = "proving")]
impl CaptureWeather {
    fn parameters(&self) -> Result<thrum_cloud_model::Parameters, JsValue> {
        let preset = match self.preset.as_deref() {
            None => crate::clouds::WeatherPreset::Fair,
            Some(name) => crate::clouds::WeatherPreset::named(name).ok_or_else(|| {
                js_error("weather preset must be clear, fair, broken or overcast")
            })?,
        };
        let mut parameters = preset.parameters();
        parameters.time_scale = self.rate.unwrap_or(parameters.time_scale);
        if let Some(wind) = self.wind {
            parameters.wind = [wind, 0.0];
        }
        Ok(parameters)
    }
}

fn cloud_selection(
    request: Option<&str>,
    diagnostic: bool,
) -> Result<crate::CloudSelection, JsValue> {
    let selection = crate::CloudSelection::named(request.unwrap_or("off")).ok_or_else(|| {
        JsValue::from_str(&format!("clouds must be {}", crate::CloudSelection::NAMES))
    })?;
    if diagnostic && selection != crate::CloudSelection::Off {
        return Err(JsValue::from_str(
            "clouds and diagnostic-clouds are mutually exclusive",
        ));
    }
    Ok(selection)
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
        game::install_route(&mut self.page.shell, route, drawn).map_err(js_error)
    }

    /// Collect GPU completions without admitting time and report whether a frame
    /// slot is free, so the route page drains callbacks before a step only where needed.
    /// # Errors
    /// Returns a device polling fault.
    pub fn route_slot_ready(&mut self) -> Result<bool, JsValue> {
        game::route_slot_ready(&mut self.page.shell).map_err(js_error)
    }

    /// Release completed route samples older than the page's copied tick.
    pub fn discard_route_samples_before(&mut self, tick: u32) {
        self.page
            .shell
            .discard_report_samples_before(u64::from(tick));
    }

    /// Latest committed authority tick.
    #[must_use]
    pub fn committed_tick(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "route runs stay far below 2^53 ticks"
        )]
        let tick = game::committed_tick(&self.page.shell) as f64;
        tick
    }

    /// The next route step's page time in milliseconds from the clock origin:
    /// `(tick + 1 + phi)·P`. Route mode starts the clock with `set_visible(true, 0)`.
    #[must_use]
    pub fn route_step_ms(&self, phi: f64) -> f64 {
        game::route_step_ms(&self.page.shell, phi)
    }

    /// Apply the moving route's scripted render scale before recording a frame.
    ///
    /// # Errors
    /// Reduced scale requires portable temporal reconstruction.
    #[cfg(feature = "proving")]
    pub fn route_render_scale(&mut self, routed_tick: u32) -> Result<(), JsValue> {
        self.page
            .shell
            .set_render_scale(crate::camera::route_render_scale(u64::from(routed_tick)))
            .map_err(|error| js_error(error.to_string()))
    }

    /// Enable the quality governor on a prepared browser proof route.
    #[cfg(feature = "proving")]
    pub fn enable_route_governor(&mut self) -> bool {
        self.page.shell.enable_governor()
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
        game::route_report(&self.page.shell, from).to_string()
    }

    /// Latest submitted Shapes report as JSON for the browser proof.
    #[cfg(feature = "proving")]
    pub fn placed_cloud_report(&self) -> Option<String> {
        crate::clouds::qualification::placed_capture_report(game::cloud_input(&self.page.shell)?)
            .map(|report| report.to_string())
    }

    /// Draw the authored valley from fetched cooked products, with its authored camera.
    /// The products survive device replacement: a new device stages the newest fetched
    /// generation again. Publications fetched later by [`CookedValley::next_publication`] swap
    /// in as they upload. While the device is lost the contribution is held, with no GPU call,
    /// and attaches right after the replacement device is published; a refusal then is
    /// reported in [`Self::cooked_status`] errors. A later attach replaces a held one.
    /// # Errors
    /// Returns an invalid embedded recipe or the scene's refusal of the contribution.
    pub fn attach_cooked_valley(
        &mut self,
        valley: &CookedValley,
        effect_slab: Option<bool>,
        content_preview: Option<bool>,
    ) -> Result<(), JsValue> {
        let source = CookedSource::Remote(valley.feed.clone());
        #[cfg(not(feature = "proving"))]
        if effect_slab.is_some() || content_preview == Some(true) {
            return Err(js_error(
                "qualification contributions require the proving build",
            ));
        }
        #[cfg(feature = "proving")]
        let preview = content_preview.unwrap_or(false);
        #[cfg(feature = "proving")]
        if preview && (effect_slab.is_some() || self.temporal_inputs) {
            return Err(js_error(
                "content preview needs the ordinary valley forward pass",
            ));
        }
        #[cfg(feature = "proving")]
        let (camera, assets) = if preview {
            crate::showcase::prepare_content_preview(source)
        } else if self.temporal_inputs {
            crate::showcase::prepare_temporal_showcase(source)
        } else {
            crate::showcase::prepare_showcase(source)
        }
        .map_err(js_error)?;
        #[cfg(not(feature = "proving"))]
        let (camera, assets) = crate::showcase::prepare_showcase(source).map_err(js_error)?;
        let assets = if let Some(reader) = self.page.shell.host().mesh_rows() {
            let generation = valley
                .feed
                .generation()
                .ok_or_else(|| js_error("missing fetched generation"))?;
            let rows = thrum_asset_gpu::rows::CookedRows::from_generation(
                &generation,
                crate::showcase::entity_mesh_names(self.content),
                self.page.shell.host().mesh_row_binding_limit(),
            )
            .map_err(js_error)?
            .with_reader(reader);
            assets.with_rows(rows).map_err(js_error)?
        } else {
            assets
        };
        let stats = assets.stats();
        #[cfg(feature = "dev-material")]
        let material = assets.material();
        #[cfg(feature = "proving")]
        let mut observation = None;
        #[cfg(feature = "proving")]
        let contribution: Box<dyn thrum_render::contribution::SceneContribution> = if preview {
            let rows = self
                .page
                .shell
                .host()
                .mesh_rows()
                .ok_or_else(|| js_error("content preview needs cooked mesh rows"))?;
            let (contribution, record) = crate::content_preview::attach(
                assets,
                rows,
                self.page.shell.host().reconstruction(),
            );
            observation = Some(record);
            contribution
        } else if let Some(refraction) = effect_slab {
            let (effect, witness) = crate::effect::DiagnosticEffect::new();
            witness.lock().map_err(js_error)?.controls.slab_origin_scale = [0., 240., -6., 2.];
            Box::new(thrum_render::contribution::SceneContributions::new(vec![
                Box::new(assets),
                Box::new(effect.with_slab(refraction)),
            ]))
        } else {
            Box::new(assets)
        };
        #[cfg(not(feature = "proving"))]
        let contribution: Box<dyn thrum_render::contribution::SceneContribution> = Box::new(assets);
        let contribution = crate::clouds::with_shade(
            contribution,
            game::cloud_input(&self.page.shell),
            self.cloud_sky,
            game::cloud_shapes(&self.page.shell),
            game::cloud_ground(&self.page.shell),
            game::cloud_selection(&self.page.shell),
        )
        .map_err(js_error)?;
        #[cfg(feature = "proving")]
        let contribution = if game::test_bank(&self.page.shell) {
            crate::host::scene_source::test_bank::alongside(contribution)
        } else {
            contribution
        };
        self.page
            .shell
            .set_scene_contribution(contribution)
            .map_err(js_error)?;
        game::set_scene_camera(&mut self.page.shell, camera).map_err(js_error)?;
        self.page
            .game
            .0
            .demo
            .observe_camera(game::camera_orientation(&self.page.shell));
        let stats_for_errors = Arc::clone(&stats);
        self.page.on_attachment_error(move |error| {
            if let Ok(mut stats) = stats_for_errors.lock() {
                stats.errors.push(format!(
                    "cooked valley attach on the replacement device: {error}"
                ));
            }
        });
        self.cooked = Some(stats);
        #[cfg(feature = "proving")]
        {
            self.content_preview = observation;
        }
        #[cfg(feature = "dev-material")]
        {
            self.material = Some(material);
            self.material_reported = 0;
        }
        Ok(())
    }

    /// Submitted placement, mesh-row and lease generations for the opt-in valley preview.
    #[must_use]
    #[cfg(feature = "proving")]
    pub fn content_preview_frame(&self) -> Option<String> {
        let mut row = self.content_preview.as_ref()?.frame()?;
        row["submissionId"] = serde_json::json!(self.page.shell.host().last_submission());
        row["presented"] = serde_json::json!(self.page.shell.presented());
        Some(row.to_string())
    }

    /// Cooked generation drawn, swaps and loader errors, or `None` without cooked content.
    pub fn cooked_status(&self) -> Option<String> {
        let stats = self.cooked.as_ref()?.lock().ok()?;
        Some(
            serde_json::json!({
                "drawn": stats.drawn,
                "rowGeneration": self.page.shell.host().mesh_rows().and_then(|rows| rows.report().generation),
                "rowWrittenBytes": self.page.shell.host().mesh_rows().map(|rows| rows.report().submitted_bytes),
                "drawPath": format!("{:?}", stats.draw_path),
                "initialDownlevel": self.page.game.0.initial_downlevel,
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
        game::set_time_of_day(&mut self.page.shell, fraction).map_err(js_error)?;
        self.page
            .game
            .0
            .demo
            .observe_time(game::time_of_day(&self.page.shell));
        Ok(())
    }

    /// Observe the solar time of the latest committed world endpoint.
    pub fn time_of_day(&self) -> f32 {
        game::time_of_day(&self.page.shell)
    }

    /// Look up/down within the scene's supported camera range.
    /// # Errors
    /// Refuses a nonfinite pitch or one outside plus/minus 0.6 radians.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), JsValue> {
        game::set_camera_pitch(&mut self.page.shell, radians).map_err(js_error)?;
        self.page
            .game
            .0
            .demo
            .observe_camera(game::camera_orientation(&self.page.shell));
        Ok(())
    }

    /// Turn the camera within plus/minus pi radians.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside that range.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), JsValue> {
        game::set_camera_yaw(&mut self.page.shell, radians).map_err(js_error)?;
        self.page
            .game
            .0
            .demo
            .observe_camera(game::camera_orientation(&self.page.shell));
        Ok(())
    }

    /// Set the authored camera position for a cloud capture.
    ///
    /// # Errors
    /// Refuses an invalid eye, missing cooked camera or active route.
    #[cfg(feature = "proving")]
    pub fn set_cloud_capture_eye(&mut self, x: f32, y: f32, z: f32) -> Result<(), JsValue> {
        if game::cloud_input(&self.page.shell).is_none() {
            return Err(js_error("cloud capture requires Shade or Sky"));
        }
        game::set_capture_eye(&mut self.page.shell, [x, y, z]).map_err(js_error)
    }

    /// Move the cloud inspection camera, optionally cutting both histories.
    /// # Errors
    /// Refuses an invalid eye or a missing authored camera.
    #[cfg(feature = "proving")]
    pub fn move_cloud_capture_eye(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        cut: bool,
    ) -> Result<(), JsValue> {
        game::set_haze_eye(&mut self.page.shell, [x, y, z], cut).map_err(js_error)
    }

    /// Stage a layer profile change for the cloud inspection.
    /// # Errors
    /// Refuses an unavailable cloud layer or an out-of-range scale.
    #[cfg(feature = "proving")]
    pub fn cloud_capture_scale(&mut self, percent: u32) -> Result<(), JsValue> {
        let shell = &mut self.page.shell;
        let knob = shell
            .host()
            .quality_knobs()
            .into_iter()
            .find(|knob| knob.name.ends_with("cloud layer scale"))
            .ok_or_else(|| js_error("cloud layer scale is unavailable"))?;
        shell
            .set_quality_knob(&knob.name, percent)
            .map_err(js_error)
    }

    /// Whether the latest submitted cloud layer carried its history.
    #[cfg(feature = "proving")]
    pub fn cloud_capture_history(&self) -> Option<bool> {
        Some(
            game::cloud_input(&self.page.shell)?
                .sky_products()?
                .history_valid,
        )
    }

    /// Hold solar lighting at the current tick while weather continues to evolve.
    #[cfg(feature = "proving")]
    pub fn hold_cloud_capture_sun(&mut self) {
        game::hold_capture_sun(&mut self.page.shell);
    }

    /// Load the camera and Sun trajectory for the opt-in haze inspection.
    ///
    /// # Errors
    /// Refuses a malformed take.
    #[cfg(feature = "proving")]
    pub fn configure_haze_take(&mut self, take: &str) -> Result<(), JsValue> {
        self.haze_take = Some(HazeTake::parse(take).map_err(js_error)?);
        Ok(())
    }

    /// Move the camera and Sun for the configured elapsed-time sample.
    ///
    /// # Errors
    /// Refuses a missing take, missing valley camera or invalid elapsed time.
    #[cfg(feature = "proving")]
    pub fn sample_haze_take(&mut self, elapsed_seconds: f32) -> Result<String, JsValue> {
        if !elapsed_seconds.is_finite() || elapsed_seconds < 0.0 {
            return Err(js_error("invalid haze take time"));
        }
        let pose = self
            .haze_take
            .ok_or_else(|| js_error("haze take is not configured"))?
            .at(elapsed_seconds);
        game::set_haze_eye(&mut self.page.shell, pose.eye, pose.cut).map_err(js_error)?;
        game::scrub_haze_sun(&mut self.page.shell, pose.solar_hours / 24.0).map_err(js_error)?;
        game::set_camera_yaw(&mut self.page.shell, pose.yaw_degrees.to_radians())
            .map_err(js_error)?;
        serde_json::to_string(&pose).map_err(js_error)
    }

    /// Prepared 3D shader and graph identity for fixed submitted-frame comparisons.
    #[cfg(feature = "proving")]
    #[must_use]
    pub fn renderer_signature(&self) -> String {
        let host = self.page.shell.host();
        serde_json::json!({
            "shader":host.scene_shader_source(),
            "graph":host.scene_graph().map(|graph| format!("{graph:?}")).ok(),
        })
        .to_string()
    }

    /// Request named GPU pass timestamps for the held haze cost capture.
    #[cfg(feature = "proving")]
    pub fn enable_haze_timings(&mut self) {
        use thrum_app::Game as _;
        self.page
            .shell
            .update_profiler(self.page.game.controls_mut(), true);
    }

    /// Capture-off browser timing samples and the selected volume allocation.
    #[must_use]
    #[cfg(feature = "proving")]
    pub fn haze_cost(&self) -> String {
        let shell = &self.page.shell;
        let samples = shell
            .samples()
            .iter()
            .filter_map(|sample| {
                let timings = sample.gpu_pass_ns.as_ref()?;
                let pass_ns: std::collections::BTreeMap<_, _> = timings
                    .passes()
                    .iter()
                    .map(|pass| (pass.name.as_ref(), pass.nanoseconds))
                    .collect();
                Some(serde_json::json!({
                    "submission":sample.submission_order,
                    "frame_gpu_ns":timings.timed_pass_span_nanoseconds(),
                    "pass_gpu_ns":pass_ns,
                }))
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "frames":samples,
            "view_volume":shell.host().view_volume_storage().map(|volume|
                serde_json::json!({"grid":volume.grid,"resources":volume.resources})),
        })
        .to_string()
    }

    /// Sample normalized player movement for the next authoritative world tick.
    ///
    /// # Errors
    /// Returns a JavaScript error for a nonfinite or out-of-range axis.
    pub fn set_controls(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
        game::set_controls(&mut self.page.shell, [x, y, 0.0]).map_err(js_error)
    }

    /// Committed position of the one local player in this V1 fixture.
    #[must_use]
    pub fn controlled_position(&self) -> String {
        game::controlled_position(&self.page.shell).map_or_else(
            || "player unavailable".to_owned(),
            |position| format!("player x={:.3} y={:.3}", position[0], position[1]),
        )
    }

    /// Submit a capture of the committed endpoint for the page's oracle and
    /// return `tick=<n>;extent=<w>x<h>;generation=<g>`. One capture at a time.
    /// Like [`Self::frame`], it does no GPU work once the device is lost.
    /// # Errors
    /// Refuses a lost device or a second outstanding capture, or returns a
    /// recording failure.
    #[cfg(feature = "proving")]
    pub fn begin_capture(&mut self) -> Result<String, JsValue> {
        if self.capture.is_some() && !self.page.shell.graphics_lost() {
            return Err(JsValue::from_str("a capture is already outstanding"));
        }
        let pending = self.page.shell.begin_capture().map_err(js_error)?;
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
    #[cfg(feature = "proving")]
    pub fn try_finish_capture(&mut self) -> Result<Option<Vec<u8>>, JsValue> {
        let Some(pending) = self.capture.as_mut() else {
            return Ok(None);
        };
        match self.page.shell.try_finish_capture(pending) {
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
        self.page.shell.presented()
    }

    /// True after 600 GPU-complete frame samples have been collected.
    #[must_use]
    #[cfg(feature = "proving")]
    pub fn complete(&self) -> bool {
        self.page.shell.samples().len() == SAMPLES
    }

    /// Committed mover and submitted row observations for the exit scene.
    #[must_use]
    pub fn exit_status(&self) -> Option<String> {
        if self.content != crate::GameContent::Exit {
            return None;
        }
        let samples = self.page.shell.samples();
        let counts = samples
            .iter()
            .rev()
            .find_map(|sample| sample.camera_culling);
        let row_generation = self
            .page
            .shell
            .host()
            .mesh_rows()
            .and_then(|rows| rows.report().generation);
        Some(
            serde_json::json!({
                "tick": game::committed_tick(&self.page.shell),
                "mover": game::exit_mover_position(&self.page.shell),
                "row_generation": row_generation,
                "active": counts.map(|counts| counts.active),
                "in_view": counts.map(|counts| counts.drawn),
                "culling": counts.map(|counts| format!("{:?}", counts.mode)),
            })
            .to_string(),
        )
    }

    /// Per-frame GPU pass sum plus surface blit from the held exit stop.
    #[must_use]
    #[cfg(feature = "proving")]
    pub fn exit_sweep_samples(&self) -> String {
        let rows = self
            .page
            .shell
            .samples()
            .iter()
            .skip(100)
            .filter_map(|sample| {
                let timings = sample.gpu_pass_ns.as_ref()?;
                let present = sample.gpu_present_ns?;
                let counts = sample.camera_culling?;
                if counts.active != 10_000
                    || sample.timing_fault
                    || !sample.surface_presented
                    || timings.get("synthetic Sun shadow near").is_none()
                    || timings.get("synthetic Sun shadow far").is_none()
                {
                    return None;
                }
                Some(serde_json::json!({
                    "tick": sample.tick, "submission": sample.submission_order,
                    "gpu_ns": timings.sum() + present, "in_view": counts.drawn,
                }))
            })
            .collect::<Vec<_>>();
        serde_json::json!({"recipe":"Portable","samples":rows}).to_string()
    }

    /// Final callback, world and GPU report after 600 completed frames.
    #[must_use]
    pub fn report(&self) -> String {
        let samples = self.page.shell.samples();
        let status = self.page.shell.status();
        let (shed_views, report_drops) = game::dropped(&self.page.shell);
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
            game::tick(&self.page.shell),
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
            game::camera_culling(&self.page.shell).resolved,
        ) + &format!(
            "; culling_requested={:?} culling_reason={:?}",
            game::camera_culling(&self.page.shell).requested,
            game::camera_culling(&self.page.shell).reason
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

#[cfg(feature = "proving")]
async fn effect_diagnostic_device() -> Result<
    (
        wgpu::Instance,
        wgpu::Device,
        wgpu::Queue,
        thrum_render::capabilities::DeviceCapabilities,
    ),
    JsValue,
> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let OpenedDevice {
        device,
        queue,
        capabilities: caps,
        ..
    } = DeviceRecipe {
        profile: DeviceProfile::Portable,
        restriction: Some(&DeviceRestriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        }),
        ..Default::default()
    }
    .open(&instance, None)
    .await
    .map_err(js_error)?;
    Ok((instance, device, queue, caps))
}

/// Run the external diagnostic's exact state script on Chrome's WebGPU device.
/// # Errors
/// Returns a target preparation, state mismatch or recovery failure.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn qualify_effect_state() -> Result<String, JsValue> {
    let (instance, device, queue, caps) = effect_diagnostic_device().await?;
    Box::pin(crate::effect::qualification::run(
        &device, &queue, &caps, &instance,
    ))
    .await
    .map_err(js_error)
}

/// Check that the trail answers posts on WebGPU even while all views are dropped.
/// # Errors
/// Returns a target, exact-answer or recovery failure.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn qualify_effect_tick() -> Result<String, JsValue> {
    let (instance, device, queue, caps) = effect_diagnostic_device().await?;
    Box::pin(crate::effect::qualification::run_tick(
        &device, &queue, &caps, &instance,
    ))
    .await
    .map_err(js_error)
}

/// Run the late slab's submitted-input script on Chrome's hardware WebGPU device.
/// # Errors
/// Returns a graph, depth, refraction or motion mismatch from the target.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn qualify_effect_after_opaque() -> Result<String, JsValue> {
    let (instance, device, queue, caps) = effect_diagnostic_device().await?;
    Box::pin(crate::effect::after_opaque::run(
        &device, &queue, &caps, &instance,
    ))
    .await
    .map_err(js_error)
}

/// Qualify cooked leases on the browser's real fetched feed and replacement device.
/// # Errors
/// Returns a fetch, GPU, texel or publication mismatch.
#[cfg(feature = "proving")]
#[wasm_bindgen]
pub async fn qualify_lent_products(origin: String) -> Result<String, JsValue> {
    Box::pin(crate::lent::browser::run(origin))
        .await
        .map_err(js_error)
}
