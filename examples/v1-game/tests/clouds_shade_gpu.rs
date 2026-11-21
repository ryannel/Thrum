//! The real game's atmosphere consumes a GPU-written optical-depth field.
use crate::support;

use std::time::Duration;
use thrum_render::runtime::FrameConfig;
use v1_game::{CloudSelection, GameHost, GameSetup};

fn fill(device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::Texture, depth: f32) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("prescribed GPU Sun-path seam probe"),
        source: wgpu::ShaderSource::Wgsl(format!(
            "@group(0) @binding(0) var field:texture_storage_2d<rgba16float,write>;
             @compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {{
             if all(id.xy<textureDimensions(field)) {{ textureStore(field,vec2<i32>(id.xy),vec4<f32>({depth},0.,0.,0.)); }} }}"
        ).into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("GPU optical depth seam test"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("GPU shade seam target"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        }],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(target.width().div_ceil(8), target.height().div_ceil(8), 1);
    }
    queue.submit([encoder.finish()]);
}

#[test]
#[cfg(target_os = "macos")]
fn game_receives_gpu_shade_and_keeps_ambient_light() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "game_receives_gpu_shade_and_keeps_ambient_light",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device(&test_gpu, "portable", true);
        let mut game = GameSetup {
            clouds: CloudSelection::Shade,
            ..GameSetup::default()
        }
        .prepare(
            &device,
            &queue,
            &caps,
            FrameConfig {
                render_extent: [320, 180],
                surface: None,
            },
        )
        .await
        .expect("game");
        game.drive(Duration::ZERO, None).expect("origin");
        game.drive(Duration::from_nanos(16_666_667), None)
            .expect("weather tick");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal");
        while game.poll_completed().expect("completion").is_some() {}
        let (target, _) = game
            .cloud_ground_product()
            .expect("registered GPU provider");
        fill(&device, &queue, &target, 0.0);
        let clear = game.capture_pixels().expect("clear submitted scene");
        fill(&device, &queue, &target, 0.7);
        let shade = game.capture_pixels().expect("GPU shaded scene");
        let mut changed = 0;
        let mut lit = 0;
        for (a, b) in clear
            .as_chunks::<4>()
            .0
            .iter()
            .zip(shade.as_chunks::<4>().0)
        {
            if a[..3] != b[..3] {
                changed += 1;
                lit += usize::from(b[..3].iter().map(|&v| usize::from(v)).sum::<usize>() > 3);
            }
        }
        assert!(
            changed > 100,
            "GPU field changes actual direct-lit receivers"
        );
        assert_eq!(lit, changed, "ambient-lit receivers never become black");
        fill(&device, &queue, &target, 0.0);
        assert_eq!(clear, game.capture_pixels().expect("clear recovery"));
    });
}
