//! Compare the production WGSL wetness function to the f64 model on Metal.
use thrum_rain_model::{
    Canopy, Ground, GroundClass, GroundStores, Profile, Rain, RainState, Shelter, TickReport, Tree,
    WetPoint, band_fill, wetness_at,
};
use thrum_render::{binding, runtime::DeviceSite, wetness::PreparedWetness};
use thrum_water::wet_ground::{
    self as gpu_ground, GroundCell, HollowWetness, HollowsGround, ShoreWetness, WetGround,
};
use thrum_water_model::Exposure;
use wgpu::util::DeviceExt;

const DIM: usize = 16;
const CELL: f64 = 0.1;
const POINTS: usize = 4096;
const FILL_TOLERANCE: f64 = 0.0005;
const WATER_TOLERANCE: f64 = 0.000_001;

fn shelter(name: &str, low: [f64; 3], high: [f64; 3], share: f64) -> Shelter {
    let [x, y, z] = low;
    let [xx, _, zz] = high;
    let a = [x, y, z];
    let b = [xx, y, z];
    let c = [xx, y, zz];
    let d = [x, y, zz];
    Shelter {
        name: name.into(),
        triangles: vec![[a, b, c], [a, c, d]],
        share,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "fixture projects f64 state to GPU metres"
)]
fn run(opened: &thrum_test_support::Opened) {
    let profile = Profile::default();
    let canopy = Canopy::prepare(
        [0.0; 2],
        [DIM; 2],
        &[
            shelter("roof", [0.3, 1.4, 0.2], [1.1, 1.4, 1.3], 0.0),
            shelter("crown", [0.55, 0.75, 0.44], [1.46, 0.75, 1.45], 0.5),
        ],
        &profile,
    )
    .expect("two shelter layers");
    let bed: Vec<_> = (0..DIM * DIM)
        .map(|i| {
            let x = (i % DIM) as f64;
            let z = (i / DIM) as f64;
            0.002 * ((x - 11.0).powi(2) + (z - 9.0).powi(2))
        })
        .collect();
    let ground = Ground::new([0.0; 2], CELL, [DIM; 2], bed.clone()).expect("prepared ground");
    let class = GroundClass {
        soak: 2.0 / 3600.0,
        percolation: 0.5 / 3600.0,
        pores: 0.0012,
        relief: 0.0009,
        steepest: 0.53,
        share: 0.55,
        porosity: 0.7,
        film_roughness: 0.10,
    };
    let mut stores = GroundStores::new(&[class]).expect("class");
    let tree = Tree::prepare(&ground, &canopy, &profile).expect("tree");
    let mut rain = RainState::new(&tree);
    let mut report = TickReport::default();
    for weather in [
        Rain {
            flux: 12.0 / 3600.0,
            drying: 0.0,
        },
        Rain {
            flux: 0.0,
            drying: 55.0 / 3600.0,
        },
        Rain {
            flux: 8.0 / 3600.0,
            drying: 0.0,
        },
    ] {
        for _ in 0..8 {
            rain.step_with(&tree, weather, &stores, 0, 11.0, 1, &mut report)
                .expect("rain step");
            stores.step(weather, 11.0).expect("stores");
        }
    }
    let levels = rain.levels(&tree);
    let hollows: Vec<_> = levels
        .iter()
        .enumerate()
        .map(|(i, &level)| HollowWetness {
            level: level as f32,
            mark: rain.marks()[i] as f32,
            margin: rain.bands()[i] as f32,
            held_fill: rain.held_fill(&tree, &class, i) as f32,
        })
        .collect();
    let cells = bed
        .iter()
        .zip(tree.labels())
        .map(|(&bed, &hollow)| GroundCell {
            bed: bed as f32,
            hollow,
            pit: if hollow == thrum_water_model::NO_HOLLOW {
                bed as f32
            } else {
                tree.node(hollow).lower as f32
            },
            class: 0,
        })
        .collect();
    let render_class = gpu_ground::GroundClass {
        relief: class.relief as f32,
        steepest: class.steepest as f32,
        share: class.share as f32,
        porosity: class.porosity as f32,
        film_roughness: class.film_roughness as f32,
        pattern: gpu_ground::ReliefPattern::default(),
    };
    let shore = ShoreWetness {
        bounds: [[2.0, 0.0], [3.0, 1.0]],
        level: 0.08,
        swash: 0.14,
        margin: 0.1,
    };
    let classes = [render_class];
    let mut field = WetGround::new(
        canopy.canopy_grid(),
        Some(HollowsGround {
            origin: [0.0; 2],
            spacing: CELL as f32,
            size: [DIM as u32; 2],
            cells,
        }),
        &classes,
        hollows.len(),
    )
    .expect("projection");
    field.set_shore(Some(shore));
    let reaches = std::array::from_fn(|i| {
        let s = stores.store(0, i);
        [s.surface as f32, s.fill as f32]
    });
    assert!(field.publish(&classes, &[reaches], &hollows));

    let mut state = 0xe5c9_8712_u32;
    let mut next = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(state >> 8) / f64::from(1u32 << 24)
    };
    let mut points = Vec::with_capacity(POINTS);
    let mut expected = Vec::with_capacity(POINTS);
    let mut slope_wrong = 0;
    let mut exposure_wrong = 0;
    for i in 0..POINTS {
        let (x, z) = (next() * 1.59, next() * 1.59);
        let index = (z / CELL) as usize * DIM + (x / CELL) as usize;
        let (point, normal, on_bed) = match i % 8 {
            0 | 1 => ([x, bed[index], z], [0.0, 1.0, 0.0], true),
            2 => ([x, bed[index], z], [0.6, 0.8, 0.0], true),
            3 => ([x, 1.05, z], [0.0, 1.0, 0.0], false),
            4 => ([x, 0.63, z], [0.0, 1.0, 0.0], false),
            5 => ([x, 1.65, z], [1.0, 0.0, 0.0], false),
            6 => ([x, 1.77, z], [0.48, 0.877_268_487_978_452_4, 0.0], false),
            _ => (
                [2.2 + next() * 0.5, 0.15 + next() * 0.3, next() * 0.99],
                [0.0, 1.0, 0.0],
                false,
            ),
        };
        let point = point.map(|v| f64::from(v as f32));
        let normal = normal.map(|v| f64::from(v as f32));
        let sample = |normal| {
            if on_bed {
                rain.wetness(&tree, &stores, 0, &canopy, point, normal, None)
                    .expect("inside bed")
            } else {
                wetness_at(&stores, 0, &canopy, point, normal, None)
            }
        };
        let mut correct = sample(normal);
        if i % 8 == 7 {
            correct = correct.with_band(band_fill(
                point[1] - f64::from(shore.level + shore.swash),
                f64::from(shore.margin),
            ));
        }
        let flat = sample([0.0, 1.0, 0.0]);
        if (flat.surface - correct.surface).abs() > WATER_TOLERANCE {
            slope_wrong += 1;
        }
        let open = thrum_rain_model::wetness(
            &stores,
            0,
            WetPoint {
                exposure: 1.0,
                normal,
                pattern: None,
            },
        );
        if (open.pores - correct.pores).abs() > FILL_TOLERANCE {
            exposure_wrong += 1;
        }
        points.push([
            [point[0] as f32, point[1] as f32, point[2] as f32, 0.0],
            [normal[0] as f32, normal[1] as f32, normal[2] as f32, 0.0],
        ]);
        expected.push(correct);
    }

    let prepared =
        PreparedWetness::new(&opened.device, &opened.queue, &field.provider()).expect("field");
    let output = probe(opened, &prepared, &points);
    compare(&points, &output, &expected, slope_wrong, exposure_wrong);
}

