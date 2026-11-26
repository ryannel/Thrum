//! The hand-authored lake outlet, a stand-in for the RIVERS solver's chart.
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_sea_model::{Forcing, Spectrum};
use thrum_water::Water;
use thrum_water_model::{Bed, Body, BodyKind, Chart, End, Map, Stage};

use crate::{
    SharedMap, SharedReport,
    island::{self, Wind},
};

/// The source end across the lake outlet and the mouth across the tidal sill, in world x.
pub const SOURCE: f64 = 210.0;
/// The mouth edge in world x.
pub const MOUTH: f64 = 350.0;
/// The centreline in world z.
pub const CENTRE: f64 = 5.0;
/// The fixed post in the narrows, in world x/z.
pub const POST: [f32; 2] = [257.0, 4.2];
/// The float is released just inside the source join, in world x/z.
pub const RELEASE: [f64; 2] = [213.5, CENTRE];

/// The outlet's winds follow the reach. The sea swell comes in from offshore.
pub fn forcing(wind: Wind) -> Forcing {
    let mut forcing = wind.forcing();
    forcing.wind_direction = std::f64::consts::FRAC_PI_2;
    for swell in &mut forcing.swells {
        swell.direction = -std::f64::consts::FRAC_PI_2;
    }
    forcing
}

fn outlet(bed: &Bed, running: bool) -> Body {
    let (origin, spacing, [nx, nz]) = bed.lattice();
    let centres = (0..nz).flat_map(|z| {
        (0..nx).map(move |x| {
            // The island lattice is much smaller than u32::MAX.
            let x = f64::from(u32::try_from(x).expect("island lattice"));
            let z = f64::from(u32::try_from(z).expect("island lattice"));
            [origin[0] + x * spacing, origin[1] + z * spacing]
        })
    });
    let mut chart = Chart {
        levels: Vec::new(),
        currents: Vec::new(),
        footprint: Vec::new(),
        ends: [(1, SOURCE, false), (0, MOUTH, true)]
            .map(|(body, x, mouth)| End {
                body,
                segment: [[x, 0.0], [x, 10.0]],
                length: 12.0,
                mouth,
            })
            .to_vec(),
    };
    for [x, z] in centres {
        let along = ((x - SOURCE) / (MOUTH - SOURCE)).clamp(0.0, 1.0);
        let narrows = (-((x - 256.0) / 16.0).powi(2)).exp();
        chart.levels.push(11.75 - 12.55 * along);
        chart
            .currents
            .push([f64::from(running) * (0.5 + narrows), 0.0]);
        chart
            .footprint
            .push((SOURCE..=MOUTH).contains(&x) && (0.0..=10.0).contains(&z));
    }
    Body {
        name: "Lake outlet".into(),
        kind: BodyKind::Chart(chart),
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 100.0,
        gains: [0.0, 0.0, 0.30, 0.72],
    }
}

/// Make the example map using the same cut ground triangles submitted by the game.
/// # Errors
/// Returns invalid chart, terrain or forcing.
pub fn map(forcing: &Forcing, running: bool) -> Result<Map, Box<dyn Error>> {
    let bed = island::terrain_bed(include_bytes!("../assets/meshes/channel-island.bin"))?;
    let mut bodies = island::bodies(forcing, 0.5);
    bodies.push(outlet(&bed, running));
    let mut map =
        Map::prepare(bed, bodies, island::map_profile(false)).map_err(|e| format!("{e:?}"))?;
    map.set_forcing(forcing).map_err(|e| format!("{e:?}"))?;
    Ok(map)
}

/// Inspection views from the source, bank, narrows, mouth and above the whole outlet.
pub const VIEWS: [Camera; 6] = [
    Camera::look_at([205.0, 14.2, 10.5], [215.0, 11.6, 4.5], 61.0),
    Camera::look_at([233.0, 11.7, 13.0], [233.0, 9.4, 3.0], 61.0),
    Camera::look_at([252.0, 10.2, 9.0], [265.0, 6.9, 4.8], 60.0),
    Camera::look_at([343.0, 2.35, 12.0], [353.0, -0.1, 5.0], 61.0),
    Camera::look_at([280.0, 45.5, 24.0], [280.0, 5.5, 5.0], 96.0),
    Camera::look_at([317.0, 4.2, 12.0], [305.0, 2.4, 5.0], 61.0),
];

/// The authored channel and its live float, with diagnostics for submitted foam.
pub struct Scene {
    /// The game on the shared sky shell.
    pub game: SkyGame,
    /// Water preparation report.
    pub report: SharedReport,
    /// The same map used by the game and its stage controls.
    pub map: SharedMap,
    /// Last submitted foam endpoints.
    pub foam: thrum_water::foam::Tap,
    /// Mailbox used by the float and the target proof.
    pub query: thrum_water::query::QueryMailbox,
    /// Snapshot of the simulated float.
    pub float: Box<dyn Fn() -> crate::floating::Body>,
}

