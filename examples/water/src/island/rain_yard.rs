//! Yard placements and inspection views for the rain selection.
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, Controls, CookedMaterial, Display, MeshPlacement, SkyGame};
use thrum_rain_model::{
    Canopy, Ground, GroundClass, GroundStores, Profile, Rain, RainState, Shelter, TickReport, Tree,
};
use thrum_water_model::{Bed, Body, BodyKind, HollowOptics, Hollows, Map, NO_HOLLOW, Stage};

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
const TRACK_ROWS: usize = 60;
const PAVING: usize = 1;
const FLOAT_CELL: [usize; 2] = [71, 104 - TRACK_ROWS];

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

pub(super) fn input_controls(
    controls: &mut Controls,
) -> Result<impl Fn(&Controls) -> Weather + use<>, Box<dyn Error>> {
    let rate = controls
        .slider("rain.rate", "Rain (mm/h)", 0.0..=100.0)
        .default(60.0)
        .step(1.0)
        .add()?;
    let drying = controls
        .slider("rain.drying", "Drying (mm/h)", 0.0..=100.0)
        .default(0.0)
        .step(1.0)
        .add()?;
    let scale = controls
        .slider("rain.time_scale", "Rain time scale", 1.0..=600.0)
        .default(1.0)
        .step(1.0)
        .add()?;
    Ok(move |controls: &Controls| {
        Weather::new(
            controls.get(rate),
            controls.get(drying),
            controls.get(scale),
        )
    })
}

pub(super) fn float_readout(
    controls: &mut Controls,
) -> Result<impl FnMut(&mut Controls, f64) + use<>, Box<dyn Error>> {
    let line = controls.readout(
        "rain.float_level",
        "Float waterline",
        Display::plain(3, " m"),
    )?;
    Ok(move |controls: &mut Controls, level: f64| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "yard display uses f32 metres"
        )]
        let level = level as f32;
        controls.show(line, level);
    })
}

/// Each side of the raised divider has its own ground stores and outlets.
struct GroundPatch {
    tree: Tree,
    rain: RainState,
    report: TickReport,
}

pub(super) struct Yard {
    patches: [GroundPatch; 2],
    stores: GroundStores,
    levels: Vec<f64>,
}

impl Yard {
    pub(super) fn size(&self) -> (usize, usize) {
        (SIZE.iter().product(), self.levels.len())
    }

    pub(super) fn install(
        mut self,
        game: SkyGame,
        water: &thrum_water::Water,
        map: crate::SharedMap,
        input: Arc<Mutex<Weather>>,
        mut readout: impl FnMut(&mut Controls, f64) + 'static,
    ) -> SkyGame {
        let float = self.float();
        let displayed_float = float.clone();
        #[cfg(target_arch = "wasm32")]
        let status_map = Arc::clone(&map);
        #[cfg(target_arch = "wasm32")]
        let float_point = self.patches[PAVING]
            .tree
            .ground()
            .centre(FLOAT_CELL[1] * SIZE[0] + FLOAT_CELL[0]);
        float
            .install(game, water)
            .with_status_readouts(move |controls| {
                let level = displayed_float.waterline();
                readout(controls, level);
                #[cfg(target_arch = "wasm32")]
                browser_status(
                    &status_map.lock().expect("yard water map"),
                    float_point,
                    level,
                );
            })
            .with_fixed_update(move |tick, dt| {
                if tick > 0 {
                    let weather = *input.lock().expect("yard weather");
                    let mut map = map.lock().expect("yard water map");
                    self.step(weather, dt, &mut map)
                        .expect("prepared yard rain tick");
                    float.advance(tick, &map);
                }
            })
    }

    pub(super) fn float(&self) -> crate::floating::YardFloat {
        let index = FLOAT_CELL[1] * SIZE[0] + FLOAT_CELL[0];
        let ground = self.patches[PAVING].tree.ground();
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
        let prepare_patch =
            |first: usize, end: usize| -> Result<GroundPatch, thrum_rain_model::RainError> {
                let width = SIZE[0];
                let origin = [ORIGIN[0], ORIGIN[1] + first as f64 * CELL];
                let ground = Ground::new(
                    origin,
                    CELL,
                    [width, end - first],
                    bed.heights()[first * width..end * width].to_vec(),
                )?;
                let tree = Tree::prepare(&ground, &canopy, &profile)?;
                let rain = RainState::new(&tree);
                Ok(GroundPatch {
                    tree,
                    rain,
                    report: TickReport::default(),
                })
            };
        let patches = [
            prepare_patch(0, TRACK_ROWS)?,
            prepare_patch(TRACK_ROWS, SIZE[1])?,
        ];
        let track = GroundClass {
            soak: 9.0 / 3600.0,
            percolation: 3.0 / 3600.0,
            pores: 0.008,
            relief: 0.0005,
            porosity: 0.55,
            ..GroundClass::bare(0.0)
        };
        let stores = GroundStores::new(&[track, GroundClass::bare(0.0)])?;
        let count = patches.iter().map(|patch| patch.tree.hollow_count()).sum();
        let mut yard = Self {
            patches,
            stores,
            levels: Vec::with_capacity(count),
        };
        yard.refresh_levels();
        Ok(yard)
    }

