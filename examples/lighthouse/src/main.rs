//! Thrum lighthouse: a striped lighthouse with a lit lantern on a sandy shore, a few rocks at its
//! foot, under a physically based sky, with Sun and camera-orbit sliders.
//!
//! Run it with `./dev scene examples/lighthouse`. Edit a slider in the panel,
//! `lighthouse.params.json` or `material.wgsl` (press L to reload them) or any file under
//! `assets/` while it runs. See README.md.

#[cfg(target_os = "macos")]
fn main() {
    thrum_app::sky::run(
        thrum_app::Config::new("thrum-lighthouse", "Thrum lighthouse"),
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
            .default(6.0)
            .step(0.5)
            .display(Display::plain(1, "°"))
            .add()?;
        let azimuth = controls
            .slider("sun.azimuth", "Sun azimuth", 0.0..=360.0)
            .default(250.0)
            .step(1.0)
            .display(Display::plain(0, "°"))
            .add()?;
        let orbit = controls
            .slider("camera.orbit", "Camera orbit", 0.0..=360.0)
            .default(200.0)
            .step(1.0)
            .display(Display::plain(0, "°"))
            .add()?;
        let distance = controls
            .slider("camera.distance", "Camera distance", 12.0..=60.0)
            .default(30.0)
            .step(0.5)
            .display(Display::plain(1, " m"))
            .add()?;

        // The tower stands at the origin; rocks lie around its foot, in metres.
        let tower = MeshPlacement::new("meshes/lighthouse", "textures/white").on_ground();
        let stone = CookedMaterial {
            base_colour: [0.20, 0.19, 0.18, 1.0],
            roughness: 0.9,
            metalness: 0.0,
        };
        let rocks = [
            ([3.2, -1.0], 1.2, 20.0),
            ([-2.6, 2.4], 0.8, 140.0),
            ([1.0, 4.5], 1.5, 75.0),
        ];
        let mut meshes = vec![tower];
        meshes.extend(rocks.iter().map(|&([x, z], s, yaw)| {
            MeshPlacement::new("meshes/rock", "textures/white")
                .yaw_degrees(yaw)
                .scale(s)
                .at([x, 0.0, z])
                .on_ground()
                .material(stone)
        }));

        let camera = orbit_camera(200.0, 30.0);
        let game = SkyGame::new(controls, camera, move |controls, sky| {
            sky.sun_elevation_degrees = controls.get(elevation);
            sky.sun_azimuth_degrees = controls.get(azimuth);
            sky.camera = orbit_camera(controls.get(orbit), controls.get(distance));
        });
        Ok(game
            .with_meshes(meshes)?
            .with_ground(Some([0.62, 0.52, 0.38])))
    }

    /// A camera at eye height on a circle around the tower, looking up at its lantern.
    fn orbit_camera(degrees: f32, distance: f32) -> Camera {
        let (s, c) = degrees.to_radians().sin_cos();
        Camera::look_at([distance * s, 1.7, -distance * c], [0.0, 7.0, 0.0], 50.0)
    }
}
