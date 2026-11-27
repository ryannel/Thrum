//! H5 checks the lent operation against the stored products and records model differences.
use super::{filtered, observer};
use crate::pass_key::AtmosphereStage;
use crate::{
    atmosphere::{AtmosphereFrame, reflection::tests as reference},
    capabilities::{DeviceCapabilities, DeviceProfile, DeviceRestriction},
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
    test_device::{Opened, open},
};
use std::fmt::Write as _;

mod stored;

#[cfg(feature = "creation-counters")]
#[test]
fn metal_facet_reflection_follows_overlay_directions_with_other_light_held() {
    let Some(Opened {
        gpu, device, queue, ..
    }) = open(
        "isolated water facet reflection",
        Some(wgpu::Backend::Metal),
        DeviceProfile::Portable,
        &DeviceRestriction::default(),
    )
    else {
        return;
    };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 140);
    let mut scene = SyntheticScene::grid(1).unwrap();
    scene.sky_overlay = true;
    let frame = AtmosphereFrame::new([0.0, 1.0, 0.0], 90_000.0, [1.0; 3], 2.0, 1).unwrap();
    let mut prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
        &caps,
        &queue,
        &scene,
        FrameLighting::Atmosphere(frame),
        1.0,
    ))
    .unwrap();
    let target = OffscreenTarget::new(&device, &prepared, 8, 8).unwrap();
    filtered::submit(&device, &queue, &mut prepared, &target);
    let surface = prepared.scene_surface(&queue);
    let atmosphere = surface.environment.unwrap();
    let overlay = atmosphere.overlay_target().unwrap();
    queue.write_buffer(
        atmosphere.overlay_classification_witness().unwrap(),
        0,
        bytemuck::bytes_of(&crate::atmosphere::overlay::ChartLighting {
            mode: 1,
            cloudy: 1,
            haze: 1.0,
            classifications: 1,
        }),
    );
    let products = surface.environment_products_with_cloud().unwrap();
    let probe = observer::Probe::new(
        &device,
        &products,
        "let ray = normalize(q.xyz);",
        [
            "vec4(environment_sky_radiance(ray),1.0)",
            "vec4(environment_clear_sky_radiance(ray),1.0)",
            "vec4(environment_diffuse(ray),1.0)",
            "vec4(sun_irradiance(vec3(0.0)),1.0)",
        ],
    );
    let directions = [[1.0, 0.25, 0.0, 0.0], [-1.0, 0.25, 0.0, 0.0]];
    let [before, after] = [false, true].map(|swap| {
        observer::write_field(&queue, &overlay, |_, _, face| match face {
            0 | 1 if (face == 0) != swap => [24_000.0, 80.0, 40.0, 0.1],
            0 | 1 => [40.0, 80.0, 24_000.0, 0.1],
            _ => [0.0, 0.0, 0.0, 1.0],
        });
        // Rebuild only the directional reflection mip. This deliberately
        // leaves the Sun, diffuse, clear chart and exposure products alone.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            atmosphere
                .reflection
                .encode(AtmosphereStage::Reflection(0), &mut pass)
                .unwrap();
        }
        queue.submit([encoder.finish()]);
        probe.sample(&device, &queue, &products, &directions, 0)
    });
    for (index, (old, new)) in before.iter().zip(&after).enumerate() {
        assert_eq!(
            old[1..]
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            new[1..]
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>()
        );
        let hot = 2 * index;
        let cold = 2 - hot;
        assert!(old[0][hot] > 5.0 * old[0][cold], "{old:?}");
        assert!(new[0][cold] > 5.0 * new[0][hot], "{new:?}");
        eprintln!(
            "Metal facet {index}: {:?} -> {:?}; clear, diffuse and direct terms identical",
            old[0], new[0]
        );
    }
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded roughness and reference coordinates round to GPU inputs"
)]
fn metal_h5_lent_reflection_direction_roughness_source_night_and_exposure() {
    pollster::block_on(async {
        let Some(Opened {
            gpu, device, queue, ..
        }) = open(
            "metal_h5_lent_reflection_direction_roughness_source_night_and_exposure",
            Some(wgpu::Backend::Metal),
            DeviceProfile::Portable,
            &DeviceRestriction {
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            },
        )
        else {
            return;
        };
        let adapter = gpu.adapter;
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
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
        let mut radiance_difference = [0.0_f64; 3];
        let mut split_sum_difference = [0.0_f64; 2];
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
            let sky = &atmosphere.textures.sky_view;
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
            for stage in
                std::iter::once(AtmosphereStage::Dfg).chain((0..6).map(AtmosphereStage::Reflection))
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                atmosphere.reflection.encode(stage, &mut pass).unwrap();
            }
            // Stage zero is DFG, followed by all seven filtered cube levels.
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                atmosphere
                    .reflection
                    .encode(AtmosphereStage::Reflection(6), &mut pass)
                    .unwrap();
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
                                        radiance_difference[c] =
                                            radiance_difference[c].max((value - expected).abs());
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
                            split_sum_difference[c] = split_sum_difference[c]
                                .max((f64::from(value[c]) - expected[c]).abs());
                        }
                        assert!(value[0] >= 0.0 && value[1] >= 0.0 && value[0] + value[1] <= 1.001);
                    }
                }
            }
        }
        println!("H5 maximum absolute RGB differences from P7: {radiance_difference:?}");
        println!("H5 maximum absolute split-sum differences from P8: {split_sum_difference:?}");
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
    observer::write_field(queue, &atmosphere.textures.sky_view, |_, _, _| {
        [1024.0, 512.0, 256.0, 0.0]
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for stage in (0..7).map(AtmosphereStage::Reflection) {
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
                exposure: products.exposure.clone(),
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
