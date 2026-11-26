use thrum_water::map_gpu::MapGpu;
use thrum_water_model::{
    BRINK, Bed, Body, BodyKind, Chart, ChartSamples, End, FlowSample, Gauge, Map, Profile, River,
    Stage,
};
use wgpu::util::DeviceExt;

const EPS_DEPTH: f64 = 0.000_04;
const EPS_CURRENT: f64 = 0.000_004;

const PROBE: &str = r"
@group(0) @binding(0) var<storage, read> points: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> answers: array<vec4<f32>>;
@compute @workgroup_size(32) fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&points) { return; }
    let a = map_sample(points[id.x]);
    let depth = a.row.water.x - a.bed;
    answers[id.x] = vec4(select(-1.0, f32(a.body), a.body != NO_BODY && depth > 0.0),
        depth, a.row.water.yz);
}
";

fn sample(
    gpu: &thrum_test_support::Opened,
    cpu: &Map,
    points: &[[f32; 2]],
    source: impl FnOnce(String) -> String,
) -> Vec<[f32; 4]> {
    let (device, queue) = (&gpu.device, &gpu.queue);
    let map = MapGpu::new(device, queue, cpu).expect("river map products");
    let view = map.view();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("river probe points"),
        contents: bytemuck::cast_slice(points),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (size_of::<[f32; 4]>() * points.len()) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("river probe answers"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("river map probe"),
        source: wgpu::ShaderSource::Wgsl((source(map.source(0)) + PROBE).into()),
    });
    let pipeline = thrum_render::runtime::DeviceSite::untracked(device.clone()).compute(
        &wgpu::ComputePipelineDescriptor {
            label: Some("river map probe"),
            layout: None,
            module: &shader,
            entry_point: Some("probe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        },
    );
    let mut entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: input.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: output.as_entire_binding(),
        },
    ];
    entries.extend(map.entries(&view));
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("river map bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(points.len().div_ceil(32)).expect("points"),
            1,
            1,
        );
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &output, 0..size);
    let submission = queue.submit([encoder.finish()]);
    bytemuck::cast_slice(&readback.wait(device, submission)).to_vec()
}

fn compare(gpu: &thrum_test_support::Opened, map: &Map, points: &[[f32; 2]]) {
    for (&point, answer) in points
        .iter()
        .zip(sample(gpu, map, points, std::convert::identity))
    {
        let expected = map.sample(point.map(f64::from)).filter(|a| a.depth > 0.0);
        assert_eq!(
            answer[0],
            expected.map_or(-1.0, |a| a.body as f32),
            "wet owner at {point:?}"
        );
        if let Some(expected) = expected {
            assert!(
                (f64::from(answer[1]) - expected.depth).abs() < EPS_DEPTH,
                "depth at {point:?}: GPU {} vs model {}",
                answer[1],
                expected.depth
            );
            for (got, want) in answer[2..].iter().copied().zip(expected.row.current) {
                assert!(
                    (f64::from(got) - want).abs() < EPS_CURRENT,
                    "current at {point:?}: GPU {got} vs model {want}"
                );
            }
        }
    }
}

