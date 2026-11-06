use super::DeviceWatch;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn public_destroy_delivers_immutable_generation_without_a_window_or_callback_cycle() {
    pollster::block_on(async {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(descriptor);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, _queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let (send, receive) = mpsc::channel();
        let watch = DeviceWatch::register(&device, 19, move |generation| {
            let _ = send.send(generation);
        });
        assert!(!watch.is_lost());
        device.destroy();
        // Destroy marks invalid first. Maintenance, rather than a synchronous
        // callback assumption, delivers the public loss notification.
        let deadline = Instant::now() + Duration::from_secs(2);
        while !watch.is_lost() {
            device.poll(wgpu::PollType::Poll).unwrap();
            assert!(
                Instant::now() < deadline,
                "destroy notification was not delivered"
            );
            std::thread::yield_now();
        }
        assert_eq!(receive.recv_timeout(Duration::from_secs(1)).unwrap(), 19);
        assert_eq!(watch.generation, 19);
    });
}
