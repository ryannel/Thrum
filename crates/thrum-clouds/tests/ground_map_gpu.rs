//! Compare deployed ground-density arithmetic with the public headless query.
use std::sync::Arc;
use thrum_cloud_model::{
    Affinity, Density, Field, Fog, Ground, GroundProfile, GroundState, Parameters, Profile, State,
    Surface, SurfacePoint, Weather,
};
use thrum_clouds::{GroundConfig, GroundField};
use thrum_render::{binding, runtime::DeviceSite};
use thrum_test_support::{Opened, Restriction, readback::Readback};
use wgpu::util::DeviceExt;

const PROBE: &str = r"
@group(0) @binding(3) var<storage, read> points: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> answer: array<vec4<f32>>;
@compute @workgroup_size(32)
fn probe(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x >= arrayLength(&points) { return; }
    let p = points[id.x].xyz;
    let surface = ground_surface(p.xz);
    let sigma = ground_extinction(p, surface, true);
    var shifted_shore = surface;
    shifted_shore.z = ground_surface(p.xz - ground_frame.travel).z;
    answer[id.x] = vec4(sigma, ground_extinction(p, shifted_shore, true), 2.0 * sigma, 0.0);
}
";

const SEED: u32 = 7429;
const LINE_STEPS: usize = 60;
const LINE_START: f64 = -135.0;
const LINE_END: f64 = -75.0;

#[test]
fn world_probes_and_sight_line_keep_surface_and_air_in_their_own_frames() {
    let Some(Opened { device, queue, .. }) = thrum_test_support::open(
        "cloud ground map",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &Restriction::default(),
    ) else {
        return;
    };
    let site = DeviceSite::untracked(device.clone());
    let config = fixture();
    let model = Field::new(
        SEED,
        Profile {
            pitch: 3_000.0,
            turret_pitch: 800.0,
            turret_least: 0.2,
            turret_fade: 0.12,
            period: 1800.0,
            detail_mean: 0.5,
            erosion: 0.0,
        },
        vec![
            thrum_cloud_model::Type {
                base: 1600.0,
                depth: [450.0, 650.0],
                liquid_water: 0.000_2,
                droplet_radius: 10e-6,
                deck: 1.0,
                profile: [0.2; 2],
            };
            2
        ],
    )
    .expect("model");
    let weather = Weather {
        field: &model,
        surface: Some(&config.surface),
        ground: Some(&config.fog),
    };
    let mut field = GroundField::new(&device, &queue, &config, SEED).expect("ground field");
    let mut points = vec![
        [-121.0, 9.0, 2.0, 0.0],
        [-61.5, 34.0, -2.0, 0.0],
        [18.0, 9.0, 4.0, 0.0],
        [45.0, 9.0, -2.0, 0.0],
        [84.5, 12.0, 3.0, 0.0],
        [119.0, 17.0, 7.0, 0.0],
        [140.0, 90.0, 1.0, 0.0],
        [280.0, 12.0, 1.0, 0.0],
    ];
    points.extend((0_i16..60).map(|i| [-134.5 + f32::from(i), 12.0, 0.0, 0.0]));
    let probes = Probe::new(&site, &field, &points);
    for row in 0..config.fog.rows().len() {
        let mut state = State::new(
            Parameters {
                coverage: 0.0,
                kind: 0.0,
                wind: [18.25, 0.375],
                time_scale: 1.0,
            },
            5.0,
            0,
        )
        .expect("state");
        let mut amounts = vec![0.0; config.fog.rows().len()];
        amounts[row] = 0.8;
        state.set_ground(GroundState::new(amounts, 0.0, 100_000.0).expect("amounts"));
        state.step(1, 8.0).expect("travel");
        assert_eq!(state.travel(), [146.0, 3.0]);
        field.upload(&queue, &state, state.travel());
        let results = probes.read(&device, &queue);
        let reference: Vec<_> = points
            .iter()
            .map(|&[x, y, z, _]| {
                weather
                    .extinction([x, y, z].map(f64::from), &state, |_| None)
                    .value
            })
            .collect();
        let expected_depth = weather
            .optical_depth(
                [LINE_START, 12.0, 0.0],
                [LINE_END, 12.0, 0.0],
                &state,
                600,
                Density::Coarse,
                |_| None,
            )
            .expect("public optical depth")
            .value;
        let matches = |channel: usize| {
            let correct_points = results.iter().zip(&reference).all(|(values, &expected)| {
                let got = f64::from(values[channel]);
                (got > 1e-7) == (expected > 1e-7) && (got - expected).abs() < 3e-6
            });
            let depth: f64 = results[results.len() - LINE_STEPS..]
                .iter()
                .map(|values| f64::from(values[channel]))
                .sum();
            (correct_points, (depth - expected_depth).abs() < 2e-4, depth)
        };
        let (points_ok, depth_ok, depth) = matches(0);
        eprintln!(
            "row {row}, non-whole-texel travel {:?}, optical depth {depth:.6} vs {expected_depth:.6}",
            state.travel()
        );
        assert!(
            reference.iter().any(|&sigma| sigma > 1e-4),
            "row {row} must be exercised"
        );
        assert!(
            reference.contains(&0.0),
            "row {row} must also have a clear probe"
        );
        assert!(
            points_ok && depth_ok,
            "ground density mismatch: row {row}, points={points_ok}, depth={depth_ok}"
        );
        if row == 2 {
            assert!(expected_depth > 0.1, "line crosses the bank interior");
            assert_eq!(results[3][0], 0.0, "no bank inland of the shore margin");
            assert!(!matches(1).0, "moving the shore must fail the world probes");
            assert!(
                !matches(2).1,
                "doubling extinction must fail the optical depth"
            );
        }
    }
}

