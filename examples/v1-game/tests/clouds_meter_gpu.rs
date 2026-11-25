//! Reads illumination and the exposure meter from the same closed-deck submission.
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface},
    runtime::{FrameConfig, FrameError},
};
use v1_game::{CloudSelection, GameHost as _, GameSetup, SceneCamera, clouds::WeatherPreset};

struct Meter(Arc<Mutex<Option<EnvironmentProducts>>>);

impl SceneContribution for Meter {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        *self.0.lock().expect("meter observer") = surface.environment_products_with_cloud();
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

fn light(gpu: &thrum_test_support::Opened, environment: &EnvironmentProducts) -> [f32; 4] {
    let device = &gpu.device;
    let device_site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("horizontal receiver result"),
        size: 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let entries = &[thrum_render::binding::storage(
        0,
        wgpu::ShaderStages::COMPUTE,
        false,
        16,
    )];
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries,
    });
    let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&environment.layout), Some(&layout)],
        immediate_size: 0,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}
@group(1) @binding(0) var<storage,read_write> result: vec4<f32>;
@compute @workgroup_size(1) fn main() {{
    let point = atmosphere.camera_altitude.xyz;
    let direct = sun_irradiance(point) * max(environment_sun_direction().y, 0.0);
    let factor = cloud_factor(point);
    result = vec4(dot(environment_light.horizontal_irradiance.rgb,vec3(0.2126,0.7152,0.0722)),
        dot(direct,vec3(0.2126,0.7152,0.0722)),factor,1.0);
}}",
                environment.shader_source
            )
            .into(),
        ),
    });
    let pipeline = device_site.compute(&wgpu::ComputePipelineDescriptor {
        label: None,
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
        pass.set_bind_group(0, environment.group(0), &[]);
        pass.set_bind_group(1, &binding, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    let read = thrum_test_support::readback::Readback::buffer(device, &mut encoder, &buffer, ..);
    let bytes = read.wait(device, gpu.queue.submit([encoder.finish()]));
    std::array::from_fn(|i| f32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().expect("float")))
}

thrum_test_support::macos_gpu_test! {
    fn metal_incident_meter_matches_a_horizontal_receiver_under_overcast() {
        let Some(gpu) = thrum_test_support::open(
            "closed deck meter", Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable, &thrum_test_support::Restriction::default(),
        ) else { return };
        assert_eq!(gpu.gpu.adapter.get_info().backend, wgpu::Backend::Metal);
        pollster::block_on(async {
            let mut host = GameSetup {
                clouds: CloudSelection::WorldLight,
                cloud_weather: WeatherPreset::Overcast.parameters(),
                ..GameSetup::default()
            }.prepare(&gpu.device, &gpu.queue, &gpu.capabilities, FrameConfig {
                render_extent: [32, 32],
                surface: Some(thrum_render::present::SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm, extent: [32, 32],
                }),
            }).await.expect("world light");
            host.set_scene_camera(SceneCamera::perspective(
                [0.0, 40.0, 0.0], [0.0, 40.0, -60.0], 0.9, 1.0, 200_000.0,
            ).expect("observer")).expect("camera");
            host.set_time_of_day(0.5).expect("noon");
            let environment = Arc::new(Mutex::new(None));
            host.set_game_contribution(Box::new(Meter(Arc::clone(&environment)))).expect("lend incident light");
            host.enable_submitted_capture(1).expect("submitted exposure tap");
            let output = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("closed deck capture"),
                size: wgpu::Extent3d { width: 32, height: 32, depth_or_array_layers: 1 },
                mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }).create_view(&wgpu::TextureViewDescriptor::default());
            let report = host.drive(Duration::ZERO, Some(&thrum_render::runtime::SurfaceOutput {
                view: &output, extent: [32, 32],
            })).expect("submit deck");
            assert!(report.submitted);
            let image = thrum_test_support::wait::until("meter", Duration::from_secs(4), || {
                drop(host.poll_completed().expect("complete deck"));
                host.try_submitted_image().expect("same submitted frame")
            });
            let words = image.exposure_gpu_words.expect("meter words");
            assert_eq!(words[4] & 0b1100, 0, "valid and metered");
            let lux = f32::from_bits(words[3]);
            let [diffuse, direct, factor, _] = light(&gpu,
                environment.lock().expect("observer").as_ref().expect("lent products"));
            assert!(factor < 0.05 && diffuse > 0.0 && direct > 1000.0, "{diffuse} {direct} {factor}");
            let expected = direct.mul_add(factor, diffuse);
            assert!((lux - expected).abs() < expected * 0.005, "{lux} vs receiver {expected}");
            println!("Metal: {lux} lux = {diffuse} diffuse + {direct} direct × {factor}");
        });
    }
}
