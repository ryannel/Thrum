//! Yard placements and inspection views for the rain selection.
use std::{
    error::Error,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use thrum_app::sky::{Camera, Controls, CookedMaterial, Display, MeshPlacement, SkyGame};
use thrum_rain_model::{
    Canopy, Ground, GroundClass, GroundStores, Profile, Rain, RainState, RingProfile, Shelter,
    TickReport, Tree,
};
use thrum_water::{
    interaction::{BodyPressure, Mailbox},
    query::{BodyQuery, QueryAnswer, QueryPoint},
};
use thrum_water_model::{
    Bed, Body, BodyKind, Exposure, HollowOptics, Hollows, Impulse, Map, NO_HOLLOW, RainBands,
    RainBandsProfile, RateHistory, RingTable, Stage,
};

pub(super) fn camera(index: usize) -> Camera {
    match index {
        0 => Camera::look_at([144.8, 19.82, 63.35], [149.0, 18.04, 66.4], 60.0),
        1 => Camera::look_at([152.8, 19.82, 63.55], [148.0, 18.04, 66.5], 60.0),
        2 => Camera::look_at([150.2, 19.82, 68.3], [149.6, 18.04, 63.7], 60.0),
        4 => Camera::look_at([147.2, 18.613, 65.45], [147.2, 18.113, 66.2], 67.0),
        5 => Camera::look_at([148.8, 19.813, 65.0], [148.0, 18.113, 66.2], 72.0),
        6 => Camera::look_at([161.0, 41.2, 31.0], [187.0, 11.2, 0.0], 56.0),
        7 => super::CAMERAS[0],
        8 => Camera::look_at([148.23, 19.74, 67.95], [148.23, 18.04, 66.15], 74.0),
        9 | 10 => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "camera positions use GPU metres"
            )]
            let [x, z] = STONES[index - 9].map(|coordinate| coordinate as f32);
            Camera::look_at([x, 19.74, z + 1.73], [x, 18.04, z], 59.0)
        }
        12 => Camera::look_at([153.65, 19.82, 66.0], [152.15, 18.78, 68.05], 56.0),
        13 => Camera::look_at([146.15, 18.62, 64.82], [149.0, 18.42, 66.0], 59.0),
        14 => Camera::look_at([147.45, 19.82, 65.70], [147.7, 18.10, 65.94], 61.0),
        15 => Camera::look_at([144.85, 19.82, 64.8], [149.25, 18.73, 66.3], 56.0),
        _ => Camera::look_at([149.0, 33.12, 64.94], [149.0, 18.12, 65.0], 45.0),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the yard camera uses GPU metres"
)]
pub(super) fn walk_camera(marker: &crate::floating::YardFloat) -> Camera {
    let position = marker.marker_position();
    let x = if position.y > 0.0 {
        position.x
    } else {
        WALK_ENDPOINTS[0][0]
    } as f32;
    Camera::look_at([x, 19.74, 67.95], [x + 0.15, 18.04, 66.15], 74.0)
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
const PROP: usize = 2;
// Cross both puddles on the camera side of the post and floating marker.
const WALK_ENDPOINTS: [[f64; 2]; 2] = [[147.0, 66.39], [149.64, 66.47]];
// Two drops have room to travel along each basin. The third starts
// on the pothole side of the sill, at both fills.
const STONES: [[f64; 2]; 3] = [[147.00, 66.14], [149.64, 65.52], [147.93, 66.23]];
const WALK_SECONDS: f64 = 4.4;
const STEP_SECONDS: f64 = 0.34;
const FOOT_IMPULSE_PER_METRE: f64 = 20.0;
const MARKER_ID: u32 = 1;
const FLOAT_CELL: [usize; 2] = [71, 104 - TRACK_ROWS];

#[derive(Clone, Copy)]
pub(super) struct Weather {
    rain: Rain,
    scale: f64,
    walking: bool,
    stone: [bool; STONES.len()],
}

impl Weather {
    pub(super) fn new(rain: f32, drying: f32, scale: f32) -> Self {
        Self {
            rain: Rain {
                flux: f64::from(rain) / 3600.0,
                drying: f64::from(drying) / 3600.0,
            },
            scale: f64::from(scale),
            walking: false,
            stone: [false; STONES.len()],
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
    footfalls: bool,
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
    let mut walk = None;
    let mut stones = [None; STONES.len()];
    if footfalls {
        walk = Some(
            controls
                .slider("rain.walk", "Walk through the puddles", 0.0..=1.0)
                .step(1.0)
                .add()?,
        );
        for (slot, (key, name)) in stones.iter_mut().zip([
            ("rain.stone_left", "Drop a stone into the pothole"),
            ("rain.stone_right", "Drop a stone into the paving puddle"),
            ("rain.stone_sill", "Drop a stone beside the sill"),
        ]) {
            *slot = Some(controls.slider(key, name, 0.0..=1.0).step(1.0).add()?);
        }
    }
    Ok(move |controls: &Controls| {
        let mut weather = Weather::new(
            controls.get(rate),
            controls.get(drying),
            controls.get(scale),
        );
        weather.walking = walk.is_some_and(|key| controls.get(key) > 0.5);
        weather.stone = stones.map(|key| key.is_some_and(|key| controls.get(key) > 0.5));
        weather
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

#[derive(Clone, Copy)]
pub(super) enum FootfallLevels {
    NearAndFar,
    WithFine,
}

pub(super) struct Yard {
    patches: [GroundPatch; 2],
    canopy: Arc<Canopy>,
    stores: GroundStores,
    wet_ground: Option<thrum_water::wet_ground::WetGround>,
    wet_classes: Option<[thrum_water::wet_ground::GroundClass; 3]>,
    wet_rows: Vec<thrum_water::wet_ground::HollowWetness>,
    levels: Vec<f64>,
    replies: super::RainYardReplies,
    pub(super) draw_float: bool,
    pub(super) draw_rings: bool,
    pub(super) measure_regions: bool,
    pub(super) footfalls: Option<FootfallLevels>,
}

impl Yard {
    pub(super) fn has_wet_ground_views(&self) -> bool {
        self.stores.classes().len() > PROP
    }

    pub(super) fn size(&self) -> (usize, usize) {
        (SIZE.iter().product(), self.levels.len())
    }

    pub(super) fn bands(&self) -> Result<RainBands, thrum_water_model::WaterError> {
        static TABLE: OnceLock<Arc<RingTable>> = OnceLock::new();
        let schedule = RainBandsProfile {
            count: 256.0,
            ..RainBandsProfile::default()
        };
        let table = TABLE.get_or_init(|| {
            Arc::new(
                RingProfile {
                    count: schedule.count,
                    ..RingProfile::default()
                }
                .table(),
            )
        });
        let canopy = Arc::clone(&self.canopy);
        RainBands::new(
            schedule,
            Arc::clone(table),
            RateHistory::dry(1.0 / 60.0, -1, 62),
            canopy,
        )
    }

    pub(super) fn replies(&self) -> super::RainYardReplies {
        Arc::clone(&self.replies)
    }

    pub(super) fn marker(&self) -> Option<crate::floating::YardFloat> {
        self.footfalls
            .is_some()
            .then(|| crate::floating::YardFloat::new(WALK_ENDPOINTS[0], 0.0))
    }

    pub(super) fn install(
        mut self,
        game: SkyGame,
        water: &thrum_water::Water,
        map: crate::SharedMap,
        input: Arc<Mutex<Weather>>,
        marker: Option<crate::floating::YardFloat>,
        mut readout: impl FnMut(&mut Controls, f64) + 'static,
    ) -> SkyGame {
        let game = match &self.wet_ground {
            Some(wet) => game.with_wetness(wet.provider()),
            None => game,
        };
        let float = self.float();
        let displayed_float = float.clone();
        let mailbox = water.query_mailbox();
        let interaction = water.interaction_mailbox();
        let mut walk = Walk::default();
        #[cfg(target_arch = "wasm32")]
        let displayed_replies = self.replies();
        #[cfg(target_arch = "wasm32")]
        let status_map = Arc::clone(&map);
        #[cfg(target_arch = "wasm32")]
        let water_report = water.report();
        let points = self.query_points();
        let game = if self.draw_float {
            float.install(game, water.optics_bodies())
        } else {
            game
        };
        let game = if let Some(marker) = &marker {
            // The walking marker is a visual cue. The float alone publishes the water's opaque bounds.
            marker.install(game, thrum_water::OpticsBodies::default())
        } else {
            game
        };
        game.with_status_readouts(move |controls| {
            let level = displayed_float.waterline();
            readout(controls, level);
            #[cfg(target_arch = "wasm32")]
            browser_status(
                &status_map.lock().expect("yard water map"),
                displayed_replies.lock().expect("yard query reply").as_ref(),
                level,
                water_report.lock().expect("rain report").rain_drops,
            );
        })
        .with_fixed_update_endpoint(move |endpoint, dt| {
            let tick = endpoint.tick;
            if tick == 0 {
                return;
            }
            let weather = *input.lock().expect("yard weather");
            {
                let mut map = map.lock().expect("yard water map");
                self.step(weather, dt, &mut map)
                    .expect("prepared yard rain tick");
                if let Some(bands) = map.rain() {
                    let tick = i64::try_from(tick).expect("yard tick in range");
                    let mut history = bands.history().clone();
                    // The interval ending at this endpoint has just been committed.
                    history.last = tick - 2;
                    history.tick = dt.as_secs_f64();
                    history.commit(weather.rain.flux);
                    map.set_rate_history(history)
                        .expect("valid yard rain history");
                }
                if let Some(interaction) = &interaction {
                    if let Some(marker) = &marker {
                        walk.step(tick, dt, weather, &map, interaction, marker);
                    } else {
                        follow(interaction, tick, [147.2, 66.2], 0);
                    }
                }
            }
            if let Some(mailbox) = &mailbox {
                let ready = mailbox.drain(endpoint, dt).into_iter().next();
                if let Some(answer) = ready {
                    float.advance(tick, wet_level(&answer.points[0]));
                    *self.replies.lock().expect("yard query reply") = Some(answer);
                }
                mailbox.post(
                    endpoint,
                    vec![BodyQuery {
                        id: 0,
                        generation: 0,
                        points: points
                            .map(|position| QueryPoint {
                                position,
                                radius: 0.0,
                                use_tick: tick,
                            })
                            .into(),
                    }],
                );
            }
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "body query positions use world f32 metres"
    )]
    fn query_points(&self) -> [[f32; 2]; 5] {
        let float_point = self.patches[PAVING]
            .tree
            .ground()
            .centre(FLOAT_CELL[1] * SIZE[0] + FLOAT_CELL[0])
            .map(|x| x as f32);
        [
            float_point,
            [152.4, 65.0],
            [151.75, 67.8],
            [147.2, 66.2],
            [149.75, 66.35],
        ]
    }

    pub(super) fn float(&self) -> crate::floating::YardFloat {
        let index = FLOAT_CELL[1] * SIZE[0] + FLOAT_CELL[0];
        let ground = self.patches[PAVING].tree.ground();
        crate::floating::YardFloat::new(ground.centre(index), ground.heights()[index])
    }

    pub(super) fn prepare(
        profile: Profile,
        wet_look: Option<bool>,
    ) -> Result<Self, Box<dyn Error>> {
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
        let film_classes = [
            GroundClass {
                soak: 15.0 / 3600.0,
                percolation: 2.0 / 3600.0,
                pores: 0.003,
                relief: 0.00085,
                share: 0.68,
                porosity: 0.84,
                steepest: 0.34,
                film_roughness: 0.19,
            },
            GroundClass {
                soak: 2.0 / 3600.0,
                percolation: 0.4 / 3600.0,
                pores: 0.00045,
                relief: 0.00062,
                share: 0.47,
                porosity: 0.65,
                steepest: 0.16,
                film_roughness: 0.075,
            },
            GroundClass {
                soak: 4.0 / 3600.0,
                percolation: 2.0 / 3600.0,
                pores: 0.00065,
                relief: 0.00018,
                share: 0.46,
                porosity: 0.42,
                steepest: 0.16,
                film_roughness: 0.16,
            },
        ];
        let legacy = [
            GroundClass {
                soak: 9.0 / 3600.0,
                percolation: 3.0 / 3600.0,
                pores: 0.008,
                relief: 0.0005,
                porosity: 0.55,
                ..GroundClass::bare(0.0)
            },
            GroundClass::bare(0.0),
        ];
        let stores = GroundStores::new(if wet_look.is_some() {
            &film_classes[..]
        } else {
            &legacy
        })?;
        let count = patches.iter().map(|patch| patch.tree.hollow_count()).sum();
        let wet_classes = wet_look
            .filter(|&enabled| enabled)
            .map(|_| wet_classes(&film_classes));
        let wet_ground = wet_classes
            .as_ref()
            .map(|classes| {
                prepare_wet_ground(&canopy, &patches, classes, count)
                    .ok_or("yard wetness exceeds the render projection")
            })
            .transpose()?;
        let mut yard = Self {
            patches,
            canopy: Arc::new(canopy),
            stores,
            wet_ground,
            wet_classes,
            wet_rows: Vec::with_capacity(if wet_classes.is_some() { count } else { 0 }),
            levels: Vec::with_capacity(count),
            replies: Arc::default(),
            draw_float: true,
            draw_rings: true,
            measure_regions: false,
            footfalls: None,
        };
        yard.refresh_levels();
        yard.publish_wetness();
        Ok(yard)
    }

    fn refresh_levels(&mut self) {
        self.levels.clear();
        for patch in &self.patches {
            self.levels.extend(patch.rain.levels(&patch.tree));
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPU projection of rain-model state"
    )]
    fn publish_wetness(&mut self) {
        let Some((wet_ground, classes)) = self.wet_ground.as_mut().zip(self.wet_classes.as_ref())
        else {
            return;
        };
        self.wet_rows.clear();
        for (class, patch) in self.patches.iter().enumerate() {
            for hollow in 0..patch.tree.hollow_count() {
                let level = self.levels[self.wet_rows.len()];
                self.wet_rows.push(thrum_water::wet_ground::HollowWetness {
                    level: level as f32,
                    mark: patch.rain.marks()[hollow] as f32,
                    margin: patch.rain.bands()[hollow] as f32,
                    held_fill: patch.rain.held_fill(
                        &patch.tree,
                        &self.stores.classes()[class],
                        hollow,
                    ) as f32,
                });
            }
        }
        let stores = [PROP, 0, PAVING].map(|class| {
            std::array::from_fn(|reach| {
                let store = self.stores.store(class, reach);
                [store.surface as f32, store.fill as f32]
            })
        });
        assert!(wet_ground.publish(classes, &stores, &self.wet_rows));
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
                scatter: [0.08, 0.14, 0.23],
                bed_tint: [0.49, 0.68, 0.83],
            });
            self.levels.len()
        ];
        optics[..track_hollows].fill(Some(HollowOptics {
            extinction: 90.0,
            scatter: [0.17, 0.092, 0.045],
            bed_tint: [1.0; 3],
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
        self.publish_wetness();
        map.set_levels(BODY, &self.levels)
            .map_err(|error| format!("{error:?}"))?;
        Ok(())
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "authored yard values in GPU metres"
)]
fn wet_classes(classes: &[GroundClass]) -> [thrum_water::wet_ground::GroundClass; 3] {
    [PROP, 0, PAVING].map(|index| {
        let class = classes[index];
        let pattern = if index == PAVING {
            // The opaque yard's mesh and albedo share the same half-metre joints.
            thrum_water::wet_ground::ReliefPattern {
                period: [0.5; 2],
                bank: 0.05,
                elevation: [0.0; 2],
            }
        } else if index == 0 {
            // The two ruts leave a crown as high as the track shoulders.
            thrum_water::wet_ground::ReliefPattern {
                elevation: [18.064, 18.11],
                ..thrum_water::wet_ground::ReliefPattern::default()
            }
        } else {
            thrum_water::wet_ground::ReliefPattern::default()
        };
        thrum_water::wet_ground::GroundClass {
            relief: class.relief as f32,
            steepest: class.steepest as f32,
            share: class.share as f32,
            porosity: class.porosity as f32,
            film_roughness: class.film_roughness as f32,
            pattern,
        }
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "yard dimensions and bed in GPU units"
)]
fn prepare_wet_ground(
    canopy: &Canopy,
    patches: &[GroundPatch; 2],
    classes: &[thrum_water::wet_ground::GroundClass],
    count: usize,
) -> Option<thrum_water::wet_ground::WetGround> {
    use thrum_water::wet_ground::{GroundCell, HollowsGround, WetGround};
    let mut cells = Vec::with_capacity(SIZE.iter().product());
    let mut offset = 0;
    for (class, patch) in patches.iter().enumerate() {
        let bed = patch.tree.ground().heights();
        cells.extend(
            bed.iter()
                .zip(patch.tree.labels())
                .map(|(&bed, &label)| GroundCell {
                    bed: bed as f32,
                    hollow: if label == NO_HOLLOW {
                        NO_HOLLOW
                    } else {
                        label + offset
                    },
                    pit: if label == NO_HOLLOW {
                        bed as f32
                    } else {
                        patch.tree.node(label).lower as f32
                    },
                    class: (class + 1) as u8,
                }),
        );
        offset += patch.tree.hollow_count() as u32;
    }
    WetGround::new(
        canopy.canopy_grid(),
        Some(HollowsGround {
            origin: ORIGIN.map(|v| v as f32),
            spacing: CELL as f32,
            size: SIZE.map(|v| v as u32),
            cells,
        }),
        classes,
        count,
    )
}

#[derive(Default)]
struct Walk {
    elapsed: f64,
    next_step: f64,
    stones: [bool; STONES.len()],
    right_foot: bool,
}

impl Walk {
    fn step(
        &mut self,
        tick: u64,
        dt: Duration,
        weather: Weather,
        map: &Map,
        mailbox: &Mailbox,
        marker: &crate::floating::YardFloat,
    ) {
        let depth = |point| {
            map.sample(point)
                .filter(|hit| hit.body == BODY)
                .map_or(0.0, |hit| hit.depth.max(0.0))
        };
        let impulse = |place, vertical, radius, source| {
            if vertical > 0.0 {
                mailbox.post_impulse(Impulse {
                    id: source,
                    place,
                    vertical,
                    radius,
                    tick,
                });
            }
        };
        for (i, (drop, was_down)) in weather.stone.into_iter().zip(&mut self.stones).enumerate() {
            if drop && !*was_down {
                let place = STONES[i];
                if depth(place) > 0.0 {
                    impulse(place, 1.20, 0.075, 1 + i as u64);
                }
            }
            *was_down = drop;
        }
        let (place, height) = if weather.walking {
            let phase = (self.elapsed / WALK_SECONDS).clamp(0.0, 1.0);
            let place = std::array::from_fn(|axis| {
                WALK_ENDPOINTS[0][axis]
                    + phase * (WALK_ENDPOINTS[1][axis] - WALK_ENDPOINTS[0][axis])
            });
            if self.elapsed >= self.next_step && self.elapsed <= WALK_SECONDS {
                let foot = [
                    place[0],
                    place[1] + if self.right_foot { 0.045 } else { -0.045 },
                ];
                let depth = depth(foot);
                // A thin sheet is struck by the heel. Deeper water wets a
                // broader area of the foot before it reaches the bed.
                let heel_radius = 0.039 + 0.33 * (depth - 0.020).clamp(0.0, 0.045);
                impulse(foot, FOOT_IMPULSE_PER_METRE * depth, heel_radius, 0);
                self.next_step += STEP_SECONDS;
                self.right_foot = !self.right_foot;
            }
            self.elapsed += dt.as_secs_f64();
            let floor = map.hollow_bed_at(place).expect("footpath is in the yard");
            let bob = 0.025
                * (std::f64::consts::TAU * self.elapsed / STEP_SECONDS)
                    .sin()
                    .abs();
            (place, floor + 0.15 + bob)
        } else {
            self.elapsed = 0.0;
            self.next_step = 0.0;
            self.right_foot = false;
            (
                std::array::from_fn(|axis| {
                    f64::midpoint(WALK_ENDPOINTS[0][axis], WALK_ENDPOINTS[1][axis])
                }),
                0.0,
            )
        };
        marker.move_marker(tick, place, height);
        follow(mailbox, tick, place, MARKER_ID);
    }
}

fn follow(mailbox: &Mailbox, tick: u64, place: [f64; 2], id: u32) {
    mailbox.post_bodies(
        tick,
        Some(id),
        vec![BodyPressure {
            id,
            place,
            force: 0.0,
            footprint: [0.1; 2],
            heading: 0.0,
        }],
    );
}

fn wet_level(answer: &QueryAnswer) -> Option<f64> {
    (answer.surface.wet && answer.surface.body == Some(BODY))
        .then_some(answer.surface.sample.height)
}

#[cfg(target_arch = "wasm32")]
fn browser_status(
    map: &Map,
    reply: Option<&thrum_water::query::BodyAnswers>,
    float_waterline: f64,
    rain_drops: Option<[usize; 2]>,
) {
    let status = js_sys::Array::new();
    let column = |point| {
        map.sample(point)
            .filter(|hit| hit.body == BODY && hit.depth > 0.0)
            .map(|hit| hit.row.level)
    };
    for height in [
        column([147.2, 66.2]),
        column([149.75, 66.35]),
        reply.and_then(|answer| wet_level(&answer.points[0])),
        reply.and_then(|answer| wet_level(&answer.points[2])),
    ] {
        status.push(&height.map_or(js_sys::wasm_bindgen::JsValue::NULL, Into::into));
    }
    status.push(&float_waterline.into());
    if let Some(counts) = rain_drops {
        for count in counts {
            status.push(&u32::try_from(count).expect("bounded drops").into());
        }
    }
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
        let mut yard =
            Yard::prepare(Profile::default(), None).expect("prepare authored yard triangles");
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
        let mut transferred = 0.0_f64;
        let tree = &yard.patches[PAVING].tree;
        assert_eq!(tree.hollow_count(), 2, "the paving has two hollows");
        let sill = tree.node(0).sill;
        let brim = tree.node(tree.node(0).parent.expect("paving joins")).sill;
        assert_eq!(sill.to_bits(), tree.node(1).sill.to_bits());
        let [lower, upper] = if tree.node(0).lower < tree.node(1).lower {
            [0, 1]
        } else {
            [1, 0]
        };
        assert_eq!(tree.node(lower).over, Some(upper));
        let upper_volume = |yard: &Yard| {
            yard.patches[PAVING]
                .rain
                .puddles()
                .find_map(|(node, volume)| (node == upper).then_some(volume))
                .unwrap_or(0.0)
        };
        for tick in 1..=1200 {
            let before = upper_volume(&yard);
            yard.step(Weather::new(100.0, 0.0, 150.0), step, &mut map)
                .expect("rain tick");
            if let (Some(left), Some(right)) = (wet(&map, pits[0]), wet(&map, pits[1])) {
                if left.row.level.to_bits() == right.row.level.to_bits() {
                    merged = true;
                    eprintln!("yard merged after {tick} ticks at {:.6} m", left.row.level);
                    break;
                }
                separated = true;
                if (left.row.level - sill).abs() < 1e-7 && right.row.level < sill {
                    // The upper catchment charges its own rain and runoff separately.
                    // While the pools are separate, any additional volume crossed the sill.
                    let gain = yard.patches[PAVING].report.hollows[upper as usize];
                    let own = gain.on_water + gain.runoff + gain.held_runoff + gain.joined
                        - gain.left
                        - gain.topped
                        - gain.bed
                        - gain.dried;
                    transferred += (upper_volume(&yard) - before - own).max(0.0);
                }
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
        assert!(
            transferred > 1e-5,
            "the upper pit receives water across the sill"
        );
        eprintln!(
            "yard spill before merge: {:.3} litres to the upper pit",
            transferred * 1_000.0
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
        for _ in 0..300 {
            yard.step(Weather::new(100.0, 0.0, 600.0), step, &mut map)
                .expect("rain tick");
        }
        assert!(
            wet(&map, [147.1, 62.1]).is_some(),
            "the track keeps its own water"
        );
        let full = wet(&map, pits[0])
            .expect("the joined puddle remains")
            .row
            .level;
        assert!((full - brim).abs() < 1e-7, "paving reaches its outlet");
        eprintln!("yard brim {brim:.6} m");
        assert!(
            wet(&map, shelter).is_none(),
            "the roof stays dry at the brim"
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
