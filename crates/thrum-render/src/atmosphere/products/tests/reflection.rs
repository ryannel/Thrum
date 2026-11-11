//! H5 applies P7/P8's independent references to the lent operation.
use super::{filtered, observer};
use crate::{
    atmosphere::{AtmosphereFrame, reflection::tests as reference},
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
};
use std::fmt::Write as _;

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded roughness and reference coordinates round to GPU inputs"
)]
fn metal_h5_lent_reflection_direction_roughness_source_night_and_exposure() {
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
        let frame = AtmosphereFrame::new([1.0, 0.0, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &caps,
            &queue,
            &scene,
            FrameLighting::Atmosphere(frame),
            1.0,
        )
        .await
        .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, 8, 8).unwrap();
        filtered::submit(&device, &queue, &mut prepared, &target);
        let groups = reference::probe_groups();
        for azimuth in [0.0_f64, std::f64::consts::FRAC_PI_2] {
            let frame = AtmosphereFrame::new(
                [azimuth.cos() as f32, 0.0, azimuth.sin() as f32],
                1.0,
                [1.0; 3],
                2.0,
                1,
            )
            .unwrap();
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Atmosphere(frame),
                    1.0,
                )
                .unwrap();
            let surface = prepared.scene_surface(&queue);
            let atmosphere = surface.environment.unwrap();
            let sky = &atmosphere.textures[2];
            let mut bytes = reference::analytic_sky(azimuth);
            // Known absolute night radiance 1/16, separate from unit-Sun RGB.
            for pixel in bytes.as_chunks_mut::<8>().0 {
                pixel[6..8].copy_from_slice(&half::f16::from_f32(4096.0).to_le_bytes());
            }
            queue.write_texture(
                sky.as_image_copy(),
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(192 * 8),
                    rows_per_image: Some(108),
                },
                sky.size(),
            );
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            for stage in 0..=6 {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                atmosphere.reflection.encode(stage, &mut pass).unwrap();
            }
            // Stage zero is DFG, followed by all seven filtered cube levels.
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                atmosphere.reflection.encode(7, &mut pass).unwrap();
            }
            queue.submit([encoder.finish()]);
            for lux in [1.0_f32, 0.0, 2.0] {
                let frame = AtmosphereFrame::new(
                    [azimuth.cos() as f32, 0.0, azimuth.sin() as f32],
                    lux,
                    [1.0; 3],
                    2.0,
                    1,
                )
                .unwrap();
                prepared
                    .update_frame(
                        &queue,
                        &mut target,
                        &scene,
                        FrameLighting::Atmosphere(frame),
                        1.0,
                    )
                    .unwrap();
                let products = prepared
                    .scene_surface(&queue)
                    .environment_products()
                    .unwrap();
                for products in [&products, &filtered::exposure_two(&prepared, &queue)] {
                    for level in 0..7 {
                        let roughness = 0.045 + 0.955 * level as f32 / 6.0;
                        let queries: Vec<_> = groups
                            .iter()
                            .flatten()
                            .map(|d| [d[0], d[1], d[2], roughness])
                            .collect();
                        let actual = observer::sample(
                            &device,
                            &queue,
                            products,
                            "vec4(environment_reflection(q.xyz,q.w,0.5).radiance,1.0)",
                            &queries,
                            0,
                        );
                        let mut index = 0;
                        for (group_index, directions) in groups.iter().enumerate() {
                            let first = actual[index];
                            for direction in directions {
                                let length = direction
                                    .iter()
                                    .map(|v| f64::from(*v).powi(2))
                                    .sum::<f64>()
                                    .sqrt();
                                for c in 0..3 {
                                    let value = f64::from(actual[index][c]);
                                    if level == 0 || (level == 6 && group_index < 6) {
                                        let scale = if level == 0 { 0.5 } else { 1.0 / 3.0 };
                                        let expected = f64::from(lux)
                                            * (1.5 + scale * f64::from(direction[c]) / length)
                                            + 0.0625;
                                        assert!(
                                            (value - expected).abs()
                                                <= f64::from(lux) * (1.0 / 1024.0 + 0.019)
                                                    + 0.0625 / 1024.0,
                                            "H5 azimuth {azimuth} lux {lux} level {level} direction {direction:?} channel {c} actual {value} expected {expected}"
                                        );
                                    }
                                    assert!(
                                        (value - f64::from(first[c])).abs()
                                            < f64::from(lux) * 0.01 + 1e-6,
                                        "H5 seam"
                                    );
                                    if lux == 0.0 {
                                        assert!(
                                            (value - 0.0625).abs() <= 0.0625 / 1024.0,
                                            "H5 absolute night"
                                        );
                                    }
                                }
                                index += 1;
                            }
                        }
                    }
                    let nodes = [
                        (0, 32),
                        (8, 48),
                        (32, 32),
                        (32, 64),
                        (64, 32),
                        (64, 64),
                        (0, 64),
                    ];
                    let queries: Vec<_> = nodes
                        .iter()
                        .map(|&(x, y)| [x as f32 / 64.0, 0.045 + 0.955 * y as f32 / 64.0, 0.0, 0.0])
                        .collect();
                    let actual = observer::sample(
                        &device,
                        &queue,
                        products,
                        "vec4(environment_reflection(vec3(0.0,1.0,0.0),q.y,q.x).split_sum,0.0,1.0)",
                        &queries,
                        0,
                    );
                    for ((x, y), value) in nodes.into_iter().zip(actual) {
                        let expected = reference::dfg_reference(
                            f64::from(x) / 64.0,
                            0.045 + 0.955 * f64::from(y) / 64.0,
                            256,
                        );
                        for c in 0..2 {
                            assert!(
                                (f64::from(value[c]) - expected[c]).abs() < 0.006,
                                "H5 DFG {x} {y}"
                            );
                        }
                        assert!(value[0] >= 0.0 && value[1] >= 0.0 && value[0] + value[1] <= 1.001);
                    }
                }
            }
        }
        furnace(&device, &queue, &mut prepared, &mut target, &scene);
    });
}

