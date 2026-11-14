use super::super::*;

#[test]
fn taau_shaders_validate() {
    for source in [
        include_str!("../taau.wgsl"),
        include_str!("../reference.wgsl"),
    ] {
        let module = wgpu::naga::front::wgsl::parse_str(source).expect("WGSL");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("valid temporal shader");
    }
}

#[test]
fn taau_prepares_and_resets_on_real_metal() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let mut scene = SyntheticScene::grid(4).unwrap();
        scene.view = scene
            .view
            .with_jitter_sequence(0, [64, 64])
            .with_reconstruction(true);
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .unwrap();
        let mut targets: Vec<_> = (0..2)
            .map(|_| OffscreenTarget::new(&device, &prepared, 64, 64).unwrap())
            .collect();
        let (first, second) = targets.split_at_mut(1);
        second[0].share_reconstruction_history(&prepared, &first[0]);
        for i in 0..4 {
            let target = &mut targets[i % 2];
            scene.view = scene.view.with_history_reset(i == 3);
            prepared
                .update_frame(
                    &queue,
                    target,
                    &scene,
                    FrameLighting::Diagnostic(SyntheticLight::default()),
                    1.0,
                )
                .unwrap();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared.encode(&mut encoder, 41, target, None).unwrap();
            queue.submit([encoder.finish()]);
            prepared.commit_frame();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        }
        assert!(prepared.history_identity()[1] > 0);
    });
}
