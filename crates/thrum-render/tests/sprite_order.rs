//! Pixel witnesses for exact layer, sort and stable identity ordering.

#[expect(
    dead_code,
    reason = "each sprite proof uses a subset of the shared host fixture"
)]
mod support {
    pub(crate) mod sprite_fixture;
}
use support::sprite_fixture as fixture;

use thrum_render::{
    sprite::{SpriteRect, SpriteRow, SpriteScene},
    synthetic::CaptureSource,
};

const COLORS: [[u8; 4]; 4] = [
    [233, 78, 62, 255],
    [39, 223, 127, 255],
    [82, 146, 238, 255],
    [239, 180, 51, 255],
];
const EXTENT: [u32; 2] = [48, 16];

fn row(frame: u32, x: f64, layer: i32, sort: f64) -> SpriteRow {
    SpriteRow {
        layer,
        sort: Some(sort),
        scale: [4.0; 2],
        ..SpriteRow::new(frame, [x, 2.0])
    }
}

#[test]
fn explicit_keys_hold_in_readback_while_other_rows_upload_and_slots_change_owner() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("exact sprite order", "portable")
    else {
        return;
    };
    let frames = [0, 1, 2, 3].map(|x| SpriteRect {
        origin: [x, 0],
        size: [1; 2],
    });
    let pixels: Vec<_> = COLORS.into_iter().flatten().collect();
    let mut config = SpriteScene::new(fixture::sheet([4, 1], &pixels, &frames), 2);
    config.world_capacity = 10;
    let mut scene = fixture::Scene::new(
        vec![
            row(3, 4.0, 0, 2.0),   // world/frame tie
            row(3, 12.0, -1, 3.0), // frame layer behind the world
        ],
        1,
    );
    scene.world_rows = vec![
        (0, 1, 80, row(0, -20.0, 0, 3.0)),
        (1, 1, 10, row(1, -20.0, 0, 3.0)), // only identity decides
        (2, 1, 79, row(1, -12.0, 0, 7.0)),
        (3, 1, 25, row(2, -12.0, 0, 8.0)),
        (4, 1, 78, row(0, -4.0, -1, 90.0)),
        (5, 1, 26, row(2, -4.0, 0, 1.0)),
        (6, 1, 27, row(0, 4.0, 0, 2.0)),
        (7, 1, 28, row(1, 12.0, 0, 1.0)),
        (8, 1, 29, row(0, 18.0, 0, 0.0)), // unrelated upload
        (9, 1, 30, row(3, 18.0, 0, 1.0)),
    ];
    let mut host = fixture::configured_host(&caps, &queue, EXTENT, scene, config);
    for tick in 1..=96 {
        let source = host.continuous_source_mut();
        source.world_rows[8].3.position[1] = 2.0 + f64::from(tick % 2);
        source.world_rows[9].3.sort = Some(if tick % 2 == 0 { -1.0 } else { 1.0 });
        if tick == 32 {
            source.world_rows[0] = (0, 2, 3, row(0, -20.0, 0, 3.0));
        } else if tick == 64 {
            source.world_rows[1] = (1, 2, 2, row(1, -20.0, 0, 3.0));
        }
        let image = fixture::capture(&mut host, &device, EXTENT, CaptureSource::Sdr);
        let tie = usize::from((32..64).contains(&tick));
        let last = if tick % 2 == 0 { 0 } else { 3 };
        for (x, winner) in [5, 13, 21, 29, 37, 43]
            .into_iter()
            .zip([tie, 2, 2, 3, 1, last])
        {
            let address = (7 * 48 + x) * 4;
            assert_eq!(
                &image.rgba[address..address + 4],
                &COLORS[winner],
                "tick {tick} x={x}"
            );
        }
    }
}

#[test]
fn frame_rows_use_the_first_world_crossing_and_keep_input_ties() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities: caps,
        ..
    }) = fixture::open("mixed sprite order", "portable")
    else {
        return;
    };
    let frames = [0, 1, 2, 3].map(|x| SpriteRect {
        origin: [x, 0],
        size: [1; 2],
    });
    // Translucent quads make the pixel sensitive to all four positions in
    // the draw order, even though the quads fully overlap.
    let pixels = COLORS.map(|[r, g, b, _]| [r, g, b, 128]).concat();
    let mut config = SpriteScene::new(fixture::sheet([4, 1], &pixels, &frames), 4);
    config.world_capacity = 2;
    let pixel = |image: &thrum_render::runtime::SubmittedImage| {
        let address = (7 * 48 + 25) * 4;
        <[u8; 4]>::try_from(&image.rgba[address..address + 4]).unwrap()
    };
    let cases = [
        (1.0, 0, [2, 3, 0, 1]), // F, G, A, B
        (0.0, 0, [1, 2, 3, 0]), // B, F, G, A
        (1.0, 1, [0, 1, 2, 3]), // A, B, F, G
    ];
    let mut witnesses = Vec::new();
    for (band, layer, order) in cases {
        config.order_band = band;
        let mut scene =
            fixture::Scene::new(vec![row(2, 0.0, layer, 0.25), row(3, 0.0, layer, 0.25)], 1);
        scene.world_rows = vec![
            (0, 1, 1, row(0, 0.0, 0, 0.0)),
            (1, 1, 2, row(1, 0.0, 0, 1.0)),
        ];
        let mut host = fixture::configured_host(&caps, &queue, EXTENT, scene, config.clone());
        fixture::capture(&mut host, &device, EXTENT, CaptureSource::Sdr);
        let rows = &mut host.continuous_source_mut().world_rows;
        rows[0].3.sort = Some(0.75);
        rows[1].3.sort = Some(0.25);
        let actual = fixture::capture(&mut host, &device, EXTENT, CaptureSource::Sdr);

        // An independent ordering control uses four distinct layers.
        let reference = fixture::Scene::new(
            order
                .into_iter()
                .zip(0..)
                .map(|(frame, layer)| row(frame, 0.0, layer, 0.0))
                .collect(),
            1,
        );
        let mut reference =
            fixture::configured_host(&caps, &queue, EXTENT, reference, config.clone());
        let expected = fixture::capture(&mut reference, &device, EXTENT, CaptureSource::Sdr);
        assert_eq!(
            pixel(&actual),
            pixel(&expected),
            "band {band}, frame layer {layer}"
        );
        witnesses.push(pixel(&expected));
    }
    witnesses.sort_unstable();
    witnesses.dedup();
    assert_eq!(
        witnesses.len(),
        cases.len(),
        "each ordering has a distinct overlap pixel"
    );
}
