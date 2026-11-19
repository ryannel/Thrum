//! Device setup shared by scene-renderer unit tests.

pub(in crate::synthetic) async fn device(
    test_gpu: &thrum_test_support::Gpu,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = test_gpu.adapter.clone();
    let (device, queue) =
        crate::test_device::portable(test_gpu, Some(wgpu::Features::empty())).await;
    (adapter, device, queue)
}
