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
                a,
                MirrorRoughness::default(),
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
        0.0,
        MirrorRoughness::default(),
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
    let spectrum = thrum_sea_model::Spectrum {
        long: vec![],
        cascades: [160.0, 37.0, 9.0]
            .map(|length| thrum_sea_model::Cascade::new(length, &[]).expect("empty cascade")),
        tail_slope_variance: 0.01,
    };
    let roughness = MirrorRoughness::new(&spectrum, 4.0, 0.0);
    let inputs = fields.inputs(profile, &view, Some(&map), &[], 0.0, 0.2, roughness);
    let wet = Vec3::new(-12.0, 0.0, 35.0);
    let deep = Vec3::new(31.0, 0.0, 42.0);
    let shallow_bits = bits(&inputs, &view, wet).expect("shallow sight point");
    let deep_bits = bits(&inputs, &view, deep).expect("deep sight point");
    assert_eq!(shallow_bits & (SOURCE | SHALLOW), SOURCE | SHALLOW);
    assert_eq!(deep_bits & (SOURCE | SHALLOW), 0);
    assert_eq!(inputs.reach_slope_scene[2], 0.0);
    assert!(inputs.dispatch[2] < view.render_extent[0]);

    let clear = Optics {
        extinction: [0.06; 3],
        ..profile
    };
    let clear_inputs = fields.inputs(clear, &view, Some(&map), &[], 0.0, 0.2, roughness);
    assert_eq!(
        bits(&clear_inputs, &view, deep).expect("depth on screen") & (SOURCE | SHALLOW),
        SOURCE | SHALLOW
    );
    assert_eq!(clear_inputs.reach_slope_scene[2], 0.0);

    map.set_wave_gains(0, [0.25; 4]).expect("sheltered sea");
    let sheltered = fields.inputs(profile, &view, Some(&map), &[], 0.0, 0.2, roughness);
    assert_eq!(sheltered.reach_slope_scene[2], 1.0);
    assert_eq!(sheltered.dispatch, [0, 0, 853, 479]);
    assert_eq!(
        bits(&sheltered, &view, deep).expect("deep mirror point"),
        SOURCE
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
