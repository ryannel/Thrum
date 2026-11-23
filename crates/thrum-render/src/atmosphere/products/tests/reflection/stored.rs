//! Independent H5 checks of the producer's actual lent textures.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "nonnegative table coordinates fit in one texture and fixture inputs round to GPU floats"
)]

use super::{filtered, observer, reference};
use crate::{
    atmosphere::{AtmosphereFrame, EnvironmentProducts},
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
};

const RADIANCE: &str =
    "vec4(environment_reflection(q.xyz,q.w,0.5).radiance,environment_storage_scale())";
const PAIR: &str = "vec4(environment_reflection(vec3(0.0,1.0,0.0),q.y,q.x).split_sum,0.0,environment_storage_scale())";
const NODES: [(usize, usize); 7] = [
    (0, 32),
    (8, 48),
    (32, 32),
    (32, 64),
    (64, 32),
    (64, 64),
    (0, 64),
];
const DIRECTIONS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

fn fixture(
    gpu: &thrum_test_support::Gpu,
) -> (
    wgpu::Device,
    wgpu::Queue,
    SyntheticScene,
    PreparedSynthetic,
    OffscreenTarget,
) {
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    pollster::block_on(async {
        let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 140);
        let scene = SyntheticScene::grid(1).expect("unit grid fixture");
        let mut prepared =
            PreparedSynthetic::prepare_with_capabilities(&caps, &queue, &scene, lighting(1.0), 1.0)
                .await
                .expect("prepared atmosphere fixture");
        let target = OffscreenTarget::new(&device, &prepared, 8, 8).expect("offscreen fixture");
        filtered::submit(&device, &queue, &mut prepared, &target);
        (device, queue, scene, prepared, target)
    })
}

fn lighting(lux: f32) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new([1.0, 0.0, 0.0], lux, [1.0; 3], 2.0, 1)
            .expect("valid physical Sun fixture"),
    )
}

fn bound(values: &[f64], stored_rounding: f64) -> f64 {
    let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (maximum - minimum) / 32.0 + 1e-5 * maximum + stored_rounding
}

fn within<const N: usize>(actual: [f32; 4], expected: [f64; N], limits: [f64; N]) -> bool {
    (0..N).all(|c| (f64::from(actual[c]) - expected[c]).abs() <= limits[c])
}

fn table_query(x: f64, y: f64) -> [f32; 4] {
    [
        (x / 64.0) as f32,
        (0.045 + 0.955 * y / 64.0) as f32,
        0.0,
        0.0,
    ]
}

#[test]
fn metal_split_sum_reads_stored_table() {
    let Some(gpu) = thrum_test_support::gpu("H5 stored table", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (device, queue, _, prepared, _) = fixture(&gpu);
    let surface = prepared.scene_surface(&queue);
    let table = &surface.environment.as_ref().unwrap().reflection.dfg;
    let texels = reference::read_texture(&device, &queue, &reference::reader(&device), table, 0);
    assert_eq!(table.width(), 65);
    assert_eq!(table.height(), 65);
    let texel = |x: usize, y: usize, c: usize| f64::from(texels[y * 65 + x][c]);
    let mut cases: Vec<_> = NODES
        .into_iter()
        .map(|(x, y)| (table_query(x as f64, y as f64), x as f64, y as f64))
        .collect();
    cases.push(([0.5, 0.0, 0.0, 0.0], 32.0, 0.0));
    for (x, y) in [(4, 12), (8, 36), (48, 36), (36, 56)] {
        for fraction in [0.25, 0.5, 0.75] {
            let x = f64::from(x) + fraction;
            let y = f64::from(y) + fraction;
            cases.push((table_query(x, y), x, y));
        }
    }
    let queries: Vec<_> = cases.iter().map(|case| case.0).collect();
    for exposure in [1.0, 2.0] {
        let products = filtered::with_exposure(&prepared, &queue, exposure);
        let actual = observer::sample(&device, &queue, &products, PAIR, &queries, 0);
        assert_eq!(actual[0][3].to_bits(), exposure.to_bits());
        let mut wrong_queries = Vec::new();
        let mut comparisons = Vec::new();
        for ((query, x, y), value) in cases.iter().copied().zip(actual) {
            let (col, row) = (x as usize, y as usize);
            let (right, down) = (x.ceil() as usize, y.ceil() as usize);
            let (u, v) = (x.fract(), y.fract());
            let samples = |c| {
                [
                    texel(col, row, c),
                    texel(right, row, c),
                    texel(col, down, c),
                    texel(right, down, c),
                ]
            };
            let expected = std::array::from_fn(|c| {
                let [a, b, d, e] = samples(c);
                (a * (1.0 - u) + b * u) * (1.0 - v) + (d * (1.0 - u) + e * u) * v
            });
            let limits = std::array::from_fn(|c| {
                let values = samples(c);
                bound(
                    &values,
                    filtered::stored_step(values.into_iter().fold(0.0, f64::max), 1.0),
                )
            });
            assert!(
                within::<2>(value, expected, limits),
                "H5 table {query:?}: {value:?} vs {expected:?} ± {limits:?}"
            );
            assert!(value[0] >= 0.0 && value[1] >= 0.0 && value[0] + value[1] <= 1.001);
            if col != right && row != down {
                wrong_queries.extend([
                    table_query(x, y.round()),
                    table_query(x.round(), y),
                    [query[1], query[0], 0.0, 0.0],
                ]);
                comparisons.extend([(query, expected, limits); 3]);
            }
        }
        let wrong = observer::sample(&device, &queue, &products, PAIR, &wrong_queries, 0);
        for (index, (value, (query, expected, limits))) in
            wrong.into_iter().zip(comparisons).enumerate()
        {
            assert!(
                !within(value, expected, limits),
                "H5 table wrong lookup {} survived at {query:?}",
                index % 3
            );
        }
    }
    println!("H5 stored table: 20 right and 36 wrong lookups per exposure");
}

const CUBE_WRITER: &str = r"
@group(0) @binding(0) var cube: texture_storage_2d_array<rgba16float, write>;
override level: f32;
@compute @workgroup_size(8,8)
fn store(@builtin(global_invocation_id) texel: vec3<u32>) {
    if any(texel.xy >= textureDimensions(cube)) { return; }
    let face = f32(texel.z);
    textureStore(cube, texel.xy, texel.z,
        vec4(1024.0 * vec3(1.0 + face/8.0, 2.0 + level/8.0, 3.0 + (face+level)/16.0), 4096.0));
}";

fn write_cube(device: &wgpu::Device, queue: &wgpu::Queue, cube: &wgpu::Texture) {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("stored cube probe writer"),
        source: wgpu::ShaderSource::Wgsl(CUBE_WRITER.into()),
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for level in 0..cube.mip_level_count() {
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("stored cube probe writer"),
            layout: None,
            module: &module,
            entry_point: Some("store"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[("level", f64::from(level))],
                ..Default::default()
            },
            cache: None,
        });
        let view = cube.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            base_mip_level: level,
            mip_level_count: Some(1),
            ..Default::default()
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        let groups = (cube.width() >> level).div_ceil(8);
        pass.dispatch_workgroups(groups, groups, 6);
    }
    queue.submit([encoder.finish()]);
}