    fn refresh_levels(&mut self) {
        self.levels.clear();
        for patch in &self.patches {
            self.levels.extend(patch.rain.levels(&patch.tree));
        }
    }

    pub(super) fn body(&self) -> Result<Body, Box<dyn Error>> {
        let mut bed = Vec::with_capacity(SIZE.iter().product());
        let mut labels = Vec::with_capacity(bed.capacity());
        let mut offset = 0;
        for patch in &self.patches {
            bed.extend_from_slice(patch.tree.ground().heights());
            labels.extend(patch.tree.labels().iter().map(|&label| {
                if label == NO_HOLLOW {
                    NO_HOLLOW
                } else {
                    label + offset
                }
            }));
            offset += u32::try_from(patch.tree.hollow_count())?;
        }
        let track_hollows = self.patches[0].tree.hollow_count();
        let mut optics = vec![
            Some(HollowOptics {
                extinction: 3.0,
                scatter: [0.15, 0.17, 0.2],
            });
            self.levels.len()
        ];
        optics[..track_hollows].fill(Some(HollowOptics {
            extinction: 90.0,
            scatter: [0.17, 0.092, 0.045],
        }));
        let hollows = Hollows::new(ORIGIN, CELL, SIZE, bed, labels, self.levels.clone())
            .and_then(|hollows| hollows.with_optical_types(optics))
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
        let step = dt.as_secs_f64() * weather.scale;
        for (class, patch) in self.patches.iter_mut().enumerate() {
            patch.rain.step_with(
                &patch.tree,
                weather.rain,
                &self.stores,
                class,
                step,
                1,
                &mut patch.report,
            )?;
        }
        self.stores.step(weather.rain, step)?;
        self.refresh_levels();
        map.set_levels(BODY, &self.levels)
            .map_err(|error| format!("{error:?}"))?;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_status(map: &Map, float_point: [f64; 2], float_waterline: f64) {
    let status = js_sys::Array::new();
    for point in [[147.2, 66.2], [149.75, 66.35], float_point, [151.75, 67.8]] {
        let level = map
            .sample(point)
            .filter(|hit| hit.body == BODY && hit.depth > 0.0)
            .map_or(js_sys::wasm_bindgen::JsValue::NULL, |hit| {
                hit.row.level.into()
            });
        status.push(&level);
    }
    status.push(&float_waterline.into());
    drop(js_sys::Reflect::set(
        &js_sys::global(),
        &"waterRainYard".into(),
        &status,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yard_fills_and_parts_with_stable_map_identity() {
        let mut yard = Yard::prepare().expect("prepare authored yard triangles");
        let float = yard.float();
        let initial_float_line = float.waterline();
        let float_point = yard.patches[PAVING]
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
        let mut first_spill = None;
        let tree = &yard.patches[PAVING].tree;
        assert_eq!(tree.hollow_count(), 2, "the paving has two hollows");
        let sill = tree.node(0).sill;
        assert_eq!(sill.to_bits(), tree.node(1).sill.to_bits());
        for tick in 1..=1200 {
            yard.step(Weather::new(100.0, 0.0, 150.0), step, &mut map)
                .expect("rain tick");
            float.advance(tick, &map);
            if let (Some(left), Some(right)) = (wet(&map, pits[0]), wet(&map, pits[1])) {
                if left.row.level.to_bits() == right.row.level.to_bits() {
                    merged = true;
                    eprintln!("yard merged after {tick} ticks at {:.6} m", left.row.level);
                    break;
                }
                separated = true;
                for (hollow, level) in [left.row.level, right.row.level].into_iter().enumerate() {
                    if (level - sill).abs() < 1e-7 && first_spill.is_none() {
                        first_spill = Some(hollow);
                        eprintln!("yard spill from pit {hollow} at {tick} ticks, {sill:.6} m");
                    }
                }
            }
        }
        assert!(separated && merged, "both pits fill separately, then join");
        assert_eq!(
            first_spill,
            Some(0),
            "the lower pit spills into the higher pit first"
        );
        assert!(float.waterline() > initial_float_line);
        assert!(
            (float.waterline()
                - wet(&map, float_point)
                    .expect("water beneath the float")
                    .row
                    .level)
                .abs()
                < 0.000_001
        );
        assert!(wet(&map, shelter).is_none());
        assert!(
            yard.stores.store(0, 0).fill > 0.0,
            "the track takes rain into its pores"
        );
        assert!(
            yard.stores.store(PAVING, 0).fill <= 0.0,
            "paving has no pores"
        );
        for _ in 0..60 {
            yard.step(Weather::new(100.0, 0.0, 600.0), step, &mut map)
                .expect("rain tick");
        }
        assert!(
            wet(&map, [147.1, 62.1]).is_some(),
            "the track keeps its own water"
        );
        let mut parted = false;
        for tick in 1..=1200 {
            yard.step(Weather::new(0.0, 100.0, 600.0), step, &mut map)
                .expect("drying tick");
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