/// Build the channel on the normal window and WebGPU routes. Paired proofs use the same scene
/// with the water contribution omitted.
/// # Errors
/// Returns invalid authored water, mesh, camera or controls.
/// # Panics
/// The scene's callbacks panic if the shared map lock has been poisoned.
pub fn build(wind: Wind, draw_water: bool, running: bool) -> Result<Scene, Box<dyn Error>> {
    let forcing = forcing(wind);
    let spectrum = Spectrum::build(&forcing)?;
    let map = Arc::new(Mutex::new(map(&forcing, running)?));
    let mut profile = crate::profile(&spectrum);
    profile.optics = Some(thrum_water::Optics::default());
    profile.query = Some(thrum_water::query::QueryLimits {
        points: 24,
        max_age: 8,
    });
    let fallback = crate::floating::fallback(&spectrum, profile.waves.choppiness)?;
    let mut water = Water::mapped(Arc::clone(&map), spectrum, profile)?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded authored wind speeds"
    )]
    let wind_speed = forcing.wind_speed as f32;
    let hulls = water.enable_foam([wind_speed, 0.0])?;
    let report = water.report();
    let foam = water.tap_foam();
    let query = water.query_mailbox().expect("channel query");
    let mut controls = Controls::new();
    let camera = controls
        .slider(
            "camera.channel",
            "Source / Bank / Narrows / Mouth / Top / Lower",
            0.0..=5.0,
        )
        .step(1.0)
        .default(2.0)
        .add()?;
    let lake = controls
        .slider("lake.stage", "Lake stage (m)", 0.0..=1.0)
        .step(0.01)
        .add()?;
    let tide = controls
        .slider("sea.stage", "Tide (m)", -0.5..=0.5)
        .step(0.01)
        .default(0.0)
        .add()?;
    let stage_map = Arc::clone(&map);
    let game = SkyGame::new(controls, VIEWS[2], move |c, sky| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded camera selector"
        )]
        let index = c.get(camera).round() as usize;
        sky.camera = VIEWS[index];
        sky.sun_elevation_degrees = 48.0;
        sky.sun_azimuth_degrees = 165.0;
        sky.exposure_compensation_ev = 0.3;
        sky.temporal_inputs = true;
        let mut map = stage_map.lock().expect("channel map");
        map.set_stage(1, f64::from(c.get(lake)))
            .expect("bounded lake stage");
        map.set_stage(0, f64::from(c.get(tide)))
            .expect("bounded tide");
    })
    .with_ground(None)
    .with_temporal_sampling(cfg!(target_arch = "wasm32"))
    .with_tick_duration(Duration::from_nanos(16_666_667))?;
    let mut meshes = vec![
        MeshPlacement::new("meshes/channel-island", "textures/white"),
        MeshPlacement::new("meshes/pine-bark", "textures/white")
            .at([POST[0], 6.25, POST[1]])
            .scale(0.32)
            .material(thrum_app::sky::CookedMaterial {
                base_colour: [0.18, 0.084, 0.039, 1.0],
                roughness: 0.94,
                metalness: 0.0,
            }),
    ];
    let trees = [
        [208.0, 18.0, 1.0],
        [218.0, 19.0, 0.75],
        [223.0, 16.0, 0.64],
        [244.0, -5.0, 0.9],
        [251.0, -4.0, 0.64],
        [260.0, -5.0, 0.86],
        [272.0, 18.0, 0.95],
        [283.0, -6.0, 0.73],
    ];
    for [x, z, scale] in trees {
        let ground = map
            .lock()
            .expect("channel map")
            .bed()
            .height([f64::from(x), f64::from(z)])
            .expect("channel bank");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "authored tree placements use GPU metres"
        )]
        let at = [x, ground as f32, z];
        for (mesh, colour) in [
            ("meshes/pine-bark", [0.17, 0.088, 0.048, 1.0]),
            ("meshes/pine-needles", [0.049, 0.103, 0.035, 1.0]),
        ] {
            meshes.push(
                MeshPlacement::new(mesh, "textures/white")
                    .at(at)
                    .scale(scale)
                    .material(thrum_app::sky::CookedMaterial {
                        base_colour: colour,
                        roughness: 0.9,
                        metalness: 0.0,
                    }),
            );
        }
    }
    let (game, lease) = game.with_meshes_and_texture_lease(
        meshes,
        Some(("water island", include_str!("../island.wgsl"))),
        &["textures/foam-rank"],
    )?;
    water.lend_foam_pattern(lease);
    let (game, float) = crate::floating::install_channel_float(
        game,
        Arc::clone(&map),
        &water,
        fallback,
        hulls,
        f64::from(wind_speed),
    );
    let game = if draw_water {
        game.with_contribution(Box::new(water), || ContentStatus::Drawn(1))
    } else {
        game
    };
    Ok(Scene {
        game,
        report,
        map,
        foam,
        query,
        float,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn chart_connects_the_authored_island() {
        let map = super::map(&super::Wind::Trade.forcing(), true).expect("channel map");
        assert_eq!(map.sample(super::RELEASE).expect("outlet").body, 3);
    }
}
