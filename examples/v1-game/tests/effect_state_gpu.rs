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
        let record = Box::pin(v1_game::effect::qualification::run(&device, &queue, &caps))
            .await
            .expect("exact target state script");
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m2");
        std::fs::create_dir_all(&output).expect("evidence directory");
        std::fs::write(output.join("state-record.txt"), record).expect("target record");
    });
}
