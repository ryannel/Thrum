use super::*;
use thrum_render::contribution::EffectTime;

fn frame(height: f32) -> ContributionFrame {
    let eye = Vec3::new(0.0, height, 0.0);
    let view = thrum_render::synthetic::SyntheticView::perspective(
        eye.to_array(),
        [0.0, 0.0, 24.0],
        [0.0, 1.0, 0.0],
        70_f32.to_radians(),
        1.77,
        0.1,
        30_000.0,
    )
    .expect("fixture view");
    ContributionFrame {
        time: EffectTime {
            epoch: 1,
            timeline: 0,
            tick: 0,
            tick_duration: std::time::Duration::from_secs_f64(1.0 / 60.0),
            alpha: 1.0,
            display_phase: 1.0,
            capture: true,
        },
        exposure_side: 0,
        environment: None,
        view_proj: view.view_proj_columns(),
        render_extent: [853, 479],
        view_uniform_offset: 0,
        camera: eye.to_array(),
        generation: 1,
        history_reset: false,
    }
}

fn bits(inputs: &Inputs, frame: &ContributionFrame, p: Vec3) -> Option<u32> {
    let clip = Mat4::from_cols_array_2d(&frame.view_proj) * p.extend(1.0);
    let uv = clip.truncate().truncate() / clip.w * Vec2::new(0.5, -0.5) + 0.5;
    if clip.w <= 0.1 || uv.cmplt(Vec2::ZERO).any() || uv.cmpge(Vec2::ONE).any() {
        return None;
    }
    let xy = (uv * UVec2::from_array(frame.render_extent).as_vec2()).as_uvec2() * GRID
        / UVec2::from_array(frame.render_extent);
    let [x, y] = xy.to_array().map(|v| v as usize);
    Some(inputs.mask[y][x / 4][x % 4])
}

// An independent segment/plane test for a box, including a ray that starts inside it.
fn meets(p: Vec3, ray: Vec3, low: Vec3, high: Vec3, reach: f32) -> bool {
    let inside = |p: Vec3| p.cmpge(low - 0.0003).all() && p.cmple(high + 0.0003).all();
    inside(p)
        || (0..3).any(|axis| {
            ray[axis].abs() > 1e-7
                && [low[axis], high[axis]].into_iter().any(|face| {
                    let along = (face - p[axis]) / ray[axis];
                    (0.0..=reach).contains(&along) && inside(p + along * ray)
                })
        })
}

