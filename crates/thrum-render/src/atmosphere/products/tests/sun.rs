//! H1's f64 quadrature is independent of the chart and the forward output.
use crate::{
    atmosphere::{AtmosphereFrame, spectrum::EARTH},
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView},
};

fn transmission(point: [f64; 3], sun: [f64; 3]) -> [f64; 3] {
    const R: f64 = 6_371_000.0;
    const TOP: f64 = R + 100_000.0;
    let radial = [point[0], point[1] + R, point[2]];
    let radius = radial.iter().map(|x| x * x).sum::<f64>().sqrt();
    let b: f64 = radial.into_iter().zip(sun).map(|(x, y)| x * y).sum();
    if b < 0.0 && radius * radius - b * b < R * R {
        return [0.0; 3];
    }
    let discriminant = b * b + TOP * TOP - radius * radius;
    if discriminant <= 0.0 || (radius >= TOP && b >= 0.0) {
        return [1.0; 3];
    }
    let start = if radius > TOP {
        -b - discriminant.sqrt()
    } else {
        0.0
    };
    let end = -b + discriminant.sqrt();
    let dt = (end - start) / 8192.0;
    let mut column = [0.0; 3];
    for i in 0..8192 {
        let distance = start + (f64::from(i) + 0.5) * dt;
        let h = ((radial[0] + sun[0] * distance).powi(2)
            + (radial[1] + sun[1] * distance).powi(2)
            + (radial[2] + sun[2] * distance).powi(2))
        .sqrt()
            - R;
        column[0] += (-h / 8000.0).exp() * dt;
        column[1] += (-h / 1200.0).exp() * dt;
        column[2] += (1.0 - (h - 25_000.0).abs() / 15_000.0).max(0.0) * dt;
    }
    let spectral: [f64; 8] = std::array::from_fn(|i| {
        (-(f64::from(EARTH.rayleigh[i]) * column[0]
            + f64::from(EARTH.mie_extinction) * column[1]
            + f64::from(EARTH.ozone[i]) * column[2]))
            .exp()
    });
    let rgb = EARTH.rgb.map(|weights| {
        weights
            .into_iter()
            .zip(spectral)
            .map(|(w, x)| f64::from(w) * x)
            .sum::<f64>()
    });
    let low = rgb.into_iter().fold(f64::INFINITY, f64::min);
    if low >= 0.0 {
        return rgb;
    }
    let luminance = (0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]).max(0.0);
    rgb.map(|x| (luminance + (x - luminance) * luminance / (luminance - low)).max(0.0))
}

#[test]
fn metal_h1_lent_sun_meets_low_altitude_f64_quadrature() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let caps = DeviceCapabilities::capture(&adapter, &device, 130);
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = SyntheticView::perspective(
            [0.0, 245.0, 16.0],
            [0.0, 245.0, -100.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            32_000.0,
        )
        .unwrap();
        let initial = AtmosphereFrame::new([1.0, 0.0, 0.0], 1.0, [1.0; 3], 245.0, 1).unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Atmosphere(initial),
            1.0,
        )
        .await
        .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, 8, 8).unwrap();
        let products = prepared
            .scene_surface(&queue)
            .environment_products()
            .unwrap();
        for degrees in [0.0_f32, 0.5, 1.0, 2.0, 5.0] {
            let (y, x) = degrees.to_radians().sin_cos();
            let frame = AtmosphereFrame::new([x, y, 0.0], 1.0, [1.0; 3], 245.0, 1).unwrap();
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Atmosphere(frame),
                    1.0,
                )
                .unwrap();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared.encode(&mut encoder, 130, &target, None).unwrap();
            queue.submit([encoder.finish()]);
            prepared.commit_frame();
            let heights = [0.0_f32, 0.1, 2.0, 6.15];
            let queries = heights.map(|height| [0.0, height, 16.0, 0.0]);
            let results = super::observer::sample(
                &device,
                &queue,
                &products,
                "vec4(sun_irradiance(q.xyz),1.0)",
                &queries,
                prepared.contribution_frame().exposure_side,
            );
            for ((height, result), query) in heights.into_iter().zip(results).zip(queries) {
                let expected = transmission(
                    [0.0, f64::from(height), 0.0],
                    [f64::from(x), f64::from(y), 0.0],
                );
                for channel in 0..3 {
                    let error = (f64::from(result[channel]) - expected[channel]).abs()
                        / expected[channel].max(0.001);
                    assert!(
                        error <= [0.006, 0.009, 0.016][channel],
                        "H1 height {height} sun {degrees} query {query:?} channel {channel}, GPU {} f64 {} relative error {error}",
                        result[channel],
                        expected[channel]
                    );
                }
            }
        }
    });
}

