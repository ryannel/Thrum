//! Placed shadow paths are independent of the visible layer.

#[cfg(target_os = "macos")]
mod placed_fixture;
#[cfg(target_os = "macos")]
mod scene_fixture;

thrum_test_support::macos_gpu_test! {
    fn placed_ground_is_identical_with_and_without_visible_clouds() {
        run();
    }
}

#[cfg(target_os = "macos")]
fn run() {
    let Some(gpu) = thrum_test_support::open(
        "placed shadow-only",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let fixture = placed_fixture::Fixture::new();
    assert!(fixture.shape.footprint().column([0.0; 2])[0] > 0.0);
    let angle = 20.0_f32.to_radians();
    let sun = [angle.cos(), angle.sin(), 0.0];
    let arms = [
        ([25.0, 40.0], true),
        ([0.0, 40.0], true),
        ([0.0, 10.0], true),
        ([25.0, 40.0], false),
    ];
    let mut samples = Vec::new();
    pollster::block_on(async {
        for (range, resident) in arms {
            let (_, shadow) = fixture.submit(&gpu, range, false, resident, sun).await;
            let (_, sky) = fixture.submit(&gpu, range, true, resident, sun).await;
            assert!(shadow.sky_products().is_none());
            let resources = shadow.resource_bytes();
            assert!(
                resources
                    .iter()
                    .all(|(name, _)| !name.contains("cloud layer") && name != "cloud detail noise")
            );
            assert!(sky.sky_products().is_some());
            assert_eq!(
                shadow
                    .shapes_report()
                    .expect("shadow shapes")
                    .volume_instances,
                usize::from(resident && range[1] > 20.0)
            );
            assert_eq!(shadow.shapes_report(), sky.shapes_report());
            let depth = placed_fixture::ground(&gpu, &shadow);
            assert_eq!(depth, placed_fixture::ground(&gpu, &sky));
            assert!(depth.iter().any(|&byte| byte != 0));
            samples.push(depth);
        }
    });
    assert_ne!(samples[0], samples[1]);
    assert_ne!(samples[0], samples[2]);
    assert_eq!(samples[2], samples[3]);
}