#[expect(
    clippy::print_stderr,
    reason = "retain the Metal error and negative controls"
)]
fn compare(
    points: &[[[f32; 4]; 2]],
    output: &[[f32; 4]],
    expected: &[thrum_rain_model::Wetness],
    slope_wrong: usize,
    exposure_wrong: usize,
) {
    let mut largest_fill = 0_f64;
    let mut largest_surface = 0_f64;
    for (i, (gpu, cpu)) in output.iter().zip(expected).enumerate() {
        let fill = (f64::from(gpu[0]) - cpu.pores).abs();
        let surface = (f64::from(gpu[1]) - cpu.surface).abs();
        largest_fill = largest_fill.max(fill);
        largest_surface = largest_surface.max(surface);
        assert!(
            fill <= FILL_TOLERANCE && surface <= WATER_TOLERANCE,
            "point {i}, {:?}: GPU {gpu:?}, model {cpu:?}",
            points[i]
        );
    }
    assert!(
        slope_wrong >= 64,
        "wrong slope missed at {slope_wrong} points"
    );
    assert!(
        exposure_wrong >= 128,
        "open-only stores missed at {exposure_wrong} points"
    );
    eprintln!(
        "Metal {POINTS} points, max fill error {largest_fill:.8}, max surface error {largest_surface:.9} m, flat wrong {slope_wrong}, open wrong {exposure_wrong}"
    );
}

fn probe(
    opened: &thrum_test_support::Opened,
    field: &PreparedWetness,
    points: &[[[f32; 4]; 2]],
) -> Vec<[f32; 4]> {
    let device = &opened.device;
    let queue = &opened.queue;
    let stage = wgpu::ShaderStages::COMPUTE;
    let size = (points.len() * size_of::<[f32; 4]>()) as u64;
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wetness point samples"),
        contents: bytemuck::cast_slice(points),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("wetness answers"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wetness model probe"),
        entries: &[
            binding::storage(0, stage, true, 32),
            binding::storage(1, stage, false, 16),
            binding::texture(
                2,
                stage,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            ),
        ],
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wetness model probe"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
            field.entry(2),
        ],
    });
    let source = field.shader(0, 2)
        + r"
        struct Point { position: vec4<f32>, normal: vec4<f32> }
        @group(0) @binding(0) var<storage, read> points: array<Point>;
        @group(0) @binding(1) var<storage, read_write> answers: array<vec4<f32>>;
        @compute @workgroup_size(64) fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
            if id.x >= arrayLength(&points) { return; }
            let p = points[id.x];
            let w = ground_wetness(p.position.xyz, p.normal.xyz);
            answers[id.x] = vec4(w.pores, w.surface, w.cover, 0.0);
        }
    ";
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wetness evaluator"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let device_site = DeviceSite::untracked(device.clone());
    let pipeline_layout = device_site.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wetness probe"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device_site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("wetness probe"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
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
    let read =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &output, 0..size);
    bytemuck::cast_slice(&read.wait(device, queue.submit([encoder.finish()]))).to_vec()
}

#[test]
fn wetness_tracks_model_at_scattered_points() {
    let Some(opened) = thrum_test_support::open(
        "wetness_tracks_model_at_scattered_points",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    run(&opened);
}
