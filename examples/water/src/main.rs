//! Water example on the engine's normal window and controls.
fn main() {
    let island = island_preset();
    let draw_float =
        cfg!(target_arch = "wasm32") || std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some();
    thrum_app::sky::run(
        thrum_app::Config::new("water-scene", "Thrum water"),
        move || match island {
            Some(preset) => water_scene::island::build(preset, true).map(|(game, _, _)| game),
            None => water_scene::build_with_report(draw_float).map(|(game, _)| game),
        },
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn island_preset() -> Option<water_scene::island::Wind> {
    std::env::var("THRUM_WATER_ISLAND")
        .ok()
        .as_deref()
        .and_then(water_scene::island::Wind::named)
}

#[cfg(target_arch = "wasm32")]
fn island_preset() -> Option<water_scene::island::Wind> {
    let location = js_sys::Reflect::get(&js_sys::global(), &"location".into()).ok()?;
    let query = js_sys::Reflect::get(&location, &"search".into())
        .ok()?
        .as_string()?;
    query
        .trim_start_matches('?')
        .split('&')
        .find_map(|parameter| {
            parameter
                .strip_prefix("island=")
                .and_then(water_scene::island::Wind::named)
        })
}
