//! Yard placements and inspection views for the rain selection.
use std::{error::Error, time::Duration};
use thrum_app::sky::{Camera, CookedMaterial, MeshPlacement};
use thrum_rain_model::{
    Canopy, Ground, GroundClass, Profile, Rain, RainState, Shelter, TickReport, Tree,
};
use thrum_water_model::{Bed, Body, BodyKind, Map, Stage};

pub(super) fn camera(index: usize) -> Camera {
    match index {
        0 => Camera::look_at([144.8, 19.82, 63.35], [149.0, 18.04, 66.4], 60.0),
        1 => Camera::look_at([152.8, 19.82, 63.55], [148.0, 18.04, 66.5], 60.0),
        2 => Camera::look_at([150.2, 19.82, 68.3], [149.6, 18.04, 63.7], 60.0),
        _ => Camera::look_at([149.0, 33.12, 64.94], [149.0, 18.12, 65.0], 45.0),
    }
}

pub(super) fn placements() -> [MeshPlacement; 5] {
    [
        (
            "meshes/rain-yard-paving",
            "textures/rain-yard-paving",
            [0.42, 0.37, 0.29],
            0.88,
        ),
        (
            "meshes/rain-yard-track",
            "textures/rain-yard-track",
            [0.23, 0.135, 0.074],
            0.97,
        ),
        (
            "meshes/rain-yard-roof",
            "textures/white",
            [0.15, 0.12, 0.094],
            0.87,
        ),
        (
            "meshes/rain-yard-posts",
            "textures/white",
            [0.21, 0.107, 0.048],
            0.92,
        ),
        (
            "meshes/rain-yard-wall",
            "textures/white",
            [0.39, 0.29, 0.21],
            0.95,
        ),
    ]
    .map(|(name, texture, [r, g, b], roughness)| {
        MeshPlacement::new(name, texture).material(CookedMaterial {
            base_colour: [r, g, b, 1.0],
            roughness,
            metalness: 0.0,
        })
    })
}

const ORIGIN: [f64; 2] = [144.0, 61.0];
const CELL: f64 = 0.05;
const SIZE: [usize; 2] = [200, 160];
const BODY: usize = 3;
const FLOAT_CELL: [usize; 2] = [71, 104];

#[derive(Clone, Copy)]
pub(super) struct Weather {
    rain: Rain,
    scale: f64,
}

impl Weather {
    pub(super) fn new(rain: f32, drying: f32, scale: f32) -> Self {
        Self {
            rain: Rain {
                flux: f64::from(rain) / 3600.0,
                drying: f64::from(drying) / 3600.0,
            },
            scale: f64::from(scale),
        }
    }
}

impl Default for Weather {
    fn default() -> Self {
        Self::new(60.0, 0.0, 1.0)
    }
}

pub(super) struct Yard {
    tree: Tree,
    rain: RainState,
    report: TickReport,
}

impl Yard {
    pub(super) fn float(&self) -> crate::floating::YardFloat {
        let index = FLOAT_CELL[1] * SIZE[0] + FLOAT_CELL[0];
        let ground = self.tree.ground();
        crate::floating::YardFloat::new(ground.centre(index), ground.heights()[index])
    }

    pub(super) fn prepare() -> Result<Self, Box<dyn Error>> {
        // The floor of each cell is sampled from the very triangles the
        // opaque pass will draw. The roof is a separate first-hit receiver.
        let mut triangles = super::mesh_triangles(
            include_bytes!("../../assets/meshes/rain-yard-paving.bin"),
            201 * 101 * 12,
            200 * 100 * 24,
        );
        triangles.extend(super::mesh_triangles(
            include_bytes!("../../assets/meshes/rain-yard-track.bin"),
            201 * 61 * 12,
            200 * 60 * 24,
        ));
        let centres = ORIGIN.map(|coord| coord + CELL / 2.0);
        let bed = Bed::from_triangles(centres, CELL, SIZE, f64::MAX, &triangles)
            .map_err(|error| format!("{error:?}"))?;
        if bed.heights().contains(&f64::MAX) {
            return Err("yard cell has no ground triangle".into());
        }
        let ground = Ground::new(ORIGIN, CELL, SIZE, bed.heights().to_vec())?;
        let profile = Profile::default();
        let canopy = Canopy::prepare(
            ORIGIN,
            [100, 80],
            &[Shelter {
                name: "yard roof".into(),
                triangles: super::mesh_triangles(
                    include_bytes!("../../assets/meshes/rain-yard-roof.bin"),
                    24 * 12,
                    36 * 4,
                ),
                share: 0.0,
            }],
            &profile,
        )?;
        let tree = Tree::prepare(&ground, &canopy, &profile)?;
        let rain = RainState::new(&tree);
        Ok(Self {
            tree,
            rain,
            report: TickReport::default(),
        })
    }

