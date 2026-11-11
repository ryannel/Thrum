//! Thrum starter: one cooked rock on lit ground under a physically based sky, a camera, and Sun
//! sliders.
//!
//! `./dev starter` opens the Mac window with its panel and saved parameter/material reloads.
//! `./dev starter --web` serves the same scene in Chrome. Its DOM controls are live, and
//! `?params=starter.params.json` applies a parameter file. Material hooks run on the Mac.
//! Both commands re-cook saved files under `assets/`. See README.md.

fn main() {
    thrum_app::sky::run(
        thrum_app::Config::new("thrum-starter", "Thrum starter"),
        scene::build,
    );
}

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
        // False colour by exposure, one band per stop: grey is a metered mid-grey card.
        let zones = controls.toggle("display.exposure_zones", "Exposure zones", false)?;

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
            sky.exposure_zones = controls.is_on(zones);
        });
        Ok(game.with_meshes(vec![rock])?)
    }
}
