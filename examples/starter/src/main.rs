//! Thrum starter: one cooked rock on lit ground under a physically based sky, a camera, and Sun
//! sliders.
//!
//! Run it with `./dev starter`, which cooks `assets/`, watches them for edits and opens the
//! window. Edit a slider in the panel, `starter.params.json` or `material.wgsl` (press L to
//! reload them) or any file under `assets/` while it runs. See README.md.

#[cfg(target_os = "macos")]
fn main() {
    thrum_app::sky::run(
        thrum_app::Config::new("thrum-starter", "Thrum starter"),
        scene::build,
    );
}

/// The window runs on the Mac only.
#[cfg(not(target_os = "macos"))]
fn main() {}

/// Everything Mac-only sits in this one module behind one `cfg`.
#[cfg(target_os = "macos")]
mod scene {
    use std::error::Error;
    use thrum_app::sky::{Camera, Controls, CookedMaterial, Display, MeshPlacement, SkyGame};

    pub(crate) fn build() -> Result<SkyGame, Box<dyn Error>> {
        let mut controls = Controls::new();
        let elevation = controls
            .slider("sun.elevation", "Sun elevation", -8.0..=60.0)
            .default(25.0)
            .step(0.5)
            .display(Display::plain(1, "°"))
            .add()?;
        let azimuth = controls
            .slider("sun.azimuth", "Sun azimuth", 0.0..=360.0)
            .default(150.0)
            .step(1.0)
            .display(Display::plain(0, "°"))
            .add()?;

        // One rock at the origin, its lowest point resting on the ground (Y = 0), in metres.
        let rock = MeshPlacement::new("meshes/rock", "textures/white")
            .on_ground()
            .material(CookedMaterial {
                base_colour: [0.34, 0.29, 0.23, 1.0],
                roughness: 0.85,
                metalness: 0.0,
            });

        let camera = Camera::look_at([0.0, 1.6, 5.5], [0.0, 0.7, 0.0], 35.0);
        let game = SkyGame::new(controls, camera, move |controls, sky| {
            sky.sun_elevation_degrees = controls.get(elevation);
            sky.sun_azimuth_degrees = controls.get(azimuth);
        });
        Ok(game.with_meshes(vec![rock])?)
    }
}
