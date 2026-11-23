//! Check game-directed placement against the submitted GPU map.

#[cfg(target_os = "macos")]
mod fixture;

thrum_test_support::macos_gpu_test! {
    fn directed_weather_map_matches_the_games_query() {
        run();
    }
}

#[cfg(target_os = "macos")]
fn run() {
    use v1_game::GameHost as _;
    let Some(gpu) = thrum_test_support::open(
        "game directed clouds",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mut fixture = fixture::Fixture::new(true);
    // Confirm a changed stack can still serve the same game's initial shape state.
    assert_eq!(fixture.replace_stack(), 2);
    let mut host = pollster::block_on(fixture.host(&gpu));
    let mut frames = fixture::Frames::new(&gpu);
    let report = frames.placed_submitted(&mut host, |report| {
        report.volume_instances == 1 && report.volume_refusals == 1
    });
    assert_eq!(report.directed_refusals, 1);
    assert_eq!(report.invalidations, 1);
    let (texture, origin, state) = host
        .cloud_placed_input()
        .expect("Shapes")
        .placed_map()
        .expect("submitted map");
    assert_eq!(
        state
            .shapes()
            .iter()
            .map(|shape| shape.id)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert!(
        state
            .shapes()
            .iter()
            .all(|shape| shape.target.to_bits() == 1.0_f64.to_bits())
    );
    let raw = thrum_test_support::readback::texture(
        &gpu.device,
        &gpu.queue,
        &texture,
        thrum_test_support::readback::Mip::layer(0, 0),
        ..,
    );
    let got = raw.as_chunks::<16>().0.iter().map(|row| {
        let words = row.as_chunks::<4>().0;
        std::array::from_fn::<_, 4, _>(|i| f32::from_le_bytes(words[i]))
    });
    let width = texture.width() as usize;
    let profile = v1_game::clouds::shade_config();
    let field = thrum_cloud_model::Field::new(profile.seed, profile.model, profile.types)
        .expect("game weather field");
    let mut volumes = [0, 0];
    let mut clear = 0;
    let mut inside = 0;
    for (at, [slot, weight, tau, _]) in got.enumerate() {
        let x = origin[0]
            + (f64::from(u32::try_from(at % width).expect("texel")) + 0.5) * fixture::TEXEL;
        let z = origin[1]
            + (f64::from(u32::try_from(at / width).expect("texel")) + 0.5) * fixture::TEXEL;
        let expected = field.query([x, 0.0, z], &state).optical_depth;
        if expected == 0.0 {
            assert_eq!([slot.to_bits(), tau.to_bits()], [0; 2], "{x}, {z}");
            clear += 1;
        } else {
            assert!(slot > 0.0, "{x}, {z}");
            assert!(
                (f64::from(tau) - expected).abs() < 0.001,
                "{x}, {z}: {tau} vs {expected}"
            );
            if expected > 1.0 {
                assert!(
                    (2.0 * f64::from(tau) - expected).abs() > 0.001,
                    "double density must fail"
                );
                inside += 1;
            }
            volumes[usize::from(weight > 0.0)] += 1;
        }
    }
    assert!(
        clear > 100 && inside > 4 && volumes.iter().all(|&count| count > 4),
        "clear={clear}, deep={inside}, column/volume={volumes:?}"
    );
}

thrum_test_support::macos_gpu_test! {
    fn clear_pixels_match_the_same_view_without_clouds() {
        clear_pixels();
    }
}

#[cfg(target_os = "macos")]
fn clear_pixels() {
    use v1_game::{GameHost as _, GameSetup, SceneCamera};
    let Some(gpu) = thrum_test_support::open(
        "directed clear pixels",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mut fixture = fixture::Fixture::new(true);
    assert_eq!(fixture.replace_stack(), 2);
    let mut placed = pollster::block_on(fixture.host(&gpu));
    let mut absent = pollster::block_on(GameSetup::default().prepare(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        fixture::frame_config(),
    ))
    .expect("cloud-free game");
    let camera = SceneCamera::perspective(
        [0.0, 40.0, 0.0],
        [0.0, 1700.0, -1700.0],
        0.75,
        1.0,
        200_000.0,
    )
    .expect("sky-facing camera");
    absent.set_scene_camera(camera).expect("matched camera");
    let mut frames = fixture::Frames::new(&gpu);
    frames.placed_submitted(&mut placed, |report| {
        report.volume_instances == 1 && !report.history_invalid
    });
    for host in [&mut placed, &mut absent] {
        host.set_time_of_day(0.5).expect("matched noon");
    }
    let actual = placed.capture_pixels().expect("placed image");
    let control = absent.capture_pixels().expect("cloud-free image");
    frames.retire(&mut placed);
    frames.retire(&mut absent);

    let sky = placed.cloud_sky_products().expect("actual trace");
    let texture = &sky.fresh_transport;
    let raw = thrum_test_support::readback::texture(
        &gpu.device,
        &gpu.queue,
        texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    );
    let width = texture.width() as usize;
    let height = texture.height() as usize;
    let mut clear = 0;
    let mut changed = 0;
    for (index, (a, b)) in actual
        .as_chunks::<4>()
        .0
        .iter()
        .zip(control.as_chunks::<4>().0)
        .enumerate()
    {
        let x = (index % fixture::EXTENT[0] as usize) / 2;
        let y = (index / fixture::EXTENT[0] as usize) / 2;
        let sky_only = (y.saturating_sub(1)..=(y + 1).min(height - 1)).all(|py| {
            (x.saturating_sub(1)..=(x + 1).min(width - 1)).all(|px| {
                // Binary16 transmittance is exactly one wherever the march found no density.
                let alpha = (py * width + px) * 8 + 6;
                raw[alpha..alpha + 2] == 0x3c00_u16.to_le_bytes()
            })
        });
        if sky_only {
            assert_eq!(a, b, "clear source and neighbouring samples at ({x}, {y})");
            clear += 1;
        } else if a != b {
            changed += 1;
        }
    }
    assert!(
        clear > 100 && changed > 10,
        "clear={clear}, changed={changed}"
    );
}
