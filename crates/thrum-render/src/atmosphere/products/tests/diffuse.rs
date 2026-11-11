//! H3 uses the preserved P6 f64 hemisphere referee with contribution-side reads.
use super::{filtered, observer};
use crate::{
    atmosphere::{AtmosphereFrame, gpu::tests::diffuse as reference},
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
};

#[test]
fn metal_h3_lent_diffuse_meets_independent_hemisphere_integral() {
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
        let scene = SyntheticScene::grid(1).unwrap();
        let initial = AtmosphereFrame::new([0.0, 1.0, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
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
        let queries: Vec<_> = (0..14)
            .map(|i| {
                let n = reference::normal(i);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "unit reference normals round once to GPU inputs"
                )]
                [n[0] as f32, n[1] as f32, n[2] as f32, 0.0]
            })
            .collect();
        for (degrees, azimuth) in [
            (90.0_f32, 0.0_f32),
            (45.0, 0.0),
            (45.0, 90.0),
            (0.0, 0.0),
            (-3.0, 0.0),
            (-6.0, 0.0),
            (-9.0, 0.0),
            (-12.0, 0.0),
        ] {
            let (y, r) = degrees.to_radians().sin_cos();
            let (z, x) = azimuth.to_radians().sin_cos();
            let frame = AtmosphereFrame::new([r * x, y, r * z], 1.0, [1.0; 3], 2.0, 1).unwrap();
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Atmosphere(frame),
                    1.0,
                )
                .unwrap();
            filtered::submit(&device, &queue, &mut prepared, &target);
            let surface = prepared.scene_surface(&queue);
            let pixels: Vec<_> =
                observer::texels(&device, &queue, &surface.environment.unwrap().textures[2])
                    .iter()
                    .map(|p| [p[0] / 1024.0, p[1] / 1024.0, p[2] / 1024.0])
                    .collect();
            let coarse =
                reference::integrate(&pixels, f64::from(azimuth).to_radians(), [1.0; 3], 64);
            let fine =
                reference::integrate(&pixels, f64::from(azimuth).to_radians(), [1.0; 3], 128);
            let products = surface.environment_products().unwrap();
            for products in [&products, &filtered::exposure_two(&prepared, &queue)] {
                let actual = observer::sample(
                    &device,
                    &queue,
                    products,
                    "vec4(environment_diffuse(q.xyz),1.0)",
                    &queries,
                    0,
                );
                for c in 0..3 {
                    let mass = fine.mass[c];
                    let coefficient_queries: Vec<_> =
                        (0..9_u8).map(|i| [f32::from(i), 0.0, 0.0, 0.0]).collect();
                    let coefficients = observer::sample(
                        &device,
                        &queue,
                        products,
                        "environment_light.sky_diffuse[u32(q.x)]",
                        &coefficient_queries,
                        0,
                    );
                    for (j, coefficient) in coefficients.iter().enumerate() {
                        assert!(
                            (coarse.coefficients[j][c] - fine.coefficients[j][c]).abs()
                                <= 0.0025 * mass,
                            "H3 unresolved coefficient"
                        );
                        assert!(
                            (f64::from(coefficient[c]) - fine.coefficients[j][c]).abs()
                                <= 0.01 * mass
                                    + 2.0
                                        * (coarse.coefficients[j][c] - fine.coefficients[j][c])
                                            .abs()
                                    + 1e-12,
                            "H3 projection"
                        );
                    }
                    for (i, value) in actual.iter().enumerate() {
                        let refinement = (fine.direct[i][c] - coarse.direct[i][c]).abs();
                        assert!(refinement <= 0.0025 * mass, "H3 unresolved integral");
                        let [x, y, z] = reference::normal(i);
                        let basis = [
                            1.0,
                            x,
                            y,
                            z,
                            x * x - z * z,
                            y * y - z * z,
                            x * y,
                            x * z,
                            y * z,
                        ];
                        let projection_error: f64 = basis
                            .iter()
                            .enumerate()
                            .map(|(j, b)| {
                                b.abs()
                                    * (f64::from(coefficients[j][c]) - fine.coefficients[j][c])
                                        .abs()
                            })
                            .sum();
                        let bound = 3.0 * mass / 32.0 + projection_error + 2.0 * refinement + 1e-12;
                        assert!(
                            (f64::from(value[c]) - fine.direct[i][c]).abs() <= bound,
                            "H3 Sun {degrees} azimuth {azimuth} normal {i} channel {c} actual {} expected {} bound {bound}",
                            value[c],
                            fine.direct[i][c]
                        );
                    }
                }
            }
        }
    });
}