fn furnace(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
) {
    let surface = prepared.scene_surface(queue);
    let atmosphere = surface.environment.expect("valid furnace fixture");
    observer::write_field(queue, &atmosphere.textures[2], |_, _, _| {
        [1024.0, 512.0, 256.0, 0.0]
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for stage in 1..=7 {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        atmosphere
            .reflection
            .encode(stage, &mut pass)
            .expect("valid furnace fixture");
    }
    queue.submit([encoder.finish()]);
    let mut queries = Vec::new();
    for base in [0.0, 0.18, 0.65, 1.0] {
        for metal in [0.0, 1.0] {
            for roughness in [0.045, 0.5, 1.0] {
                for cosine in [-0.5, 0.0, 0.01, 0.5, 1.0] {
                    queries.push([base, metal, roughness, cosine]);
                }
            }
        }
    }
    let mut baseline: Vec<[f32; 4]> = Vec::new();
    for lux in [1.0_f32, 0.0, 2.0] {
        let frame = AtmosphereFrame::new([1.0, 0.0, 0.0], lux, [1.0; 3], 2.0, 1)
            .expect("valid furnace fixture");
        prepared
            .update_frame(queue, target, scene, FrameLighting::Atmosphere(frame), 1.0)
            .expect("valid furnace fixture");
        let products = prepared
            .scene_surface(queue)
            .environment_products()
            .expect("valid furnace fixture");
        for products in [&products, &filtered::exposure_two(prepared, queue)] {
            let mut consumer = crate::atmosphere::EnvironmentProducts {
                layout: products.layout.clone(),
                shader_source: products.shader_source.clone(),
                groups: products.groups.clone(),
            };
            write!(
                consumer.shader_source,
                r"
fn furnace(q:vec4<f32>)->vec4<f32>{{
    if q.w<=0.0 {{return vec4(0.0,0.0,0.0,1.0);}}
    let environment=environment_reflection(vec3(0.0,1.0,0.0),q.z,q.w);
    let ess=mix(vec3(0.04),vec3(q.x),q.y)*environment.split_sum.x+vec3(environment.split_sum.y);
    let diffuse=(1.0-q.y)*q.x*(vec3(1.0)-ess)*vec3(1.0,0.5,0.25)*{lux};
    return vec4(diffuse+environment.radiance*ess,1.0);
}}
"
            )
            .expect("append furnace consumer");
            let actual = observer::sample(device, queue, &consumer, "furnace(q)", &queries, 0);
            for (i, (value, q)) in actual.iter().zip(&queries).enumerate() {
                for (c, source) in [1.0, 0.5, 0.25].into_iter().enumerate() {
                    assert!(
                        value[c].is_finite()
                            && value[c] >= 0.0
                            && value[c] <= lux * source * 1.002 + 1e-6,
                        "H5 furnace {q:?}: {value:?}"
                    );
                    if q[3] <= 0.0 || lux.to_bits() == 0 {
                        assert_eq!(value[c], 0.0);
                    }
                    if lux.to_bits() == 2.0_f32.to_bits() {
                        assert!(
                            (value[c] - 2.0 * baseline[i][c]).abs() < 2e-6,
                            "H5 source scaling"
                        );
                    }
                }
            }
            if lux.to_bits() == 1.0_f32.to_bits() {
                baseline = actual;
            }
        }
    }
}
