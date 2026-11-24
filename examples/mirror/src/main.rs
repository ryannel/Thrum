//! Run the mirror pool with `./dev scene examples/mirror`.

#[cfg(target_os = "macos")]
mod cost;
#[cfg(target_os = "macos")]
mod inspection;

#[cfg_attr(
    target_os = "macos",
    expect(
        clippy::print_stderr,
        reason = "report inspection errors to its caller"
    )
)]
fn main() {
    #[cfg(target_os = "macos")]
    if std::env::var_os("THRUM_MIRROR_COST").is_some() {
        if let Err(error) = cost::run() {
            eprintln!("mirror cost: {error}");
            std::process::exit(1);
        }
        return;
    }
    #[cfg(target_os = "macos")]
    if let Some(path) = std::env::var_os("THRUM_MIRROR_INSPECT") {
        let compare_chrome = std::env::var_os("THRUM_MIRROR_COMPARE_CHROME").is_some();
        if let Err(error) = inspection::run(std::path::Path::new(&path), compare_chrome) {
            eprintln!("mirror inspection: {error}");
            std::process::exit(1);
        }
        return;
    }
    thrum_app::sky::run(
        thrum_app::Config::new("mirror-scene", "Mirror pool"),
        || mirror_scene::build().map_err(thrum_app::sky::SkyBuildError::Scene),
    );
}