fn cube_query(direction: [f32; 3], level: f64) -> [f32; 4] {
    let [x, y, z] = direction;
    [
        x,
        y,
        z,
        (0.045 + 0.955 * (level + 1.0 / 1024.0).min(6.0) / 6.0) as f32,
    ]
}

fn check_cube(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    products: &EnvironmentProducts,
    lux: f64,
    exposure: f32,
) {
    let mut queries = Vec::new();
    let mut comparisons = Vec::new();
    for (face, direction) in DIRECTIONS.into_iter().enumerate() {
        for half_level in 0..=12 {
            let level = f64::from(half_level) / 2.0;
            let biased = (level + 1.0 / 1024.0).min(6.0);
            let colour = |mip: f64| {
                [
                    1.0 + face as f64 / 8.0,
                    2.0 + mip / 8.0,
                    3.0 + (face as f64 + mip) / 16.0,
                ]
            };
            let low = colour(biased.floor());
            let high = colour(biased.ceil());
            let expected = std::array::from_fn(|c| {
                lux * (low[c] * (1.0 - biased.fract()) + high[c] * biased.fract()) + 0.0625
            });
            let midpoint = half_level % 2 != 0;
            let ends = if midpoint {
                [level.floor(), level.ceil()]
            } else {
                [(level - 1.0).max(0.0), (level + 1.0).min(6.0)]
            };
            let limits = std::array::from_fn(|c| {
                let values = ends.map(|mip| colour(mip)[c]);
                bound(
                    &values.map(|value| lux * value + 0.0625),
                    lux * filtered::stored_step(values[1], 1024.0)
                        + filtered::stored_step(0.0625, 65536.0),
                )
            });
            let mut probes = vec![cube_query(direction, level)];
            if lux > 0.0 {
                let [x, y, z] = direction;
                probes.extend([
                    cube_query([-x, -y, -z], level),
                    cube_query([y, z, x], level),
                ]);
                if midpoint {
                    probes.extend(ends.map(|mip| cube_query(direction, mip)));
                } else {
                    probes.push(cube_query(
                        direction,
                        if half_level == 12 { 5.0 } else { level + 1.0 },
                    ));
                }
            }
            for (index, query) in probes.into_iter().enumerate() {
                queries.push(query);
                comparisons.push((index == 0, expected, limits));
            }
        }
    }
    let actual = observer::sample(device, queue, products, RADIANCE, &queries, 0);
    assert_eq!(actual[0][3].to_bits(), exposure.to_bits());
    for ((query, value), (right, expected, limits)) in
        queries.into_iter().zip(actual).zip(comparisons)
    {
        assert_eq!(
            within::<3>(value, expected, limits),
            right,
            "H5 cube Sun {lux}, query {query:?}, correct lookup {right}: {value:?} vs {expected:?} ± {limits:?}"
        );
    }
}

#[test]
fn metal_reflection_reads_stored_cube() {
    let Some(gpu) = thrum_test_support::gpu("H5 stored cube", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (device, queue, scene, mut prepared, mut target) = fixture(&gpu);
    let surface = prepared.scene_surface(&queue);
    write_cube(
        &device,
        &queue,
        &surface.environment.unwrap().reflection.cube,
    );
    for lux in [1.0, 0.0, 2.0] {
        prepared
            .update_frame(&queue, &mut target, &scene, lighting(lux), 1.0)
            .unwrap();
        for exposure in [1.0, 2.0] {
            let products = filtered::with_exposure(&prepared, &queue, exposure);
            check_cube(&device, &queue, &products, f64::from(lux), exposure);
        }
    }
    println!(
        "H5 stored cube: 78 right lookups per Sun and exposure, 270 wrong per nonzero Sun and exposure"
    );
}
