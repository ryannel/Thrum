use std::path::Path;

pub(super) fn run(record_name: &str) {
    pollster::block_on(async {
        let recipes: &[&str] = if cfg!(target_os = "macos") {
            &["portable", "apple-tile"]
        } else {
            &["portable"]
        };
        let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m2");
        std::fs::create_dir_all(&output).expect("evidence directory");
        let mut records = Vec::new();
        for recipe in recipes {
            let instance =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("real target adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .expect("real target device");
            let caps =
                thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1)
                    .with_recipe_override(recipe)
                    .expect("target recipe");
            let record = Box::pin(v1_game::effect::qualification::run(
                &device, &queue, &caps, &instance,
            ))
            .await
            .expect("every-texel state, cadence and recovery script");
            assert!(
                record.contains("cadence-run:14:exact"),
                "runtime cadence boundary was exercised"
            );
            std::fs::write(output.join(format!("{record_name}-{recipe}.txt")), &record)
                .expect("recipe record");
            records.push(format!("{recipe}: {record}"));
        }
        std::fs::write(
            output.join(format!("{record_name}-record.txt")),
            records.join("\n"),
        )
        .expect("target record");
    });
}
