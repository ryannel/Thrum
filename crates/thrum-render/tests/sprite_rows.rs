//! Read submitted geometry and animation back from a 2D host on Metal.

#[expect(
    dead_code,
    reason = "each sprite proof uses a subset of the shared host fixture"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use thrum_render::{
    runtime::{FrameHost, SpriteSceneRequests},
    sprite::{NamedSheet, SpriteRect, SpriteRow, SpriteScene},
    synthetic::CaptureSource,
};

const EXTENT: [u32; 2] = [96, 64];
const RED: [u8; 4] = [228, 72, 55, 255];
const GREEN: [u8; 4] = [29, 216, 106, 255];
const BLUE: [u8; 4] = [85, 113, 245, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 255];

fn art() -> thrum_render::sprite::EmbeddedSheet {
    let mut pixels = Vec::new();
    for _ in 0..10 {
        for colour in [RED, GREEN, BLUE] {
            for _ in 0..12 {
                pixels.extend(colour);
            }
        }
    }
    fixture::sheet(
        [36, 10],
        &pixels,
        &[0, 12, 24].map(|x| SpriteRect {
            origin: [x, 0],
            size: [12, 10],
        }),
    )
}

fn row(frame: u32, position: [f64; 2], degrees: f32, scale: [f32; 2]) -> SpriteRow {
    SpriteRow {
        rotation: degrees.to_radians(),
        scale,
        ..SpriteRow::new(frame, position)
    }
}

fn pixel_check(
    image: &thrum_render::runtime::SubmittedImage,
    pivot: [f64; 2],
    degrees: f64,
    scale: [f64; 2],
    colour: [u8; 4],
) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let mut checked = 0;
    for (index, pixel) in image.rgba.as_chunks::<4>().0.iter().enumerate() {
        let width = EXTENT[0] as usize;
        let x = (index % width) as f64 + 0.5 - f64::from(EXTENT[0]) * 0.5 - pivot[0];
        let y = f64::from(EXTENT[1]) * 0.5 - (index / width) as f64 - 0.5 - pivot[1];
        let u = (x * cos + y * sin) / scale[0];
        let v = (x * sin - y * cos) / scale[1];
        // Rasterizer edge ownership is unspecified across backends.
        if [u, v, u - 12.0, v - 10.0]
            .iter()
            .any(|edge| edge.abs() < 0.003)
        {
            continue;
        }
        let inside = (0.0..12.0).contains(&u) && (0.0..10.0).contains(&v);
        assert_eq!(
            *pixel,
            if inside { colour } else { CLEAR },
            "pixel {index}: ({x},{y}) -> ({u},{v}) at {degrees} degrees"
        );
        checked += usize::from(inside);
    }
    assert!(checked > 30);
}

fn submit(
    host: &mut fixture::Host,
    device: &wgpu::Device,
    tick: u64,
    timeline: u64,
    alpha: f32,
) -> thrum_render::runtime::SubmittedImage {
    let world = host.world_mut();
    world.tick = tick;
    world.timeline = timeline;
    world.alpha = alpha;
    world.running = false;
    fixture::capture(host, device, EXTENT, CaptureSource::Sdr)
}

