//! Open a test device using this build of the renderer's recipe.

use crate::capabilities::{DeviceProfile, DeviceRecipe, DeviceRestriction, OpenedDevice};
use thrum_test_support::Gpu;

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
