//! Complete cooked-valley preparation residency with both recipe generations alive.
#[cfg(target_os = "macos")]
fn main() -> Result<(), String> {
    pollster::block_on(run())
}

#[cfg(not(target_os = "macos"))]
fn main() -> Result<(), String> {
    Err("this diagnostic requires macOS Metal".to_owned())
}

#[cfg(target_os = "macos")]
async fn run() -> Result<(), String> {
    use thrum_render::runtime::FrameConfig;
    use thrum_render::{
        capabilities::{DeviceCapabilities, DeviceRecipe, OpenedDevice},
        present::SurfaceProfile,
    };

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let OpenedDevice {
        adapter,
        device,
        queue,
        capabilities,
        ..
    } = DeviceRecipe::default()
        .open(&instance, None)
        .await
        .map_err(|error| error.to_string())?;
    let width = std::env::var("APPLE_TILE_RESIDENCY_WIDTH")
        .unwrap_or_else(|_| "1280".to_owned())
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    let height = std::env::var("APPLE_TILE_RESIDENCY_HEIGHT")
        .unwrap_or_else(|_| "720".to_owned())
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    let extent = [width, height];
    let config = FrameConfig {
        render_extent: extent,
        surface: Some(SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent,
        }),
    };
    let portable = capabilities
        .with_recipe_override("portable")
        .map_err(|error| format!("{error:?}"))?;
    let mut old = v1_game::selected_setup()?
        .prepare(&device, &queue, &portable, config)
        .await?;
    v1_game::attach_cooked_scene(&mut old)?
        .ok_or("the probe needs THRUM_SCENE=valley and its cooked store")?;
    queue.submit([]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| error.to_string())?;
    let old_bytes = portable.resident_device_bytes();
    let apple = DeviceCapabilities::capture(&adapter, &device, 2)
        .with_recipe_override("apple-tile")
        .map_err(|error| format!("{error:?}"))?;
    let mut new = v1_game::selected_setup()?
        .prepare(&device, &queue, &apple, config)
        .await?;
    v1_game::attach_cooked_scene(&mut new)?.ok_or("missing cooked contribution")?;
    queue.submit([]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| error.to_string())?;
    let overlap_bytes = apple.resident_device_bytes();
    let old_storage = old.storage_report();
    drop(old);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| error.to_string())?;
    let selected_bytes = apple.resident_device_bytes();
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "target":"mac-metal", "adapter":format!("{:?}", adapter.get_info()),
            "extent":config.render_extent, "cooked_store":std::env::var("THRUM_ASSET_STORE").ok(),
            "portable_registry":portable.registry_digest(), "apple_registry":apple.registry_digest(),
            "portable_driver_bytes":old_bytes, "retained_both_driver_bytes":overlap_bytes,
            "apple_after_old_retired_driver_bytes":selected_bytes,
            "portable_storage":old_storage, "apple_storage":new.storage_report(),
            "scope":"complete prepared cooked hosts, two slots each, stars and proof overlay; capture observer OFF; queued uploads completed before each measurement; no image-history advance or presentation"
        })
    ).map_err(|error| error.to_string())?;
    Ok(())
}