#[test]
fn interpolates_only_adjacent_matching_endpoints_and_steps_art_by_tick() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("sprite world rows", "portable")
    else {
        return;
    };
    let old = row(0, [-22.0, 12.0], 350.0, [0.75, 1.25]);
    let new = row(1, [12.0, 4.0], 10.0, [1.75, 0.75]);
    let mut scene = fixture::Scene::new(Vec::new(), 1);
    scene.world_rows.push((0, 1, 9, old));
    let mut config = SpriteScene::new(art(), 1);
    config.world_capacity = 2;
    let mut host = fixture::configured_host(&caps, &queue, EXTENT, scene, config);
    pixel_check(
        &submit(&mut host, &device, 0, 0, 1.0),
        old.position,
        350.0,
        [0.75, 1.25],
        RED,
    );
    host.continuous_source_mut().world_rows[0].3 = new;
    for alpha in [0.0_f32, 0.25, 0.5, 1.0, 0.5] {
        let image = submit(&mut host, &device, 1, 0, alpha);
        let a = f64::from(alpha);
        pixel_check(
            &image,
            [-22.0 + 34.0 * a, 12.0 - 8.0 * a],
            350.0 + 20.0 * a,
            [0.75 + a, 1.25 - 0.5 * a],
            GREEN,
        );
        assert_eq!(image.sample.moving, 1);
    }
    // Reuse within one tick in the same physical slot. Then check a new
    // generation with the same stable ID independently of the ID test.
    for (tick, generation, identity) in [(2, 2, 10), (3, 3, 10), (4, 3, 11)] {
        let replacement = row(2, [-4.0 * tick as f64, 2.0], 22.0, [1.0; 2]);
        host.continuous_source_mut().world_rows[0] = (0, generation, identity, replacement);
        let image = submit(&mut host, &device, tick, 0, 0.0);
        pixel_check(&image, replacement.position, 22.0, [1.0; 2], BLUE);
        assert_eq!(image.sample.moving, 0);
    }
    // A replacement timeline may have an apparently adjacent tick and matching IDs.
    host.continuous_source_mut().world_rows[0].3 = old;
    let image = submit(&mut host, &device, 5, 9, 0.25);
    pixel_check(&image, old.position, 350.0, [0.75, 1.25], RED);
    assert_eq!(image.sample.moving, 0);
    let rewind = submit(&mut host, &device, 1, 10, 0.0);
    assert_eq!(rewind.rgba, image.rgba);
    let skipped = submit(&mut host, &device, 3, 10, 0.0);
    assert_eq!(skipped.sample.moving, 0);

    // Two display samples per tick, including a held tick and a loop.
    let frames = [0, 0, 1, 1, 1, 2, 2, 0];
    for (index, frame) in frames.into_iter().enumerate() {
        let tick = index as u64 + 6;
        host.continuous_source_mut().world_rows[0].3.frame = frame;
        for alpha in [0.0, 0.5] {
            let image = submit(&mut host, &device, tick, 10, alpha);
            pixel_check(
                &image,
                old.position,
                350.0,
                [0.75, 1.25],
                [RED, GREEN, BLUE][frame as usize],
            );
        }
        if index == 4 {
            let paused = submit(&mut host, &device, tick, 10, 1.0);
            pixel_check(&paused, old.position, 350.0, [0.75, 1.25], GREEN);
        }
    }
}

#[test]
fn a_tick_page_pair_bridges_a_missed_observation_without_bridging_a_rewind() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("published sprite pair", "portable")
    else {
        return;
    };
    let mut scene = fixture::Scene::new(Vec::new(), 1);
    scene
        .world_rows
        .push((0, 1, 9, row(0, [-30.0, 10.0], 0.0, [1.0; 2])));
    let mut config = SpriteScene::new(art(), 0);
    config.world_capacity = 1;
    let mut host = fixture::configured_host(&caps, &queue, EXTENT, scene, config);

    let previous = row(0, [-12.0, 10.0], 350.0, [0.75, 1.25]);
    let current = row(1, [8.0, 2.0], 10.0, [1.75, 0.75]);
    let scene = host.continuous_source_mut();
    scene.world_rows[0].3 = current;
    scene.published_poses.insert(0, Some(previous.into()));
    let skipped = submit(&mut host, &device, 2, 0, 0.25);
    pixel_check(&skipped, [-7.0, 8.0], 355.0, [1.0, 1.125], GREEN);
    assert_eq!(skipped.sample.moving, 1);

    let rewind = submit(&mut host, &device, 1, 1, 0.25);
    pixel_check(&rewind, current.position, 10.0, [1.75, 0.75], GREEN);
    assert_eq!(rewind.sample.moving, 0);

    let scene = host.continuous_source_mut();
    scene.world_rows[0] = (0, 2, 10, previous);
    scene.published_poses.insert(0, None);
    let reused_slot = submit(&mut host, &device, 3, 1, 0.25);
    pixel_check(&reused_slot, previous.position, 350.0, [0.75, 1.25], RED);
    assert_eq!(reused_slot.sample.moving, 0);
}

