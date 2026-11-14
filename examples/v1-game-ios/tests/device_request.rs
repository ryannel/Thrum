//! Exercise the same request the iPhone shell calls, on this Mac's Metal GPU.

#[test]
#[cfg(target_os = "macos")]
fn iphone_request_enables_native_content_and_limits() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let (adapter, device, _) =
        v1_game_ios::device::request_device(&instance).expect("iPhone request");
    assert!(adapter.get_info().name.contains("Apple"));
    let content =
        thrum_render::capabilities::DeviceCapabilities::requested_content_features(&adapter);
    assert!(device.features().contains(content));
    assert_eq!(device.limits(), adapter.limits());
    assert!(device.limits().max_storage_buffers_per_shader_stage >= 8);
    assert_eq!(
        thrum_asset_gpu::texture_profiles(device.features())[0].name(),
        "apple-astc"
    );
    eprintln!(
        "adapter={:?}; profiles={:?}; limits={:?}",
        adapter.get_info(),
        thrum_asset_gpu::texture_profiles(device.features()),
        device.limits()
    );
}