fn fixture() -> Map {
    const SIDE: usize = 12;
    const CELLS: usize = SIDE * SIDE;
    let cell = |x, z| z * SIDE + x;
    let mut heights = vec![8.0; CELLS];
    for x in 3..=8 {
        heights[cell(x, 3)] = 3.0;
    }
    // The ridge crosses above the channel's low stage. Behind it is a hollow
    // deeper than the low stage that takes the ridge's wetting gauge.
    heights[cell(5, 4)] = 3.6;
    heights[cell(5, 5)] = 2.9;
    // The upper-only lip drops into a reach which is entirely dry at low stage.
    heights[cell(8, 4)] = 3.5;
    for z in 5..=8 {
        heights[cell(8, z)] = 0.5;
    }
    let dry = FlowSample {
        levels: heights.iter().map(|&h| h - 0.05).collect(),
        currents: vec![[0.0; 2]; CELLS],
        loss: vec![0.0; CELLS],
        gains: vec![[0.0; 2]; CELLS],
    };
    let mut low = dry.clone();
    let mut high = dry.clone();
    let mut wetting = vec![None; CELLS];
    for x in 3..=8 {
        let i = cell(x, 3);
        low.levels[i] = 3.25;
        low.currents[i] = [0.4, 0.0];
        high.levels[i] = 4.0;
        high.currents[i] = [0.8, 0.1];
    }
    for (x, z, gate) in [(5, 4, 0.47), (5, 5, 0.47), (8, 4, 0.34)] {
        high.levels[cell(x, z)] = 4.0;
        wetting[cell(x, z)] = Some(gate);
    }
    let mut lower = dry.clone();
    let mut lower_wetting = vec![None; CELLS];
    for z in 5..=8 {
        lower.levels[cell(8, z)] = 1.1;
        lower.currents[cell(8, z)] = [0.0, 0.6];
        lower_wetting[cell(8, z)] = Some(0.34);
    }
    let make_body = |name: &str, sample: &FlowSample, footprint: Vec<bool>, ends: Vec<End>| Body {
        name: name.into(),
        kind: BodyKind::Chart(Chart {
            levels: sample.levels.clone(),
            currents: sample.currents.clone(),
            footprint,
            ends,
        }),
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 100.0,
        gains: [0.0; 4],
    };
    let footprint = |points: &[(usize, usize)]| {
        let mut cells = vec![false; CELLS];
        for &(x, z) in points {
            cells[cell(x, z)] = true;
        }
        cells
    };
    let mut main_points: Vec<_> = (3..=8).map(|x| (x, 3)).collect();
    main_points.extend([(5, 4), (5, 5), (8, 4)]);
    let bodies = vec![
        make_body(
            "Ridge and lip",
            &low,
            footprint(&main_points),
            vec![End {
                body: BRINK,
                segment: [[7.5, 4.0], [8.5, 4.0]],
                length: 0.5,
                mouth: false,
            }],
        ),
        make_body(
            "Fall receiver",
            &dry,
            footprint(&[(8, 5), (8, 6), (8, 7), (8, 8)]),
            Vec::new(),
        ),
    ];
    let mut map = Map::prepare(
        Bed::new([0.0; 2], 1.0, [SIDE; 2], heights).expect("fixture bed"),
        bodies,
        Profile {
            margin_height: 0.1,
            swash_ratio: 0.7,
        },
    )
    .expect("fixture map");
    map.add_river(River {
        name: "Test river".into(),
        gauges: vec![0.0, 1.0],
        gauge: Gauge::Stage(Stage {
            min: 0.0,
            max: 1.0,
            value: 0.0,
        }),
        charts: vec![
            ChartSamples {
                body: 0,
                samples: vec![low, high],
                wetting: vec![wetting],
            },
            ChartSamples {
                body: 1,
                samples: vec![dry, lower],
                wetting: vec![lower_wetting],
            },
        ],
    })
    .expect("sampled river");
    map
}

#[test]
fn metal_samples_the_ridge_and_the_dry_reach_at_their_wetting_gauges() {
    let Some(gpu) = thrum_test_support::open(
        "river sample mixing",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mut map = fixture();
    let points = [
        [4.5, 3.0],
        [5.0, 4.0],
        [5.0, 5.0],
        [8.0, 4.0],
        [8.0, 5.0],
        [8.0, 6.0],
        [8.0, 7.0],
        [8.0, 8.0],
    ];
    for gauge in [0.0, 0.33, 0.35, 0.46, 0.48, 0.73, 1.0] {
        map.set_gauge(0, gauge).expect("fixture stage");
        let wet = |point: [f32; 2]| {
            map.sample(point.map(f64::from))
                .is_some_and(|a| a.depth > 0.0)
        };
        assert!(wet(points[0]));
        assert_eq!(wet(points[2]), gauge > 0.47, "hollow");
        assert!(
            points[3..]
                .iter()
                .all(|&point| wet(point) == (gauge > 0.34)),
            "lip and reach"
        );
        compare(&gpu, &map, &points);
    }
    // Both fields must be witnesses for the two shader faults, independently.
    map.set_gauge(0, 0.33).expect("both features below wetting");
    let without_wetting = sample(&gpu, &map, &[points[2], points[5]], |source| {
        assert!(source.contains("if b.w < 0.0"));
        source.replace("if b.w < 0.0", "if b.w <= 1.0")
    });
    assert!(
        without_wetting
            .iter()
            .all(|a| a[0] >= 0.0 && f64::from(a[1]) > EPS_DEPTH)
    );
    map.set_gauge(0, 0.73).expect("both features above wetting");
    let frozen = sample(&gpu, &map, &[points[0], points[2], points[5]], |source| {
        let select_layer = "let upper = i32(body.joins[0].w);";
        assert!(source.contains(select_layer));
        source.replace(select_layer, "let upper = 0i;")
    });
    let expected = map
        .sample(points[0].map(f64::from))
        .expect("running chart")
        .depth;
    assert!((f64::from(frozen[0][1]) - expected).abs() > EPS_DEPTH);
    assert!(frozen[1..].iter().all(|a| a[0] < 0.0));
    println!(
        "Metal mutation witnesses (hollow, fall reach): interpolation without wetting={without_wetting:?}, frozen first sample={:?}",
        &frozen[1..]
    );
}
