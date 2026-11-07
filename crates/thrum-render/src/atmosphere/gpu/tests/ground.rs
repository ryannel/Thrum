use super::super::{
    AtmospherePrepared, EARTH, EXTENTS, GROUND_VALID, LUT_PLANES, LUT_SHADER, prepare_compute,
};
use crate::atmosphere::AtmosphereFrame;
use crate::synthetic::SyntheticView;
use std::{f64::consts::PI, sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

const READ: &str = r"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> output: array<vec4<f32>>;
@compute @workgroup_size(8,8)
fn read(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(source);
    if any(id.xy>=size) { return; }
    output[id.y*size.x+id.x]=textureLoad(source,vec2<i32>(id.xy),0);
}
";

const MATH: &str = concat!(
    concat!(
        include_str!("../../storage.wgsl"),
        "\n",
        concat!(
            include_str!("../../geometry.wgsl"),
            "\n",
            include_str!("../../common.wgsl")
        )
    ),
    r"
@group(0) @binding(6) var<storage,read_write> output: array<vec4<f32>>;
var<workgroup> sums: array<vec3<f32>,64>;
@compute @workgroup_size(64)
fn moments(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let mu_samples=16u<<group.x;
    var sum=vec3(0.0);
    for(var index=lane;index<mu_samples*16u;index+=64u) {
        let sample=ground_hemisphere_sample(index,mu_samples,16u);
        sum+=vec3(1.0,sample.y,sample.y*sample.y)*sample.w;
    }
    sums[lane]=sum;
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride { sums[lane]+=sums[lane+stride]; }
        workgroupBarrier();
    }
    if lane==0u { output[group.x]=vec4(sums[0],0.0); }
}
@compute @workgroup_size(64)
fn coordinates(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=GROUND_SUN_ENTRIES { return; }
    let mu=ground_sun_mu(id.x);
    output[id.x]=vec4(mu,ground_irradiance_uv(mu),0.0);
}
"
);

fn pipeline(device: &wgpu::Device, source: &'static str, entry: &str) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(entry),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn collect(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    output: &wgpu::Buffer,
    bytes: u64,
) -> Vec<[f32; 4]> {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(output, 0, &readback, 0, bytes);
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("ground receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(20)),
        })
        .expect("ground correctness completes");
    receive
        .recv()
        .expect("ground map callback")
        .expect("ground mapped");
    readback
        .slice(..)
        .get_mapped_range()
        .expect("ground data")
        .as_chunks::<16>()
        .0
        .iter()
        .map(|bytes| {
            std::array::from_fn(|word| {
                f32::from_le_bytes(
                    bytes[word * 4..word * 4 + 4]
                        .try_into()
                        .expect("f32 record"),
                )
            })
        })
        .collect()
}

fn math(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    entry: &str,
    count: u64,
    groups: u32,
) -> Vec<[f32; 4]> {
    let pipeline = pipeline(device, MATH, entry);
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: count * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 6,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(groups, 1, 1);
    }
    collect(device, queue, encoder, &output, count * 16)
}

struct Fixture {
    device: wgpu::Device,
    queue: wgpu::Queue,
    uniform: wgpu::Buffer,
    textures: [wgpu::Texture; 4],
    producers: [(wgpu::ComputePipeline, wgpu::BindGroup); 2],
    reader: wgpu::ComputePipeline,
}
impl Fixture {
    fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &[0; 128],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let textures = std::array::from_fn(|index| {
            let [width, height] = if index < 2 { [1, 1] } else { EXTENTS[index] };
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("controlled transport inputs"),
                size: wgpu::Extent3d {
                    width,
                    height: height * LUT_PLANES[index],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        });
        let views = textures
            .each_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(EARTH.shader(LUT_SHADER).into()),
        });
        let night = super::super::NightProfile::new(&device, &queue, false);
        let producers = [2, 3].map(|index| {
            prepare_compute(&device, index, &uniform, &views, &sampler, &shader, &night)
        });
        let reader = pipeline(&device, READ, "read");
        Self {
            device,
            queue,
            uniform,
            textures,
            producers,
            reader,
        }
    }

    fn constant(&self, index: usize, colour: [u16; 4]) {
        let texture = &self.textures[index];
        let data: Vec<_> = (0..texture.width() * texture.height())
            .flat_map(|_| colour)
            .flat_map(u16::to_le_bytes)
            .collect();
        self.queue.write_texture(
            texture.as_image_copy(),
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * 8),
                rows_per_image: Some(texture.height()),
            },
            texture.size(),
        );
    }

    fn run(&self, index: usize) -> Vec<[f32; 4]> {
        let texture = &self.textures[index];
        let bytes = u64::from(texture.width()) * u64::from(texture.height()) * 16;
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.reader.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            let (pipeline, group) = &self.producers[index - 2];
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            if index == 3 {
                pass.dispatch_workgroups(128, 1, 1);
            } else {
                pass.dispatch_workgroups(
                    texture.width().div_ceil(8),
                    texture.height().div_ceil(8),
                    1,
                );
            }
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.reader);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(texture.width().div_ceil(8), texture.height().div_ceil(8), 1);
        }
        collect(&self.device, &self.queue, encoder, &output, bytes)
    }
}

