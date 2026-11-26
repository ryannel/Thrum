//! Metal proof for the camera-independent point test of a hollows body.
use thrum_water::map_gpu::{self, MapGpu};
use thrum_water_model::{Bed, Body, BodyKind, Hollows, Map, NO_HOLLOW, Profile, Stage};
use wgpu::util::DeviceExt;

const SIZE: [usize; 2] = [13, 7];
const SPACING: f64 = 0.0625;
const EPS: f64 = 0.00004; // WATERS WP4.1's metre tolerance.
const SILLS: [f64; 5] = [0.015, 0.026, 0.03125, 0.036, 0.039];

fn bed() -> Vec<f64> {
    let mut bed = vec![0.09; SIZE[0] * SIZE[1]];
    for z in 1..6 {
        for x in 1..12 {
            let dz = (z as f64 - 3.0).abs();
            let left = 0.009 * (x as f64 - 3.0).abs();
            let right = 0.006 + 0.009 * (x as f64 - 9.0).abs();
            bed[z * SIZE[0] + x] = left.min(right) + 0.009 * dz;
        }
    }
    bed
}

fn map(heights: &[f64]) -> Map {
    let labels = (0..heights.len())
        .map(|index| {
            if heights[index] >= 0.09 {
                NO_HOLLOW
            } else {
                u32::from(index % SIZE[0] >= 6)
            }
        })
        .collect();
    let h = Hollows::new(
        [0.0; 2],
        SPACING,
        SIZE,
        heights.to_vec(),
        labels,
        vec![0.0, 0.006],
    )
    .expect("hollows declaration");
    Map::prepare(
        Bed::new([-1.0; 2], 1.0, [4, 4], vec![2.0; 16]).expect("ordinary bed"),
        vec![Body {
            name: "two hollows".into(),
            kind: BodyKind::Hollows(h),
            level: 0.0,
            fetch: 1.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            gains: [0.0; 4],
        }],
        Profile {
            margin_height: 0.2,
            swash_ratio: 0.7,
        },
    )
    .expect("prepared hollows")
}

fn probe(
    gpu: &thrum_test_support::Opened,
    map: &MapGpu,
    points: &[[f32; 2]],
    shader_map: &str,
) -> Vec<[f32; 4]> {
    let thrum_test_support::Opened { device, queue, .. } = gpu;
    let points_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("hollows probe points"),
        contents: bytemuck::cast_slice(points),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = size_of_val(points) as u64 * 2;
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hollows probe results"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let source = shader_map.to_owned()
        + r"
        @group(0) @binding(0) var<storage, read> points: array<vec2<f32>>;
        @group(0) @binding(1) var<storage, read_write> answers: array<vec4<f32>>;
        @compute @workgroup_size(64) fn probe(@builtin(global_invocation_id) at: vec3<u32>) {
            if at.x >= arrayLength(&points) { return; }
            let sample = map_sample(points[at.x]);
            answers[at.x] = vec4(select(f32(sample.body), -1.0, sample.body == NO_BODY),
                sample.row.water.x, sample.bed, sample.row.water.x - sample.bed);
        }
    ";
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("hollows point test"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("hollows probe"),
        layout: None,
        module: &module,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let view = map.view();
    let mut entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: points_buffer.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: result.as_entire_binding(),
        },
    ];
    entries.extend(map.entries(&view));
    entries.extend(map.hollows_entry());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("hollows probe"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(points.len().div_ceil(64)).expect("point count"),
            1,
            1,
        );
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &result, 0..size);
    bytemuck::cast_slice(&readback.wait(device, queue.submit([encoder.finish()]))).to_vec()
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "probe points are in GPU metres and indices are bounded by the fixture"
)]
fn hollows_match_at_five_fills() {
    let Some(gpu) = thrum_test_support::open(
        "hollows_match_at_five_fills",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let heights = bed();
    let mut map = map(&heights);
    let mut seed = 0x1234_5678u32;
    let mut position = |cells: usize| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (f64::from(seed >> 8) * (cells as f64 * SPACING / 16_777_216.0)) as f32
    };
    let scattered: Vec<_> = (0..4096)
        .map(|_| [position(SIZE[0]), position(SIZE[1])])
        .collect();
    for (fill, left) in SILLS.into_iter().enumerate() {
        // Two levels until both puddles reach the sill and become a single sheet.
        let right = [0.01, 0.018, 0.024, 0.036, 0.039][fill];
        assert!(map.set_levels(0, &[left, right]).unwrap(), "fill {fill}");
        let gpu_map = MapGpu::new(&gpu.device, &gpu.queue, &map).unwrap();
        let mut points = scattered.clone();
        let wet = |x, z| map.sample([x, z]).is_some_and(|sample| sample.depth > 0.0);
        let mut crossings = 0;
        for z in 0..SIZE[1] {
            for x in 0..SIZE[0] {
                let p = [(x as f64 + 0.5) * SPACING, (z as f64 + 0.5) * SPACING];
                for (axis, bound) in SIZE.into_iter().enumerate() {
                    let edge = if axis == 0 { x + 1 } else { z + 1 };
                    if edge >= bound {
                        continue;
                    }
                    let mut neighbour = p;
                    neighbour[axis] += SPACING;
                    if wet(p[0], p[1]) == wet(neighbour[0], neighbour[1]) {
                        continue;
                    }
                    crossings += 1;
                    for delta in [-0.001, 0.001] {
                        let mut near = p;
                        near[axis] = (edge as f64).mul_add(SPACING, delta);
                        points.push(near.map(|n| n as f32));
                    }
                }
            }
        }
        assert!(
            crossings > 10,
            "fill {fill} has {crossings} waterline crossings"
        );
        let source = gpu_map.source(0);
        let answers = probe(&gpu, &gpu_map, &points, &source);
        for (&point, &actual) in points.iter().zip(&answers) {
            let world = point.map(f64::from);
            let expected = map.sample(world);
            let index = (world[1] / SPACING) as usize * SIZE[0] + (world[0] / SPACING) as usize;
            assert!((f64::from(actual[2]) - heights[index]).abs() < EPS);
            assert_eq!(
                actual[0],
                expected.map_or(-1.0, |_| 0.0),
                "fill {fill}, {point:?}"
            );
            if let Some(sample) = expected {
                assert!((f64::from(actual[1]) - sample.row.level).abs() < EPS);
                assert!((f64::from(actual[3]) - sample.depth).abs() < EPS);
            }
        }
        if fill == 2 {
            let one_level = source.replace(
                "rectangle.size_offsets.w + cell.y",
                "rectangle.size_offsets.w",
            );
            let wrong = probe(&gpu, &gpu_map, &points, &one_level);
            let changed = answers.iter().zip(wrong).filter(|(a, b)| {
                a[0].to_bits() != b[0].to_bits() || f64::from((a[1] - b[1]).abs()) >= EPS
            });
            assert!(changed.count() > 100, "one level hid the second hollow");
        }
    }
}

