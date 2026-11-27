//! Compare the active valley's material provider and hollows map with the CPU model.

mod rain_support;

use rain_support as rain;
use thrum_render::{binding, runtime::DeviceSite, wetness::PreparedWetness};
use thrum_test_support::{Opened, readback::Readback};
use thrum_water::map_gpu::{self, MapGpu};
use v1_game::{GameContent, GameSetup, RainSelection, rain_witness_points};
use wgpu::util::DeviceExt;

const FILL: f64 = 0.0005; // PUDDLES WP5.2
const SURFACE: f64 = 0.000_001; // PUDDLES WP5.2
const DEPTH: f64 = 0.000_04; // WATERS WP4.1
// Row 3801, face 37, where a hard shelter boundary used to disagree with Metal.
const BOULDER_FACE: [f32; 4] = [-20.673_788, 129.929_69, -125.482_59, 0.0];

fn read(
    gpu: &Opened,
    field: &PreparedWetness,
    map: &MapGpu,
    points: &[[[f32; 4]; 2]],
) -> Vec<[[f32; 4]; 2]> {
    let Opened { device, queue, .. } = gpu;
    let site = DeviceSite::untracked(device.clone());
    let size = size_of_val(points) as u64;
    let stage = wgpu::ShaderStages::COMPUTE;
    let hollows = map.hollows_entry().expect("hollows binding");
    let wgpu::BindingResource::Buffer(hollow_buffer) = &hollows.resource else {
        panic!("the hollows map must be a storage buffer");
    };
    let [bed, map_uniform] = map_gpu::bindings(stage);
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("valley model comparison"),
        entries: &[
            binding::storage(0, stage, true, 32),
            binding::storage(1, stage, false, 32),
            binding::texture(
                2,
                stage,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            ),
            bed,
            map_uniform,
            binding::storage(hollows.binding, stage, true, hollow_buffer.buffer.size()),
        ],
    });
    // Both evaluators carry this helper so each can compile on its own.
    let map_source = map.source(0).replace("bed_reaches(", "map_bed_reaches(");
    let source = field.shader(0, 2)
        + &map_source
        + r"
        struct Point { position: vec4<f32>, normal: vec4<f32> }
        @group(0) @binding(0) var<storage, read> points: array<Point>;
        @group(0) @binding(1) var<storage, read_write> answers: array<array<vec4<f32>, 2>>;
        @compute @workgroup_size(64) fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
            if id.x >= arrayLength(&points) { return; }
            let p = points[id.x];
            let wet = ground_wetness(p.position.xyz, p.normal.xyz);
            let puddle = map_sample(p.position.xz);
            answers[id.x][0] = vec4(wet.pores, wet.surface, wet.cover, wet_exposure(p.position.xyz, wet_load(0u)));
            answers[id.x][1] = vec4(select(-1.0, f32(puddle.body), puddle.body != NO_BODY),
                puddle.row.water.x, puddle.row.water.x - puddle.bed, 0.0);
        }";
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production valley ground and water evaluators"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline_layout = site.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("valley model comparison"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("valley model comparison"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("valley sample points"),
        contents: bytemuck::cast_slice(points),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("valley model answers"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let view = map.view();
    let mut entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: input.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: output.as_entire_binding(),
        },
        field.entry(2),
    ];
    entries.extend(map.entries(&view));
    entries.push(hollows);
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("valley evaluation"),
        layout: &layout,
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(points.len().div_ceil(64)).expect("bounded samples"),
            1,
            1,
        );
    }
    let readback = Readback::buffer(device, &mut encoder, &output, 0..size);
    bytemuck::cast_slice(&readback.wait(device, queue.submit([encoder.finish()]))).to_vec()
}

#[test]
fn dry_valley_matches_the_model() {
    compare_model(RainSelection::Shower);
}

#[test]
fn soaked_valley_matches_the_model() {
    compare_model(RainSelection::Soaked);
}

#[test]
fn receding_valley_matches_the_model() {
    compare_model(RainSelection::HalfDry);
}

fn compare_model(selection: RainSelection) {
    let Some(gpu) = rain::open("valley model on Metal") else {
        return;
    };
    let store = rain::Store::cook(true);
    pollster::block_on(async {
        let mut host = store
            .host(
                &gpu,
                GameSetup {
                    content: GameContent::Exit,
                    rain: selection,
                    ..GameSetup::default()
                },
            )
            .await;
        let pixels = host.capture_pixels().expect("real valley submission");
        assert_eq!(
            pixels.len(),
            (rain::EXTENT[0] * rain::EXTENT[1] * 4) as usize
        );
        let (provider, map) = host
            .source()
            .rain_proof_products()
            .expect("production rain projection");
        let field =
            PreparedWetness::new(&gpu.device, &gpu.queue, &provider).expect("wetness field");
        let map = MapGpu::new(&gpu.device, &gpu.queue, &map).expect("production hollows bindings");
        let expected = rain_witness_points(selection).expect("f64 model samples");
        let points = expected
            .iter()
            .map(|sample| sample.point)
            .collect::<Vec<_>>();
        let output = read(&gpu, &field, &map, &points);
        assert_eq!(output.len(), expected.len());
        let mut boulder_face_checked = false;
        let mut wet = 0;
        for ([actual, water], sample) in output.into_iter().zip(&expected) {
            let position = sample.point[0];
            let model = sample.wetness;
            assert!(
                (f64::from(actual[3]) - sample.exposure).abs() < FILL,
                "{selection:?} {position:?} normal {:?}, canopy actual {} expected {}",
                sample.point[1],
                actual[3],
                sample.exposure
            );
            assert!(
                (f64::from(actual[0]) - model.pores).abs() < FILL,
                "{selection:?} {position:?} {actual:?} {model:?}"
            );
            assert!(
                (f64::from(actual[1]) - model.surface).abs() < SURFACE,
                "{selection:?} {position:?} {actual:?} {model:?}"
            );
            if position
                .iter()
                .zip(BOULDER_FACE)
                .all(|(&a, b)| (a - b).abs() < 0.000_1)
            {
                boulder_face_checked = true;
                assert!(
                    (0.49..0.51).contains(&sample.exposure),
                    "the steep face uses the canopy's material transition"
                );
                println!(
                    "{selection:?} boulder {position:?} | CPU exposure {:.8}, pores {:.8}, surface {:.8} m | Metal exposure {:.8}, pores {:.8}, surface {:.8} m",
                    sample.exposure, model.pores, model.surface, actual[3], actual[0], actual[1]
                );
            }
            if let Some([level, depth]) = sample.puddle {
                wet += usize::from(depth > 0.0);
                assert!(water[0].abs() < f32::EPSILON, "{position:?} {water:?}");
                assert!(
                    (f64::from(water[1]) - level).abs() < DEPTH,
                    "{position:?} {water:?} {level}"
                );
                assert!(
                    (f64::from(water[2]) - depth).abs() < DEPTH,
                    "{position:?} {water:?} {depth}"
                );
            } else if sample.patch {
                assert!(water[0] < 0.0, "{position:?} {water:?}");
            }
        }
        assert!(boulder_face_checked, "reviewed boulder face is sampled");
        if selection == RainSelection::Soaked {
            assert!(wet > 60, "visible filled hollows");
            assert!(
                expected[2].wetness.pores - expected[3].wetness.pores > 0.2,
                "rock column requires the exposure field"
            );
            assert!(
                expected[0].wetness.surface - expected[1].wetness.surface > SURFACE,
                "cooked terrain requires the slope response"
            );
        }
    });
}