#[test]
fn gpu_ground_quadrature_boundary_zero_units_and_profile_cache() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("ground transport adapter");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("ground transport device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fixture = Fixture::new(device, queue);
        let moments = math(&fixture.device, &fixture.queue, "moments", 3, 3);
        for result in &moments {
            assert!(
                (f64::from(result[0]) - PI).abs() < 3e-6,
                "constant sky integrates to pi L"
            );
        }
        for (component, truth) in [(1, 2.0 * PI / 3.0), (2, PI / 2.0)] {
            let errors = moments
                .iter()
                .map(|value| (f64::from(value[component]) - truth).abs())
                .collect::<Vec<_>>();
            assert!(
                errors[0] > errors[1] * 3.0 && errors[1] > errors[2] * 3.0,
                "midpoint angular refinement {errors:?}"
            );
        }
        let coordinates = math(&fixture.device, &fixture.queue, "coordinates", 128, 2);
        for (entry, value) in coordinates.iter().enumerate() {
            let t = 2.0 * entry as f64 / 127.0 - 1.0;
            assert!((f64::from(value[0]) - (t.signum() * t * t * PI * 0.5).sin()).abs() < 2e-6);
            assert!(
                (f64::from(value[1]) * 128.0 - 0.5 - entry as f64).abs() < 0.002,
                "Sun-grid roundtrip"
            );
        }

        fixture.constant(0, [0; 4]);
        fixture.constant(1, [0; 4]);
        let zero = fixture.run(3);
        assert!(zero.iter().flatten().all(|value| value.to_bits() == 0));
        fixture.constant(0, [0x3c00; 4]);
        let unit = fixture.run(3);
        for plane in unit.as_chunks::<128>().0 {
            assert_eq!(
                plane[0], [0.0; 4],
                "Sun below the planet has no first-order source"
            );
            assert!(plane[127].iter().all(|value| *value > 0.0));
        }
        assert!(
            unit.iter()
                .flatten()
                .all(|value| value.is_finite() && *value >= 0.0 && *value < 11685.0)
        );
        let mut changed = [0.0_f32; 32];
        changed[19] = 2000.0;
        changed[20..24].copy_from_slice(&[0.6, -0.8, 0.0, 0.0]);
        changed[24..28].copy_from_slice(&[1.0 / 0.2126, 0.0, 0.0, 1.0 / 1024.0]);
        fixture.queue.write_buffer(
            &fixture.uniform,
            0,
            changed.map(f32::to_le_bytes).as_flattened(),
        );
        assert_eq!(
            fixture.run(3),
            unit,
            "profile product ignores camera/Sun rotation/intensity/colour"
        );

        fixture.constant(0, [0; 4]);
        fixture.constant(1, [0x5c00; 4]); // encoded M=1/256.
        let a = fixture.run(3);
        fixture.constant(1, [0x6000; 4]); // encoded M=1/128.
        let b = fixture.run(3);
        for (a, b) in a.iter().zip(&b) {
            for channel in 0..4 {
                assert_eq!(b[channel], 2.0 * a[channel], "linear unit multiple source");
            }
        }

        // Zero volume source plus controlled Esky=1: only the ground half's
        // terminal boundary should emit rho/pi, including the h=0 tangent row.
        fixture.constant(1, [0; 4]);
        fixture.constant(3, [0x6400; 4]); // 1024 gain.
        let mut frame = [0.0_f32; 32];
        frame[21] = 1.0;
        frame[28..32].copy_from_slice(&super::super::super::model::sky_mapping(0.0));
        fixture.queue.write_buffer(
            &fixture.uniform,
            0,
            frame.map(f32::to_le_bytes).as_flattened(),
        );
        let sky = fixture.run(2);
        for (pixel, value) in sky.iter().enumerate() {
            let expected = if pixel / (EXTENTS[2][0] as usize) < 54 {
                1024.0 * 0.1 / PI
            } else {
                0.0
            };
            for channel in &value[..3] {
                assert!(
                    (f64::from(*channel) - expected).abs() < 0.02,
                    "terminal boundary pixel{pixel}: {value:?}"
                );
            }
        }
        terminal_ground_uses_local_sun(&fixture);
        cache_lifecycle(&fixture);
        assert!(scope.pop().await.is_none());
    });
}