#[test]
fn ordinary_map_keeps_its_shader_and_binding_set() {
    let Some(gpu) = thrum_test_support::open(
        "ordinary_map_keeps_its_shader_and_binding_set",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let map = Map::prepare(
        Bed::new([0.0, 0.0], 1.0, [2, 2], vec![-1.0; 4]).unwrap(),
        vec![Body {
            name: "sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            fetch: 1.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            gains: [1.0; 4],
        }],
        Profile {
            margin_height: 0.2,
            swash_ratio: 0.7,
        },
    )
    .unwrap();
    let map = MapGpu::new(&gpu.device, &gpu.queue, &map).unwrap();
    assert!(map.hollows_entry().is_none());
    assert_eq!(map.source(3), map_gpu::declarations());
}

mod frame_fixture;

#[test]
fn mapped_surface_submits_hollows_through_the_frame_host() {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use thrum_render::runtime::SurfaceOutput;
    let Some(gpu) = thrum_test_support::open(
        "mapped_surface_submits_hollows_through_the_frame_host",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let extent = frame_fixture::QUERY_EXTENT;
    let (mut host, target) = frame_fixture::query_host(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        frame_fixture::Clock::new(false),
        extent,
    );
    let mut map = map(&bed());
    assert!(map.set_levels(0, &[0.03125, 0.024]).unwrap());
    let spectrum = thrum_sea_model::Spectrum::build(&thrum_sea_model::Forcing {
        wind_speed: 0.0,
        wind_direction: 0.0,
        fetch: 10_000.0,
        swells: Vec::new(),
    })
    .unwrap();
    let water = thrum_water::Water::mapped(
        Arc::new(Mutex::new(map)),
        spectrum,
        thrum_water::Profile::default(),
    )
    .unwrap();
    let report = water.report();
    host.set_scene_contribution(Box::new(water)).unwrap();
    assert_eq!(host.world().endpoint(0).tick, 0);
    let frame = host
        .drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &target,
                extent,
            }),
        )
        .unwrap();
    assert!(frame.submitted);
    let report = report.lock().unwrap();
    assert!(
        report
            .resources
            .iter()
            .any(|(name, bytes)| name == "water hollows" && *bytes > 0)
    );
}
