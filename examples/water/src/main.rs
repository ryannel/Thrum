//! Water example on the engine's normal window and controls.
use water_scene::settings::Settings;
fn main() {
    let settings = Settings::read();
    let draw_float =
        cfg!(target_arch = "wasm32") || std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some();
    thrum_app::sky::run(
        thrum_app::Config::new("water-scene", "Thrum water"),
        move || match settings {
            Settings::Island { wind, foam } => {
                water_scene::island::build_foam(wind, true, foam).map(|(game, _, _, _)| game)
            }
            Settings::Floating(floating) => match floating.wake {
                Some(wake) => water_scene::floating::build_wake(
                    floating.wind,
                    floating.lake,
                    floating.speed,
                    floating.heading,
                    wake.demo(draw_float),
                )
                .map(|scene| scene.game),
                None => water_scene::floating::build(
                    floating.wind,
                    floating.lake,
                    floating.speed,
                    floating.heading,
                    floating.chase,
                )
                .map(|scene| scene.game),
            },
            Settings::OpenSea => water_scene::build_with_report(draw_float).map(|(game, _)| game),
        },
    );
}
