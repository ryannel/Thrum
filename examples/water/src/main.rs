//! Water example on the engine's normal window and controls.
use water_scene::settings::Settings;
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--prepare-rivers")
    {
        let path = std::env::args_os().nth(2).expect("river output directory");
        prepare_rivers(std::path::Path::new(&path));
        return;
    }
    let settings = settings();
    let draw_float =
        cfg!(target_arch = "wasm32") || std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some();
    thrum_app::sky::run(
        thrum_app::Config::new("water-scene", "Thrum water"),
        move || {
            let built = match settings {
                Settings::Island { wind, foam } => {
                    water_scene::island::build_foam(wind, true, foam).map(|(game, _, _, _)| game)
                }
                Settings::LakeDive(wind) => {
                    water_scene::island::build_lake_dive(wind, true).map(|(game, _)| game)
                }
                Settings::SeaDive(wind) => {
                    water_scene::island::build_sea_dive(wind, true).map(|(game, _)| game)
                }
                Settings::RainYard(wind) => {
                    water_scene::island::build_rain_yard(wind, true).map(|(game, _, _, _)| game)
                }
                Settings::River(wind) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    let directory = water_scene::river::store_directory(
                        std::env::var_os("THRUM_WATER_RIVER_STORE").map(std::path::PathBuf::from),
                        std::env::var_os(thrum_app::sky::STORE_VARIABLE)
                            .as_deref()
                            .map(std::path::Path::new),
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    let directory = Some(directory.as_path());
                    #[cfg(target_arch = "wasm32")]
                    let directory = None;
                    water_scene::river::build(wind, true, directory).map(|scene| scene.game)
                }
                Settings::Channel(wind) => {
                    water_scene::channel::build(wind, true, true).map(|scene| scene.game)
                }
                Settings::Surf(wind) => {
                    water_scene::island::build_surf(wind, true).map(|(game, _, _, _)| game)
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
                Settings::OpticsSea(wind) => {
                    water_scene::build_optics_sea(wind, draw_float, true).map(|(game, _)| game)
                }
                Settings::OpticsShade(wind) => {
                    water_scene::build_optics_shade(wind, draw_float, true).map(|(game, _)| game)
                }
                Settings::OpenSea => {
                    water_scene::build_with_report(draw_float).map(|(game, _)| game)
                }
            };
            built.map_err(thrum_app::sky::SkyBuildError::Scene)
        },
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[expect(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "native packaging command's two key lines and errors"
)]
fn prepare_rivers(directory: &std::path::Path) {
    match water_scene::river::prepare_files(directory) {
        Ok(keys) => {
            for key in keys {
                println!("{key}");
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn settings() -> Settings {
    Settings::from_variables(|name| std::env::var(name).ok())
}

#[cfg(target_arch = "wasm32")]
fn settings() -> Settings {
    let query = js_sys::Reflect::get(&js_sys::global(), &"location".into())
        .and_then(|location| js_sys::Reflect::get(&location, &"search".into()))
        .ok()
        .and_then(|search| search.as_string());
    Settings::from_query(&query.unwrap_or_default())
}
