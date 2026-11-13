//! Fetched-feed qualification on the browser's real WebGPU device.
use super::{LentDiagnostic, LentWitness};
use crate::{FrameConfig, FrameHost};
use std::{
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};
use thrum_asset::{BrowserFetch, DevClient, Fetch, Ktx2, RemoteGeneration};
use thrum_asset_gpu::{CookedAssets, CookedConfig, LeaseRequest, RemoteFeed};
use thrum_render::{capabilities::DeviceCapabilities, contribution::SceneContributions};

async fn complete(queue: &wgpu::Queue) {
    let state = Arc::new(Mutex::new((false, None::<std::task::Waker>)));
    let callback = Arc::clone(&state);
    queue.on_submitted_work_done(move || {
        let mut state = callback.lock().expect("GPU completion");
        state.0 = true;
        if let Some(waker) = state.1.take() {
            waker.wake();
        }
    });
    std::future::poll_fn(|cx| {
        let mut state = state.lock().expect("GPU completion");
        if state.0 {
            Poll::Ready(())
        } else {
            state.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    })
    .await;
}

async fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, String> {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("lent readback"),
        size: buffer.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &readback, 0, buffer.size());
    queue.submit([encoder.finish()]);
    let state = Arc::new(Mutex::new((None, None::<std::task::Waker>)));
    let callback = Arc::clone(&state);
    readback.map_async(wgpu::MapMode::Read, .., move |result| {
        let mut state = callback.lock().expect("map");
        state.0 = Some(result);
        if let Some(waker) = state.1.take() {
            waker.wake();
        }
    });
    std::future::poll_fn(|cx| {
        let mut state = state.lock().expect("map");
        if let Some(result) = state.0.take() {
            Poll::Ready(result)
        } else {
            state.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    })
    .await
    .map_err(|error| error.to_string())?;
    let bytes = readback
        .get_mapped_range(..)
        .map_err(|error| error.to_string())?
        .to_vec();
    readback.unmap();
    Ok(bytes)
}

async fn verify(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    witness: &Mutex<LentWitness>,
    references: &[Ktx2],
) -> Result<u64, String> {
    let (set, outputs) = {
        let witness = witness.lock().map_err(|error| error.to_string())?;
        (
            witness.offered.clone().ok_or("no offered set")?,
            witness.outputs.clone(),
        )
    };
    let mut checked = 0;
    for ((product, output), reference) in set.products.iter().zip(&outputs).zip(references) {
        let actual = read(device, queue, output).await?;
        let mut offset = 0;
        for level in reference.levels() {
            let bytes = reference.level_data(level);
            let length = bytes.len() * if reference.is_3d() { 4 } else { 1 };
            let actual = &actual[offset..offset + length];
            let equal = if reference.is_3d() {
                actual
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(bytes)
                    .all(|(a, b)| a[0] == *b)
            } else {
                actual == bytes
            };
            if !equal {
                return Err(format!("{}: mip texels differ", product.name));
            }
            checked += bytes.len() as u64;
            offset += length;
        }
    }
    Ok(checked)
}

fn references(generation: &RemoteGeneration) -> Result<Vec<Ktx2>, String> {
    (0..13)
        .map(|index| {
            let name = if index == 12 {
                "foam.linear".into()
            } else {
                format!("cloud{index}")
            };
            let entry = generation
                .resolve(&name, &[thrum_asset::Profile::Portable])
                .ok_or("reference name missing")?;
            Ktx2::open(
                generation
                    .product(&entry.product)
                    .ok_or("reference bytes missing")?,
            )
            .map_err(|error| error.to_string())
        })
        .collect()
}

/// Run first load, one-volume edit and device replacement against a generated fetched store.
pub(crate) async fn run(origin: String) -> Result<String, String> {
    let mut client = DevClient::new(BrowserFetch, &origin).map_err(|error| error.to_string())?;
    let first = client
        .poll()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("no first generation")?;
    let first_references = references(&first)?;
    let feed = RemoteFeed::new(first);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|error| error.to_string())?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|error| error.to_string())?;
    let caps = DeviceCapabilities::capture(&adapter, &device, 1);
    let mut host =
        FrameHost::new_with_capabilities(&device, &queue, &caps, FrameConfig::default()).await?;
    let mut config = CookedConfig::new(thrum_asset_gpu::CookedSource::Remote(feed.clone()), vec![]);
    config.leased = (0..12)
        .map(|index| LeaseRequest {
            name: format!("cloud{index}"),
            dimension: wgpu::TextureDimension::D3,
        })
        .collect();
    config.leased.push(LeaseRequest {
        name: "foam.linear".into(),
        dimension: wgpu::TextureDimension::D2,
    });
    let assets = CookedAssets::new(config).map_err(|error| error.to_string())?;
    let lease = assets.lease();
    let stats = assets.stats();
    let (consumer, witness) = LentDiagnostic::new(lease.clone());
    host.set_scene_contribution(Box::new(SceneContributions::new(vec![
        Box::new(consumer),
        Box::new(assets),
    ])))?;
    let mut clock = Duration::ZERO;
    for _ in 0..1000 {
        clock += Duration::from_millis(16);
        host.drive(clock, None)?;
        complete(&queue).await;
        host.poll_completed()?;
        if stats.lock().map_err(|error| error.to_string())?.drawn == Some(1) {
            break;
        }
    }
    let initial = lease.offered().ok_or("first load never offered")?;
    let checked_first = verify(&device, &queue, &witness, &first_references).await?;
    BrowserFetch
        .get(&format!("{origin}/advance"))
        .await
        .map_err(|error| error.to_string())?;
    let edited = client
        .poll()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("no edit")?;
    let edited_references = references(&edited)?;
    feed.publish(edited);
    for _ in 0..1000 {
        clock += Duration::from_millis(16);
        host.drive(clock, None)?;
        complete(&queue).await;
        host.poll_completed()?;
        if stats.lock().map_err(|error| error.to_string())?.drawn == Some(2) {
            break;
        }
    }
    let changed = lease.offered().ok_or("no edited set")?;
    let count = initial
        .products
        .iter()
        .zip(&changed.products)
        .filter(|(a, b)| a.hash != b.hash)
        .count();
    if count != 1 {
        return Err(format!("edit changed {count} products"));
    }
    let checked_edit = verify(&device, &queue, &witness, &edited_references).await?;
    let (replacement, replacement_queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|error| error.to_string())?;
    let caps = DeviceCapabilities::capture(&adapter, &replacement, 2);
    let mut candidate = host
        .device_prepare_inputs()
        .map_err(|error| error.to_string())?
        .prepare(&caps, &replacement_queue, None)
        .await
        .map_err(|error| error.to_string())?;
    device.destroy();
    host.publish_device(&mut candidate)
        .map_err(|error| error.to_string())?;
    if lease.offered().is_some() {
        return Err("lease retained old device".into());
    }
    for _ in 0..1000 {
        clock += Duration::from_millis(16);
        host.drive(clock, None)?;
        complete(&replacement_queue).await;
        host.poll_completed()?;
        if stats
            .lock()
            .map_err(|error| error.to_string())?
            .drawn_device_generation
            == Some(2)
        {
            break;
        }
    }
    let checked_recovery = verify(
        &replacement,
        &replacement_queue,
        &witness,
        &edited_references,
    )
    .await?;
    Ok(format!(
        "{{\"products\":13,\"changed\":1,\"device_generation\":2,\"checked_bytes\":[{checked_first},{checked_edit},{checked_recovery}]}}"
    ))
}
