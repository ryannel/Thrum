use std::time::Duration;

use crowd_2d_status::CrowdStatus;
use thrum_app::{
    Game, KeyCode, interface_key,
    web::{ControlTiming, PageGame, PageRuntime, interface_pointer},
};
use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::RemoteFeed;
use thrum_render::capabilities::FIRST_DEVICE_GENERATION;
use thrum_ui::Event;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen]
pub struct CrowdPage {
    page: PageGame<CrowdStatus>,
}

#[wasm_bindgen]
pub(crate) async fn create_scene(
    canvas: HtmlCanvasElement,
    store: String,
) -> Result<CrowdPage, JsValue> {
    let mut client = DevClient::new(BrowserFetch, &store).map_err(js)?;
    let generation = client
        .poll()
        .await
        .map_err(js)?
        .ok_or_else(|| js("the font store has no generation"))?;
    let feed = RemoteFeed::new(generation);
    let page = PageGame::new(
        canvas,
        FIRST_DEVICE_GENERATION,
        CrowdStatus::new(thrum_asset_gpu::CookedSource::Remote(feed.clone())),
        ControlTiming::Immediate,
    )
    .await
    .map_err(js)?;
    wasm_bindgen_futures::spawn_local(async move {
        while let Ok(generation) = client.poll().await {
            if let Some(generation) = generation {
                feed.publish(generation);
            }
        }
    });
    Ok(CrowdPage { page })
}

impl CrowdPage {
    fn page_runtime(&self) -> &dyn PageRuntime {
        &self.page
    }
    fn page_runtime_mut(&mut self) -> &mut dyn PageRuntime {
        &mut self.page
    }

    fn event(&mut self, event: Event<KeyCode>, now: f64) -> Result<(), JsValue> {
        self.page
            .game
            .interface_input(
                event,
                Duration::try_from_secs_f64(now / 1000.0).map_err(js)?,
            )
            .map(|_| ())
            .map_err(js)
    }
}

thrum_app::export_page_methods!(CrowdPage);

#[wasm_bindgen]
impl CrowdPage {
    #[expect(
        clippy::unused_self,
        reason = "the shared page reads the title from its game instance"
    )]
    pub fn title(&self) -> String {
        "Crowd".to_owned()
    }

    pub fn report(&self) -> String {
        self.page.shell.graphics_fault().unwrap_or_default()
    }

    pub fn witness(&self) -> String {
        self.page.game.witness()
    }

    pub fn set_interface_output(&mut self, density: f64) {
        self.page.shell.set_interface_output(density, [0.0; 4]);
    }

    pub fn interface_key(
        &mut self,
        code: &str,
        down: bool,
        repeat: bool,
        shift: bool,
        now: f64,
    ) -> Result<(), JsValue> {
        if let Some(code) = KeyCode::from_code(code) {
            self.event(interface_key(code, down, repeat, shift), now)?;
        }
        Ok(())
    }

    pub fn interface_pointer(
        &mut self,
        source: &str,
        phase: &str,
        id: u32,
        point: Vec<f64>,
        now: f64,
    ) -> Result<(), JsValue> {
        let point = point.try_into().map_err(|_| js("pointer needs x and y"))?;
        self.event(
            interface_pointer(source, phase, id, point).map_err(js)?,
            now,
        )
    }

    pub fn interface_interrupt(&mut self, now: f64) -> Result<(), JsValue> {
        self.event(Event::Interrupted, now)
    }

    pub fn begin_capture(&mut self) -> Result<(), JsValue> {
        self.page.shell.begin_submitted_capture(1).map_err(js)
    }

    pub fn next_capture(&mut self) -> Result<Option<js_sys::Array>, JsValue> {
        let shell = &mut self.page.shell;
        let Some(image) = shell.next_submitted_image().map_err(js)? else {
            return Ok(None);
        };
        let data = (image.sample.surface_presented && image.sample.interface.is_some_and(|stage| stage.stamp.is_some())).then(|| {
            let info = serde_json::json!({
                "submission": image.sample.submission_order, "tick": image.sample.tick,
                "extent": image.extent, "world_rows": image.sample.sprites.map(|sprites| sprites.world_rows),
                "scale": image.sample.interface.and_then(|stage| stage.stamp.map(|stamp| stamp.scale)),
                "encoding": "rgba8-srgb", "presented": image.sample.surface_presented,
                "source": format!("{:?}", image.pixel_source),
            });
            let out = js_sys::Array::new();
            out.push(&JsValue::from_str(&info.to_string()));
            out.push(&js_sys::Uint8Array::from(image.rgba.as_slice()));
            out
        });
        shell.recycle_submitted_image(image);
        if data.is_some() {
            shell.end_submitted_capture();
        }
        Ok(data)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "adapts owned error results through map_err"
)]
fn js(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