#[test]
fn receiving_rectangles_hold_rays_whose_surface_pixel_is_outside_the_source() {
    let low = Vec3::new(-1.6, -1.0, 22.7);
    let high = Vec3::new(1.6, 2.7, 25.3);
    let body = BodyBounds::new(low.to_array(), high.to_array()).expect("test box");
    let config = Optics::default();
    let mut covered = [0_u32; 3];
    for height in [1.0, 2.0, 6.0, 10.0, 40.0] {
        let frame = frame(height);
        let eye = Vec3::from_array(frame.camera);
        for wave in [0_u8, 1] {
            let a = 0.45 * f32::from(wave);
            let inputs = Shallows::default().inputs(
                config,
                &frame,
                None,
                &[body],
                0.0,
                [0.0, a],
                MirrorRoughness::default(),
                None,
            );
            assert_ne!(inputs.dispatch, [0; 4]);
            for z in 1_u16..320 {
                for x in -20_i16..=20 {
                    let z = f32::from(z) * 0.15;
                    let x = f32::from(x) * 0.13;
                    let phase = z * 0.68 + x * 0.45;
                    let y = a * phase.sin();
                    let p = Vec3::new(x, y, z);
                    let Some(mask) = bits(&inputs, &frame, p) else {
                        continue;
                    };
                    let normal = Vec3::new(-0.45 * a * phase.cos(), 1.0, -0.68 * a * phase.cos())
                        .normalize();
                    let incident = (p - eye).normalize();
                    let c = -normal.dot(incident);
                    if c <= 0.0 {
                        continue;
                    }
                    let reflected = incident + 2.0 * c * normal;
                    let eta = 1.0 / 1.333;
                    let refracted = eta * incident
                        + (eta * c - (1.0_f32 - eta * eta * (1.0 - c * c)).sqrt()) * normal;
                    for (index, (ray, reach)) in [
                        (reflected, config.reflection_reach),
                        (refracted, config.sight_depth()),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if meets(p, ray, low, high, reach) {
                            assert_ne!(
                                mask & 1,
                                0,
                                "camera {height} wave {wave} point {p} ray {ray}"
                            );
                            covered[index] += 1;
                            if index == 0 && mask & SOURCE == 0 {
                                covered[2] += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(covered.into_iter().all(|count| count > 30), "{covered:?}");
}

#[test]
fn a_raised_panel_has_separate_source_and_reflected_regions() {
    let frame = frame(2.0);
    let config = Optics::default();
    let low = Vec3::new(-1.0, 1.25, 12.0);
    let high = Vec3::new(1.0, 2.65, 12.25);
    let body = BodyBounds::new(low.to_array(), high.to_array()).expect("test box");
    let inputs = Shallows::default().inputs(
        config,
        &frame,
        None,
        &[body],
        0.0,
        [0.0; 2],
        MirrorRoughness::default(),
        None,
    );
    let eye = Vec3::from_array(frame.camera);
    for x in [low.x, high.x] {
        for y in [low.y, high.y] {
            for z in [low.z, high.z] {
                let real = Vec3::new(x, y, z);
                let virtual_point = Vec3::new(x, -y, z);
                let receiver = eye.lerp(virtual_point, eye.y / (eye.y + y));
                let source = bits(&inputs, &frame, real).expect("raised panel is on screen");
                let reflection =
                    bits(&inputs, &frame, receiver).expect("reflected panel is on screen");
                assert_ne!(source & SOURCE, 0);
                assert_eq!(
                    reflection & (SOURCE | 1),
                    1,
                    "receiver {receiver} for {real}"
                );
            }
        }
    }
}

#[test]
fn see_through_admission_tracks_sight_depth_and_leaves_deep_map_tiles_uncopied() {
    use thrum_water_model::{Bed, Body, Stage};

    let mut map = Map::prepare(
        Bed::new(
            [-40.0, 15.0],
            2.5,
            [33, 17],
            (0..17)
                .flat_map(|_| (0..33).map(|x| if x < 16 { -1.0 } else { -45.0 }))
                .collect(),
        )
        .expect("bed"),
        vec![Body {
            name: "test sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 100.0,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 1.0,
            swash_ratio: 0.7,
        },
    )
    .expect("map");
    let view = frame(10.0);
    let profile = Optics::default();
    let mut fields = Shallows::default();
    // No resolved wave height at all, but an unresolved RMS slope of 0.1
    // makes this rougher than the scene shader's mirror limit.
    let mut spectrum = thrum_sea_model::Spectrum {
        long: vec![],
        cascades: [160.0, 37.0, 9.0]
            .map(|length| thrum_sea_model::Cascade::new(length, &[]).expect("empty cascade")),
        tail_slope_variance: 0.01,
    };
    let roughness = MirrorRoughness::new(&spectrum, 4.0, 0.0);
    let inputs = fields.inputs(
        profile,
        &view,
        Some(&map),
        &[],
        0.0,
        [0.0, 0.2],
        roughness,
        None,
    );
    let wet = Vec3::new(-12.0, 0.0, 35.0);
    let deep = Vec3::new(31.0, 0.0, 42.0);
    let shallow_bits = bits(&inputs, &view, wet).expect("shallow sight point");
    let deep_bits = bits(&inputs, &view, deep).expect("deep sight point");
    assert_eq!(shallow_bits & (SOURCE | SHALLOW), SOURCE | SHALLOW);
    assert_eq!(deep_bits & (SOURCE | SHALLOW), 0);
    assert_eq!(inputs.reach_slope_scene[2], 0.0);
    assert!(inputs.dispatch[2] < view.render_extent[0]);
    let foamy = fields.inputs(
        profile,
        &view,
        Some(&map),
        &[],
        0.0,
        [0.0, 0.2],
        roughness.with_foam(true),
        None,
    );
    assert_eq!(foamy.dispatch, inputs.dispatch);

    // With whitewater disabled, a slope of 0.05 still cannot mirror. The
    // same spectrum can mirror when whitewater removes slope variance.
    spectrum.tail_slope_variance = 0.0025;
    let slight = MirrorRoughness::new(&spectrum, 4.0, 0.0);
    let bare = fields.inputs(
        profile,
        &view,
        Some(&map),
        &[],
        0.0,
        [0.0, 0.2],
        slight,
        None,
    );
    let foam = fields.inputs(
        profile,
        &view,
        Some(&map),
        &[],
        0.0,
        [0.0, 0.2],
        slight.with_foam(true),
        None,
    );
    assert_eq!(bare.dispatch, inputs.dispatch);
    assert_eq!(foam.dispatch, [0, 0, 853, 479]);

    let clear = Optics {
        extinction: [0.06; 3],
        ..profile
    };
    let clear_inputs = fields.inputs(
        clear,
        &view,
        Some(&map),
        &[],
        0.0,
        [0.0, 0.2],
        roughness,
        None,
    );
    assert_eq!(
        bits(&clear_inputs, &view, deep).expect("depth on screen") & (SOURCE | SHALLOW),
        SOURCE | SHALLOW
    );
    assert_eq!(clear_inputs.reach_slope_scene[2], 0.0);

    map.set_wave_gains(0, [0.25; 4]).expect("sheltered sea");
    let sheltered = fields.inputs(
        profile,
        &view,
        Some(&map),
        &[],
        0.0,
        [0.0, 0.2],
        roughness,
        None,
    );
    assert_eq!(sheltered.reach_slope_scene[2], 1.0);
    assert_eq!(sheltered.dispatch, [0, 0, 853, 479]);
    assert_eq!(
        bits(&sheltered, &view, deep).expect("deep mirror point"),
        SOURCE
    );
}

#[test]
fn basin_mirror_admission_projects_only_tiles_that_can_mirror_at_their_depth() {
    use thrum_water_model::{Bed, Body, Stage};

    let map = Map::prepare(
        Bed::new(
            [-128.0; 2],
            8.0,
            [33, 33],
            (0..33)
                .flat_map(|z| {
                    (0..33).map(move |x| match (x, z) {
                        (0 | 32, _) | (_, 0 | 32) => 3.0,
                        (1..=7, _) => -0.2,
                        _ => -45.0,
                    })
                })
                .collect(),
        )
        .expect("bed"),
        vec![Body {
            name: "test lake".into(),
            kind: BodyKind::Basin { seed: [0.0; 2] },
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 100.0,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 1.0,
            swash_ratio: 0.7,
        },
    )
    .expect("map");
    let down = |x| {
        let mut frame = frame(10.0);
        frame.camera = [x, 10.0, 0.0];
        frame.view_proj = thrum_render::synthetic::SyntheticView::perspective(
            frame.camera,
            [x, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            60_f32.to_radians(),
            1.77,
            0.1,
            30_000.0,
        )
        .expect("fixture view")
        .view_proj_columns();
        frame
    };
    let rough = MirrorRoughness {
        tail_rms_floor: 0.1,
        shortest_height: 0.49,
    };
    let profile = Optics::default();
    let mut fields = Shallows::default();
    let mut inputs = |x, profile, roughness| {
        fields.inputs(
            profile,
            &down(x),
            Some(&map),
            &[],
            0.0,
            [0.0, 0.2],
            roughness,
            None,
        )
    };
    let deep = inputs(16.0, profile, rough);
    assert_eq!(deep.dispatch, [0; 4]);
    assert_eq!(deep.reach_slope_scene[2], 0.0);
    let calm = inputs(16.0, profile, MirrorRoughness::default());
    assert_eq!(calm.dispatch, [0, 0, 853, 479]);
    assert_eq!(calm.reach_slope_scene[2], 1.0);
    assert_eq!(
        bits(&calm, &down(16.0), Vec3::new(16.0, 0.0, 1.0)),
        Some(SOURCE)
    );
    let shelf = inputs(-96.0, profile, rough);
    assert_eq!(shelf.dispatch, [0, 0, 853, 479]);
    assert_eq!(shelf.reach_slope_scene[2], 1.0);
    assert_eq!(
        bits(&shelf, &down(-96.0), Vec3::new(-96.0, 0.0, 1.0)),
        Some(SOURCE | SHALLOW)
    );
    assert_eq!(inputs(16.0, profile, rough).dispatch, [0; 4]);
    assert_eq!(
        inputs(185.0, profile, MirrorRoughness::default()).dispatch,
        [0; 4]
    );
}

#[test]
fn rough_chart_and_basin_copy_only_shallow_tiles() {
    use thrum_water_model::{Bed, Body, Chart, End, Stage};

    let wet: Vec<_> = (0..65)
        .flat_map(|z| (0..65).map(move |x| (1..64).contains(&x) && (1..64).contains(&z)))
        .collect();
    let heights = wet
        .iter()
        .enumerate()
        .map(|(i, &inside)| match (inside, i % 65 < 32) {
            (false, _) => 2.0,
            (true, true) => -0.5,
            (true, false) => -3.0,
        })
        .collect();
    let bed = Bed::new([-80.0; 2], 2.5, [65, 65], heights).expect("split bed");
    let chart = Chart {
        levels: vec![0.0; wet.len()],
        currents: vec![[0.0; 2]; wet.len()],
        footprint: wet.clone(),
        ends: vec![],
    };
    let joined = Chart {
        footprint: wet
            .iter()
            .enumerate()
            .map(|(i, &inside)| inside && i % 65 >= 5)
            .collect(),
        ends: vec![End {
            body: 0,
            segment: [[-67.5, -77.5], [-67.5, 77.5]],
            length: 2.5,
            mouth: false,
        }],
        ..chart.clone()
    };
    let mut view = frame(30.0);
    view.view_proj = thrum_render::synthetic::SyntheticView::perspective(
        view.camera,
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        70_f32.to_radians(),
        1.77,
        0.1,
        30_000.0,
    )
    .expect("split bed view")
    .view_proj_columns();
    let profile = Optics {
        extinction: [4.0; 3],
        ..Optics::default()
    };
    let roughness = MirrorRoughness {
        tail_rms_floor: 0.1,
        shortest_height: 0.02,
    };
    assert!((0.5..3.0).contains(&profile.sight_depth()));
    assert!(!roughness.can_mirror(1.0, 0.5, profile.mirror_limit));
    let make_map = |name: &str, kind, body_gain, source| {
        let body = Body {
            name: name.into(),
            kind,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 150.0,
            gains: [body_gain; 4],
        };
        let mut bodies = Vec::new();
        if let Some((gain, stage)) = source {
            bodies.push(Body {
                name: "joined basin".into(),
                kind: BodyKind::Basin { seed: [-75.0, 0.0] },
                gains: [gain; 4],
                stage,
                ..body.clone()
            });
        }
        bodies.push(body);
        Map::prepare(
            bed.clone(),
            bodies,
            thrum_water_model::Profile {
                margin_height: 0.3,
                swash_ratio: 0.0,
            },
        )
        .expect("level water map")
    };
    let fixed = Stage {
        min: 0.0,
        max: 0.0,
        value: 0.0,
    };
    let mut long_join = joined.clone();
    long_join.ends[0].length = 200.0;
    for (name, kind, body_gain, source, band_height) in [
        ("basin", BodyKind::Basin { seed: [0.0; 2] }, 1.0, None, 0.02),
        ("unjoined chart", BodyKind::Chart(chart), 1.0, None, 0.02),
        (
            "chart with a rough source",
            BodyKind::Chart(joined.clone()),
            1.0,
            Some((1.0, fixed)),
            0.02,
        ),
        (
            "chart with a sheltered source offscreen",
            BodyKind::Chart(joined.clone()),
            1.0,
            Some((
                0.2,
                Stage {
                    min: 0.05,
                    max: 0.05,
                    value: 0.05,
                },
            )),
            0.02,
        ),
        (
            "chart with a partial join",
            BodyKind::Chart(long_join),
            0.4,
            Some((1.0, fixed)),
            0.0,
        ),
    ] {
        let map = make_map(name, kind, body_gain, source);
        let tail = MirrorRoughness {
            shortest_height: band_height,
            ..roughness
        };
        if body_gain < 1.0 {
            // On the level chart the furthest wet edge has the least
            // source influence and therefore the lowest mixed gain.
            let mixed_gain = map.row(1, [77.5, 0.0]).gains[3];
            assert!(tail.can_mirror(body_gain, 3.0, profile.mirror_limit));
            assert!(!tail.can_mirror(mixed_gain, 3.0, profile.mirror_limit));
            println!("{name}: declared gain {body_gain}, minimum wet gain {mixed_gain}");
        }
        let inputs =
            Shallows::default().inputs(profile, &view, Some(&map), &[], 0.0, [0.0; 2], tail, None);
        assert_shallow_copy(name, &inputs, &view);
    }

    // Backwater can carry the receiving body's gains beyond its geometric
    // join. Presentation maps from the same tick have the same revision.
    let mut mouth = joined;
    mouth.ends[0].mouth = true;
    let stages = Stage {
        min: -0.4,
        max: 0.05,
        value: 0.0,
    };
    let map = make_map(
        "chart with a mouth",
        BodyKind::Chart(mouth),
        1.0,
        Some((0.2, stages)),
    );
    let low = map
        .sample_stages(&[-0.4, 0.0])
        .expect("low receiving stage");
    let high = map
        .sample_stages(&[0.05, 0.0])
        .expect("high receiving stage");
    assert_eq!(low.spatial_revision(), high.spatial_revision());
    let mut fields = Shallows::default();
    for (name, map, expected) in [
        ("low", &low, 0.0),
        ("high", &high, 1.0),
        ("low again", &low, 0.0),
    ] {
        let inputs = fields.inputs(
            profile,
            &view,
            Some(map),
            &[],
            0.0,
            [0.0; 2],
            roughness,
            None,
        );
        println!(
            "{name} receiving stage: mirror={}",
            inputs.reach_slope_scene[2]
        );
        assert_eq!(
            inputs.reach_slope_scene[2], expected,
            "{name} receiving stage"
        );
    }
}

fn assert_shallow_copy(name: &str, inputs: &Inputs, view: &ContributionFrame) {
    println!(
        "{name}: mirror={} dispatch={:?} deep={:?}",
        inputs.reach_slope_scene[2],
        inputs.dispatch,
        bits(inputs, view, Vec3::new(32.0, 0.0, 1.0))
    );
    assert_eq!(inputs.reach_slope_scene[2], 0.0, "{name} scene mirror");
    let shallow = Vec3::new(-20.0, 0.0, 1.0);
    let deep = Vec3::new(32.0, 0.0, 1.0);
    assert_eq!(
        bits(inputs, view, shallow),
        Some(SOURCE | SHALLOW),
        "{name} shelf"
    );
    assert_eq!(bits(inputs, view, deep), Some(0), "{name} deep water");
    let cells = inputs.mask.iter().flatten().flatten();
    let shallow_cells = cells.clone().filter(|&&flags| flags & SHALLOW != 0).count();
    let copied_cells = cells.filter(|&&flags| flags & SOURCE != 0).count();
    assert_eq!(copied_cells, shallow_cells, "{name} copied cells");
    assert!((0..view.render_extent[0]).contains(&inputs.dispatch[2]) && inputs.dispatch[2] > 0);
    println!(
        "{name}: {copied_cells} shallow cells copied, dispatch {:?}",
        inputs.dispatch
    );
}

#[test]
fn source_clips_to_the_actual_frustum_when_the_box_straddles_near_depth() {
    let matrix = glam::camera::rh::proj::directx::perspective_infinite_reverse(
        70_f32.to_radians(),
        1.0,
        0.1,
    );
    let view = View {
        matrix,
        inverse: matrix.inverse(),
        extent: UVec2::splat(600),
        grid: GRID,
    };
    let box_at = |min, max| Bounds {
        min: Vec3::from_array(min),
        max: Vec3::from_array(max),
    };
    assert!(
        view.project(box_at([19.0, -1.0, -1.0], [21.0, 1.0, 1.0]))
            .is_none()
    );
    let right = view
        .project(box_at([0.12, -0.04, -1.0], [1.2, 0.04, 0.2]))
        .expect("clipped box");
    assert!(right.low.x > 300 && right.high.x == 600);
    assert!(right.low.y > 120 && right.high.y < 480);
    let enveloping = view
        .project(box_at([-2.0, -2.0, -2.0], [2.0, 2.0, 2.0]))
        .expect("view inside box");
    assert_eq!(enveloping.low, UVec2::ZERO);
    assert_eq!(enveloping.high, view.extent);
}

#[test]
fn film_admission_preserves_ordinary_readers_when_dry_or_out_of_view() {
    let view = frame(3.0);
    let body = BodyBounds::new([-1.0, -0.2, 23.0], [1.0, 1.0, 25.0]).unwrap();
    let behind = BodyBounds::new([-1.0, 0.0, -6.0], [1.0, 0.1, -4.0]).unwrap();
    let visible = BodyBounds::new([-1.0, 0.0, 6.0], [1.0, 0.1, 8.0]).unwrap();
    let pack = |boxes: &[BodyBounds], film| {
        Shallows::default().inputs(
            Optics::default(),
            &view,
            None,
            boxes,
            0.0,
            [0.0; 2],
            MirrorRoughness::default(),
            film,
        )
    };
    let ordinary = pack(&[body], None);
    let offscreen = pack(&[body], Some(behind));
    assert_eq!(ordinary.mask, offscreen.mask);
    assert_eq!(ordinary.dispatch, offscreen.dispatch);
    assert_eq!(pack(&[], Some(behind)).dispatch, [0; 4]);
    let film = pack(&[], Some(visible));
    assert_ne!(film.dispatch, [0; 4]);
    assert_ne!(film.film_sources, [0; 4]);
    assert_eq!(
        film.reach_slope_scene[2].to_bits(),
        0,
        "film has its own reader"
    );
}