#[test]
fn metal_h1_lent_receiver_matrix_at_two_camera_origins() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let caps = DeviceCapabilities::capture(&adapter, &device, 130);
        let near = [
            [0.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 245.0, 0.0],
            [0.0, 1500.0, 0.0],
            [0.0, 5000.0, 0.0],
            [20_000.0, 245.0, 0.0],
            [-20_000.0, 245.0, 0.0],
        ];
        let upper = [
            [0.0, 100_000.0, 0.0],
            [0.0, 120_000.0, 0.0],
            [0.0, 130_000.0, 0.0],
        ];
        for (altitude, points, elevations) in [
            (
                245.0,
                near.as_slice(),
                [-2.0_f32, -0.6, 0.0, 1.0, 30.0].as_slice(),
            ),
            (
                100_000.0,
                upper.as_slice(),
                [-13.0_f32, -10.07, -1.0, 5.0].as_slice(),
            ),
        ] {
            for eye in [[0.0, 245.0, 16.0], [8132.0, 123.0, -3211.0]] {
                let mut scene = SyntheticScene::grid(1).unwrap();
                scene.view = SyntheticView::perspective(
                    eye,
                    [eye[0], eye[1], eye[2] - 100.0],
                    [0.0, 1.0, 0.0],
                    1.0,
                    1.0,
                    0.1,
                    32_000.0,
                )
                .unwrap();
                for &degrees in elevations {
                    let (y, x) = degrees.to_radians().sin_cos();
                    let frame = AtmosphereFrame::new([x, y, 0.0], 117_000.0, [1.0; 3], altitude, 1)
                        .unwrap();
                    let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                        &caps,
                        &queue,
                        &scene,
                        FrameLighting::Atmosphere(frame),
                        1.0,
                    )
                    .await
                    .unwrap();
                    let target = OffscreenTarget::new(&device, &prepared, 8, 8).unwrap();
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    prepared.encode(&mut encoder, 130, &target, None).unwrap();
                    queue.submit([encoder.finish()]);
                    prepared.commit_frame();
                    let products = prepared
                        .scene_surface(&queue)
                        .environment_products()
                        .unwrap();
                    let queries: Vec<_> = points
                        .iter()
                        .map(|p| [eye[0] + p[0], eye[1] + p[1] - altitude, eye[2] + p[2], 0.0])
                        .collect();
                    for products in [&products, &super::filtered::exposure_two(&prepared, &queue)] {
                        let results = super::observer::sample(
                            &device,
                            &queue,
                            products,
                            "vec4(sun_irradiance(q.xyz),1.0)",
                            &queries,
                            0,
                        );
                        for (point, result) in points.iter().zip(results) {
                            let expected = transmission(
                                point.map(f64::from),
                                [f64::from(x), f64::from(y), 0.0],
                            );
                            for i in 0..3 {
                                let actual = f64::from(result[i]) / (117_000.0 * 1.0);
                                assert!(
                                    (actual - expected[i]).abs() <= 0.05 * expected[i] + 0.005,
                                    "H1 receiver {point:?} sun {degrees} channel {i} actual {actual} expected {}",
                                    expected[i]
                                );
                                if expected.iter().all(|v| v.to_bits() == 0) {
                                    assert_eq!(actual, 0.0);
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}
