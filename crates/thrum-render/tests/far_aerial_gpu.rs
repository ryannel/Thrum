//! Lent far aerial: retained independent air paths, finite parity and continuity.
mod support;
use std::sync::{Arc, Mutex};
use thrum_render::{
    atmosphere::{AtmosphereFrame, EnvironmentProducts, SunOpticalDepth},
    capabilities::DeviceCapabilities,
    contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};
use wgpu::util::DeviceExt;

struct Borrow(Arc<Mutex<Option<EnvironmentProducts>>>);
impl SceneContribution for Borrow {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: false,
            compute: false,
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        *self.0.lock().expect("observer") = surface.environment_products();
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

const QUERY: &str = r"
@group(1) @binding(0) var<storage,read> queries:array<vec4<f32>>;
@group(1) @binding(1) var<storage,read_write> output:array<vec4<f32>>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let q=queries[id.x];let direction=vec3(cos(q.x),sin(q.x),0.0);
    let point=atmosphere.camera_altitude.xyz+direction*q.y;
    let a=environment_aerial(point,vec2(0.5));
    output[id.x*3u]=vec4(a.transmission,0.0);
    output[id.x*3u+1u]=vec4(a.scatter,0.0);
    output[id.x*3u+2u]=vec4(environment_sky_radiance(direction),0.0);
}
";

fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    env: &EnvironmentProducts,
    input: &[[f32; 4]],
) -> Vec<[f32; 4]> {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("far air observer"),
        entries: &[true, false]
            .into_iter()
            .enumerate()
            .map(|(i, read_only)| wgpu::BindGroupLayoutEntry {
                binding: u32::try_from(i).expect("two bindings"),
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect::<Vec<_>>(),
    });
    let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(input),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bytes = params.size() * 3;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&env.layout), Some(&layout)],
        immediate_size: 0,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("lent aerial observer"),
        source: wgpu::ShaderSource::Wgsl(format!("{}\n{QUERY}", env.shader_source).into()),
    });
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("lent aerial observer"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, env.group(0), &[]);
        pass.set_bind_group(1, &group, &[]);
        pass.dispatch_workgroups(u32::try_from(input.len()).expect("bounded queries"), 1, 1);
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &output, ..);
    let submission = queue.submit([encoder.finish()]);
    // The returned bytes carry no alignment guarantee, so decode each vec4 by value.
    readback
        .wait(device, submission)
        .as_chunks::<16>()
        .0
        .iter()
        .map(|texel| {
            std::array::from_fn(|i| {
                f32::from_le_bytes(texel[i * 4..i * 4 + 4].try_into().expect("four bytes"))
            })
        })
        .collect()
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded reference cases narrow to physical shader inputs"
)]
fn lent_air_extends_continuously_and_preserves_the_finite_operation() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "far aerial reference",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let rows: Vec<Vec<f64>> = include_str!("data/far_air.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            l.split_whitespace()
                .map(|n| n.parse().expect("reference number"))
                .collect()
        })
        .collect();
    let inputs: Vec<_> = rows
        .iter()
        .map(|r| [r[0].to_radians() as f32, r[1] as f32, 0.0, 0.0])
        .collect();
    let mut results = Vec::new();
    pollster::block_on(async {
        let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        for provider in [false, true] {
            let mut output = Vec::new();
            for (elevation, queries) in inputs.as_chunks::<8>().0.iter().enumerate() {
                let mut scene = SyntheticScene::grid(1).expect("scene");
                let angle = queries[0][0];
                scene.view = SyntheticView::perspective(
                    [0.0, 260.48465, 0.0],
                    [angle.cos(), 260.48465 + angle.sin(), 0.0],
                    [0.0, 1.0, 0.0],
                    1.0,
                    1.0,
                    0.1,
                    200_000.0,
                )
                .expect("reference direction camera");
                scene.sun_optical_depth =
                    provider.then(|| SunOpticalDepth::gpu_target([16, 16], 1).expect("target"));
                let lighting = FrameLighting::Atmosphere(
                    AtmosphereFrame::new([0.0, 1.0, 0.0], 100_000.0, [1.0; 3], 260.48465, 1)
                        .expect("physical environment"),
                );
                let mut host =
                    support::host_with_extent(&caps, &queue, scene, lighting, [32, 32]).await;
                let env = Arc::new(Mutex::new(None));
                host.set_scene_contribution(Box::new(Borrow(Arc::clone(&env))))
                    .expect("attach read-only observer");
                host.capture_pixels().expect("produce air");
                output.extend(probe(
                    &device,
                    &queue,
                    env.lock()
                        .expect("environment")
                        .as_ref()
                        .expect("lent products"),
                    queries,
                ));
                assert_eq!(output.len(), (elevation + 1) * 24);
            }
            results.push(output);
        }
    });
    let mut maximum = 0.0_f64;
    let mut scatter_maximum = 0.0_f64;
    for (i, row) in rows.iter().enumerate() {
        let actual = results[1][i * 3];
        if row[1] <= 32000.0 {
            assert_eq!(
                &results[0][i * 3..i * 3 + 3],
                &results[1][i * 3..i * 3 + 3],
                "finite operation changed at {row:?}"
            );
        }
        for c in 0..3 {
            assert!((0.0..=1.0).contains(&actual[c]));
            maximum = maximum.max((f64::from(actual[c]) - row[c + 2]).abs());
            scatter_maximum = scatter_maximum
                .max((f64::from(results[1][i * 3 + 1][c]) - row[c + 5]).abs() / 100_000.0);
        }
        if row[1].to_bits() == 32001.0_f64.to_bits() {
            for quantity in 0..2 {
                for (&a, &b) in results[1][i * 3 + quantity][..3]
                    .iter()
                    .zip(&results[1][(i - 1) * 3 + quantity][..3])
                {
                    // One fp16 relative spacing, with a small arithmetic allowance.
                    assert!(
                        (a - b).abs() <= b.abs() / 1024.0 + 1e-6,
                        "32 km discontinuity: quantity {quantity}, near {b}, far {a}"
                    );
                }
            }
        }
        if row[1].to_bits() == 1e7_f64.to_bits() {
            for (&scatter, &sky) in results[1][i * 3 + 1][..3]
                .iter()
                .zip(&results[1][i * 3 + 2][..3])
            {
                assert!(
                    (scatter - sky).abs() <= 0.002 * (1.0 + sky.abs()),
                    "not tending to sky"
                );
            }
        }
    }
    eprintln!(
        "far aerial maximum RGB transmission discrepancy against retained independent air reference: {maximum}"
    );
    eprintln!("maximum RGB scattering discrepancy / incident lux: {scatter_maximum}");
}
