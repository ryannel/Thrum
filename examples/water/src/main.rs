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
    #[cfg(target_arch = "wasm32")]
    if let Some((name, underwater, dry)) = lens_probe() {
        thrum_app::sky::run(
            thrum_app::Config::new("water-scene", "Thrum water lens check"),
            move || {
                let fixture = water_scene::lens_probe::Case::new(
                    name,
                    water_scene::lens_probe::BROWSER_DATUM,
                );
                water_scene::lens_probe::browser_game(&fixture, underwater, dry)
                    .map_err(thrum_app::sky::SkyBuildError::Scene)
            },
        );
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
                Settings::Dive(wind, site) => {
                    water_scene::island::build_dive(wind, site, true).map(|(game, _)| game)
                }
                Settings::RainYard(wind) => {
                    water_scene::island::build_rain_ground(wind, true, true)
                        .map(|(game, _, _, _)| game)
                }
                Settings::WetShore(wind) => {
                    water_scene::island::build_wet_shore(wind, true).map(|(game, _, _, _)| game)
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
    Settings::from_query(&page_query())
}

#[cfg(target_arch = "wasm32")]
fn page_query() -> String {
    js_sys::Reflect::get(&js_sys::global(), &"location".into())
        .and_then(|location| js_sys::Reflect::get(&location, &"search".into()))
        .ok()
        .and_then(|search| search.as_string())
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn lens_probe() -> Option<(&'static str, bool, bool)> {
    let query = page_query();
    let params = |key| {
        query
            .trim_start_matches('?')
            .split('&')
            .filter_map(|part| part.split_once('='))
            .find_map(|(name, value)| (name == key).then_some(value))
    };
    let name = params("lens")?;
    let name = water_scene::lens_probe::NAMES
        .into_iter()
        .chain([
            "sea",
            "short-trough",
            "mapped-trough",
            "trade-admission",
            "carried-crest",
            "carried-gain",
            "carried-weight",
            "trough-wet",
            "impact",
        ])
        .find(|&known| name == known)?;
    match params("arm")? {
        "baseline" => Some((name, false, false)),
        "split" => Some((name, true, false)),
        "dry-baseline" if name == "mapped-trough" => Some((name, false, true)),
        "dry"
            if matches!(
                name,
                "sea" | "short-trough" | "mapped-trough" | "trade-admission"
            ) =>
        {
            Some((name, true, true))
        }
        _ => None,
    }
}