fn terminal_ground_uses_local_sun(fixture: &Fixture) {
    fixture.constant(0, [0; 4]);
    fixture.constant(1, [0; 4]);
    fixture.constant(3, [0x6400; 4]);
    let elevation = -3.0_f32.to_radians();
    let mut frame = [0.0_f32; 32];
    frame[19] = 100_000.0;
    frame[20..24].copy_from_slice(&[elevation.cos(), elevation.sin(), 0.0, 1.0]);
    frame[24..28].copy_from_slice(&[1.0, 1.0, 1.0, 1.0 / 1024.0]);
    frame[28..32].copy_from_slice(&super::super::super::model::sky_mapping(100_000.0));
    fixture.queue.write_buffer(
        &fixture.uniform,
        0,
        frame.map(f32::to_le_bytes).as_flattened(),
    );
    let baseline = fixture.run(2);
    let step: Vec<_> = (0..128)
        .cycle()
        .take(256)
        .flat_map(|entry| if entry < 64 { [0_u16; 4] } else { [0x6400; 4] })
        .flat_map(u16::to_le_bytes)
        .collect();
    let texture = &fixture.textures[3];
    fixture.queue.write_texture(
        texture.as_image_copy(),
        &step,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(128 * 8),
            rows_per_image: Some(texture.height()),
        },
        texture.size(),
    );
    let sampled = fixture.run(2);
    // Independent planet-centred f64 intersection: row45 at 100km reaches
    // ground whose local Sun is +2.5 degrees Sunward, negative opposite it.
    let radius = 6_371_000.0_f64;
    let height = 100_000.0;
    let horizon = (-(height * (2.0 * radius + height)).sqrt() / (radius + height)).asin();
    let angle = horizon - (PI * 0.5 + horizon) * (8.0_f64 / 53.0).powi(2);
    let distance = -(radius + height) * angle.sin()
        - ((radius + height).powi(2) * angle.sin().powi(2) - height * (2.0 * radius + height))
            .sqrt();
    let sun_elevation = -3.0_f64.to_radians();
    for (x, lit) in [(96_u32, true), (0, false)] {
        let azimuth = ((f64::from(x) + 0.5) / 192.0 * 2.0 - 1.0) * PI;
        let nx = angle.cos() * azimuth.cos() * distance / radius;
        let ny = (radius + height + angle.sin() * distance) / radius;
        let local_mu = nx * sun_elevation.cos() + ny * sun_elevation.sin();
        assert!(if lit {
            local_mu > 0.03
        } else {
            local_mu < -0.1
        });
        assert!(sun_elevation.sin() < 0.0, "camera Sun is below the horizon");
        let pixel = (45 * EXTENTS[2][0] + x) as usize;
        assert!(baseline[pixel][0] > 0.0);
        if lit {
            assert_eq!(
                sampled[pixel], baseline[pixel],
                "terminal local Sun selects the lit plateau"
            );
        } else {
            assert_eq!(
                &sampled[pixel][..3],
                &[0.0; 3],
                "opposite terminal selects the zero plateau"
            );
        }
    }
}

fn cache_lifecycle(fixture: &Fixture) {
    let directional = fixture.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: crate::synthetic::LIGHT_BYTES as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let frame = |sun, illuminance, colour, height, epoch| {
        AtmosphereFrame::new(sun, illuminance, colour, height, epoch).expect("valid cache sample")
    };
    let first = frame([0.0, 1.0, 0.0], 127_000.0, [1.0; 3], 2.0, 1);
    let mut prepared = AtmospherePrepared::new(
        &fixture.device,
        &fixture.queue,
        1,
        first,
        &SyntheticView::default(),
        &directional,
        &super::sky_samples(&fixture.device),
    );
    assert_eq!(
        prepared.validity & GROUND_VALID,
        0,
        "unsubmitted products are invalid"
    );
    // The owner acknowledges submitted profile work only at commit. A later
    // profile update without commit must remain invalid across cancellation.
    prepared.commit();
    let rotated = frame([0.6, -0.8, 0.0], 0.0, [1.0 / 0.2126, 0.0, 0.0], 2000.0, 1);
    prepared.update(&fixture.queue, rotated, &SyntheticView::default(), [20, 10]);
    assert_ne!(prepared.validity & GROUND_VALID, 0);
    let changed = frame([0.0, 1.0, 0.0], 100.0, [1.0; 3], 2.0, 2);
    prepared.update(&fixture.queue, changed, &SyntheticView::default(), [10, 20]);
    assert_eq!(prepared.validity & GROUND_VALID, 0);
    prepared.update(&fixture.queue, changed, &SyntheticView::default(), [10, 20]);
    assert_eq!(
        prepared.validity & GROUND_VALID,
        0,
        "no commit after a dropped frame"
    );
}
