//! GPU readbacks of the shipping sky held to values written in the tests: the
//! night checks and the upper-twilight guard. The f64 Monte Carlo reference
//! and quadrature these values were first compared with are a diagnostic and
//! no longer in the tree (GATE-TIME M2); the written values stand alone.

mod night;
mod query;
mod readback;

#[test]
fn shipping_spectral_twilight_matches_its_written_radiance() {
    let Some(crate::test_device::Opened {
        gpu: test_gpu,
        device,
        queue,
        ..
    }) = crate::test_device::open(
        "shipping_spectral_twilight_matches_its_written_radiance",
        None,
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        },
    )
    else {
        return;
    };
    pollster::block_on(async {
        let adapter = test_gpu.adapter.clone();
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let [width, height] = crate::atmosphere::stage::Chart::MultipleScattering
            .extent()
            .map(|v| v as usize);
        shipping_spectral_twilight(&device, &queue, width * height);
        assert!(scope.pop().await.is_none());
    });
}

// Same physical observation as the eight-node comparison in the G3s evidence.
// Exercise both spectral atlas planes and the actual Sky lookup in one row.
fn shipping_spectral_twilight(device: &wgpu::Device, queue: &wgpu::Queue, plane_pixels: usize) {
    let (sv, cv) = 20.0_f32.to_radians().sin_cos();
    let (sa, ca) = 95.104_59_f32.to_radians().sin_cos();
    let mut probe = readback::Probe::new(device, queue, &[[cv * ca, sv, cv * sa, 0.0]]);
    let sky = probe.sky(device, queue, -5.084_449, 244.922_67);
    let actual = sky.rays_full[0];
    for (channel, expected) in [2.982_646e-5, 4.289_131e-5, 8.114_284e-5]
        .into_iter()
        .enumerate()
    {
        assert!(
            (actual[channel] - expected).abs() <= 0.025 * expected,
            "upper twilight spectral radiance: actual {actual:?}, channel {channel} expected {expected}"
        );
    }
    let multiple = probe.multiple(device, queue);
    assert_eq!(multiple.len(), plane_pixels * 2);
    for bank in multiple.chunks_exact(plane_pixels) {
        assert!(bank.iter().flatten().all(|v| (0.0..=65_504.0).contains(v)));
        for lane in 0..4 {
            assert!(bank.iter().any(|v| v[lane] > 0.0), "empty wavelength plane");
        }
    }
}
