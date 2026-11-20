//! Open a test device using this build of the renderer's recipe.

use crate::capabilities::{
    DeviceCapabilities, DeviceProfile, DeviceRecipe, DeviceRestriction, OpenedDevice,
};
use thrum_test_support::Gpu;

/// The gate's admission and this build's device, from [`open`].
pub(crate) struct Opened {
    /// The admitted instance and adapter.
    pub(crate) gpu: Gpu,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) capabilities: DeviceCapabilities,
}

/// Ask the test kit's gate for a GPU, on `backend` where given, and open this build's recipe for `profile` with
/// `restriction`, as generation 1. None means the gate reported a skip.
///
/// The kit's own call returns another build's renderer types, so the crate's unit tests use this one.
pub(crate) fn open(
    test: &str,
    backend: Option<wgpu::Backend>,
    profile: DeviceProfile,
    restriction: &DeviceRestriction,
) -> Option<Opened> {
    let gpu = thrum_test_support::gpu(test, backend)?;
    let OpenedDevice {
        device,
        queue,
        capabilities,
        ..
    } = pollster::block_on(
        DeviceRecipe {
            profile,
            generation: 1,
            restriction: Some(restriction),
            ..Default::default()
        }
        .open_admitted(&gpu.instance, &gpu.adapter),
    )
    .expect("admitted GPU test device");
    Some(Opened {
        gpu,
        device,
        queue,
        capabilities,
    })
}

pub(crate) async fn portable(
    gpu: &Gpu,
    features: Option<wgpu::Features>,
) -> (wgpu::Device, wgpu::Queue) {
    let OpenedDevice { device, queue, .. } = DeviceRecipe {
        profile: DeviceProfile::Portable,
        restriction: Some(&DeviceRestriction {
            features,
            ..Default::default()
        }),
        ..Default::default()
    }
    .open_admitted(&gpu.instance, &gpu.adapter)
    .await
    .expect("admitted GPU test device");
    (device, queue)
}
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use crate::capabilities::apple::storage_mode as metal_storage_mode;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) fn metal_storage_mode(_: &wgpu::Texture) -> Option<&'static str> {
    unreachable!("the test must admit Metal before querying a texture's storage mode")
}

#[cfg(test)]
mod tests {
    use crate::capabilities::{DeviceProfile, DeviceRestriction};

    #[test]
    fn open_admits_and_opens_this_builds_restricted_recipe() {
        let backend = cfg!(target_os = "macos").then_some(wgpu::Backend::Metal);
        let restriction = DeviceRestriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        };
        let Some(opened) = super::open(
            "open_admits_and_opens_this_builds_restricted_recipe",
            backend,
            DeviceProfile::Portable,
            &restriction,
        ) else {
            return;
        };
        if let Some(backend) = backend {
            assert_eq!(opened.gpu.adapter.get_info().backend, backend);
        }
        assert_eq!(opened.device.features(), wgpu::Features::empty());
        assert_eq!(opened.capabilities.features(), wgpu::Features::empty());
        assert_eq!(opened.capabilities.generation(), 1);
        let submission = opened.queue.submit([]);
        opened
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("the opened queue completes a submission");
        assert_eq!(
            opened.device.limits(),
            DeviceProfile::Portable.limits(&opened.gpu.adapter)
        );
    }
}
