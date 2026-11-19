//! Read-only native inspection, scoped to retained wgpu handles.
#[cfg(target_os = "macos")]
use objc2_metal::MTLTexture;
use objc2_metal::{MTLDevice, MTLResource};

/// Read the actual Metal texture's dimensions while the wgpu lease is held.
/// A failed surface configure can leave wgpu's descriptor out of agreement.
#[cfg(target_os = "macos")]
#[must_use]
#[expect(
    unsafe_code,
    reason = "Read-only size through a retained Metal texture guard"
)]
pub fn drawable_extent(texture: &wgpu::Texture) -> Option<[u32; 2]> {
    // SAFETY: The guard retains the texture for the two synchronous property
    // reads. No native handle escapes or changes ownership.
    let native = unsafe { texture.as_hal::<wgpu::hal::api::Metal>() }?;
    let raw = native.raw_handle();
    Some([raw.width().try_into().ok()?, raw.height().try_into().ok()?])
}

#[expect(
    unsafe_code,
    reason = "Read-only Metal facts through a retained wgpu adapter guard; no native mutation or ownership transfer"
)]
pub(super) fn unified_memory(adapter: &wgpu::Adapter) -> Option<bool> {
    // SAFETY: The guard retains the adapter. This only reads its device property,
    // and does not release, mutate, record commands or transfer the native object.
    let native = unsafe { adapter.as_hal::<wgpu::hal::api::Metal>() }?;
    Some(native.raw_device().hasUnifiedMemory())
}

#[expect(
    unsafe_code,
    reason = "Read-only Metal storage mode through a retained wgpu texture guard"
)]
pub(crate) fn storage_mode(texture: &wgpu::Texture) -> Option<&'static str> {
    // SAFETY: The guard retains the live texture for this synchronous property
    // read. No native handle escapes and there is no mutation or command encoding.
    let native = unsafe { texture.as_hal::<wgpu::hal::api::Metal>() }?;
    let mode = native.raw_handle().storageMode();
    Some(match mode {
        objc2_metal::MTLStorageMode::Memoryless => "Memoryless",
        objc2_metal::MTLStorageMode::Private => "Private",
        objc2_metal::MTLStorageMode::Shared => "Shared",
        _ => "Other",
    })
}

#[expect(
    unsafe_code,
    reason = "Read-only complete Metal device residency through a retained wgpu guard for explicit diagnostics"
)]
pub(super) fn resident_bytes(device: &wgpu::Device) -> Option<u64> {
    // SAFETY: The guard retains the device for a synchronous counter read.
    // No native resource is mutated, released or used for command encoding.
    let native = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }?;
    u64::try_from(native.raw_device().currentAllocatedSize()).ok()
}
