//! Stone circle at dawn: seven cooked standing stones and a fallen altar stone on lit ground
//! under the physically based sky. Sliders move the Sun and orbit the camera around the circle.
//!
//! Run it with `./dev scene examples/stones`.

#[cfg(target_os = "macos")]
fn main() {
    thrum_app::sky::run(
        thrum_app::Config::new("thrum-stones", "Stone circle at dawn"),
        scene::build,
    );
}

/// The window runs on the Mac only.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod scene {
    use std::error::Error;
    use thrum_app::sky::{Camera, Controls, CookedMaterial, Display, MeshPlacement, SkyGame};

    /// Radius of the circle in metres, and the number of standing stones in it.
    const RING_RADIUS: f32 = 6.0;
    const STONES: usize = 7;
    const GRANITE: CookedMaterial = CookedMaterial {
        base_colour: [0.42, 0.40, 0.36, 1.0],
        roughness: 0.9,
        metalness: 0.0,
    };

    pub(crate) fn build() -> Result<SkyGame, Box<dyn Error>> {
        let mut controls = Controls::new();
        let mut degrees = |key: &str, label: &str, range, default| {
            let slider = controls.slider(key, label, range).default(default);
            slider.step(0.5).display(Display::plain(1, "°")).add()
        };
        let elevation = degrees("sun.elevation", "Sun elevation", -6.0..=40.0, 3.0)?;
        let azimuth = degrees("sun.azimuth", "Sun azimuth", 40.0..=140.0, 85.0)?;
        let orbit = degrees("camera.orbit", "Camera orbit", 0.0..=360.0, 270.0)?;

        // Standing stones in three heights, the rock's long X axis on end (broad end down), each
        // turned to face the centre, and one fallen stone lying in the middle.
        let mut stones: Vec<MeshPlacement> = (0..STONES)
            .map(|i| {
                let bearing = i as f32 * 360.0 / STONES as f32;
                let (sin, cos) = bearing.to_radians().sin_cos();
                rock()
                    .rotate_degrees([0.0, 0.0, 1.0], -90.0)
                    .yaw_degrees(bearing)
                    .scale(0.85 + 0.12 * (i % 3) as f32)
                    .at([RING_RADIUS * sin, 0.0, -RING_RADIUS * cos])
            })
            .collect();
        stones.push(rock().yaw_degrees(23.0).scale(0.6));

        let game = SkyGame::new(controls, orbit_camera(270.0), move |controls, sky| {
            sky.sun_elevation_degrees = controls.get(elevation);
            sky.sun_azimuth_degrees = controls.get(azimuth);
            sky.camera = orbit_camera(controls.get(orbit));
        });
        Ok(game.with_meshes(stones)?)
    }

    /// The starter's rock in granite, resting on the ground.
    fn rock() -> MeshPlacement {
        MeshPlacement::new("meshes/rock", "textures/white")
            .on_ground()
            .material(GRANITE)
    }

    /// A camera 17 m from the centre at standing height, at a bearing clockwise from north.
    fn orbit_camera(bearing_degrees: f32) -> Camera {
        let (sin, cos) = bearing_degrees.to_radians().sin_cos();
        Camera::look_at([17.0 * sin, 1.7, -17.0 * cos], [0.0, 1.4, 0.0], 40.0)
    }
}