    pub(super) fn body(&self) -> Result<Body, Box<dyn Error>> {
        let hollows = self
            .tree
            .water_hollows(&self.rain.levels(&self.tree))
            .map_err(|error| format!("{error:?}"))?;
        Ok(Body {
            name: "Rain yard".into(),
            kind: BodyKind::Hollows(hollows),
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1.0,
            gains: [0.0; 4],
        })
    }

    pub(super) fn step(
        &mut self,
        weather: Weather,
        dt: Duration,
        map: &mut Map,
    ) -> Result<(), Box<dyn Error>> {
        self.rain.step(
            &self.tree,
            weather.rain,
            GroundClass::bare(0.0),
            dt.as_secs_f64() * weather.scale,
            1,
            &mut self.report,
        )?;
        map.set_levels(BODY, &self.rain.levels(&self.tree))
            .map_err(|error| format!("{error:?}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yard_fills_and_parts_with_stable_map_identity() {
        let mut yard = Yard::prepare().expect("prepare authored yard triangles");
        let float = yard.float();
        let initial_float_line = float.waterline();
        let float_point = yard
            .tree
            .ground()
            .centre(FLOAT_CELL[1] * SIZE[0] + FLOAT_CELL[0]);
        let mut map = super::super::prepare_island_map(
            &super::super::Wind::Calm.forcing(),
            None,
            Some(&yard),
        )
        .expect("yard remains outside the lake's full shore margin");
        let pits = [[147.2, 66.2], [149.75, 66.35]];
        let shelter = [151.75, 67.8];
        let wet = |map: &Map, point| {
            map.sample(point)
                .filter(|hit| hit.body == BODY && hit.depth > 0.0)
        };
        assert!(wet(&map, pits[0]).is_none());
        let generation = map.generation(BODY);
        let step = Duration::from_micros(16_667);
        let mut merged = false;
        let mut separated = false;
        for tick in 1..=1200 {
            yard.step(Weather::new(100.0, 0.0, 600.0), step, &mut map)
                .unwrap();
            float.advance(tick, &map);
            if let (Some(left), Some(right)) = (wet(&map, pits[0]), wet(&map, pits[1])) {
                if left.row.level.to_bits() == right.row.level.to_bits() {
                    merged = true;
                    eprintln!("yard merged after {tick} ticks at {:.6} m", left.row.level);
                    break;
                }
                separated = true;
            }
        }
        assert!(separated && merged, "both pits fill separately, then join");
        assert!(float.waterline() > initial_float_line);
        assert!((float.waterline() - wet(&map, float_point).unwrap().row.level).abs() < 0.000_001);
        assert!(wet(&map, shelter).is_none());
        let mut parted = false;
        for tick in 1..=1200 {
            yard.step(Weather::new(0.0, 100.0, 600.0), step, &mut map)
                .unwrap();
            if let (Some(left), Some(right)) = (wet(&map, pits[0]), wet(&map, pits[1]))
                && left.row.level.to_bits() != right.row.level.to_bits()
            {
                parted = true;
                eprintln!(
                    "yard parted after {tick} drying ticks at {:.6} and {:.6} m",
                    left.row.level, right.row.level
                );
                break;
            }
        }
        assert!(parted, "both pits still hold water when they part");
        assert_eq!(map.generation(BODY), generation);
        assert_eq!(map.held_level_tables, 0);
    }
}
