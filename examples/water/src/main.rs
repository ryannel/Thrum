//! Water example on the engine's normal window and controls.
fn main() {
    let draw_float = std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some();
    thrum_app::sky::run(
        thrum_app::Config::new("water-scene", "Thrum water"),
        move || water_scene::build_with_report(draw_float).map(|(game, _)| game),
    );
}
