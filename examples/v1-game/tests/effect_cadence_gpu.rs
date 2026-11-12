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
            &device, &queue, &caps, &instance,
        ))
        .await
        .expect("consumer readback and cadence script");
        assert!(
            record.contains("cadence-run:14:exact"),
            "runtime cadence boundary was exercised"
        );
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m2");
        std::fs::create_dir_all(&output).expect("directory");
        std::fs::write(output.join("cadence-record.txt"), record).expect("target record");
    });
}
