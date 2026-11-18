//! Water example on the engine's normal window and controls.
fn main() {
    let island = island_preset();
    let floating = option("bodies", "THRUM_WATER_BODIES");
    let draw_float =
        cfg!(target_arch = "wasm32") || std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some();
    thrum_app::sky::run(
        thrum_app::Config::new("water-scene", "Thrum water"),
        move || {
            if let Some(wind) = floating
                .as_deref()
                .and_then(water_scene::island::Wind::named)
            {
                let speed = option("speed", "THRUM_WATER_SPEED")
                    .and_then(|value| value.parse::<f32>().ok())
                    .unwrap_or(0.0)
                    .clamp(0.0, 15.0);
                let beam = option("heading", "THRUM_WATER_HEADING").as_deref() == Some("beam");
                let heading = if beam {
                    std::f32::consts::FRAC_PI_2
                } else {
                    std::f32::consts::PI
                };
                let chase = option("view", "THRUM_WATER_VIEW").as_deref() == Some("chase");
                let lake = option("location", "THRUM_WATER_LOCATION").as_deref() == Some("lake");
                return water_scene::floating::build(wind, lake, speed, heading, chase)
                    .map(|scene| scene.game);
            }
            match island {
                Some(preset) => water_scene::island::build(preset, true).map(|(game, _, _)| game),
                None => water_scene::build_with_report(draw_float).map(|(game, _)| game),
            }
        },
    );
}

fn island_preset() -> Option<water_scene::island::Wind> {
    option("island", "THRUM_WATER_ISLAND")
        .as_deref()
        .and_then(water_scene::island::Wind::named)
}

#[cfg(not(target_arch = "wasm32"))]
fn option(_: &str, environment: &str) -> Option<String> {
    std::env::var(environment).ok()
}

#[cfg(target_arch = "wasm32")]
fn option(key: &str, _: &str) -> Option<String> {
    let location = js_sys::Reflect::get(&js_sys::global(), &"location".into()).ok()?;
    let query = js_sys::Reflect::get(&location, &"search".into())
        .ok()?
        .as_string()?;
    query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|parameter| parameter.split_once('='))
        .find_map(|(name, value)| (name == key).then(|| value.to_owned()))
}
