//! Keep the WebGPU event loop free while the fixtures wait for actual GPU work.
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use std::time::Instant;
#[cfg(target_arch = "wasm32")]
pub(super) use web_time::Instant;

#[cfg(target_arch = "wasm32")]
mod browser {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = setTimeout)]
        pub(super) fn timeout(callback: &js_sys::Function, ms: u32) -> i32;
    }
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::unused_async,
        reason = "native uses the same fixture suspension points"
    )
)]
pub(crate) async fn pause() {
    #[cfg(target_arch = "wasm32")]
    {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            browser::timeout(&resolve, 1);
        });
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .expect("browser timer");
    }
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(Duration::from_millis(1));
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn copy_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &wgpu::Buffer,
) -> Vec<u8> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("view proof readback"),
        size: source.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(source, 0, &buffer, 0, source.size());
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        tx.send(result).expect("map caller");
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        device.poll(wgpu::PollType::Poll).expect("poll map");
        match rx.try_recv() {
            Ok(result) => {
                result.expect("map");
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                assert!(Instant::now() < deadline, "map timed out");
                pause().await;
            }
            Err(error) => panic!("map callback: {error}"),
        }
    }
    buffer.get_mapped_range(..).expect("mapped bytes").to_vec()
}