fn fixture() -> GroundConfig {
    let nodes: Vec<_> = (-1..=1)
        .flat_map(|z| {
            (-12..=14).map(move |x| SurfacePoint {
                ground: if x <= 0 {
                    -8.0
                } else {
                    4.0 * (f64::from(x) * 0.6).sin() + f64::from(x + z)
                },
                water: (x <= 0).then_some(0.0),
            })
        })
        .collect();
    let surface = Surface::new([-240.0, -20.0], [27, 3], 20.0, 160.0, &nodes).expect("shore");
    let even = Fog {
        liquid_water: 0.000_14,
        droplet_radius: 10e-6,
        depth: 70.0,
        depression: 4.0,
        softness: 23.0,
        pooling: 0.0,
        drifting: false,
        affinity: Affinity::Any,
        centre: [-245.0, 0.0],
        radius: [210.0, 160.0],
        noise: 0.3,
        noise_scale: 40.0,
        radiative: false,
    };
    let fog = Ground::new(
        GroundProfile {
            shore_margin: 35.0,
            calm: [2.0, 8.0],
            twilight: [-6.0, 10.0],
            cooling_seconds: 10_800.0,
            warming_seconds: 3600.0,
        },
        vec![
            Fog {
                pooling: 1.0,
                affinity: Affinity::Land,
                ..even
            },
            even,
            Fog {
                drifting: true,
                affinity: Affinity::Water,
                ..even
            },
        ],
    )
    .expect("fog rows");
    GroundConfig {
        surface: Arc::new(surface),
        fog: Arc::new(fog),
    }
}

struct Probe {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    output: wgpu::Buffer,
    count: u32,
}

impl Probe {
    fn new(device: &DeviceSite, field: &GroundField, points: &[[f32; 4]]) -> Self {
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ground map points"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(points),
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ground map answers"),
            size: input.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let stages = wgpu::ShaderStages::COMPUTE;
        let mut entries = GroundField::layout(0).to_vec();
        entries.extend([
            binding::storage(3, stages, true, 0),
            binding::storage(4, stages, false, 0),
        ]);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ground map probe"),
            entries: &entries,
        });
        let mut entries = field.entries(0).to_vec();
        entries.extend([
            wgpu::BindGroupEntry {
                binding: 3,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: output.as_entire_binding(),
            },
        ]);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ground map probe"),
            layout: &layout,
            entries: &entries,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ground map probe"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{}\n{PROBE}", GroundField::source(0, 0)).into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ground map probe"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("ground map probe"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("probe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            pipeline,
            group,
            output,
            count: u32::try_from(points.len()).expect("small test"),
        }
    }

    fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<[f32; 4]> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(self.count.div_ceil(32), 1, 1);
        }
        let readback = Readback::buffer(device, &mut encoder, &self.output, ..);
        let sent = queue.submit([encoder.finish()]);
        readback
            .wait(device, sent)
            .as_chunks::<16>()
            .0
            .iter()
            .copied()
            .map(bytemuck::cast)
            .collect()
    }
}
