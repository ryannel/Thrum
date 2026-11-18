//! Authored mirror pool and its scene-local renderer contribution.

mod mover;
mod pool;
pub use pool::{Pool, PoolPlane};

use std::{error::Error, time::Duration};
use thrum_app::sky::{Camera, ContentStatus, Controls, CookedMaterial, MeshPlacement, SkyGame};
use thrum_render::synthetic::SyntheticPointLight;

const LEVEL: f32 = 250.0;
const BANK_EDGE: f32 = -15.99;
const LAMP_Z: f32 = -17.4;
const LAMP_BASE: f32 = LEVEL + 0.24 * (BANK_EDGE - LAMP_Z);
const FOV: f32 = 58.0;

/// Build the scene used by the window and inspection capture.
///
/// # Errors
/// Returns invalid control declarations, scene data or cooked placements.
pub fn build() -> Result<SkyGame, Box<dyn Error>> {
    let mut controls = Controls::new();
    let height = controls
        .slider("camera.height", "Eye height", 1.5..=10.0)
        .default(1.5)
        .add()?;
    let tilt = controls
        .slider("camera.tilt", "Look down", -10.0..=44.0)
        .default(9.0)
        .add()?;
    let pan = controls
        .slider("camera.pan", "Pan", -30.0..=30.0)
        .default(0.0)
        .add()?;
    let orbit = controls
        .slider("camera.orbit", "Orbit", -35.0..=35.0)
        .default(0.0)
        .add()?;
    let elevation = controls
        .slider("sun.elevation", "Sun elevation", -18.0..=80.0)
        .default(51.0)
        .add()?;
    let azimuth = controls
        .slider("sun.azimuth", "Sun azimuth", 0.0..=360.0)
        .default(130.0)
        .add()?;
    let camera = camera(1.5, 9.0, 0.0, 0.0);
    let game = SkyGame::new(controls, camera, move |c, sky| {
        sky.camera = self::camera(c.get(height), c.get(tilt), c.get(pan), c.get(orbit));
        sky.sun_elevation_degrees = c.get(elevation);
        sky.sun_azimuth_degrees = c.get(azimuth);
    })
    .with_ground(None)
    .with_local_lights(vec![SyntheticPointLight {
        position: [-3.3, LAMP_BASE + 2.05, LAMP_Z - 0.1],
        radius: 9.0,
        candela: [420.0, 220.0, 76.0],
    }])
    .with_tick_duration(Duration::from_secs_f64(1.0 / 60.0))?;
    let pool = Pool::new(PoolPlane {
        level: LEVEL,
        min: [-19.0, -16.0],
        max: [19.0, 20.8],
    });
    Ok(game
        .with_meshes(placements())?
        .with_contribution(Box::new(mover::Mover::default()), || {
            ContentStatus::Drawn(1)
        })
        .with_contribution(Box::new(pool), || ContentStatus::Drawn(1)))
}

fn camera(height: f32, tilt: f32, pan: f32, orbit: f32) -> Camera {
    let (s, c) = orbit.to_radians().sin_cos();
    let yaw = (pan - orbit).to_radians();
    let eye = [22.0 * s, LEVEL + height, 22.0 * c];
    let pitch = tilt.to_radians();
    Camera::look_at(
        eye,
        [
            eye[0] + 100.0 * yaw.sin() * pitch.cos(),
            eye[1] - 100.0 * pitch.sin(),
            eye[2] - 100.0 * yaw.cos() * pitch.cos(),
        ],
        FOV,
    )
}

fn placements() -> Vec<MeshPlacement> {
    let stone = CookedMaterial {
        base_colour: [0.19, 0.17, 0.14, 1.0],
        roughness: 0.87,
        metalness: 0.0,
    };
    let mut meshes = vec![
        MeshPlacement::new("meshes/lighthouse", "textures/white")
            .scale(0.51)
            .at([0.0, LEVEL, -16.1])
            .on_ground(),
        MeshPlacement::new("meshes/post", "textures/white")
            .at([2.4, LEVEL - 0.31, 7.0])
            .material(stone),
        MeshPlacement::new("meshes/post", "textures/white")
            .scale(1.15)
            .at([-3.3, LAMP_BASE, LAMP_Z])
            .material(stone),
        MeshPlacement::new("meshes/hill", "textures/white")
            .at([460.0, LEVEL, -1080.0])
            .material(CookedMaterial {
                base_colour: [0.14, 0.17, 0.11, 1.0],
                ..stone
            }),
        MeshPlacement::new("meshes/bank", "textures/white")
            .at([0.0, LEVEL, BANK_EDGE])
            .material(CookedMaterial {
                base_colour: [0.23, 0.20, 0.15, 1.0],
                ..stone
            }),
        MeshPlacement::new("meshes/ground", "textures/white")
            .at([0.0, LEVEL - 0.06, 0.0])
            .material(CookedMaterial {
                base_colour: [0.25, 0.21, 0.15, 1.0],
                ..stone
            }),
    ];
    for (position, scale, yaw) in [
        ([-6.0, 0.0, -15.7], 1.6, 20.0),
        ([-9.0, 0.0, -16.2], 2.3, 140.0),
        ([5.5, 0.0, -15.4], 2.1, 75.0),
        ([10.3, 0.0, -16.3], 2.9, 180.0),
        ([18.8, 0.0, -5.0], 1.8, 40.0),
        ([-18.7, 0.0, -3.0], 1.5, 235.0),
    ] {
        meshes.push(
            MeshPlacement::new("meshes/rock", "textures/white")
                .scale(scale)
                .at([position[0], LEVEL - 0.06, position[2]])
                .yaw_degrees(yaw)
                .on_ground()
                .material(stone),
        );
    }
    meshes
}
