//! Run the mirror pool with `./dev scene examples/mirror`.

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
    if let Some(path) = std::env::var_os("THRUM_MIRROR_INSPECT") {
        if let Err(error) = inspection::run(std::path::Path::new(&path)) {
            eprintln!("mirror inspection: {error}");
            std::process::exit(1);
        }
        return;
    }
    thrum_app::sky::run(
        thrum_app::Config::new("mirror-scene", "Mirror pool"),
        mirror_scene::build,
    );
}
