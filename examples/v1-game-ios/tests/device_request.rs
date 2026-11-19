//! Exercise the same request the iPhone shell calls, on this Mac's Metal GPU.

use v1_game_ios::device::request_device;

#[test]
fn iphone_request_enables_native_content_and_limits() {
    let Some(gpu) = thrum_test_support::gpu(
        "iphone_request_enables_native_content_and_limits",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let (adapter, device, _) = request_device(&gpu.instance).expect("iPhone request");
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
