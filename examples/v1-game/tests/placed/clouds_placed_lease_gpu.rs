//! Follow a replacement publication through the real game and layer history.

#[cfg(target_os = "macos")]
mod fixture;

thrum_test_support::macos_gpu_test! {
    fn a_changed_stack_stays_pending_through_capture() {
        run();
    }
}

#[cfg(target_os = "macos")]
fn run() {
    use v1_game::GameHost as _;
    let Some(gpu) = thrum_test_support::open(
        "game placed lease",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mut fixture = fixture::Fixture::new(true);
    let mut host = pollster::block_on(fixture.host(&gpu));
    let mut frames = fixture::Frames::new(&gpu);
    let first = frames.placed_submitted(&mut host, |row| {
        row.volume_instances == 1 && row.volume_refusals == 1
    });
    assert_eq!(first.invalidations, 1);
    frames.submit(&mut host);
    assert!(host.cloud_sky_products().expect("layer").history_valid);
    let stats = fixture.sky.residency().expect("residency");
    let generation = fixture.replace_stack();
    thrum_test_support::wait::until(
        "new stack staged",
        std::time::Duration::from_secs(4),
        || {
            host.capture_pixels().expect("readonly upload submission");
            frames.retire(&mut host);
            (stats.lock().expect("stats").drawn == Some(generation)).then_some(())
        },
    );
    host.capture_pixels()
        .expect("readonly use of changed stack");
    frames.retire(&mut host);
    let pending = fixture::report(&host).expect("submitted placement report");
    assert!(pending.history_invalid);
    assert_eq!(pending.invalidations, first.invalidations);
    frames.submit(&mut host);
    let submitted = fixture::report(&host).expect("changed stack used");
    assert!(submitted.history_invalid);
    assert_eq!(submitted.invalidations, first.invalidations + 1);
    assert!(!host.cloud_sky_products().expect("layer").history_valid);
    frames.submit(&mut host);
    let settled = fixture::report(&host).expect("settled stack");
    assert!(!settled.history_invalid);
    assert_eq!(settled.invalidations, submitted.invalidations);
    assert!(host.cloud_sky_products().expect("layer").history_valid);
    let stats = stats.lock().expect("stats");
    assert_eq!(stats.swaps.len(), 2);
    assert_eq!(
        2 * stats.swaps[1].uploaded_bytes,
        stats.swaps[0].uploaded_bytes
    );
}

thrum_test_support::macos_gpu_test! {
    fn no_lease_uses_the_headless_column_and_reports_refusals() {
        no_lease();
    }
}

#[cfg(target_os = "macos")]
fn no_lease() {
    use v1_game::GameHost as _;
    let Some(gpu) = thrum_test_support::open(
        "placed game without a lease",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mut fixture = fixture::Fixture::new(false);
    let mut host = pollster::block_on(fixture.host(&gpu));
    let mut frames = fixture::Frames::new(&gpu);
    let report = frames.placed_submitted(&mut host, |row| row.volume_refusals == 2);
    assert_eq!(report.volume_instances, 0);
    assert_eq!(report.invalidations, 0);
    assert!(report.placed_map_updated);
    host.world_mut().pause().expect("hold the placement state");
    frames.submit(&mut host);
    let (texture, origin, state) = host
        .cloud_placed_input()
        .expect("Shapes")
        .placed_map()
        .expect("columns");
    let read = || {
        thrum_test_support::readback::texture(
            &gpu.device,
            &gpu.queue,
            &texture,
            thrum_test_support::readback::Mip::layer(0, 0),
            ..,
        )
    };
    let raw = read();
    let profile = v1_game::clouds::shade_config();
    let field = thrum_cloud_model::Field::new(profile.seed, profile.model, profile.types)
        .expect("headless model");
    let column = raw
        .as_chunks::<16>()
        .0
        .iter()
        .enumerate()
        .find_map(|(index, row)| {
            let words = row.as_chunks::<4>().0;
            let tau = f32::from_le_bytes(words[2]);
            (words[0] == 1.0_f32.to_le_bytes() && tau > 1.0).then_some((
                index,
                tau,
                f32::from_le_bytes(words[1]),
            ))
        })
        .expect("denied volume still draws its column");
    let width = texture.width() as usize;
    let centre = |index| (f64::from(u32::try_from(index).expect("texel")) + 0.5) * fixture::TEXEL;
    let point = [
        origin[0] + centre(column.0 % width),
        0.0,
        origin[1] + centre(column.0 / width),
    ];
    assert_eq!(column.2.to_bits(), 0.0_f32.to_bits());
    let expected = field.query(point, &state).optical_depth;
    assert!((expected - f64::from(column.1)).abs() < 0.001);
    assert_eq!(fixture.publish_stacks(2), 1);
    let resident = thrum_test_support::wait::until(
        "paused map uses the new lease",
        std::time::Duration::from_secs(4),
        || {
            host.capture_pixels().expect("draw with pending lease");
            frames.retire(&mut host);
            fixture::report(&host).filter(|row| row.volume_instances == 1)
        },
    );
    assert_eq!(resident.placement_upload_bytes, 80);
    assert!(resident.placed_map_updated && resident.history_invalid);
    assert_eq!(resident.invalidations, 0);
    let raw = read();
    let words = raw.as_chunks::<16>().0[column.0].as_chunks::<4>().0;
    assert_eq!(f32::from_le_bytes(words[1]).to_bits(), 1.0_f32.to_bits());
    assert_eq!(f32::from_le_bytes(words[2]).to_bits(), column.1.to_bits());
}

thrum_test_support::macos_gpu_test! {
    fn placement_upload_touches_only_changed_rows() {
        placement_upload();
    }
}

#[cfg(target_os = "macos")]
fn placement_upload() {
    use v1_game::GameHost as _;
    let Some(gpu) = thrum_test_support::open(
        "game placement upload",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    // Centre a shape in a coarse map texel inside the volume fade band.
    let fixture = fixture::Fixture::with_placements(true, &[[-7_840.0, -9_120.0]], [40.0, 0.0]);
    let mut host = pollster::block_on(fixture.host(&gpu));
    let mut frames = fixture::Frames::new(&gpu);
    frames.placed_submitted(&mut host, |row| row.volume_instances == 1);
    let input = host.cloud_placed_input().expect("Shapes").clone();
    let report = || input.shapes_report().expect("submitted placement report");
    let map = || input.placed_map().expect("submitted map");
    let read = || {
        let (texture, _, _) = map();
        thrum_test_support::readback::texture(
            &gpu.device,
            &gpu.queue,
            &texture,
            thrum_test_support::readback::Mip::layer(0, 2),
            ..,
        )
    };
    frames.submit(&mut host);
    let before = report();
    let travel_before = map().2.travel();
    let previous_map = read();
    frames.submit(&mut host);
    let after = report();
    let (_, previous_origin, state) = map();
    assert_ne!(
        travel_before.map(f64::to_bits),
        state.travel().map(f64::to_bits)
    );
    // Only the weight in one row changes as it drifts through the fade band.
    assert_eq!(
        [before.placement_upload_bytes, after.placement_upload_bytes],
        [80; 2]
    );
    assert!(before.placed_map_updated && after.placed_map_updated);
    assert!(
        read() != previous_map,
        "the map must contain the new weight"
    );
    host.set_scene_camera(
        v1_game::SceneCamera::perspective(
            [2_500.0, 40.0, 0.0],
            [2_500.0, 1_700.0, -1_700.0],
            0.75,
            1.0,
            200_000.0,
        )
        .expect("camera beyond the previous weather anchor"),
    )
    .expect("move camera across the weather anchor");
    frames.submit(&mut host);
    assert_ne!(map().1.map(f64::to_bits), previous_origin.map(f64::to_bits));
    assert_eq!(
        report().placement_upload_bytes,
        64 + 80 + 2 * 16,
        "anchor changes the header, one instance and two relative windows"
    );
    assert!(report().placed_map_updated);
    host.world_mut().pause().expect("pause the inline weather");
    frames.submit(&mut host);
    let paused_map = read();
    host.capture_pixels().expect("first paused draw");
    frames.retire(&mut host);
    assert_eq!(report().placement_upload_bytes, 0);
    assert!(!report().placed_map_updated);
    host.set_scene_camera(
        v1_game::SceneCamera::perspective(
            [2_500.0, 40.0, 0.0],
            [3_000.0, 1_700.0, -1_700.0],
            0.75,
            1.0,
            200_000.0,
        )
        .expect("pan at the same camera position"),
    )
    .expect("pan while weather is paused");
    frames.submit(&mut host);
    assert_eq!(report().placement_upload_bytes, 0);
    assert!(!report().placed_map_updated);
    assert!(read() == paused_map, "the skipped map retains its texels");
}
