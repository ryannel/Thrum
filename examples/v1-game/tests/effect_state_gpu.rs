//! EFFECT-GRAPH M2 target script, using the external game's public diagnostic.
#![cfg(not(target_arch = "wasm32"))]
#[test]
fn effect_state_script_matches_literal_table() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real target adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("real target device");
        let caps = thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
        let record = Box::pin(v1_game::effect::qualification::run(
            &device, &queue, &caps, &adapter,
        ))
        .await
        .expect("exact target state script");
        std::fs::create_dir_all("tmp/effect-graph-m2").expect("evidence directory");
        std::fs::write("tmp/effect-graph-m2/state-record.txt", record).expect("target record");
    });
}