#[test]
fn one_recorded_draw_binds_eight_sheets_and_writes_only_changed_world_rows() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("sprite sheet and uploads", "apple-tile")
    else {
        return;
    };
    let mut scene = fixture::Scene::new(vec![SpriteRow::new(0, [0.0, f64::NAN])], 1);
    let mut config = SpriteScene::new(art(), 1);
    config.world_capacity = 16;
    config.sheets.clear();
    let colours = [
        RED,
        GREEN,
        BLUE,
        [247, 202, 116, 255],
        [71, 175, 167, 255],
        [81, 106, 160, 255],
        [127, 66, 174, 255],
        [240, 193, 201, 255],
    ];
    for (slot, colour) in (0_u32..8).zip(colours) {
        config.sheets.push(NamedSheet {
            name: format!("sheet {slot}"),
            sheet: fixture::sheet(
                [1; 2],
                &colour,
                &[SpriteRect {
                    origin: [0; 2],
                    size: [1; 2],
                }],
            ),
        });
        for i in 0..2 {
            let x = slot * 2 + i;
            scene.world_rows.push((
                x,
                1,
                u128::from(x),
                SpriteRow {
                    sheet: if i == 0 { slot } else { 7 - slot },
                    layer: i32::try_from(x % 3).expect("bounded layer"),
                    additive: if i == 0 { 0.0 } else { 1.0 },
                    ..SpriteRow::new(0, [-23.0 + f64::from(x) * 2.0, 5.0])
                },
            ));
        }
    }
    scene
        .world_rows
        .push((16, 1, 17, SpriteRow::new(0, [0.0; 2])));
    let mut host = fixture::configured_host(&caps, &queue, EXTENT, scene.clone(), config.clone());
    let initial = submit(&mut host, &device, 0, 0, 1.0);
    let stats = initial.sample.sprites.expect("sprite telemetry");
    assert_eq!(stats.draws, 1);
    assert_eq!(stats.world_upload_rows, 16);
    assert_eq!(stats.order_upload_bytes, 16 * 4);
    assert_eq!(stats.world_rows, 16);
    assert_eq!(initial.sample.dropped_presentation, 2);
    let stride = stats.world_upload_bytes / 16;
    for x in 0..16 {
        let expected = if x % 2 == 0 {
            colours[x / 2]
        } else {
            colours[7 - x / 2]
        };
        let address = (27 * (EXTENT[0] as usize) + 25 + 2 * x) * 4;
        assert_eq!(&initial.rgba[address..address + 4], &expected);
    }

    host.continuous_source_mut().world_rows[6].3.position[1] += 1.0;
    host.continuous_source_mut().world_rows[8].3.tint[0] = 0.25;
    let changed = submit(&mut host, &device, 1, 0, 0.5);
    let stats = changed.sample.sprites.expect("sprite telemetry");
    assert_eq!(stats.world_upload_rows, 2);
    assert_eq!(stats.world_upload_bytes, 2 * stride);
    assert_eq!(stats.order_upload_bytes, 16 * 4);
    assert_eq!(stats.draws, 1);
    let repeat = submit(&mut host, &device, 1, 0, 0.5);
    let stats = repeat.sample.sprites.expect("sprite telemetry");
    assert_eq!(stats.world_upload_bytes, 0);
    assert_eq!(stats.order_upload_bytes, 0);
    assert_eq!(repeat.rgba, changed.rgba);

    let mut ninth = config.sheets[0].clone();
    ninth.name = "excess embers".into();
    config.sheets.push(ninth);
    let result = pollster::block_on(FrameHost::new_2d(
        &caps,
        &queue,
        fixture::scene_config(EXTENT),
        fixture::World::default(),
        scene,
        config,
        SpriteSceneRequests::default(),
    ));
    let error = result.err().expect("ninth sheet accepted");
    assert!(error.to_string().contains("excess embers"), "{error}");
}
