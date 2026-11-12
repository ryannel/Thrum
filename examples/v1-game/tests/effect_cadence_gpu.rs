//! Cadence changes use the same compiled graph and prebuilt pipelines.
#![cfg(not(target_arch = "wasm32"))]
#[test]
fn both_history_consumers_follow_runtime_cadence_and_discard() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("target adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("target device");
        let caps = thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
        let record = Box::pin(v1_game::effect::qualification::run(
            &device, &queue, &caps, &adapter,
        ))
        .await
        .expect("consumer readback and cadence script");
        assert!(
            record.contains("cadence-run:14:exact"),
            "runtime cadence boundary was exercised"
        );
        std::fs::create_dir_all("tmp/effect-graph-m2").expect("directory");
        std::fs::write("tmp/effect-graph-m2/cadence-record.txt", record).expect("target record");
    });
}
