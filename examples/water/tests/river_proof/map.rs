use thrum_water::map_gpu::MapGpu;
use thrum_water_model::{
    BRINK, Bed, Body, BodyKind, Chart, ChartSamples, End, FlowSample, Gauge, Map, Profile, River,
    Stage,
};
use wgpu::util::DeviceExt;

const EPS_DEPTH: f64 = 0.000_04;
const EPS_CURRENT: f64 = 0.000_004;

pub(super) const REACHES: [(usize, usize); 3] = [(3, 1), (4, 1), (5, 0)];

/// Each reach's prepared bounds include its shore margin. Search every
/// longitudinal section for its deepest owned column, including the heads
/// and tails, so a narrow or laterally moving wet path still gets sampled.
pub(super) fn transect_points(map: &Map) -> Vec<(usize, [f64; 2])> {
    let mut points = Vec::new();
    for (body, axis) in REACHES {
        let side = 1 - axis;
        let [low, high] = map.surface_bounds(body).expect("finite chart bounds");
        let along = std::iter::successors(Some(low[axis] + 0.31), |&p| Some(p + 1.5))
            .take_while(|&p| p < high[axis]);
        let across = std::iter::successors(Some(low[side] + 0.17), |&p| Some(p + 0.5))
            .take_while(|&p| p < high[side]);
        for station in along {
            let deepest = across
                .clone()
                .filter_map(|offset| {
                    let mut point = [0.0; 2];
                    point[axis] = station;
                    point[side] = offset;
                    let row = map.sample(point)?;
                    (row.body == body && row.depth > 0.0).then_some((row.depth, point))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, point)) = deepest {
                points.push((body, point));
            }
        }
    }
    points
}

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

#[test]
fn metal_map_keeps_bulk_current_through_the_backwater() {
    let Some(gpu) = thrum_test_support::open(
        "river backwater current",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let forcing = water_scene::channel::forcing(water_scene::island::Wind::Calm);
    let products = thrum_test_support::Scratch::new("river-backwater");
    let (mut map, _) =
        water_scene::river::map(&forcing, Some(products.path())).expect("island rivers");
    let points = [
        [179.668, -23.665],
        [180.0, -20.001],
        [180.0, -19.999],
        [207.499, 5.0],
        [207.501, 5.0],
        [320.87, 6.17],
        [325.515, 5.289],
        [347.499, 5.0],
        [347.501, 5.0],
    ];
    for stage in [0.0, 0.5, 1.0] {
        map.set_stage(1, stage).expect("lake stage");
        compare(&gpu, &map, &points);
    }
    map.set_stage(1, 0.5).expect("default lake stage");
    // The two backwater witnesses really carry water before they taper to
    // the receivers at the edge. A zero-current rendering row is a failure.
    for point in [points[0], points[6]] {
        let row = map.sample(point.map(f64::from)).expect("backwater owner");
        assert!(row.depth > 0.0 && row.row.current.into_iter().any(|v| v.abs() > 0.1));
    }
}

pub(super) const WETTING_GAUGES: [f64; 7] = [0.0, 0.33, 0.35, 0.46, 0.48, 0.73, 1.0];
pub(super) const HOLLOW: [f32; 2] = [5.0, 5.0];
pub(super) const FALL_REACH: [[f32; 2]; 4] = [[9.0, 5.0], [9.0, 6.0], [9.0, 7.0], [9.0, 8.0]];

pub(super) fn wetting_fixture() -> Map {
    const SIDE: usize = 12;
    const CELLS: usize = SIDE * SIDE;
    let cell = |x, z| z * SIDE + x;
    let mut heights = vec![8.0; CELLS];
    for x in 3..=10 {
        heights[cell(x, 3)] = 3.0;
    }
    // The ridge crosses above the channel's low stage. Behind it is a hollow
    // deeper than the low stage that takes the ridge's wetting gauge.
    for x in 4..=6 {
        heights[cell(x, 4)] = 3.6;
        heights[cell(x, 5)] = 2.9;
        heights[cell(x, 6)] = 2.9;
    }
    // The upper-only lip drops into a reach which is entirely dry at low stage.
    for x in 8..=10 {
        heights[cell(x, 4)] = 3.5;
        for z in 5..=8 {
            heights[cell(x, z)] = 0.5;
        }
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
    for x in 3..=10 {
        let i = cell(x, 3);
        low.levels[i] = 3.25;
        low.currents[i] = [0.4, 0.0];
        high.levels[i] = 4.0;
        high.currents[i] = [0.8, 0.1];
    }
    let hollow = (4..=6).flat_map(|x| (4..=6).map(move |z| (x, z, 0.47)));
    let lip = (8..=10).map(|x| (x, 4, 0.34));
    let gated = hollow.chain(lip);
    for (x, z, gate) in gated.clone() {
        high.levels[cell(x, z)] = 4.0;
        wetting[cell(x, z)] = Some(gate);
    }
    let mut lower = dry.clone();
    let mut lower_wetting = vec![None; CELLS];
    let receiver = (8..=10).flat_map(|x| (5..=8).map(move |z| (x, z)));
    for (x, z) in receiver.clone() {
        lower.levels[cell(x, z)] = 1.1;
        lower.currents[cell(x, z)] = [0.0, 0.6];
        lower_wetting[cell(x, z)] = Some(0.34);
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
    let main_points: Vec<_> = (3..=10)
        .map(|x| (x, 3))
        .chain(gated.map(|(x, z, _)| (x, z)))
        .collect();
    let bodies = vec![
        make_body(
            "Ridge and lip",
            &low,
            footprint(&main_points),
            vec![End {
                body: BRINK,
                segment: [[8.0, 4.0], [10.0, 4.0]],
                length: 0.5,
                mouth: false,
            }],
        ),
        make_body(
            "Fall receiver",
            &dry,
            footprint(&receiver.collect::<Vec<_>>()),
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
    let mut map = wetting_fixture();
    let points = [[4.5, 3.0], [5.0, 4.0], HOLLOW, [9.0, 4.0]]
        .into_iter()
        .chain(FALL_REACH)
        .collect::<Vec<_>>();
    for gauge in WETTING_GAUGES {
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

#[expect(
    clippy::cast_possible_truncation,
    reason = "the compute stations are authored in GPU metres"
)]
#[test]
fn metal_matches_the_island_depths_at_and_between_samples() {
    let Some(gpu) = thrum_test_support::open(
        "island river samples",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let products = thrum_test_support::Scratch::new("island-river-samples");
    let forcing = water_scene::channel::forcing(water_scene::island::Wind::Trade);
    let (mut map, files) =
        water_scene::river::map(&forcing, Some(products.path())).expect("prepared island");
    let stream = files[0].gauges();
    let river = files[1].gauges();
    for (spring, lake) in [
        (stream[0], river[0]),
        (stream[0].midpoint(stream[1]), river[1].midpoint(river[2])),
        (stream[1], river[2]),
        (stream[2], river[4]),
    ] {
        map.set_gauge(0, spring).expect("spring stage");
        map.set_stage(1, lake - 12.0).expect("lake stage");
        let points: Vec<_> = transect_points(&map)
            .into_iter()
            .map(|(_, point)| point.map(|p| p as f32))
            .collect();
        compare(&gpu, &map, &points);
        println!(
            "Metal map owner, depth and current at {} stations, lake {lake:.3} m, spring {spring:.6} m",
            points.len()
        );
    }
}
