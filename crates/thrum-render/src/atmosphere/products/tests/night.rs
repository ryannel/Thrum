//! The preserved f64 night profile through H2/H3/H4's lent library.
use super::{filtered, observer};
use crate::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView},
};

#[test]
fn metal_lent_night_sky_diffuse_and_finite_aerial_preserve_absolute_units() {
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
        let caps = DeviceCapabilities::capture(&adapter, &device, 140);
        for (height, rays, horizon, lux) in [
            (
                0.0_f32,
                [
                    1.710e-4,
                    2.747_452e-4,
                    2.989_057e-4,
                    2.427_922e-4,
                    2.135_841e-5,
                ],
                [2.135_841e-5, 1.034_311e-4],
                6.709_942e-4,
            ),
            (
                1800.0,
                [
                    1.727_028e-4,
                    2.877_571e-4,
                    3.294_886e-4,
                    2.862_687e-4,
                    2.475_446e-5,
                ],
                [9.189_556e-5, 9.973_511e-5],
                6.912_419e-4,
            ),
            (
                15_000.0,
                [
                    1.770_471e-4,
                    3.294_208e-4,
                    4.422_802e-4,
                    4.742_667e-4,
                    3.760_431e-5,
                ],
                [1.184_168e-4, 1.197_290e-4],
                7.570_950e-4,
            ),
        ] {
            let mut scene = SyntheticScene::grid(1).unwrap();
            scene.view = SyntheticView::perspective(
                [0.0, height, 0.0],
                [1000.0, height, 0.0],
                [0.0, 1.0, 0.0],
                0.25,
                1.0,
                0.1,
                32_000.0,
            )
            .unwrap();
            let frame = AtmosphereFrame::new([0.0, -1.0, 0.0], 0.0, [1.0; 3], height, 1)
                .unwrap()
                .with_nightglow();
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
            filtered::submit(&device, &queue, &mut prepared, &target);
            let products = prepared
                .scene_surface(&queue)
                .environment_products()
                .unwrap();
            let mut queries: Vec<_> = [90.0_f32, 20.0, 10.0, 5.0, -30.0]
                .into_iter()
                .map(|degrees| {
                    let (s, c) = degrees.to_radians().sin_cos();
                    [c, s, 0.0, 0.0]
                })
                .collect();
            let tangent = -(f64::from(height) * (2.0 * 6_371_000.0 + f64::from(height))).sqrt()
                / (6_371_000.0 + f64::from(height));
            #[expect(
                clippy::cast_possible_truncation,
                reason = "bounded horizon direction rounds once to physical GPU inputs"
            )]
            for delta in [-1e-6_f64, 1e-6] {
                let y = tangent + delta;
                queries.push([(1.0 - y * y).sqrt() as f32, y as f32, 0.0, 0.0]);
            }
            for products in [&products, &filtered::exposure_two(&prepared, &queue)] {
                let values = observer::sample(
                    &device,
                    &queue,
                    products,
                    "vec4(environment_sky_radiance(q.xyz),1.0)",
                    &queries,
                    0,
                );
                for (actual, expected) in values.iter().zip(rays.into_iter().chain(horizon)) {
                    for value in &actual[..3] {
                        assert!(
                            (f64::from(*value) - expected).abs() <= 0.10 * expected,
                            "H2 night height {height} actual {actual:?} expected {expected}"
                        );
                    }
                }
                let diffuse = observer::sample(
                    &device,
                    &queue,
                    products,
                    "vec4(environment_diffuse(q.xyz),1.0)",
                    &[[0.0, 1.0, 0.0, 0.0]],
                    0,
                );
                for value in &diffuse[0][..3] {
                    assert!(
                        (f64::from(*value) - lux).abs() <= 0.05 * lux,
                        "H3 night {height} {diffuse:?} vs {lux}"
                    );
                }
                if (height - 1800.0).abs() < 1.0 {
                    let values = observer::sample(
                        &device,
                        &queue,
                        products,
                        "vec4(environment_aerial(vec3(q.x,1800.0,0.0),vec2(0.5)).scatter,1.0)",
                        &[
                            [1000.0, 0.0, 0.0, 0.0],
                            [10_000.0, 0.0, 0.0, 0.0],
                            [30_000.0, 0.0, 0.0, 0.0],
                        ],
                        0,
                    );
                    for (actual, expected) in
                        values
                            .iter()
                            .zip([1.241_665_2e-6, 1.178_059_1e-5, 3.150_029e-5])
                    {
                        for value in &actual[..3] {
                            assert!(
                                (f64::from(*value) - expected).abs() <= 0.10 * expected,
                                "H4 night actual {actual:?} expected {expected}"
                            );
                        }
                    }
                }
            }
        }
    });
}
